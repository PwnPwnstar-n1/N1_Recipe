use serde::{Deserialize, Serialize};
use std::path::Path;

/// Persisted application settings (`settings.json` in the app data
/// directory). Only `language` is implemented so far; future fields
/// (launcher shortcut, dock options) extend this struct — see
/// docs/design.md for the planned shape.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// UI language: "zh-CN" | "en-US". `None` means the user has not chosen
    /// yet — the frontend picks a default from the system locale and
    /// persists it via `set_language`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

pub const SUPPORTED_LANGUAGES: [&str; 2] = ["zh-CN", "en-US"];

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
    }

    #[test]
    fn save_and_load_roundtrip() {
        let path = temp_path("roundtrip");
        let settings = Settings {
            language: Some("zh-CN".into()),
        };
        save(&path, &settings).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.language.as_deref(), Some("zh-CN"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn unknown_future_fields_are_tolerated() {
        let path = temp_path("future-fields");
        std::fs::write(&path, r#"{ "language": "en-US", "dock": { "enabled": true } }"#).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.language.as_deref(), Some("en-US"));
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
}
