use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::Manager;

/// Persisted application settings (`settings.json` in the app data
/// directory). Unknown fields are tolerated (serde ignores them), so files
/// written by older versions keep working and future fields extend this
/// struct — see docs/design.md for the planned shape.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// UI language: "zh-CN" | "en-US". `None` means the user has not chosen
    /// yet — the frontend picks a default from the system locale and
    /// persists it via `set_language`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Quick launcher settings (global shortcut).
    #[serde(default)]
    pub launcher: LauncherSettings,
    /// Desktop edge dock settings.
    #[serde(default)]
    pub dock: DockSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct LauncherSettings {
    /// Whether the global shortcut is registered.
    pub enabled: bool,
    /// Accelerator string, e.g. "Ctrl+Alt+Space".
    pub shortcut: String,
}

impl Default for LauncherSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            shortcut: "Ctrl+Alt+Space".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct DockSettings {
    /// Whether the dock is visible (collapsed handle at the screen edge).
    pub enabled: bool,
    /// "left" | "right" — which screen edge the dock anchors to.
    pub position: String,
    /// Collapse on mouse leave after `hide_delay`.
    pub auto_hide: bool,
    /// Keep the dock above other windows.
    pub always_on_top: bool,
    /// Collapsed handle width in logical pixels.
    pub collapsed_width: u32,
    /// Expanded width in logical pixels.
    pub expanded_width: u32,
    /// Mouse-leave delay before collapsing, in milliseconds.
    pub hide_delay: u32,
}

impl Default for DockSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            position: "right".into(),
            auto_hide: true,
            always_on_top: true,
            collapsed_width: 8,
            expanded_width: 360,
            hide_delay: 400,
        }
    }
}

/// Validate dock settings; returns a localized-able error message.
pub fn validate_dock(dock: &DockSettings) -> Result<(), String> {
    if dock.position != "left" && dock.position != "right" {
        return Err(format!(
            "invalid dock position: {:?} (expected \"left\" or \"right\")",
            dock.position
        ));
    }
    if !(4..=32).contains(&dock.collapsed_width) {
        return Err(format!(
            "invalid dock collapsedWidth: {} (expected 4..=32)",
            dock.collapsed_width
        ));
    }
    if !(320..=400).contains(&dock.expanded_width) {
        return Err(format!(
            "invalid dock expandedWidth: {} (expected 320..=400)",
            dock.expanded_width
        ));
    }
    if !(100..=2000).contains(&dock.hide_delay) {
        return Err(format!(
            "invalid dock hideDelay: {} (expected 100..=2000)",
            dock.hide_delay
        ));
    }
    Ok(())
}

pub const SUPPORTED_LANGUAGES: [&str; 2] = ["zh-CN", "en-US"];

/// Modifier tokens accepted in a shortcut accelerator.
const MODIFIERS: [&str; 4] = ["Ctrl", "Alt", "Shift", "Super"];

/// Validate a shortcut accelerator of the form `Modifier(+Modifier)*+Key`
/// with exactly one non-modifier key. Called before attempting OS
/// registration so the user gets a clear error instead of an opaque plugin
/// failure.
pub fn validate_shortcut(shortcut: &str) -> Result<(), String> {
    let tokens: Vec<&str> = shortcut.split('+').map(str::trim).collect();

    // empty tokens mean leading/trailing/double '+' — always invalid
    if tokens.is_empty() || tokens.iter().any(|t| t.is_empty()) {
        return Err(format!(
            "invalid shortcut: {shortcut:?} (expected e.g. Ctrl+Alt+Space)"
        ));
    }
    if tokens.len() < 2 {
        return Err(format!(
            "invalid shortcut: {shortcut:?} (expected e.g. Ctrl+Alt+Space)"
        ));
    }

    let mut has_modifier = false;
    let mut key_count = 0;
    for token in &tokens {
        if MODIFIERS.contains(token) {
            has_modifier = true;
        } else {
            key_count += 1;
            if !is_valid_key(token) {
                return Err(format!("invalid shortcut key: {token:?}"));
            }
        }
    }

    if !has_modifier {
        return Err(format!(
            "invalid shortcut: {shortcut:?} (at least one modifier required)"
        ));
    }
    if key_count != 1 {
        return Err(format!(
            "invalid shortcut: {shortcut:?} (exactly one key required)"
        ));
    }
    Ok(())
}

fn is_valid_key(token: &str) -> bool {
    if token.len() == 1 {
        return token.chars().next().is_some_and(|c| c.is_ascii_alphanumeric());
    }
    matches!(
        token,
        "Space"
            | "Tab"
            | "Enter"
            | "Escape"
            | "Backspace"
            | "Delete"
            | "Home"
            | "End"
            | "PageUp"
            | "PageDown"
            | "ArrowUp"
            | "ArrowDown"
            | "ArrowLeft"
            | "ArrowRight"
            | "F1" | "F2" | "F3" | "F4" | "F5" | "F6" | "F7" | "F8" | "F9" | "F10" | "F11" | "F12"
    )
}

/// Load settings from `path`; missing file yields defaults.
pub fn load(path: &Path) -> Result<Settings, String> {
    if !path.exists() {
        return Ok(Settings::default());
    }
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| format!("invalid settings file: {e}"))
}

/// Save settings to `path` (parent directory must exist).
pub fn save(path: &Path, settings: &Settings) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(path, raw).map_err(|e| e.to_string())
}

pub fn validate_language(language: &str) -> Result<(), String> {
    if SUPPORTED_LANGUAGES.contains(&language) {
        Ok(())
    } else {
        Err(format!(
            "unsupported language: {language} (expected one of: {})",
            SUPPORTED_LANGUAGES.join(", ")
        ))
    }
}

/// Path of `settings.json` in the app data directory.
pub fn app_settings_path<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(dir.join("settings.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("snipdeck-settings-test-{}-{name}.json", std::process::id()))
    }

    #[test]
    fn missing_file_yields_defaults() {
        let path = temp_path("missing");
        let _ = std::fs::remove_file(&path);
        let settings = load(&path).unwrap();
        assert_eq!(settings.language, None);
        // launcher defaults
        assert!(settings.launcher.enabled);
        assert_eq!(settings.launcher.shortcut, "Ctrl+Alt+Space");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let path = temp_path("roundtrip");
        let settings = Settings {
            language: Some("zh-CN".into()),
            launcher: LauncherSettings {
                enabled: true,
                shortcut: "Ctrl+Shift+Space".into(),
            },
            ..Default::default()
        };
        save(&path, &settings).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.language.as_deref(), Some("zh-CN"));
        assert_eq!(loaded.launcher.shortcut, "Ctrl+Shift+Space");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn unknown_future_fields_are_tolerated() {
        let path = temp_path("future-fields");
        std::fs::write(&path, r#"{ "language": "en-US", "dock": { "enabled": true } }"#).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.language.as_deref(), Some("en-US"));
        // old file without launcher section falls back to defaults
        assert!(loaded.launcher.enabled);
        assert_eq!(loaded.launcher.shortcut, "Ctrl+Alt+Space");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn partial_launcher_section_falls_back_per_field() {
        let path = temp_path("partial-launcher");
        std::fs::write(&path, r#"{ "launcher": { "enabled": false } }"#).unwrap();
        let loaded = load(&path).unwrap();
        assert!(!loaded.launcher.enabled);
        assert_eq!(loaded.launcher.shortcut, "Ctrl+Alt+Space");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn corrupt_file_errors() {
        let path = temp_path("corrupt");
        std::fs::write(&path, "not json at all").unwrap();
        assert!(load(&path).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn language_validation() {
        assert!(validate_language("zh-CN").is_ok());
        assert!(validate_language("en-US").is_ok());
        assert!(validate_language("fr-FR").is_err());
        assert!(validate_language("").is_err());
        assert!(validate_language("zh").is_err());
    }

    #[test]
    fn shortcut_validation_accepts_valid_forms() {
        for valid in [
            "Ctrl+Alt+Space",
            "Ctrl+Shift+Space",
            "Alt+Space",
            "Ctrl+F9",
            "Ctrl+Alt+P",
            "Super+Shift+X",
        ] {
            assert!(validate_shortcut(valid).is_ok(), "{valid} should be valid");
        }
    }

    #[test]
    fn shortcut_validation_rejects_invalid_forms() {
        for invalid in [
            "",
            "Ctrl",
            "Space",
            "A+B+C",
            "Ctrl+",
            "+Alt+Space",
            "Ctrl++Space",
            "Ctrl+Alt+A+B",
            "Ctrl+NoSuchKey",
            "Foo+Bar",
            "ctrl+alt+space", // case-sensitive token names
        ] {
            assert!(
                validate_shortcut(invalid).is_err(),
                "{invalid:?} should be rejected"
            );
        }
    }

    #[test]
    fn dock_defaults() {
        let settings = load(&temp_path("dock-defaults")).unwrap();
        let dock = &settings.dock;
        assert!(!dock.enabled);
        assert_eq!(dock.position, "right");
        assert!(dock.auto_hide);
        assert!(dock.always_on_top);
        assert_eq!(dock.collapsed_width, 8);
        assert_eq!(dock.expanded_width, 360);
        assert_eq!(dock.hide_delay, 400);
    }

    #[test]
    fn dock_section_partial_falls_back_per_field() {
        let path = temp_path("dock-partial");
        std::fs::write(&path, r#"{ "dock": { "enabled": true, "position": "left" } }"#).unwrap();
        let loaded = load(&path).unwrap();
        assert!(loaded.dock.enabled);
        assert_eq!(loaded.dock.position, "left");
        assert_eq!(loaded.dock.collapsed_width, 8); // default
        assert_eq!(loaded.dock.expanded_width, 360);
        assert_eq!(loaded.dock.hide_delay, 400);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn dock_validation() {
        let mut dock = DockSettings::default();
        assert!(validate_dock(&dock).is_ok());

        dock.position = "top".into();
        assert!(validate_dock(&dock).is_err());
        dock.position = "right".into();

        dock.collapsed_width = 3;
        assert!(validate_dock(&dock).is_err());
        dock.collapsed_width = 33;
        assert!(validate_dock(&dock).is_err());
        dock.collapsed_width = 8;

        dock.expanded_width = 319;
        assert!(validate_dock(&dock).is_err());
        dock.expanded_width = 401;
        assert!(validate_dock(&dock).is_err());
        dock.expanded_width = 360;

        dock.hide_delay = 99;
        assert!(validate_dock(&dock).is_err());
        dock.hide_delay = 2001;
        assert!(validate_dock(&dock).is_err());
        dock.hide_delay = 400;

        assert!(validate_dock(&dock).is_ok());
    }
}
