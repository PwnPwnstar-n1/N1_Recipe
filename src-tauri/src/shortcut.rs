use crate::windows;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

/// Runtime state of the launcher global shortcut. `registered` is the
/// accelerator currently registered with the OS — the single source of
/// truth for atomic shortcut updates.
#[derive(Default)]
pub struct ShortcutManager {
    pub registered: Option<String>,
}

/// Register `shortcut` with the OS and bind the toggle handler. Fails with
/// an explicit error on registration problems (conflict, invalid
/// accelerator, OS failure) — callers must treat this as the "nothing
/// changed" case.
///
/// NOTE: `on_shortcut` performs the OS registration itself (it calls
/// `register_internal` with the handler), so calling `register()` first
/// would register the same hotkey twice — the second attempt fails and the
/// first registration leaks. `on_shortcut` alone is the single atomic step.
pub fn register_shortcut<R: Runtime>(app: &AppHandle<R>, shortcut: &str) -> Result<(), String> {
    crate::diagnostics::log(app, "shortcut register begin");
    let result = app
        .global_shortcut()
        .on_shortcut(shortcut, |app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                crate::diagnostics::log(app, "shortcut triggered state=pressed");
                if let Err(error) = windows::toggle_launcher_window(app) {
                    crate::diagnostics::log(app, &format!("shortcut toggle error={error}"));
                }
            }
        })
        .map_err(|e| format!("failed to register shortcut: {e}"));
    match result {
        Ok(()) => {
            crate::diagnostics::log(app, "shortcut register result=ok");
            Ok(())
        }
        Err(error) => {
            crate::diagnostics::log(
                app,
                &format!("shortcut register result=error error={error}"),
            );
            Err(error)
        }
    }
}

/// Remove a shortcut registration (best effort — never fatal).
pub fn unregister_shortcut<R: Runtime>(app: &AppHandle<R>, shortcut: &str) {
    match app.global_shortcut().unregister(shortcut) {
        Ok(()) => crate::diagnostics::log(app, "shortcut unregister result=ok"),
        Err(error) => crate::diagnostics::log(
            app,
            &format!("shortcut unregister result=error error={error}"),
        ),
    }
}

/// Read the currently registered shortcut from the managed state.
pub fn registered_shortcut<R: Runtime>(app: &AppHandle<R>) -> Result<Option<String>, String> {
    let state = app.state::<std::sync::Mutex<ShortcutManager>>();
    let guard = state.lock().map_err(|e| e.to_string())?;
    Ok(guard.registered.clone())
}

/// Record the currently registered shortcut in the managed state.
pub fn set_registered_shortcut<R: Runtime>(
    app: &AppHandle<R>,
    registered: Option<String>,
) -> Result<(), String> {
    let state = app.state::<std::sync::Mutex<ShortcutManager>>();
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    guard.registered = registered;
    Ok(())
}

#[cfg(test)]
mod tests {
    use tauri_plugin_global_shortcut::Shortcut;

    /// The WM_HOTKEY id contract: the external runtime harness posts
    /// WM_HOTKEY with these ids to the app's `global_hotkey_app` window to
    /// drive the shortcut chain at the OS-message level. If the id layout
    /// ever changes, the harness must be updated — this test pins it.
    #[test]
    fn hotkey_id_contract() {
        let f11: Shortcut = "Ctrl+Alt+F11".parse().expect("parse F11");
        let f12: Shortcut = "Ctrl+Alt+F12".parse().expect("parse F12");
        println!("F11_ID=0x{:08X} F12_ID=0x{:08X}", f11.id(), f12.id());
        // Ctrl+Alt = ALT(0x01) | CONTROL(0x08) -> upper 16 bits 0x0009
        assert_eq!(f11.id() >> 16, 0x0009);
        assert_eq!(f12.id() >> 16, 0x0009);
    }
}
