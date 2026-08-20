use tauri::{AppHandle, Manager, PhysicalPosition, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

/// Window roles. Every window of SnipDeck belongs to exactly one role; the
/// role is the window label, so `main` / `launcher` / `dock` stay the single
/// source of truth for the frontend to know which UI to render.
pub mod roles {
    /// Full management UI (current main window).
    pub const MAIN: &str = "main";
    /// Quick launcher window — created hidden at startup, shown/hidden at
    /// runtime, never destroyed.
    pub const LAUNCHER: &str = "launcher";
    /// Desktop edge dock (later phase) — not created yet.
    pub const DOCK: &str = "dock";
}

/// Launcher window geometry. The initial height is small; the frontend
/// adjusts it to the result count.
fn launcher_config() -> (f64, f64) {
    (680.0, 96.0)
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

/// Position the launcher on the monitor under the cursor (fallback: current
/// monitor), horizontally centered, vertically ~22% from the top. All
/// coordinates are physical pixels so mixed-DPI setups stay correct.
pub fn position_launcher<R: Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    let cursor = window
        .cursor_position()
        .unwrap_or(PhysicalPosition::new(0.0, 0.0));
    let monitor = window
        .monitor_from_point(cursor.x, cursor.y)
        .ok()
        .flatten()
        .or_else(|| window.current_monitor().ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten());

    if let Some(monitor) = monitor {
        let mpos = *monitor.position();
        let msize = *monitor.size();
        let wsize = window.inner_size()?;
        let x = mpos.x + ((msize.width as i64 - wsize.width as i64) / 2) as i32;
        let y = mpos.y + ((msize.height as f64) * 0.22) as i32;
        window.set_position(PhysicalPosition::new(x, y))?;
    }
    Ok(())
}

/// Position + show + focus the launcher. The window is created once at
/// startup and NEVER destroyed at runtime.
pub fn show_launcher_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let window = get(app, roles::LAUNCHER)
        .ok_or_else(|| "launcher window not initialized".to_string())?;
    position_launcher(&window).map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn hide_launcher_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let window = get(app, roles::LAUNCHER)
        .ok_or_else(|| "launcher window not initialized".to_string())?;
    window.hide().map_err(|e| e.to_string())?;
    Ok(())
}

/// Global-shortcut behavior: visible -> hide, hidden -> show.
pub fn toggle_launcher_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let visible = get(app, roles::LAUNCHER)
        .map(|window| window.is_visible().unwrap_or(false))
        .unwrap_or(false);
    if visible {
        hide_launcher_window(app)
    } else {
        show_launcher_window(app)
    }
}
