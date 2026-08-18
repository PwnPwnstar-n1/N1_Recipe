use crate::db::Db;
use crate::models::{CategoryInfo, CopyResult, ListView, Snippet, SnippetInput};
use crate::repo::Repo;
use crate::settings::{self, Settings};
use std::collections::HashMap;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

type CmdResult<T> = Result<T, String>;

fn map_err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn validate(input: &SnippetInput) -> CmdResult<()> {
    if input.title.trim().is_empty() {
        return Err("title is required".into());
    }
    Ok(())
}

/// List snippets for a sidebar view. `view` is one of
/// all | favorites | recent | most_used | category (requires `category`).
#[tauri::command]
pub fn list_snippets(
    db: State<'_, Db>,
    view: String,
    category: Option<String>,
) -> CmdResult<Vec<Snippet>> {
    let view =
        ListView::parse(&view, category).ok_or_else(|| format!("unknown view: {view}"))?;
    let conn = db.0.lock().map_err(map_err)?;
    Repo::new(&conn).list(&view).map_err(map_err)
}

/// Full-text-ish search over title / aliases / tags / category / content.
#[tauri::command]
pub fn search_snippets(db: State<'_, Db>, query: String) -> CmdResult<Vec<Snippet>> {
    let conn = db.0.lock().map_err(map_err)?;
    Repo::new(&conn).search(&query).map_err(map_err)
}

#[tauri::command]
pub fn get_snippet(db: State<'_, Db>, id: i64) -> CmdResult<Snippet> {
    let conn = db.0.lock().map_err(map_err)?;
    Repo::new(&conn).get(id).map_err(map_err)
}

#[tauri::command]
pub fn create_snippet(db: State<'_, Db>, input: SnippetInput) -> CmdResult<Snippet> {
    validate(&input)?;
    let conn = db.0.lock().map_err(map_err)?;
    Repo::new(&conn).insert(&input).map_err(map_err)
}

#[tauri::command]
pub fn update_snippet(db: State<'_, Db>, id: i64, input: SnippetInput) -> CmdResult<Snippet> {
    validate(&input)?;
    let conn = db.0.lock().map_err(map_err)?;
    Repo::new(&conn).update(id, &input).map_err(map_err)
}

#[tauri::command]
pub fn delete_snippet(db: State<'_, Db>, id: i64) -> CmdResult<()> {
    let conn = db.0.lock().map_err(map_err)?;
    Repo::new(&conn).delete(id).map_err(map_err)
}

#[tauri::command]
pub fn toggle_favorite(db: State<'_, Db>, id: i64) -> CmdResult<Snippet> {
    let conn = db.0.lock().map_err(map_err)?;
    Repo::new(&conn).toggle_favorite(id).map_err(map_err)
}

/// Copy the snippet content to the system clipboard, then bump usage_count
/// and last_used_at. Template variables are rendered with `values` (system
/// variables are resolved at copy time). usage_count is only incremented
/// after the rendered text was actually written to the clipboard.
/// Generic over the runtime so the mock-runtime IPC tests can drive it.
#[tauri::command]
pub fn copy_snippet<R: tauri::Runtime>(
    app: AppHandle<R>,
    db: State<'_, Db>,
    id: i64,
    values: HashMap<String, String>,
) -> CmdResult<CopyResult> {
    let conn = db.0.lock().map_err(map_err)?;
    let repo = Repo::new(&conn);
    let mut snippet = repo.get(id).map_err(map_err)?;

    // Reject before writing anything: every ordinary variable must have a value.
    let missing = crate::template::missing_variables(&snippet.content, &values);
    if !missing.is_empty() {
        return Err(format!("missing variable values: {}", missing.join(", ")));
    }

    let text = crate::template::render_with_system(&app, &snippet.content, &values);
    app.clipboard().write_text(text.clone()).map_err(map_err)?;
    repo.record_usage(id).map_err(map_err)?;
    snippet.usage_count += 1;
    snippet.last_used_at = Some(crate::models::now_ms());
    Ok(CopyResult { text, snippet })
}

/// Ordered ordinary template variables of a snippet — the frontend builds the
/// variable dialog from this list. System variables are excluded.
#[tauri::command]
pub fn list_variables(db: State<'_, Db>, id: i64) -> CmdResult<Vec<String>> {
    let conn = db.0.lock().map_err(map_err)?;
    let snippet = Repo::new(&conn).get(id).map_err(map_err)?;
    Ok(crate::template::extract_variables(&snippet.content))
}

#[tauri::command]
pub fn list_categories(db: State<'_, Db>) -> CmdResult<Vec<CategoryInfo>> {
    let conn = db.0.lock().map_err(map_err)?;
    Repo::new(&conn).categories().map_err(map_err)
}

fn settings_path<R: tauri::Runtime>(app: &AppHandle<R>) -> CmdResult<std::path::PathBuf> {
    let dir = app.path().app_data_dir().map_err(map_err)?;
    Ok(dir.join("settings.json"))
}

#[tauri::command]
pub fn get_settings<R: tauri::Runtime>(app: AppHandle<R>) -> CmdResult<Settings> {
    settings::load(&settings_path(&app)?)
}

/// Persist the UI language. The frontend applies it to vue-i18n
/// immediately, so the switch takes effect without a restart.
#[tauri::command]
pub fn set_language<R: tauri::Runtime>(
    app: AppHandle<R>,
    language: String,
) -> CmdResult<Settings> {
    settings::validate_language(&language)?;
    let path = settings_path(&app)?;
    let mut current = settings::load(&path)?;
    current.language = Some(language);
    settings::save(&path, &current)?;
    Ok(current)
}

// ---------------------------------------------------------------------------
// Window lifecycle (multi-window foundation). The launcher window is a
// hidden prototype used to validate the create/show/focus/hide/close
// lifecycle; the real launcher UI arrives in Phase 3.
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn show_launcher<R: tauri::Runtime>(app: AppHandle<R>) -> CmdResult<()> {
    let window = crate::windows::get(&app, crate::windows::roles::LAUNCHER)
        .ok_or_else(|| "launcher window not initialized".to_string())?;
    window.show().map_err(map_err)?;
    window.set_focus().map_err(map_err)?;
    Ok(())
}

#[tauri::command]
pub fn hide_launcher<R: tauri::Runtime>(app: AppHandle<R>) -> CmdResult<()> {
    let window = crate::windows::get(&app, crate::windows::roles::LAUNCHER)
        .ok_or_else(|| "launcher window not initialized".to_string())?;
    window.hide().map_err(map_err)?;
    Ok(())
}
