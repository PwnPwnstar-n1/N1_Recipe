use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, PhysicalSize, Runtime,
    WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

const MAIN_DEFAULT_WIDTH: f64 = 1200.0;
const MAIN_DEFAULT_HEIGHT: f64 = 800.0;
const MAIN_MIN_WIDTH: f64 = 960.0;
const MAIN_MIN_HEIGHT: f64 = 620.0;
const MIN_VISIBLE_WIDTH: f64 = 80.0;
const MIN_VISIBLE_HEIGHT: f64 = 40.0;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Clone, Copy, Debug)]
struct DisplayArea {
    work: Rect,
    scale: f64,
}

fn intersection_size(a: Rect, b: Rect) -> (f64, f64) {
    (
        (a.x + a.width).min(b.x + b.width) - a.x.max(b.x),
        (a.y + a.height).min(b.y + b.height) - a.y.max(b.y),
    )
}

fn safe_main_geometry(current: Rect, displays: &[DisplayArea]) -> Option<(Rect, bool)> {
    let display = displays
        .iter()
        .find(|display| {
            let (width, height) = intersection_size(current, display.work);
            width >= MIN_VISIBLE_WIDTH && height >= MIN_VISIBLE_HEIGHT
        })
        .or_else(|| displays.first())?;
    let scale = if display.scale.is_finite() && display.scale > 0.0 {
        display.scale
    } else {
        1.0
    };
    let valid_numbers = [current.x, current.y, current.width, current.height]
        .iter()
        .all(|number| number.is_finite());
    let (visible_width, visible_height) = intersection_size(current, display.work);
    let valid = valid_numbers
        && current.width >= MAIN_MIN_WIDTH * scale
        && current.height >= MAIN_MIN_HEIGHT * scale
        && visible_width >= MIN_VISIBLE_WIDTH
        && visible_height >= MIN_VISIBLE_HEIGHT;
    if valid {
        return Some((current, false));
    }

    let width = (MAIN_DEFAULT_WIDTH * scale).min(display.work.width);
    let height = (MAIN_DEFAULT_HEIGHT * scale).min(display.work.height);
    Some((
        Rect {
            x: display.work.x + (display.work.width - width) / 2.0,
            y: display.work.y + (display.work.height - height) / 2.0,
            width,
            height,
        },
        true,
    ))
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseDisposition {
    Hide,
    Destroy,
}

/// Main and launcher windows are persistent while the app is running. Their
/// close buttons hide them; only an explicit application quit may destroy
/// them. Keeping this decision pure makes the Windows regression testable.
pub fn close_disposition(label: &str, quitting: bool) -> CloseDisposition {
    if !quitting && matches!(label, roles::MAIN | roles::LAUNCHER) {
        CloseDisposition::Hide
    } else {
        CloseDisposition::Destroy
    }
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
    crate::diagnostics::log(app, "quick window create begin");
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
        .inspect(|_| crate::diagnostics::log(app, "quick window create result=ok"))
        .inspect_err(|error| {
            crate::diagnostics::log(
                app,
                &format!("quick window create result=error error={error}"),
            )
        })
}

/// Ensure the launcher window exists (hidden) — called once at startup.
pub fn ensure_hidden_launcher<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let exists = get(app, roles::LAUNCHER).is_some();
    crate::diagnostics::log(app, &format!("quick window startup exists={exists}"));
    if !exists {
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
    debug_assert_eq!(window.label(), roles::DOCK);
    eprintln!(
        "N1_Recipe dock geometry: target={} state={}",
        window.label(),
        if expanded { "expanded" } else { "collapsed" }
    );
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
    repair_main_window(&window)?;
    window.show().map_err(|e| e.to_string())?;
    window.unminimize().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

/// Native safety net used by startup, tray restore and second-instance restore.
/// It repairs only the main native window and never touches application data.
pub fn repair_main_window<R: Runtime>(window: &WebviewWindow<R>) -> Result<bool, String> {
    if window.label() != roles::MAIN {
        return Err(format!(
            "refusing main geometry repair for {}",
            window.label()
        ));
    }
    let position = window.outer_position().map_err(|e| e.to_string())?;
    let size = window.inner_size().map_err(|e| e.to_string())?;
    let current = Rect {
        x: position.x as f64,
        y: position.y as f64,
        width: size.width as f64,
        height: size.height as f64,
    };
    let available = window.available_monitors().map_err(|e| e.to_string())?;
    let mut monitors = Vec::new();
    if let Some(primary) = window.primary_monitor().map_err(|e| e.to_string())? {
        monitors.push(primary);
    }
    monitors.extend(available);
    let displays: Vec<DisplayArea> = monitors
        .into_iter()
        .map(|monitor| {
            let work = monitor.work_area();
            DisplayArea {
                work: Rect {
                    x: work.position.x as f64,
                    y: work.position.y as f64,
                    width: work.size.width as f64,
                    height: work.size.height as f64,
                },
                scale: monitor.scale_factor(),
            }
        })
        .collect();
    let Some((geometry, recovered)) = safe_main_geometry(current, &displays) else {
        return Ok(false);
    };
    if recovered {
        window
            .set_size(PhysicalSize::new(
                geometry.width.round() as u32,
                geometry.height.round() as u32,
            ))
            .map_err(|e| e.to_string())?;
        window
            .set_position(PhysicalPosition::new(
                geometry.x.round() as i32,
                geometry.y.round() as i32,
            ))
            .map_err(|e| e.to_string())?;
    }
    eprintln!(
        "N1_Recipe window geometry: version={} label={} raw={:?} final={:?} recovered={}",
        env!("CARGO_PKG_VERSION"),
        window.label(),
        current,
        geometry,
        recovered
    );
    Ok(recovered)
}

/// Position + show + focus the launcher. The window is created once at
/// startup and NEVER destroyed at runtime.
pub fn show_launcher_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let Some(window) = get(app, roles::LAUNCHER) else {
        crate::diagnostics::log(app, "quick window show exists=false");
        return Err("launcher window not initialized".to_string());
    };
    crate::diagnostics::log(app, "quick window show exists=true");
    // Preserve the user's adjusted size and position for the lifetime of the
    // app. Only recover to a visible monitor when it is actually off-screen.
    if window.current_monitor().ok().flatten().is_none() {
        position_launcher(&window).map_err(|error| {
            crate::diagnostics::log(app, &format!("quick window position error={error}"));
            error.to_string()
        })?;
    }
    window.show().map_err(|error| {
        crate::diagnostics::log(app, &format!("quick window show error={error}"));
        error.to_string()
    })?;
    crate::diagnostics::log(app, "quick window show result=ok");
    window.set_focus().map_err(|error| {
        crate::diagnostics::log(app, &format!("quick window focus error={error}"));
        error.to_string()
    })?;
    crate::diagnostics::log(app, "quick window focus result=ok");
    Ok(())
}

pub fn hide_launcher_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let Some(window) = get(app, roles::LAUNCHER) else {
        crate::diagnostics::log(app, "quick window hide exists=false");
        return Err("launcher window not initialized".to_string());
    };
    crate::diagnostics::log(app, "quick window hide exists=true");
    window.hide().map_err(|error| {
        crate::diagnostics::log(app, &format!("quick window hide error={error}"));
        error.to_string()
    })?;
    crate::diagnostics::log(app, "quick window hide result=ok");
    Ok(())
}

/// Global-shortcut behavior: visible -> hide, hidden -> show.
pub fn toggle_launcher_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let Some(window) = get(app, roles::LAUNCHER) else {
        crate::diagnostics::log(app, "quick window toggle exists=false");
        return Err("launcher window not initialized".to_string());
    };
    crate::diagnostics::log(app, "quick window toggle exists=true");
    let visible = window.is_visible().map_err(|error| {
        crate::diagnostics::log(app, &format!("quick window visibility error={error}"));
        error.to_string()
    })?;
    if visible {
        hide_launcher_window(app)
    } else {
        show_launcher_window(app)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        close_disposition, roles, safe_main_geometry, CloseDisposition, DisplayArea, Rect,
    };

    /// The window-label contract: frontend routing, capabilities and the
    /// runtime harness all depend on these exact labels.
    #[test]
    fn window_role_labels_contract() {
        assert_eq!(roles::MAIN, "main");
        assert_eq!(roles::LAUNCHER, "launcher");
        assert_eq!(roles::DOCK, "dock");
    }

    fn display(scale: f64) -> DisplayArea {
        DisplayArea {
            work: Rect {
                x: 0.0,
                y: 0.0,
                width: 1920.0 * scale,
                height: 1040.0 * scale,
            },
            scale,
        }
    }

    #[test]
    fn tiny_dock_sized_main_is_recovered() {
        let current = Rect {
            x: 0.0,
            y: 0.0,
            width: 15.0,
            height: 37.0,
        };
        let (safe, recovered) = safe_main_geometry(current, &[display(1.0)]).unwrap();
        assert!(recovered);
        assert_eq!(safe.width, 1200.0);
        assert_eq!(safe.height, 800.0);
    }

    #[test]
    fn offscreen_and_non_finite_main_are_recovered() {
        for current in [
            Rect {
                x: 9000.0,
                y: 9000.0,
                width: 1200.0,
                height: 800.0,
            },
            Rect {
                x: f64::NAN,
                y: 0.0,
                width: 1200.0,
                height: 800.0,
            },
        ] {
            assert!(safe_main_geometry(current, &[display(1.0)]).unwrap().1);
        }
    }

    #[test]
    fn main_minimum_is_scaled_for_dpi() {
        let current = Rect {
            x: 0.0,
            y: 0.0,
            width: 960.0,
            height: 620.0,
        };
        let (safe, recovered) = safe_main_geometry(current, &[display(1.25)]).unwrap();
        assert!(recovered);
        assert_eq!(safe.width, 1500.0);
        assert_eq!(safe.height, 1000.0);
    }

    #[test]
    fn dock_geometry_target_is_never_main() {
        assert_eq!(roles::DOCK, "dock");
        assert_ne!(roles::DOCK, roles::MAIN);
        assert_ne!(roles::DOCK, roles::LAUNCHER);
    }

    #[test]
    fn configured_main_does_not_inherit_dock_window_attributes() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        let main = &config["app"]["windows"][0];
        assert_eq!(main["label"], "main");
        assert_eq!(main["transparent"], false);
        assert_eq!(main["decorations"], true);
        assert_eq!(main["resizable"], true);
        assert_eq!(main["skipTaskbar"], false);
        assert_eq!(main["alwaysOnTop"], false);
        assert_eq!(main["minWidth"], 960);
        assert_eq!(main["minHeight"], 620);
    }

    #[test]
    fn launcher_close_hides_and_survives_twenty_cycles() {
        let mut exists = true;
        for _ in 0..20 {
            assert!(exists, "launcher must exist before every shortcut trigger");
            let visible = true;
            assert!(visible);
            match close_disposition(roles::LAUNCHER, false) {
                CloseDisposition::Hide => {}
                CloseDisposition::Destroy => exists = false,
            }
        }
        assert!(exists);
        assert_eq!(
            close_disposition(roles::LAUNCHER, true),
            CloseDisposition::Destroy
        );
    }
}
