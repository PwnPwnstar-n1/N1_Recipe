use tauri::{AppHandle, Manager, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// Window roles. Every window of SnipDeck belongs to exactly one role; the
/// role is the window label, so `main` / `launcher` / `dock` stay the single
/// source of truth for the frontend to know which UI to render.
pub mod roles {
    /// Full management UI (current main window).
    pub const MAIN: &str = "main";
    /// Quick launcher window (Phase 3) — hidden prototype window for now.
    pub const LAUNCHER: &str = "launcher";
    /// Desktop edge dock (later phase) — not created yet.
    pub const DOCK: &str = "dock";
}

/// Launcher window geometry shared by startup creation and re-creation
/// after `close_launcher`.
fn launcher_config() -> (f64, f64) {
    (620.0, 420.0)
}

/// Get a window by role.
pub fn get<R: Runtime>(app: &AppHandle<R>, role: &str) -> Option<WebviewWindow<R>> {
    app.get_webview_window(role)
}

/// Create the (hidden) launcher window. Called ONLY during app setup (main
/// thread). At runtime the window is shown/hidden but NEVER destroyed and
/// re-created: creating a webview from an IPC/sync-command thread deadlocks
/// on Windows (upstream wry issue:
/// https://github.com/tauri-apps/wry — "Deadlock creating a new window on
/// the IPC handler on Windows"), so the launcher lives as long as the app.
pub fn create_launcher<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<WebviewWindow<R>> {
    let (width, height) = launcher_config();
    WebviewWindowBuilder::new(app, roles::LAUNCHER, WebviewUrl::App("index.html".into()))
        .title("SnipDeck")
        .inner_size(width, height)
        .resizable(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .center()
        .visible(false)
        .build()
}

/// Ensure the launcher window exists (hidden) — called once at startup.
pub fn ensure_hidden_launcher<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if get(app, roles::LAUNCHER).is_none() {
        create_launcher(app)?;
    }
    Ok(())
}
