use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicI64, Ordering};

/// Last timestamp handed out, so timestamps are strictly increasing even
/// when two events land in the same millisecond. This keeps "Recent"
/// ordering deterministic under rapid copies.
static LAST_TIMESTAMP: AtomicI64 = AtomicI64::new(0);

/// Current time as milliseconds since the Unix epoch, guaranteed strictly
/// increasing across calls within the process.
pub fn now_ms() -> i64 {
    let real = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let mut prev = LAST_TIMESTAMP.load(Ordering::Relaxed);
    loop {
        let next = real.max(prev + 1);
        match LAST_TIMESTAMP.compare_exchange_weak(prev, next, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(_) => return next,
            Err(actual) => prev = actual,
        }
    }
}

/// Supported snippet types. Serialized as uppercase strings (e.g. "PROMPT").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum SnippetType {
    Prompt,
    Payload,
    Exp,
    Command,
    Code,
    #[default]
    Text,
    Template,
    Http,
}

impl SnippetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SnippetType::Prompt => "PROMPT",
            SnippetType::Payload => "PAYLOAD",
            SnippetType::Exp => "EXP",
            SnippetType::Command => "COMMAND",
            SnippetType::Code => "CODE",
            SnippetType::Text => "TEXT",
            SnippetType::Template => "TEMPLATE",
            SnippetType::Http => "HTTP",
        }
    }

    /// Parse the value stored in SQLite. Falls back to `Http` only for
    /// impossible values (the DB has a CHECK constraint).
    pub fn from_db(s: &str) -> SnippetType {
        match s {
            "PROMPT" => SnippetType::Prompt,
            "PAYLOAD" => SnippetType::Payload,
            "EXP" => SnippetType::Exp,
            "COMMAND" => SnippetType::Command,
            "CODE" => SnippetType::Code,
            "TEXT" => SnippetType::Text,
            "TEMPLATE" => SnippetType::Template,
            _ => SnippetType::Http,
        }
    }
}

/// A full snippet as returned to the frontend.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    pub id: i64,
    pub title: String,
    pub content: String,
    pub r#type: SnippetType,
    pub category: String,
    pub language: String,
    pub tags: Vec<String>,
    pub aliases: Vec<String>,
    pub favorite: bool,
    pub usage_count: u32,
    pub last_used_at: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Payload for creating / updating a snippet.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetInput {
    pub title: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub r#type: SnippetType,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

/// Category with snippet count, derived from the snippets table.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryInfo {
    pub name: String,
    pub count: i64,
}

/// Result of a copy operation: the final text and the updated snippet.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyResult {
    pub text: String,
    pub snippet: Snippet,
}

/// Which list the sidebar is currently showing.
#[derive(Debug, Clone)]
pub enum ListView {
    All,
    Favorites,
    Recent,
    MostUsed,
    Category(String),
}

impl ListView {
    pub fn parse(name: &str, category: Option<String>) -> Option<Self> {
        match name {
            "all" => Some(ListView::All),
            "favorites" => Some(ListView::Favorites),
            "recent" => Some(ListView::Recent),
            "most_used" => Some(ListView::MostUsed),
            "category" => Some(ListView::Category(category.unwrap_or_default())),
            _ => None,
        }
    }
}
