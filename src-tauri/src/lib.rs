pub mod commands;
pub mod db;
pub mod models;
mod repo;
mod seed;

use db::Db;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let conn = db::open(&dir.join("snipdeck.db"))?;
            app.manage(Db(Mutex::new(conn)));
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
            commands::list_categories,
        ])
        .run(tauri::generate_context!())
        .expect("error while running SnipDeck");
}
