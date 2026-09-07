use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, Runtime, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder,
};

/// Window roles. Every window of SnipDeck belongs to exactly one role; the
/// role is the window label, so `main` / `launcher` / `dock` stay the single
/// source of truth for the frontend to know which UI to render.
pub mod roles {
    /// Full management UI (current main window).
    pub const MAIN: &str = "main";
    /// Quick launcher window — created hidden at startup, shown/hidden at
    /// runtime, never destroyed.
    pub const LAUNCHER: &str = "launcher";
    /// Desktop edge dock — created hidden at startup, shown/hidden and
    /// resized/repositioned at runtime, never destroyed.
    pub const DOCK: &str = "dock";
}

/// Launcher window geometry. The initial height is small; the frontend
/// adjusts it to the result count.
fn launcher_config() -> (f64, f64) {
    (720.0, 520.0)
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
        .title("N1_Recipe")
        .inner_size(width, height)
        .min_inner_size(420.0, 260.0)
        .max_inner_size(1400.0, 1000.0)
        .resizable(true)
        .decorations(true)
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

/// Create the (hidden) dock window. Called ONLY during app setup (main
/// thread); at runtime the dock is shown/hidden/resized/repositioned but
/// NEVER destroyed (same wry deadlock constraint as the launcher).
pub fn create_dock<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<WebviewWindow<R>> {
    WebviewWindowBuilder::new(app, roles::DOCK, WebviewUrl::App("index.html".into()))
        .title("N1_Recipe")
        .inner_size(360.0, 600.0)
        .resizable(false)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .build()
}

/// Ensure the dock window exists (hidden) — called once at startup.
pub fn ensure_hidden_dock<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if get(app, roles::DOCK).is_none() {
        create_dock(app)?;
    }
    Ok(())
}

/// Whether the dock's current logical width already matches the expanded
/// width. Used to make geometry commands idempotent so bursts of
/// enter/leave events cannot pile up a command backlog that executes after
/// the user's intent has moved on.
pub fn dock_matches_width<R: Runtime>(
    window: &WebviewWindow<R>,
    target_logical: f64,
) -> tauri::Result<bool> {
    let scale = window.scale_factor().unwrap_or(1.0).max(0.1);
    let size = window.inner_size()?;
    let logical = size.width as f64 / scale;
    Ok((logical - target_logical).abs() < 1.0)
}

/// Anchor the dock to the left/right edge of the PRIMARY monitor's work
/// area (respects the taskbar) and set its size to the collapsed or
/// expanded width with full work-area height. All calculations are done in
/// logical pixels via the monitor's scale factor, so mixed-DPI setups stay
/// correct. The window is borderless, so inner size == outer size.
pub fn apply_dock_geometry<R: Runtime>(
    window: &WebviewWindow<R>,
    dock: &crate::settings::DockSettings,
    expanded: bool,
) -> tauri::Result<()> {
    let monitor = window
        .primary_monitor()
        .ok()
        .flatten()
        .or_else(|| window.current_monitor().ok().flatten());

    if let Some(monitor) = monitor {
        let scale = monitor.scale_factor();
        let work = monitor.work_area();
        let work_x = work.position.x as f64 / scale;
        let work_y = work.position.y as f64 / scale;
        let work_w = work.size.width as f64 / scale;
        let work_h = work.size.height as f64 / scale;

        let width = if expanded {
            dock.expanded_width as f64
        } else {
            dock.collapsed_width as f64
        };
        let x = if dock.position == "left" {
            work_x
        } else {
            work_x + work_w - width
        };

        window.set_position(LogicalPosition::new(x, work_y))?;
        window.set_size(LogicalSize::new(width, work_h))?;
    }
    Ok(())
}

/// Apply the persisted dock settings to the dock window (geometry +
/// always-on-top) without changing its visibility.
pub fn apply_dock_settings<R: Runtime>(
    app: &AppHandle<R>,
    dock: &crate::settings::DockSettings,
) -> Result<(), String> {
    let window = get(app, roles::DOCK).ok_or_else(|| "dock window not initialized".to_string())?;
    window
        .set_always_on_top(dock.always_on_top)
        .map_err(|e| e.to_string())?;
    apply_dock_geometry(&window, dock, false).map_err(|e| e.to_string())?;
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

/// Single restore entry point for the MAIN window: show + unminimize +
/// focus. Shared by the tray "Open SnipDeck" action and the single-instance
/// second-launch callback so the restore behavior stays consistent. It never
/// moves/resizes the window, so the user's chosen monitor/position is
/// preserved unless the window is already off-screen.
pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let window = get(app, roles::MAIN).ok_or_else(|| "main window not initialized".to_string())?;
    window.show().map_err(|e| e.to_string())?;
    window.unminimize().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

/// Position + show + focus the launcher. The window is created once at
/// startup and NEVER destroyed at runtime.
pub fn show_launcher_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let window =
        get(app, roles::LAUNCHER).ok_or_else(|| "launcher window not initialized".to_string())?;
    // Preserve the user's adjusted size and position for the lifetime of the
    // app. Only recover to a visible monitor when it is actually off-screen.
    if window.current_monitor().ok().flatten().is_none() {
        position_launcher(&window).map_err(|e| e.to_string())?;
    }
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn hide_launcher_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let window =
        get(app, roles::LAUNCHER).ok_or_else(|| "launcher window not initialized".to_string())?;
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

#[cfg(test)]
mod tests {
    use super::roles;

    /// The window-label contract: frontend routing, capabilities and the
    /// runtime harness all depend on these exact labels.
    #[test]
    fn window_role_labels_contract() {
        assert_eq!(roles::MAIN, "main");
        assert_eq!(roles::LAUNCHER, "launcher");
        assert_eq!(roles::DOCK, "dock");
    }
}
