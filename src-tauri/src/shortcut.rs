use crate::settings::LauncherSettings;
use serde::Serialize;
use std::time::Instant;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RegistrationState {
    Disabled,
    Registering,
    Registered,
    Conflict,
    RegistrationFailed,
    Recovering,
    Unsupported,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BindingDiagnostic {
    pub binding: Option<String>,
    pub state: RegistrationState,
    pub last_error: Option<String>,
}

impl Default for BindingDiagnostic {
    fn default() -> Self {
        Self {
            binding: None,
            state: RegistrationState::Disabled,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutDiagnostics {
    pub primary: BindingDiagnostic,
    pub secondary: BindingDiagnostic,
    pub mouse: BindingDiagnostic,
    pub last_input_at: Option<String>,
    pub last_input_source: Option<String>,
    pub last_launcher_result: Option<String>,
    pub last_launcher_duration_ms: Option<u128>,
    pub registration_evidence_note: String,
}

pub struct ShortcutManager {
    pub primary: BindingDiagnostic,
    pub secondary: BindingDiagnostic,
    pub mouse: BindingDiagnostic,
    registered: Vec<String>,
    operation_in_progress: bool,
    last_input_at: Option<String>,
    last_input_source: Option<String>,
    last_launcher_result: Option<String>,
    last_launcher_duration_ms: Option<u128>,
}

impl Default for ShortcutManager {
    fn default() -> Self {
        Self {
            primary: BindingDiagnostic::default(),
            secondary: BindingDiagnostic::default(),
            mouse: BindingDiagnostic::default(),
            registered: Vec::new(),
            operation_in_progress: false,
            last_input_at: None,
            last_input_source: None,
            last_launcher_result: None,
            last_launcher_duration_ms: None,
        }
    }
}

fn set_slot<R: Runtime>(
    app: &AppHandle<R>,
    slot: &str,
    binding: Option<String>,
    state: RegistrationState,
    error: Option<String>,
) {
    if let Ok(mut manager) = app.state::<std::sync::Mutex<ShortcutManager>>().lock() {
        let target = if slot == "secondary" {
            &mut manager.secondary
        } else {
            &mut manager.primary
        };
        target.binding = binding;
        target.state = state;
        target.last_error = error;
    }
}

fn classify_error(error: &str) -> RegistrationState {
    if error.to_ascii_lowercase().contains("already registered") {
        RegistrationState::Conflict
    } else {
        RegistrationState::RegistrationFailed
    }
}

fn plugin_accelerator(accelerator: &str) -> String {
    accelerator
        .split('+')
        .map(|token| if token == "Win" { "Super" } else { token })
        .collect::<Vec<_>>()
        .join("+")
}

fn register_one<R: Runtime>(
    app: &AppHandle<R>,
    slot: &'static str,
    accelerator: &str,
) -> Result<(), String> {
    set_slot(
        app,
        slot,
        Some(accelerator.into()),
        RegistrationState::Registering,
        None,
    );
    crate::diagnostics::log(
        app,
        &format!("event=shortcut_register phase=begin slot={slot} binding={accelerator}"),
    );
    let source = format!("keyboard:{slot}");
    let plugin_binding = plugin_accelerator(accelerator);
    let result = app
        .global_shortcut()
        .on_shortcut(plugin_binding.as_str(), move |app, _, event| {
            if event.state == ShortcutState::Pressed {
                activate_launcher(app, &source, true);
            }
        })
        .map_err(|error| error.to_string());
    match result {
        Ok(()) => {
            set_slot(
                app,
                slot,
                Some(accelerator.into()),
                RegistrationState::Registered,
                None,
            );
            crate::diagnostics::log(
                app,
                &format!("event=shortcut_register phase=end slot={slot} result=ok"),
            );
            Ok(())
        }
        Err(error) => {
            set_slot(
                app,
                slot,
                Some(accelerator.into()),
                classify_error(&error),
                Some(error.clone()),
            );
            crate::diagnostics::log(
                app,
                &format!(
                    "event=shortcut_register phase=end slot={slot} result=error error={error}"
                ),
            );
            Err(format!("failed to register {slot} shortcut: {error}"))
        }
    }
}

fn unregister_one<R: Runtime>(app: &AppHandle<R>, accelerator: &str) {
    let result = app
        .global_shortcut()
        .unregister(plugin_accelerator(accelerator).as_str());
    crate::diagnostics::log(
        app,
        &format!(
            "event=shortcut_unregister binding={accelerator} result={}",
            if result.is_ok() { "ok" } else { "error" }
        ),
    );
}

fn desired(settings: &LauncherSettings) -> Vec<(&'static str, String)> {
    let mut bindings = Vec::new();
    if settings.enabled {
        bindings.push(("primary", settings.shortcut.clone()));
    }
    if settings.secondary_enabled {
        bindings.push(("secondary", settings.secondary_shortcut.clone()));
    }
    bindings
}

/// Register every new accelerator before persistence, then retire obsolete
/// registrations. A registration or disk failure leaves the old working set.
pub fn replace_keyboard_bindings<R: Runtime>(
    app: &AppHandle<R>,
    settings: &LauncherSettings,
    persist: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    let old = {
        let managed = app.state::<std::sync::Mutex<ShortcutManager>>();
        let mut manager = managed.lock().map_err(|e| e.to_string())?;
        if manager.operation_in_progress {
            return Err("shortcut registration is already in progress".into());
        }
        manager.operation_in_progress = true;
        manager.registered.clone()
    };
    let wanted = desired(settings);
    let mut added = Vec::new();
    let result = (|| {
        for (slot, binding) in &wanted {
            if !old.contains(binding) {
                register_one(app, slot, binding)?;
                added.push(binding.clone());
            } else {
                set_slot(
                    app,
                    slot,
                    Some(binding.clone()),
                    RegistrationState::Registered,
                    None,
                );
            }
        }
        persist()?;
        for binding in &old {
            if !wanted.iter().any(|(_, candidate)| candidate == binding) {
                unregister_one(app, binding);
            }
        }
        let managed = app.state::<std::sync::Mutex<ShortcutManager>>();
        let mut manager = managed.lock().map_err(|e| e.to_string())?;
        manager.registered = wanted.iter().map(|(_, binding)| binding.clone()).collect();
        if !settings.enabled {
            manager.primary = BindingDiagnostic::default();
        }
        if !settings.secondary_enabled {
            manager.secondary = BindingDiagnostic::default();
        }
        Ok(())
    })();
    if result.is_err() {
        for binding in &added {
            unregister_one(app, binding);
        }
    }
    if let Ok(mut manager) = app.state::<std::sync::Mutex<ShortcutManager>>().lock() {
        manager.operation_in_progress = false;
    }
    result
}

/// Startup/manual recovery. Slots are independent: one conflict does not
/// disable the other registration.
pub fn register_configured<R: Runtime>(
    app: &AppHandle<R>,
    settings: &LauncherSettings,
    recovering: bool,
) -> Result<(), String> {
    let wanted = desired(settings);
    let old = {
        let managed = app.state::<std::sync::Mutex<ShortcutManager>>();
        let mut manager = managed.lock().map_err(|e| e.to_string())?;
        if manager.operation_in_progress {
            return Err("shortcut registration is already in progress".into());
        }
        manager.operation_in_progress = true;
        std::mem::take(&mut manager.registered)
    };
    for binding in old {
        unregister_one(app, &binding);
    }
    let mut successful = Vec::new();
    let mut errors = Vec::new();
    for (slot, binding) in wanted {
        if recovering {
            set_slot(
                app,
                slot,
                Some(binding.clone()),
                RegistrationState::Recovering,
                None,
            );
        }
        match register_one(app, slot, &binding) {
            Ok(()) => successful.push(binding),
            Err(error) => errors.push(error),
        }
    }
    let managed = app.state::<std::sync::Mutex<ShortcutManager>>();
    let mut manager = managed.lock().map_err(|e| e.to_string())?;
    manager.registered = successful;
    manager.operation_in_progress = false;
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

/// Retry only registrations that are currently absent. Existing successful
/// bindings are never churned just to probe health.
pub fn retry_missing<R: Runtime>(
    app: &AppHandle<R>,
    settings: &LauncherSettings,
) -> Result<(), String> {
    let wanted = desired(settings);
    let existing = app
        .state::<std::sync::Mutex<ShortcutManager>>()
        .lock()
        .map_err(|e| e.to_string())?
        .registered
        .clone();
    let mut errors = Vec::new();
    for (slot, binding) in wanted {
        if existing.contains(&binding) {
            continue;
        }
        set_slot(
            app,
            slot,
            Some(binding.clone()),
            RegistrationState::Recovering,
            None,
        );
        match register_one(app, slot, &binding) {
            Ok(()) => {
                if let Ok(mut manager) = app.state::<std::sync::Mutex<ShortcutManager>>().lock() {
                    if !manager.registered.contains(&binding) {
                        manager.registered.push(binding);
                    }
                }
            }
            Err(error) => errors.push(error),
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

pub fn diagnostics<R: Runtime>(app: &AppHandle<R>) -> Result<ShortcutDiagnostics, String> {
    let managed = app.state::<std::sync::Mutex<ShortcutManager>>();
    let manager = managed.lock().map_err(|e| e.to_string())?;
    Ok(ShortcutDiagnostics {
        primary: manager.primary.clone(), secondary: manager.secondary.clone(), mouse: manager.mouse.clone(),
        last_input_at: manager.last_input_at.clone(), last_input_source: manager.last_input_source.clone(),
        last_launcher_result: manager.last_launcher_result.clone(), last_launcher_duration_ms: manager.last_launcher_duration_ms,
        registration_evidence_note: "Registered means the last OS registration call succeeded; actual input receipt and Launcher display are tracked separately.".into(),
    })
}

pub fn set_mouse_diagnostic<R: Runtime>(
    app: &AppHandle<R>,
    binding: Option<String>,
    state: RegistrationState,
    error: Option<String>,
) {
    if let Ok(mut manager) = app.state::<std::sync::Mutex<ShortcutManager>>().lock() {
        manager.mouse = BindingDiagnostic {
            binding,
            state,
            last_error: error,
        };
    }
}

/// Common entry for keyboard, mouse, tray, test, and settings. Input receipt
/// is recorded before window work so focus/show failures remain distinguishable.
pub fn activate_launcher<R: Runtime>(app: &AppHandle<R>, source: &str, toggle: bool) {
    if let Some(managed) = app.try_state::<std::sync::Mutex<ShortcutManager>>() {
        if let Ok(mut manager) = managed.lock() {
            manager.last_input_at = Some(chrono::Local::now().to_rfc3339());
            manager.last_input_source = Some(source.into());
        }
    }
    crate::diagnostics::log(
        app,
        &format!("event=launcher_input source={source} result=received"),
    );
    let started = Instant::now();
    let result = if toggle {
        crate::windows::toggle_launcher_window(app)
    } else {
        crate::windows::show_launcher_window(app)
    };
    let elapsed = started.elapsed().as_millis();
    let summary = match &result {
        Ok(()) => "ok".to_string(),
        Err(error) => format!("error:{error}"),
    };
    if let Some(managed) = app.try_state::<std::sync::Mutex<ShortcutManager>>() {
        if let Ok(mut manager) = managed.lock() {
            manager.last_launcher_result = Some(summary.clone());
            manager.last_launcher_duration_ms = Some(elapsed);
        }
    }
    crate::diagnostics::log(
        app,
        &format!("event=launcher_display source={source} result={summary} duration_ms={elapsed}"),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn error_classification_does_not_invent_an_owner() {
        assert_eq!(
            classify_error("HotKey already registered"),
            RegistrationState::Conflict
        );
        assert_eq!(
            classify_error("platform failure"),
            RegistrationState::RegistrationFailed
        );
    }
    #[test]
    fn desired_bindings_are_independent() {
        let mut settings = LauncherSettings::default();
        settings.secondary_enabled = true;
        assert_eq!(desired(&settings).len(), 2);
        settings.enabled = false;
        assert_eq!(
            desired(&settings),
            vec![("secondary", settings.secondary_shortcut)]
        );
    }

    #[test]
    fn advertised_keyboard_bindings_parse_in_backend() {
        use tauri_plugin_global_shortcut::Shortcut;
        for binding in [
            "Ctrl+F24",
            "Alt+ArrowUp",
            "Shift+Insert",
            "Super+P",
            "Ctrl+PrintScreen",
        ] {
            assert!(
                binding.parse::<Shortcut>().is_ok(),
                "backend must parse {binding}"
            );
        }
        assert_eq!(plugin_accelerator("Win+Shift+P"), "Super+Shift+P");
    }
}
