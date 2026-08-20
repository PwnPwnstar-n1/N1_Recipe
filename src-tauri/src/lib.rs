pub mod commands;
pub mod db;
pub mod models;
mod repo;
mod seed;
pub mod settings;
pub mod shortcut;
pub mod template;
pub mod windows;

use db::Db;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let conn = db::open(&dir.join("snipdeck.db"))?;
            app.manage(Db(Mutex::new(conn)));
            app.manage(Mutex::new(shortcut::ShortcutManager::default()));

            // Multi-window foundation: the launcher window exists (hidden)
            // from startup; runtime only shows/hides it.
            windows::ensure_hidden_launcher(app.handle())?;

            // Global shortcut lifecycle: register once at startup from the
            // persisted settings. Registration failure must NOT prevent the
            // app from starting — the launcher stays usable via the
            // Settings "Open Launcher" button.
            let settings = settings::load(&settings::app_settings_path(app.handle())?)
                .unwrap_or_default();
            if settings.launcher.enabled {
                match shortcut::register_shortcut(app.handle(), &settings.launcher.shortcut) {
                    Ok(()) => {
                        let _ = shortcut::set_registered_shortcut(
                            app.handle(),
                            Some(settings.launcher.shortcut.clone()),
                        );
                    }
                    Err(e) => {
                        eprintln!("SnipDeck: failed to register launcher shortcut: {e}");
                    }
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_snippets,
            commands::search_snippets,
            commands::get_snippet,
            commands::create_snippet,
            commands::update_snippet,
            commands::delete_snippet,
            commands::toggle_favorite,
            commands::copy_snippet,
            commands::list_variables,
            commands::list_categories,
            commands::get_settings,
            commands::set_language,
            commands::update_launcher_settings,
            commands::show_launcher,
            commands::hide_launcher,
        ])
        .run(tauri::generate_context!())
        .expect("error while running SnipDeck");
}
