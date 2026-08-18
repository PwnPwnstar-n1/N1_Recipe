use crate::db::Db;
use crate::models::{CategoryInfo, CopyResult, ListView, Snippet, SnippetInput};
use crate::repo::Repo;
use tauri::{AppHandle, State};
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
/// and last_used_at. Returns the final text plus the updated snippet.
/// Generic over the runtime so the mock-runtime IPC tests can drive it.
#[tauri::command]
pub fn copy_snippet<R: tauri::Runtime>(
    app: AppHandle<R>,
    db: State<'_, Db>,
    id: i64,
) -> CmdResult<CopyResult> {
    let conn = db.0.lock().map_err(map_err)?;
    let repo = Repo::new(&conn);
    let mut snippet = repo.get(id).map_err(map_err)?;
    app.clipboard()
        .write_text(snippet.content.clone())
        .map_err(map_err)?;
    repo.record_usage(id).map_err(map_err)?;
    snippet.usage_count += 1;
    snippet.last_used_at = Some(crate::models::now_ms());
    Ok(CopyResult {
        text: snippet.content.clone(),
        snippet,
    })
}

#[tauri::command]
pub fn list_categories(db: State<'_, Db>) -> CmdResult<Vec<CategoryInfo>> {
    let conn = db.0.lock().map_err(map_err)?;
    Repo::new(&conn).categories().map_err(map_err)
}
