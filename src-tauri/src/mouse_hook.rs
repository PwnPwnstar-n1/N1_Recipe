//! Minimal Windows XButton listener. The low-level callback only checks four
//! atomics and queues a byte; all logging and window work happens elsewhere.

use crate::settings::MouseBindingSettings;
use crate::shortcut::RegistrationState;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};
use std::sync::{mpsc, Mutex};
use tauri::{AppHandle, Runtime};

const CTRL: u8 = 1;
const ALT: u8 = 2;
const SHIFT: u8 = 4;
const WIN: u8 = 8;

static BUTTON: AtomicU8 = AtomicU8::new(0);
static MODIFIERS: AtomicU8 = AtomicU8::new(0);
static CONSUME: AtomicBool = AtomicBool::new(false);
static STARTED: AtomicBool = AtomicBool::new(false);
static THREAD_ID: AtomicU32 = AtomicU32::new(0);
static LAST_MATCH_MS: AtomicU32 = AtomicU32::new(0);
static EVENT_TX: Mutex<Option<mpsc::Sender<u8>>> = Mutex::new(None);

fn modifier_mask(value: &str) -> u8 {
    value.split('+').fold(0, |mask, token| {
        mask | match token {
            "Ctrl" => CTRL,
            "Alt" => ALT,
            "Shift" => SHIFT,
            "Win" => WIN,
            _ => 0,
        }
    })
}

fn description(settings: &MouseBindingSettings) -> String {
    if settings.modifiers.is_empty() {
        settings.button.clone()
    } else {
        format!("{}+{}", settings.modifiers, settings.button)
    }
}

pub fn configure<R: Runtime>(app: &AppHandle<R>, settings: &MouseBindingSettings) {
    LAST_MATCH_MS.store(0, Ordering::Release);
    BUTTON.store(
        if settings.enabled {
            if settings.button == "XButton2" {
                2
            } else {
                1
            }
        } else {
            0
        },
        Ordering::Release,
    );
    MODIFIERS.store(modifier_mask(&settings.modifiers), Ordering::Release);
    CONSUME.store(settings.consume, Ordering::Release);
    crate::shortcut::set_mouse_diagnostic(
        app,
        settings.enabled.then(|| description(settings)),
        if settings.enabled {
            RegistrationState::Registered
        } else {
            RegistrationState::Disabled
        },
        None,
    );
    crate::diagnostics::log(
        app,
        &format!(
            "event=mouse_binding_config enabled={} consume={} button={}",
            settings.enabled, settings.consume, settings.button
        ),
    );
}

#[cfg(windows)]
pub fn start(app: AppHandle) -> Result<(), String> {
    if STARTED.swap(true, Ordering::AcqRel) {
        return Ok(());
    }
    let (tx, rx) = mpsc::channel::<u8>();
    *EVENT_TX.lock().map_err(|e| e.to_string())? = Some(tx);
    let worker_app = app.clone();
    std::thread::Builder::new()
        .name("n1-xbutton-actions".into())
        .spawn(move || {
            while let Ok(button) = rx.recv() {
                // In preserve mode the original XButton event is still being
                // dispatched when the hook callback returns. Let it finish
                // before focusing Launcher, otherwise the original target can
                // immediately steal focus back and auto-hide Launcher.
                std::thread::sleep(std::time::Duration::from_millis(35));
                crate::shortcut::activate_launcher(
                    &worker_app,
                    if button == 1 {
                        "mouse:XButton1"
                    } else {
                        "mouse:XButton2"
                    },
                    true,
                );
            }
        })
        .map_err(|e| e.to_string())?;

    std::thread::Builder::new()
        .name("n1-xbutton-hook".into())
        .spawn(move || unsafe {
            use windows_sys::Win32::Foundation::HINSTANCE;
            use windows_sys::Win32::System::Threading::GetCurrentThreadId;
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
                UnhookWindowsHookEx, MSG, WH_MOUSE_LL,
            };
            THREAD_ID.store(GetCurrentThreadId(), Ordering::Release);
            let hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), 0 as HINSTANCE, 0);
            if hook.is_null() {
                let error = std::io::Error::last_os_error().to_string();
                crate::shortcut::set_mouse_diagnostic(
                    &app,
                    None,
                    RegistrationState::RegistrationFailed,
                    Some(error.clone()),
                );
                crate::diagnostics::log(
                    &app,
                    &format!("event=mouse_hook_install result=error error={error}"),
                );
                STARTED.store(false, Ordering::Release);
                return;
            }
            crate::diagnostics::log(&app, "event=mouse_hook_install result=ok");
            let mut message: MSG = std::mem::zeroed();
            while GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            UnhookWindowsHookEx(hook);
            crate::diagnostics::log(&app, "event=mouse_hook_uninstall result=ok");
            STARTED.store(false, Ordering::Release);
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(windows)]
unsafe extern "system" fn mouse_proc(code: i32, wparam: usize, lparam: isize) -> isize {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_LMENU, VK_LWIN, VK_RMENU, VK_RWIN, VK_SHIFT,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, MSLLHOOKSTRUCT, WM_XBUTTONDOWN,
    };
    if code >= 0 && wparam as u32 == WM_XBUTTONDOWN {
        let info = &*(lparam as *const MSLLHOOKSTRUCT);
        let button = ((info.mouseData >> 16) & 0xffff) as u8;
        if button == BUTTON.load(Ordering::Acquire) && button != 0 {
            let down = |key: u16| (GetAsyncKeyState(key as i32) as u16 & 0x8000) != 0;
            let mut actual = 0;
            if down(VK_CONTROL) {
                actual |= CTRL;
            }
            if down(VK_LMENU) || down(VK_RMENU) {
                actual |= ALT;
            }
            if down(VK_SHIFT) {
                actual |= SHIFT;
            }
            if down(VK_LWIN) || down(VK_RWIN) {
                actual |= WIN;
            }
            if actual == MODIFIERS.load(Ordering::Acquire) {
                // Some mouse software/input injection paths duplicate an
                // XButton-down message. Collapse only an identical matched
                // binding inside 250 ms so one press cannot toggle twice.
                let previous = LAST_MATCH_MS.swap(info.time, Ordering::AcqRel);
                if previous == 0 || info.time.wrapping_sub(previous) >= 250 {
                    if let Ok(sender) = EVENT_TX.lock() {
                        if let Some(sender) = sender.as_ref() {
                            let _ = sender.send(button);
                        }
                    }
                }
                if CONSUME.load(Ordering::Acquire) {
                    return 1;
                }
            }
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}

#[cfg(windows)]
pub fn shutdown() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT};
    BUTTON.store(0, Ordering::Release);
    let id = THREAD_ID.load(Ordering::Acquire);
    if id != 0 {
        unsafe {
            PostThreadMessageW(id, WM_QUIT, 0, 0);
        }
    }
    if let Ok(mut sender) = EVENT_TX.lock() {
        *sender = None;
    }
}

#[cfg(not(windows))]
pub fn start(_app: AppHandle) -> Result<(), String> {
    Err("mouse XButton bindings are supported only on Windows".into())
}
#[cfg(not(windows))]
pub fn shutdown() {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mouse_modifier_mask_is_exact() {
        assert_eq!(modifier_mask(""), 0);
        assert_eq!(modifier_mask("Ctrl+Shift"), CTRL | SHIFT);
        assert_eq!(modifier_mask("Alt+Win"), ALT | WIN);
    }
}
