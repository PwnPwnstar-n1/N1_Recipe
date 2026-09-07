use crate::db::Db;
use crate::models::{CategoryInfo, CopyResult, ListView, ScratchItem, Snippet, SnippetInput};
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
    if input.kind != "TEXT" && input.kind != "COMMAND_GROUP" {
        return Err("无效的 Snippet 类型".into());
    }
    if input.kind == "COMMAND_GROUP" && input.commands.iter().all(|c| c.trim().is_empty()) {
        return Err("命令组至少需要一条命令".into());
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
    let view = ListView::parse(&view, category).ok_or_else(|| format!("unknown view: {view}"))?;
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
#[tauri::command]
pub fn toggle_dock_pin(db: State<'_, Db>, id: i64) -> CmdResult<Snippet> {
    let conn = db.0.lock().map_err(map_err)?;
    Repo::new(&conn).toggle_dock_pin(id).map_err(map_err)
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
    let source = if snippet.kind == "COMMAND_GROUP" {
        snippet.commands.join("\n")
    } else {
        snippet.content.clone()
    };

    // Reject before writing anything: every ordinary variable must have a value.
    let missing = crate::template::missing_variables(&source, &values);
    if !missing.is_empty() {
        return Err(format!("missing variable values: {}", missing.join(", ")));
    }

    let text = crate::template::render_with_system(&app, &source, &values);
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
    let source = if snippet.kind == "COMMAND_GROUP" {
        snippet.commands.join("\n")
    } else {
        snippet.content
    };
    Ok(crate::template::extract_variables(&source))
}

#[tauri::command]
pub fn list_categories(db: State<'_, Db>) -> CmdResult<Vec<CategoryInfo>> {
    let conn = db.0.lock().map_err(map_err)?;
    Repo::new(&conn).categories().map_err(map_err)
}

fn validate_category(name: &str) -> CmdResult<&str> {
    let name = name.trim();
    if name.is_empty() {
        return Err("分类名称不能为空".into());
    }
    if name.chars().count() > 80 {
        return Err("分类名称不能超过 80 个字符".into());
    }
    Ok(name)
}

#[tauri::command]
pub fn create_category(db: State<'_, Db>, name: String) -> CmdResult<()> {
    let name = validate_category(&name)?;
    let conn = db.0.lock().map_err(map_err)?;
    Repo::new(&conn).create_category(name).map_err(|e| {
        if e.to_string().contains("UNIQUE") {
            "分类名称已存在".into()
        } else {
            map_err(e)
        }
    })
}
#[tauri::command]
pub fn rename_category(db: State<'_, Db>, old_name: String, new_name: String) -> CmdResult<()> {
    let new_name = validate_category(&new_name)?;
    let conn = db.0.lock().map_err(map_err)?;
    Repo::new(&conn)
        .rename_category(&old_name, new_name)
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                "分类名称已存在".into()
            } else {
                map_err(e)
            }
        })
}
#[tauri::command]
pub fn delete_category(db: State<'_, Db>, name: String) -> CmdResult<()> {
    let conn = db.0.lock().map_err(map_err)?;
    Repo::new(&conn).delete_category(&name).map_err(map_err)
}

#[tauri::command]
pub fn list_scratch_items(db: State<'_, Db>) -> CmdResult<Vec<ScratchItem>> {
    let conn = db.0.lock().map_err(map_err)?;
    Repo::new(&conn).scratch_list().map_err(map_err)
}
#[tauri::command]
pub fn create_scratch_item(db: State<'_, Db>, content: String) -> CmdResult<ScratchItem> {
    if content.trim().is_empty() {
        return Err("临时内容不能为空".into());
    }
    let conn = db.0.lock().map_err(map_err)?;
    Repo::new(&conn).scratch_create(&content).map_err(map_err)
}
#[tauri::command]
pub fn update_scratch_item(db: State<'_, Db>, id: i64, content: String) -> CmdResult<()> {
    if content.trim().is_empty() {
        return Err("临时内容不能为空".into());
    }
    let conn = db.0.lock().map_err(map_err)?;
    Repo::new(&conn)
        .scratch_update(id, &content)
        .map_err(map_err)
}
#[tauri::command]
pub fn delete_scratch_item(db: State<'_, Db>, id: i64) -> CmdResult<()> {
    let conn = db.0.lock().map_err(map_err)?;
    Repo::new(&conn).scratch_delete(id).map_err(map_err)
}
#[tauri::command]
pub fn clear_scratch_items(db: State<'_, Db>) -> CmdResult<()> {
    let conn = db.0.lock().map_err(map_err)?;
    Repo::new(&conn).scratch_clear().map_err(map_err)
}

#[tauri::command]
pub fn copy_text<R: tauri::Runtime>(app: AppHandle<R>, text: String) -> CmdResult<()> {
    app.clipboard().write_text(text).map_err(map_err)
}

#[tauri::command]
pub fn list_text_variables(text: String) -> Vec<String> {
    crate::template::extract_variables(&text)
}

#[tauri::command]
pub fn copy_template_text<R: tauri::Runtime>(
    app: AppHandle<R>,
    db: State<'_, Db>,
    text: String,
    values: HashMap<String, String>,
    snippet_id: Option<i64>,
) -> CmdResult<String> {
    let missing = crate::template::missing_variables(&text, &values);
    if !missing.is_empty() {
        return Err(format!("缺少变量值：{}", missing.join(", ")));
    }
    let rendered = crate::template::render_with_system(&app, &text, &values);
    app.clipboard()
        .write_text(rendered.clone())
        .map_err(map_err)?;
    if let Some(id) = snippet_id {
        let conn = db.0.lock().map_err(map_err)?;
        Repo::new(&conn).record_usage(id).map_err(map_err)?;
    }
    Ok(rendered)
}

#[tauri::command]
pub fn get_settings<R: tauri::Runtime>(app: AppHandle<R>) -> CmdResult<Settings> {
    settings::load(&settings::app_settings_path(&app)?)
}

/// Persist the UI language. The frontend applies it to vue-i18n
/// immediately, so the switch takes effect without a restart.
#[tauri::command]
pub fn set_language<R: tauri::Runtime>(app: AppHandle<R>, language: String) -> CmdResult<Settings> {
    settings::validate_language(&language)?;
    let path = settings::app_settings_path(&app)?;
    let mut current = settings::load(&path)?;
    current.language = Some(language);
    settings::save(&path, &current)?;
    // Keep the tray menu labels in sync with the UI language.
    let _ = crate::tray::rebuild_tray_menu(&app);
    Ok(current)
}

/// Persist startup behavior and reconcile the OS autostart registration.
/// `settings.json` is the single source of truth; on a registration failure
/// the persisted flag is reverted so the UI never disagrees with the OS.
#[tauri::command]
pub fn update_startup_settings<R: tauri::Runtime>(
    app: AppHandle<R>,
    startup: crate::settings::StartupSettings,
) -> CmdResult<Settings> {
    let path = settings::app_settings_path(&app)?;
    let mut current = settings::load(&path)?;
    let old = current.startup.clone();
    current.startup = startup.clone();
    settings::save(&path, &current)?;

    if let Err(e) = crate::tray::reconcile_autostart(&app, startup.launch_on_boot) {
        current.startup = old;
        let _ = settings::save(&path, &current);
        return Err(format!("autostart failed: {e}"));
    }
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

/// Apply dock visibility + geometry + always-on-top to the dock window and
/// broadcast the resulting state. Shared by the settings command and the tray
/// dock toggle; never destroys the dock window.
pub fn apply_dock_state<R: tauri::Runtime>(
    app: &AppHandle<R>,
    dock: &crate::settings::DockSettings,
) -> CmdResult<()> {
    let window = crate::windows::get(app, crate::windows::roles::DOCK)
        .ok_or_else(|| "dock window not initialized".to_string())?;
    window
        .set_always_on_top(dock.always_on_top)
        .map_err(map_err)?;
    crate::windows::apply_dock_geometry(&window, dock, false).map_err(map_err)?;

    if dock.enabled {
        window.show().map_err(map_err)?;
        app.emit("dock:settings", dock).map_err(map_err)?;
        app.emit("dock:state", "collapsed").map_err(map_err)?;
    } else {
        window.hide().map_err(map_err)?;
        app.emit("dock:settings", dock).map_err(map_err)?;
        app.emit("dock:state", "hidden").map_err(map_err)?;
    }
    Ok(())
}

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

    apply_dock_state(&app, &dock)?;
    // The tray's Show/Hide Dock label tracks dock.enabled.
    let _ = crate::tray::rebuild_tray_menu(&app);
    Ok(current)
}

#[tauri::command]
pub fn expand_dock<R: tauri::Runtime>(app: AppHandle<R>) -> CmdResult<()> {
    let window = crate::windows::get(&app, crate::windows::roles::DOCK)
        .ok_or_else(|| "dock window not initialized".to_string())?;
    debug_assert_eq!(window.label(), crate::windows::roles::DOCK);
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
    debug_assert_eq!(window.label(), crate::windows::roles::DOCK);
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

// ---------------------------------------------------------------------------
// JSON backup / restore (Phase 5A)
// ---------------------------------------------------------------------------

/// Export the whole snippet library to a pretty-printed JSON file. The
/// database is only read — a failed export never mutates data.
#[tauri::command]
pub fn export_snippets(db: State<'_, Db>, path: String) -> CmdResult<u32> {
    let conn = db.0.lock().map_err(map_err)?;
    let repo = Repo::new(&conn);
    let snippets = repo.list(&ListView::All).map_err(map_err)?;

    let backup = crate::backup::BackupFile {
        format: crate::backup::FORMAT.into(),
        version: crate::backup::CURRENT_VERSION,
        exported_at: chrono::Local::now().to_rfc3339(),
        snippets: snippets
            .into_iter()
            .map(|s| crate::backup::BackupSnippet {
                title: s.title,
                content: s.content,
                r#type: s.r#type.as_str().to_string(),
                category: s.category,
                language: s.language,
                tags: s.tags,
                aliases: s.aliases,
                favorite: s.favorite,
                usage_count: s.usage_count,
                created_at: s.created_at,
                updated_at: s.updated_at,
                kind: s.kind,
                commands: s.commands,
            })
            .collect(),
    };

    let count = backup.snippets.len() as u32;
    let json = crate::backup::to_json_pretty(&backup)?;
    std::fs::write(&path, json).map_err(|e| format!("export failed: {e}"))?;
    Ok(count)
}

/// Parse + validate a backup file, then merge it into the library in a
/// single transaction. Validation happens entirely before any database
/// write; any error leaves the database unchanged.
#[tauri::command]
pub fn import_snippets(db: State<'_, Db>, path: String) -> CmdResult<crate::backup::ImportSummary> {
    let json = std::fs::read_to_string(&path).map_err(|e| format!("import failed: {e}"))?;
    let backup = crate::backup::parse_and_validate(&json)?;
    let conn = db.0.lock().map_err(map_err)?;
    Repo::new(&conn).import_backup(&backup).map_err(map_err)
}

fn read_pack(path: &str) -> CmdResult<crate::recipe_pack::Pack> {
    let meta = std::fs::metadata(path).map_err(|_| "无法读取配方包文件".to_string())?;
    if meta.len() > crate::recipe_pack::MAX_FILE_SIZE {
        return Err("配方包超过 10 MB 限制".into());
    }
    let raw =
        std::fs::read_to_string(path).map_err(|_| "配方包必须是有效的 UTF-8 文件".to_string())?;
    crate::recipe_pack::parse(&raw)
}
#[tauri::command]
pub fn preview_recipe_pack(
    db: State<'_, Db>,
    path: String,
) -> CmdResult<crate::recipe_pack::Preview> {
    let pack = read_pack(&path)?;
    let conn = db.0.lock().map_err(map_err)?;
    crate::recipe_pack::preview(&conn, &pack)
}
#[tauri::command]
pub fn import_recipe_pack(
    db: State<'_, Db>,
    path: String,
    strategy: String,
) -> CmdResult<crate::recipe_pack::ImportResult> {
    let pack = read_pack(&path)?;
    let conn = db.0.lock().map_err(map_err)?;
    crate::recipe_pack::import(&conn, &pack, &strategy)
}
#[tauri::command]
pub fn export_recipe_pack(
    db: State<'_, Db>,
    path: String,
    package: crate::recipe_pack::PackageMeta,
    category: Option<String>,
    ids: Vec<i64>,
) -> CmdResult<u32> {
    let conn = db.0.lock().map_err(map_err)?;
    let pack = crate::recipe_pack::export(&conn, package, category.as_deref(), &ids)?;
    let count = pack.recipes.len() as u32;
    std::fs::write(path, serde_json::to_string_pretty(&pack).map_err(map_err)?)
        .map_err(|_| "无法写入配方包文件".to_string())?;
    Ok(count)
}
