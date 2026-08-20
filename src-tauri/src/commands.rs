use crate::db::Db;
use crate::models::{CategoryInfo, CopyResult, ListView, Snippet, SnippetInput};
use crate::repo::Repo;
use crate::settings::{self, Settings};
use std::collections::HashMap;
use tauri::{AppHandle, Emitter, State};
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

#[tauri::command]
pub fn get_settings<R: tauri::Runtime>(app: AppHandle<R>) -> CmdResult<Settings> {
    settings::load(&settings::app_settings_path(&app)?)
}

/// Persist the UI language. The frontend applies it to vue-i18n
/// immediately, so the switch takes effect without a restart.
#[tauri::command]
pub fn set_language<R: tauri::Runtime>(
    app: AppHandle<R>,
    language: String,
) -> CmdResult<Settings> {
    settings::validate_language(&language)?;
    let path = settings::app_settings_path(&app)?;
    let mut current = settings::load(&path)?;
    current.language = Some(language);
    settings::save(&path, &current)?;
    Ok(current)
}

/// Atomically update the launcher shortcut settings:
///
/// 1. validate the accelerator format
/// 2. register the NEW shortcut (fails -> nothing changed, old stays live)
/// 3. persist settings
/// 4. unregister the OLD shortcut (best effort)
/// 5. record the registered shortcut
///
/// A failed registration never breaks the currently working shortcut and
/// never overwrites settings.json.
#[tauri::command]
pub fn update_launcher_settings<R: tauri::Runtime>(
    app: AppHandle<R>,
    launcher: settings::LauncherSettings,
) -> CmdResult<Settings> {
    settings::validate_shortcut(&launcher.shortcut)?;
    let old = crate::shortcut::registered_shortcut(&app)?;

    if launcher.enabled && old.as_deref() != Some(launcher.shortcut.as_str()) {
        crate::shortcut::register_shortcut(&app, &launcher.shortcut)?;
    }

    let path = settings::app_settings_path(&app)?;
    let mut current = settings::load(&path)?;
    current.launcher = launcher.clone();
    settings::save(&path, &current)?;

    if let Some(old) = old {
        if old != launcher.shortcut {
            crate::shortcut::unregister_shortcut(&app, &old);
        }
    }
    crate::shortcut::set_registered_shortcut(
        &app,
        launcher.enabled.then(|| launcher.shortcut.clone()),
    )?;
    Ok(current)
}

// ---------------------------------------------------------------------------
// Window lifecycle — the launcher window is created hidden at startup and
// only shown/hidden at runtime (never destroyed; see src/windows.rs).
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn show_launcher<R: tauri::Runtime>(app: AppHandle<R>) -> CmdResult<()> {
    crate::windows::show_launcher_window(&app)
}

#[tauri::command]
pub fn hide_launcher<R: tauri::Runtime>(app: AppHandle<R>) -> CmdResult<()> {
    crate::windows::hide_launcher_window(&app)
}

// ---------------------------------------------------------------------------
// Desktop edge dock — geometry lives in windows.rs; commands only express
// intent (apply / expand / collapse) and broadcast the resulting state so
// the dock's frontend can render the matching UI. The dock window is never
// destroyed at runtime.
// ---------------------------------------------------------------------------

/// Persist dock settings and apply them immediately: position + size
/// (collapsed geometry), always-on-top, and show/hide per `enabled`.
#[tauri::command]
pub fn apply_dock_settings<R: tauri::Runtime>(
    app: AppHandle<R>,
    dock: crate::settings::DockSettings,
) -> CmdResult<Settings> {
    settings::validate_dock(&dock)?;
    let path = settings::app_settings_path(&app)?;
    let mut current = settings::load(&path)?;
    current.dock = dock.clone();
    settings::save(&path, &current)?;

    let window = crate::windows::get(&app, crate::windows::roles::DOCK)
        .ok_or_else(|| "dock window not initialized".to_string())?;
    window
        .set_always_on_top(dock.always_on_top)
        .map_err(map_err)?;
    crate::windows::apply_dock_geometry(&window, &dock, false).map_err(map_err)?;

    if dock.enabled {
        window.show().map_err(map_err)?;
        app.emit("dock:settings", &dock).map_err(map_err)?;
        app.emit("dock:state", "collapsed").map_err(map_err)?;
    } else {
        window.hide().map_err(map_err)?;
        app.emit("dock:settings", &dock).map_err(map_err)?;
        app.emit("dock:state", "hidden").map_err(map_err)?;
    }
    Ok(current)
}

#[tauri::command]
pub fn expand_dock<R: tauri::Runtime>(app: AppHandle<R>) -> CmdResult<()> {
    let window = crate::windows::get(&app, crate::windows::roles::DOCK)
        .ok_or_else(|| "dock window not initialized".to_string())?;
    let settings = settings::load(&settings::app_settings_path(&app)?)?;
    // idempotent: already expanded -> nothing to do (no redundant commands,
    // no state events to race later transitions)
    if crate::windows::dock_matches_width(&window, settings.dock.expanded_width as f64)
        .map_err(map_err)?
    {
        return Ok(());
    }
    window.show().map_err(map_err)?;
    crate::windows::apply_dock_geometry(&window, &settings.dock, true).map_err(map_err)?;
    app.emit("dock:state", "expanded").map_err(map_err)?;
    Ok(())
}

#[tauri::command]
pub fn collapse_dock<R: tauri::Runtime>(app: AppHandle<R>) -> CmdResult<()> {
    let window = crate::windows::get(&app, crate::windows::roles::DOCK)
        .ok_or_else(|| "dock window not initialized".to_string())?;
    let settings = settings::load(&settings::app_settings_path(&app)?)?;
    // idempotent: already collapsed -> nothing to do
    if crate::windows::dock_matches_width(&window, settings.dock.collapsed_width as f64)
        .map_err(map_err)?
    {
        return Ok(());
    }
    crate::windows::apply_dock_geometry(&window, &settings.dock, false).map_err(map_err)?;
    app.emit("dock:state", "collapsed").map_err(map_err)?;
    Ok(())
}
