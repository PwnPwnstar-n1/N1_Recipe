use std::collections::{HashMap, HashSet};
use tauri_plugin_clipboard_manager::ClipboardExt;

/// System variables resolved at copy time — never asked from the user.
pub const SYSTEM_VARIABLES: [&str; 4] = ["date", "time", "datetime", "clipboard"];

pub fn is_system_variable(name: &str) -> bool {
    SYSTEM_VARIABLES.contains(&name)
}

/// Extract ordinary (non-system) variables from `content` in order of first
/// appearance, deduplicated. System variables are excluded because the user
/// never fills them in.
pub fn extract_variables(content: &str) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    scan_variables(content)
        .into_iter()
        .filter(|name| !is_system_variable(name) && seen.insert(name.clone()))
        .collect()
}

/// All variable tokens in order of appearance (including system variables and
/// duplicates) — used by tests and for validation.
fn scan_variables(content: &str) -> Vec<String> {
    let mut names = Vec::new();
    let bytes = content.as_bytes();
    let mut i = 0usize;
    while i + 1 < bytes.len() {
        if bytes[i] == b'{' && bytes[i + 1] == b'{' {
            if let Some(end) = find_token_end(content, i) {
                let name = content[i + 2..end].trim();
                if is_valid_name(name) {
                    names.push(name.to_string());
                }
                i = end + 2;
                continue;
            }
        }
        i += 1;
    }
    names
}

/// Render `content`, replacing every valid `{{name}}` token whose name exists
/// in `values`. Tokens whose name is missing from `values` and malformed
/// tokens are left untouched.
pub fn render(content: &str, values: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(content.len());
    let bytes = content.as_bytes();
    let mut pos = 0usize;
    let mut i = 0usize;
    while i + 1 < bytes.len() {
        if bytes[i] == b'{' && bytes[i + 1] == b'{' {
            if let Some(end) = find_token_end(content, i) {
                let name = content[i + 2..end].trim();
                if is_valid_name(name) {
                    if let Some(value) = values.get(name) {
                        out.push_str(&content[pos..i]);
                        out.push_str(value);
                        pos = end + 2;
                        i = pos;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
    out.push_str(&content[pos..]);
    out
}

/// Resolve system variables at copy time and render. Clipboard read failures
/// (e.g. empty clipboard) resolve to an empty string.
pub fn render_with_system<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    content: &str,
    user_values: &HashMap<String, String>,
) -> String {
    let mut values = user_values.clone();
    let now = chrono::Local::now();
    values.insert("date".into(), now.format("%Y-%m-%d").to_string());
    values.insert("time".into(), now.format("%H:%M:%S").to_string());
    values.insert(
        "datetime".into(),
        now.format("%Y-%m-%d %H:%M:%S").to_string(),
    );
    let clipboard = app.clipboard().read_text().unwrap_or_default();
    values.insert("clipboard".into(), clipboard);
    render(content, &values)
}

/// Variable names in `content` that the caller did not provide values for
/// (system variables never need values).
pub fn missing_variables(content: &str, values: &HashMap<String, String>) -> Vec<String> {
    extract_variables(content)
        .into_iter()
        .filter(|name| !values.contains_key(name))
        .collect()
}

/// Find the index just before the closing `}}` of a token starting at `i`
/// (which must point at `{{`). Returns the byte index of the first `}`.
fn find_token_end(content: &str, i: usize) -> Option<usize> {
    let bytes = content.as_bytes();
    let mut j = i + 2;
    while j + 1 < bytes.len() {
        if bytes[j] == b'}' && bytes[j + 1] == b'}' {
            return Some(j);
        }
        j += 1;
    }
    None
}

fn is_valid_name(name: &str) -> bool {
    !name.trim().is_empty() && !name.contains(['{', '}'])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn extract_no_variables() {
        assert!(extract_variables("plain text, no variables").is_empty());
        assert!(extract_variables("").is_empty());
    }

    #[test]
    fn extract_single_variable() {
        assert_eq!(extract_variables("Hello {{name}}"), vec!["name"]);
    }

    #[test]
    fn extract_multiple_ordered_and_deduplicated() {
        assert_eq!(
            extract_variables("{{host}}:{{port}} -> {{host}} again"),
            vec!["host", "port"]
        );
    }

    #[test]
    fn extract_excludes_system_variables() {
        assert!(extract_variables("{{date}} {{time}} {{datetime}} {{clipboard}}").is_empty());
        // mixed: only ordinary ones, in order
        assert_eq!(
            extract_variables("{{date}} {{name}} {{clipboard}} {{goal}}"),
            vec!["name", "goal"]
        );
    }

    #[test]
    fn extract_ignores_invalid_tokens() {
        let content = "{{ }} {{123}} {{a-b}} {{}} {name} {{name}} }x{ {{unclosed";
        assert_eq!(extract_variables(content), vec!["123", "a-b", "name"]);
    }

    #[test]
    fn extract_with_unicode_content() {
        assert_eq!(
            extract_variables("目标：{{goal}} 🎯 注意 {{goal}}"),
            vec!["goal"]
        );
    }

    #[test]
    fn unicode_spaces_hyphens_and_trim_are_supported() {
        assert_eq!(
            extract_variables("{{ 文章路径 }} {{项目 名称}} {{目标-IP}} {{文章路径}}"),
            vec!["文章路径", "项目 名称", "目标-IP"]
        );
        assert_eq!(
            render(
                "{{ 文章路径 }}",
                &values(&[("文章路径", r#"C:\\a\$1\"q\""#)])
            ),
            r#"C:\\a\$1\"q\""#
        );
    }

    #[test]
    fn render_replaces_all_occurrences() {
        let content = "Target: {{host}}:{{port}}\nAgain: {{host}}";
        let rendered = render(
            content,
            &values(&[("host", "example.com"), ("port", "443")]),
        );
        assert_eq!(rendered, "Target: example.com:443\nAgain: example.com");
    }

    #[test]
    fn render_empty_string_value() {
        assert_eq!(
            render("Hello {{name}}!", &values(&[("name", "")])),
            "Hello !"
        );
    }

    #[test]
    fn render_multiline_value_and_unicode() {
        let rendered = render(
            "Goal:\n{{goal}}\n你好 {{name}} 🎯",
            &values(&[("goal", "line1\nline2"), ("name", "世界")]),
        );
        assert_eq!(rendered, "Goal:\nline1\nline2\n你好 世界 🎯");
    }

    #[test]
    fn render_missing_value_keeps_placeholder() {
        // explicit behavior: a variable with no provided value stays as-is
        assert_eq!(render("Hello {{name}}", &values(&[])), "Hello {{name}}");
        assert_eq!(
            render("{{a}} and {{b}}", &values(&[("a", "1")])),
            "1 and {{b}}"
        );
    }

    #[test]
    fn render_leaves_invalid_tokens_untouched() {
        let content = "{{ }} {{123}} {{a-b}} {x} {{name}}";
        assert_eq!(
            render(content, &values(&[("name", "N")])),
            "{{ }} {{123}} {{a-b}} {x} N"
        );
    }

    #[test]
    fn render_does_not_touch_other_text() {
        let content = "prefix {{var}} suffix\n$HOME {literal} {{var_2}}end";
        assert_eq!(
            render(content, &values(&[("var", "V"), ("var_2", "W")])),
            "prefix V suffix\n$HOME {literal} Wend"
        );
    }

    #[test]
    fn render_system_and_ordinary_together() {
        let mut v = values(&[("host", "example.com")]);
        v.insert("date".into(), "2026-08-19".into());
        let rendered = render("{{host}} on {{date}}", &v);
        assert_eq!(rendered, "example.com on 2026-08-19");
    }

    #[test]
    fn missing_variables_detects_only_ordinary() {
        let content = "{{host}} {{port}} {{date}}";
        let provided = values(&[("host", "h")]);
        assert_eq!(missing_variables(content, &provided), vec!["port"]);
        let all = values(&[("host", "h"), ("port", "p")]);
        assert!(missing_variables(content, &all).is_empty());
    }
}
