//! Windows system tray: menu, i18n sync, close-to-tray, and explicit quit.
//!
//! The tray icon is created exactly ONCE at startup (id `main-tray`); on
//! language / dock-state changes the menu is *rebuilt* in place via
//! [`TrayIcon::set_menu`] — never by creating a second tray icon.

use crate::settings;
use crate::windows;
use std::sync::atomic::Ordering;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};

pub const TRAY_ID: &str = "main-tray";

// Menu item ids (stable contract — the frontend/harness and tests depend on
// these exact strings).
pub mod menu_id {
    pub const OPEN: &str = "open";
    pub const LAUNCHER: &str = "launcher";
    pub const DOCK_TOGGLE: &str = "dock_toggle";
    pub const QUIT: &str = "quit";
}

/// Localized menu labels for the current language.
fn labels(
    lang: &str,
) -> (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
) {
    if lang == "zh-CN" {
        (
            "打开 N1_Recipe",
            "打开快速启动器",
            "显示桌面 Dock",
            "隐藏桌面 Dock",
            "退出 N1_Recipe",
        )
    } else {
        (
            "Open N1_Recipe",
            "Open Quick Launcher",
            "Show Desktop Dock",
            "Hide Desktop Dock",
            "Quit N1_Recipe",
        )
    }
}

fn build_menu<R: Runtime>(
    app: &AppHandle<R>,
    lang: &str,
    dock_enabled: bool,
) -> tauri::Result<Menu<R>> {
    let (open, launcher, show_dock, hide_dock, quit) = labels(lang);

    let open_i = MenuItem::with_id(app, menu_id::OPEN, open, true, None::<&str>)?;
    let launcher_i = MenuItem::with_id(app, menu_id::LAUNCHER, launcher, true, None::<&str>)?;
    let dock_label = if dock_enabled { hide_dock } else { show_dock };
    let dock_i = MenuItem::with_id(app, menu_id::DOCK_TOGGLE, dock_label, true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(app, menu_id::QUIT, quit, true, None::<&str>)?;

    Menu::with_items(app, &[&open_i, &launcher_i, &dock_i, &separator, &quit_i])
}

/// Toggle the desktop dock (same persist + apply semantics as the Settings
/// panel) and refresh the tray menu so the Show/Hide label stays correct.
fn toggle_dock<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let path = settings::app_settings_path(app)?;
    let mut current = settings::load(&path)?;
    current.dock.enabled = !current.dock.enabled;
    settings::save(&path, &current)?;
    crate::commands::apply_dock_state(app, &current.dock)?;
    rebuild_tray_menu(app)?;
    Ok(())
}

fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    match id {
        menu_id::OPEN => {
            if let Err(error) = windows::show_main_window(app) {
                crate::diagnostics::log(app, &format!("tray main show error={error}"));
            }
        }
        menu_id::LAUNCHER => {
            crate::diagnostics::log(app, "tray quick window requested");
            if let Err(error) = windows::show_launcher_window(app) {
                crate::diagnostics::log(app, &format!("tray quick window show error={error}"));
            }
        }
        menu_id::DOCK_TOGGLE => {
            if let Err(error) = toggle_dock(app) {
                crate::diagnostics::log(app, &format!("tray dock toggle error={error}"));
            }
        }
        menu_id::QUIT => {
            // Set the quit flag FIRST so the close-to-tray handler lets every
            // window close instead of re-hiding the main window.
            if let Some(quitting) = app.try_state::<crate::QuitState>() {
                quitting.0.store(true, Ordering::SeqCst);
            }
            app.exit(0);
        }
        _ => {}
    }
}

/// Create the (single) tray icon and its menu. Called once during app setup.
pub fn create_tray<R: Runtime>(
    app: &AppHandle<R>,
    lang: &str,
    dock_enabled: bool,
) -> tauri::Result<()> {
    let menu = build_menu(app, lang, dock_enabled)?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("N1_Recipe")
        .menu(&menu)
        // Left click restores the main window (handled below); right click
        // still opens the context menu (Windows convention).
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| handle_menu_event(app, event.id().as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Err(error) = windows::show_main_window(tray.app_handle()) {
                    crate::diagnostics::log(
                        tray.app_handle(),
                        &format!("tray click main show error={error}"),
                    );
                }
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    builder.build(app)?;
    Ok(())
}

/// Rebuild the tray menu from the current persisted language + dock state.
/// Safe to call after any settings change; reuses the existing tray icon.
pub fn rebuild_tray_menu<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let current = settings::load(&settings::app_settings_path(app)?).unwrap_or_default();
    let lang = current.language.as_deref().unwrap_or("en-US");
    let menu = build_menu(app, lang, current.dock.enabled).map_err(|e| e.to_string())?;
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(menu)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Reconcile the OS autostart registration with the persisted flag.
/// `settings.json` is the single source of truth: it wins on every change and
/// at startup (see `setup`). `enable`/`disable` are idempotent registry
/// operations.
pub fn reconcile_autostart<R: Runtime>(
    app: &AppHandle<R>,
    launch_on_boot: bool,
) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let manager = app.autolaunch();
    if launch_on_boot {
        manager.enable().map_err(|e| e.to_string())
    } else {
        manager.disable().map_err(|e| e.to_string())
    }
}

/// Whether the current process was launched by the Windows autostart entry
/// (the autostart plugin registers the exe with a `--autostart` argument).
pub fn started_via_autostart() -> bool {
    std::env::args().any(|a| a == "--autostart")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The menu-id contract: the runtime harness and the frontend depend on
    /// these exact ids to dispatch tray actions. Pin them so a refactor can't
    /// silently break the tray.
    #[test]
    fn menu_id_contract() {
        assert_eq!(menu_id::OPEN, "open");
        assert_eq!(menu_id::LAUNCHER, "launcher");
        assert_eq!(menu_id::DOCK_TOGGLE, "dock_toggle");
        assert_eq!(menu_id::QUIT, "quit");
    }

    /// Localized labels: zh-CN, en-US, and fallback to English.
    #[test]
    fn localized_labels() {
        let zh = labels("zh-CN");
        assert_eq!(zh.0, "打开 N1_Recipe");
        assert_eq!(zh.1, "打开快速启动器");
        assert_eq!(zh.2, "显示桌面 Dock");
        assert_eq!(zh.3, "隐藏桌面 Dock");
        assert_eq!(zh.4, "退出 N1_Recipe");

        let en = labels("en-US");
        assert_eq!(en.0, "Open N1_Recipe");
        assert_eq!(en.1, "Open Quick Launcher");
        assert_eq!(en.2, "Show Desktop Dock");
        assert_eq!(en.3, "Hide Desktop Dock");
        assert_eq!(en.4, "Quit N1_Recipe");

        // Any non-zh language falls back to English.
        let fr = labels("fr-FR");
        assert_eq!(fr.0, "Open N1_Recipe");
    }
}
