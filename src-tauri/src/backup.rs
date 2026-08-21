use serde::{Deserialize, Serialize};

/// Stable interchange format for the Snippet library. This is NOT a raw
/// SQLite dump — it is the migration surface that future versions must keep
/// loadable. See docs/design.md "Backup Format v1".
pub const FORMAT: &str = "snipdeck-backup";
pub const CURRENT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupFile {
    pub format: String,
    pub version: u32,
    pub exported_at: String,
    pub snippets: Vec<BackupSnippet>,
}

/// A single snippet in the backup. `id` is deliberately absent: database ids
/// are not a stable migration identity — imports always assign new ids.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BackupSnippet {
    pub title: String,
    #[serde(default)]
    pub content: String,
    pub r#type: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub usage_count: u32,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
}

/// Result of a merge import.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub added: u32,
    pub skipped_duplicates: u32,
    pub failed: u32,
}

/// Validate a parsed backup BEFORE any database write happens. This is the
/// full upfront validation that makes the import transaction all-or-nothing.
pub fn validate(backup: &BackupFile) -> Result<(), String> {
    if backup.format != FORMAT {
        return Err(format!(
            "invalid backup format: {:?} (expected {FORMAT:?})",
            backup.format
        ));
    }
    if backup.version != CURRENT_VERSION {
        return Err(format!(
            "unsupported backup version: {} (expected {CURRENT_VERSION})",
            backup.version
        ));
    }
    for (index, snippet) in backup.snippets.iter().enumerate() {
        validate_snippet(snippet)
            .map_err(|e| format!("snippet #{}: {e}", index + 1))?;
    }
    Ok(())
}

fn validate_snippet(snippet: &BackupSnippet) -> Result<(), String> {
    if snippet.title.trim().is_empty() {
        return Err("missing required field: title".into());
    }
    const TYPES: [&str; 8] = [
        "PROMPT", "PAYLOAD", "EXP", "COMMAND", "CODE", "TEXT", "TEMPLATE", "HTTP",
    ];
    if !TYPES.contains(&snippet.r#type.as_str()) {
        return Err(format!("invalid snippet type: {:?}", snippet.r#type));
    }
    Ok(())
}

/// Serialize with pretty-print (2-space indent), UTF-8, no trailing newline
/// beyond what serde produces.
pub fn to_json_pretty(backup: &BackupFile) -> Result<String, String> {
    serde_json::to_string_pretty(backup).map_err(|e| e.to_string())
}

pub fn from_str(json: &str) -> Result<BackupFile, String> {
    serde_json::from_str(json).map_err(|e| format!("invalid backup JSON: {e}"))
}

/// Read + parse + validate a backup file's contents in one place.
pub fn parse_and_validate(json: &str) -> Result<BackupFile, String> {
    let backup = from_str(json)?;
    validate(&backup)?;
    Ok(backup)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> BackupFile {
        BackupFile {
            format: FORMAT.into(),
            version: CURRENT_VERSION,
            exported_at: "2026-08-20T00:00:00Z".into(),
            snippets: vec![BackupSnippet {
                title: "ret2libc EXP".into(),
                content: "from pwn import *".into(),
                r#type: "EXP".into(),
                category: "Security/Pwn".into(),
                language: "python".into(),
                tags: vec!["pwn".into()],
                aliases: vec!["rtl".into()],
                favorite: true,
                usage_count: 3,
                created_at: 1,
                updated_at: 2,
            }],
        }
    }

    #[test]
    fn roundtrip_pretty_json() {
        let json = to_json_pretty(&sample()).unwrap();
        let parsed = parse_and_validate(&json).unwrap();
        assert_eq!(parsed.snippets, sample().snippets);
        // pretty-print is multi-line and human readable
        assert!(json.contains('\n'));
        assert!(json.contains("\"format\": \"snipdeck-backup\""));
    }

    #[test]
    fn unicode_and_multiline_content_survive() {
        let mut backup = sample();
        backup.snippets[0].title = "中文 English 🎯".into();
        backup.snippets[0].content = "line1\nline2\n第三行".into();
        let json = to_json_pretty(&backup).unwrap();
        let parsed = parse_and_validate(&json).unwrap();
        assert_eq!(parsed.snippets[0].title, "中文 English 🎯");
        assert_eq!(parsed.snippets[0].content, "line1\nline2\n第三行");
    }

    #[test]
    fn rejects_bad_format() {
        let mut backup = sample();
        backup.format = "other".into();
        assert!(validate(&backup).unwrap_err().contains("format"));
    }

    #[test]
    fn rejects_unsupported_version() {
        let mut backup = sample();
        backup.version = 999;
        assert!(validate(&backup).unwrap_err().contains("version"));
    }

    #[test]
    fn rejects_missing_title() {
        let mut backup = sample();
        backup.snippets[0].title = "   ".into();
        assert!(validate(&backup).unwrap_err().contains("title"));
    }

    #[test]
    fn rejects_invalid_type() {
        let mut backup = sample();
        backup.snippets[0].r#type = "NOPE".into();
        assert!(validate(&backup).unwrap_err().contains("type"));
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(from_str("not json").is_err());
        // valid JSON, wrong shape
        assert!(parse_and_validate("{\"foo\": 1}").is_err());
    }
}
