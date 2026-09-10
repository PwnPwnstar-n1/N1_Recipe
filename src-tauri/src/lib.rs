pub mod backup;
pub mod commands;
pub mod db;
pub mod diagnostics;
pub mod models;
pub mod recipe_pack;
mod repo;
mod seed;
pub mod settings;
pub mod shortcut;
pub mod template;
pub mod tray;
pub mod windows;

use db::Db;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::Manager;

/// Explicit-quit flag. When false, closing the main window hides it (close
/// to tray); when true (tray "Quit"), every window closes for real and the
/// process exits.
pub struct QuitState(pub AtomicBool);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        // The `--autostart` argument is added to the Windows Run entry so we
        // can tell an autostart launch from a manual one (start-hidden).
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        // Second launch -> notify the running instance and restore its main
        // window; the second process exits before opening the database.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Err(error) = windows::show_main_window(app) {
                diagnostics::log(app, &format!("main restore error={error}"));
            }
        }))
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let conn = db::open(&dir.join("snipdeck.db"))?;
            app.manage(Db(Mutex::new(conn)));
            app.manage(Mutex::new(shortcut::ShortcutManager::default()));
            app.manage(QuitState(AtomicBool::new(false)));

            let settings =
                settings::load(&settings::app_settings_path(app.handle())?).unwrap_or_default();
            diagnostics::log(
                app.handle(),
                &format!(
                    "app startup version={} mode={}",
                    env!("CARGO_PKG_VERSION"),
                    if tray::started_via_autostart() {
                        "autostart"
                    } else {
                        "manual"
                    }
                ),
            );

            // Windows are created ONCE and only show/hide at runtime (never
            // destroyed — see src/windows.rs for the wry deadlock rationale).
            windows::ensure_hidden_launcher(app.handle())?;
            windows::ensure_hidden_dock(app.handle())?;

            // Repair a poisoned/off-screen native main geometry before any
            // startup visibility decision. This does not touch settings or DB.
            if let Some(window) = windows::get(app.handle(), windows::roles::MAIN) {
                let _ = windows::repair_main_window(&window);
            }

            // Apply persisted dock geometry/visibility.
            if settings.dock.enabled {
                if let Some(window) = windows::get(app.handle(), windows::roles::DOCK) {
                    let _ = windows::apply_dock_geometry(&window, &settings.dock, false);
                    let _ = window.show();
                }
            }

            // System tray (single icon; the menu is rebuilt in place on
            // language / dock-state changes — never a second icon).
            tray::create_tray(
                app.handle(),
                settings.language.as_deref().unwrap_or("en-US"),
                settings.dock.enabled,
            )?;

            // Global shortcut lifecycle: register once at startup from the
            // persisted settings. Registration failure must NOT prevent the
            // app from starting — the launcher stays usable via the tray and
            // the Settings "Open Launcher" button.
            if settings.launcher.enabled {
                match shortcut::register_shortcut(app.handle(), &settings.launcher.shortcut) {
                    Ok(()) => {
                        if let Err(error) = shortcut::set_registered_shortcut(
                            app.handle(),
                            Some(settings.launcher.shortcut.clone()),
                        ) {
                            diagnostics::log(
                                app.handle(),
                                &format!("shortcut state error={error}"),
                            );
                        }
                    }
                    Err(e) => {
                        diagnostics::log(app.handle(), &format!("shortcut register error={e}"));
                    }
                }
            }

            // Autostart reconciliation: settings.json is the single source of
            // truth, so the OS registration is re-synced on every launch.
            let _ = tray::reconcile_autostart(app.handle(), settings.startup.launch_on_boot);

            // Start hidden: only when launched by the Windows autostart entry
            // AND the user enabled it. Manual launches always show main.
            if tray::started_via_autostart() && settings.startup.start_hidden {
                if let Some(window) = windows::get(app.handle(), windows::roles::MAIN) {
                    let _ = window.hide();
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                let quitting = window
                    .app_handle()
                    .state::<QuitState>()
                    .0
                    .load(Ordering::SeqCst);
                if windows::close_disposition(window.label(), quitting)
                    == windows::CloseDisposition::Hide
                {
                    api.prevent_close();
                    diagnostics::log(
                        window.app_handle(),
                        &format!(
                            "window close requested label={} action=hide",
                            window.label()
                        ),
                    );
                    if let Err(error) = window.hide() {
                        diagnostics::log(
                            window.app_handle(),
                            &format!("window hide error label={} error={error}", window.label()),
                        );
                    }
                }
            }
            tauri::WindowEvent::Destroyed => diagnostics::log(
                window.app_handle(),
                &format!("window destroyed label={}", window.label()),
            ),
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_snippets,
            commands::search_snippets,
            commands::get_snippet,
            commands::create_snippet,
            commands::update_snippet,
            commands::delete_snippet,
            commands::toggle_favorite,
            commands::toggle_dock_pin,
            commands::copy_snippet,
            commands::list_variables,
            commands::list_categories,
            commands::create_category,
            commands::rename_category,
            commands::delete_category,
            commands::list_scratch_items,
            commands::create_scratch_item,
            commands::update_scratch_item,
            commands::delete_scratch_item,
            commands::clear_scratch_items,
            commands::copy_text,
            commands::list_text_variables,
            commands::copy_template_text,
            commands::get_settings,
            commands::set_language,
            commands::update_launcher_settings,
            commands::show_launcher,
            commands::hide_launcher,
            commands::apply_dock_settings,
            commands::expand_dock,
            commands::collapse_dock,
            commands::update_startup_settings,
            commands::export_snippets,
            commands::import_snippets,
            commands::preview_recipe_pack,
            commands::import_recipe_pack,
            commands::export_recipe_pack,
        ])
        .run(tauri::generate_context!())
        .expect("error while running N1_Recipe");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The quit flag is the single switch between "close = hide to tray" and
    /// "close = actually exit". It must start false so a normal app lifetime
    /// never re-hides windows during a real quit.
    #[test]
    fn quit_state_starts_false_and_flips() {
        let q = QuitState(AtomicBool::new(false));
        assert!(!q.0.load(Ordering::SeqCst));
        q.0.store(true, Ordering::SeqCst);
        assert!(q.0.load(Ordering::SeqCst));
    }
}
