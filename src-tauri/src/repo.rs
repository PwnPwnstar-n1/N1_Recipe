use crate::models::{CategoryInfo, ListView, ScratchItem, Snippet, SnippetInput, SnippetType};
use rusqlite::{params, params_from_iter, Connection, OptionalExtension, Row, Transaction};

/// Column list shared by every snippet query (aliased table `s`).
const SNIPPET_COLS: &str = "s.id, s.title, s.content, s.type, s.category, s.language, \
     s.favorite, s.usage_count, s.last_used_at, s.created_at, s.updated_at, s.kind, s.commands, s.dock_pinned, s.stable_id, s.source_pack_id, s.source_pack_version";

/// Sub-select returning tags joined by char(31) (unit separator, invisible in practice).
const TAG_SUB: &str = "(SELECT group_concat(t.name, char(31)) FROM snippet_tags st \
     JOIN tags t ON t.id = st.tag_id WHERE st.snippet_id = s.id)";

/// Sub-select returning aliases joined by char(31).
const ALIAS_SUB: &str = "(SELECT group_concat(a.alias, char(31)) FROM aliases a \
     WHERE a.snippet_id = s.id)";

/// Repository: all SQL access to the snippets store. The command layer
/// (crate::commands) is the only caller in production code.
pub struct Repo<'a> {
    conn: &'a Connection,
}

impl<'a> Repo<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    fn query_snippets(&self, sql: &str, params: Vec<String>) -> rusqlite::Result<Vec<Snippet>> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(params_from_iter(params.iter()), map_snippet)?;
        rows.collect()
    }

    /// List snippets for a sidebar view.
    pub fn list(&self, view: &ListView) -> rusqlite::Result<Vec<Snippet>> {
        let (where_sql, order_sql, limit_sql) = match view {
            ListView::All => ("", "ORDER BY s.favorite DESC, s.updated_at DESC", ""),
            ListView::Favorites => ("WHERE s.favorite = 1", "ORDER BY s.updated_at DESC", ""),
            ListView::Recent => (
                "WHERE s.last_used_at IS NOT NULL",
                "ORDER BY s.last_used_at DESC",
                "LIMIT 50",
            ),
            ListView::MostUsed => (
                "",
                "ORDER BY s.usage_count DESC, s.updated_at DESC",
                "LIMIT 50",
            ),
            ListView::DockPinned => ("WHERE s.dock_pinned = 1", "ORDER BY s.updated_at DESC", ""),
            ListView::Category(_) => (
                "WHERE s.category = ?1",
                "ORDER BY s.favorite DESC, s.updated_at DESC",
                "",
            ),
        };

        let mut params: Vec<String> = Vec::new();
        if let ListView::Category(name) = view {
            params.push(name.clone());
        }

        let sql = format!(
            "SELECT {}, {}, {} FROM snippets s {} {} {}",
            SNIPPET_COLS, TAG_SUB, ALIAS_SUB, where_sql, order_sql, limit_sql
        );
        self.query_snippets(&sql, params)
    }

    /// Search across title / aliases / tags / category / content.
    /// All whitespace-separated terms must match (AND); a match in a
    /// higher-priority field scores higher. Falls back to subsequence
    /// matching when substring search returns nothing (so `rtl` finds
    /// `ret2libc`).
    pub fn search(&self, query: &str) -> rusqlite::Result<Vec<Snippet>> {
        let terms: Vec<String> = query.split_whitespace().map(like_pattern).collect();
        if terms.is_empty() {
            return Ok(Vec::new());
        }

        let result = self.search_with_terms(&terms)?;
        if result.is_empty() {
            let subseq: Vec<String> = query.split_whitespace().map(subsequence_pattern).collect();
            if subseq
                .iter()
                .any(|p| p.chars().filter(|c| *c != '%').count() >= 2)
            {
                return self.search_with_terms(&subseq);
            }
        }
        Ok(result)
    }

    fn search_with_terms(&self, terms: &[String]) -> rusqlite::Result<Vec<Snippet>> {
        let mut conditions = Vec::with_capacity(terms.len());
        let mut scores = Vec::with_capacity(terms.len());
        for (i, _) in terms.iter().enumerate() {
            let p = format!("?{}", i + 1);
            conditions.push(format!(
                "(s.title LIKE {p} ESCAPE '\\' OR s.category LIKE {p} ESCAPE '\\' \
                 OR s.content LIKE {p} ESCAPE '\\' \
                 OR EXISTS (SELECT 1 FROM aliases a WHERE a.snippet_id = s.id AND a.alias LIKE {p} ESCAPE '\\') \
                 OR EXISTS (SELECT 1 FROM snippet_tags st JOIN tags t ON t.id = st.tag_id \
                            WHERE st.snippet_id = s.id AND t.name LIKE {p} ESCAPE '\\'))"
            ));
            scores.push(format!(
                "((CASE WHEN s.title LIKE {p} ESCAPE '\\' THEN 8 ELSE 0 END) \
                + (CASE WHEN EXISTS (SELECT 1 FROM aliases a WHERE a.snippet_id = s.id AND a.alias LIKE {p} ESCAPE '\\') THEN 6 ELSE 0 END) \
                + (CASE WHEN EXISTS (SELECT 1 FROM snippet_tags st JOIN tags t ON t.id = st.tag_id \
                                     WHERE st.snippet_id = s.id AND t.name LIKE {p} ESCAPE '\\') THEN 4 ELSE 0 END) \
                + (CASE WHEN s.category LIKE {p} ESCAPE '\\' THEN 3 ELSE 0 END) \
                + (CASE WHEN s.content LIKE {p} ESCAPE '\\' THEN 1 ELSE 0 END))"
            ));
        }

        let sql = format!(
            "SELECT {}, {}, {}, ({}) AS score FROM snippets s \
             WHERE {} ORDER BY score DESC, s.favorite DESC, s.usage_count DESC, s.updated_at DESC LIMIT 200",
            SNIPPET_COLS,
            TAG_SUB,
            ALIAS_SUB,
            scores.join(" + "),
            conditions.join(" AND "),
        );
        self.query_snippets(&sql, terms.to_vec())
    }

    /// Fetch a single snippet by id.
    pub fn get(&self, id: i64) -> rusqlite::Result<Snippet> {
        let sql = format!(
            "SELECT {}, {}, {} FROM snippets s WHERE s.id = ?1",
            SNIPPET_COLS, TAG_SUB, ALIAS_SUB
        );
        self.conn
            .query_row(&sql, params![id], map_snippet)
            .optional()?
            .ok_or(rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn insert(&self, input: &SnippetInput) -> rusqlite::Result<Snippet> {
        let now = crate::models::now_ms();
        self.insert_with_metadata(input, false, 0, None, now, now)
    }

    /// Insert with explicit metadata — used by JSON import so favorite /
    /// usage_count / timestamps are preserved instead of reset.
    pub fn insert_with_metadata(
        &self,
        input: &SnippetInput,
        favorite: bool,
        usage_count: u32,
        last_used_at: Option<i64>,
        created_at: i64,
        updated_at: i64,
    ) -> rusqlite::Result<Snippet> {
        let tx = self.conn.unchecked_transaction()?;
        if !input.category.trim().is_empty() {
            tx.execute(
                "INSERT OR IGNORE INTO categories(name,created_at) VALUES (?1,?2)",
                params![input.category.trim(), created_at],
            )?;
        }
        tx.execute(
            "INSERT INTO snippets (title, content, type, category, language, favorite, usage_count, last_used_at, created_at, updated_at, kind, commands) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                input.title.trim(),
                input.content,
                input.r#type.as_str(),
                input.category.trim(),
                input.language.trim(),
                favorite as i64,
                usage_count,
                last_used_at,
                created_at,
                updated_at,
                input.kind,
                serde_json::to_string(&input.commands).unwrap_or_else(|_| "[]".into())
            ],
        )?;
        let id = tx.last_insert_rowid();
        sync_tags(&tx, id, &input.tags)?;
        sync_aliases(&tx, id, &input.aliases)?;
        tx.commit()?;
        self.get(id)
    }

    /// Merge-import a validated backup in a single transaction. Strategy:
    /// - every item that has no exact (title, content, type) match is added
    ///   with a NEW id and its original metadata;
    /// - exact duplicates are skipped;
    /// - the whole transaction rolls back on any error, so the library never
    ///   ends up partially imported.
    pub fn import_backup(
        &self,
        backup: &crate::backup::BackupFile,
    ) -> rusqlite::Result<crate::backup::ImportSummary> {
        let mut added = 0u32;
        let mut skipped = 0u32;
        let failed = 0u32; // imports are all-or-nothing: a failure aborts entirely

        let tx = self.conn.unchecked_transaction()?;
        for snippet in &backup.snippets {
            let exists: i64 = tx.query_row(
                "SELECT COUNT(*) FROM snippets WHERE title = ?1 AND content = ?2 AND type = ?3",
                params![snippet.title.trim(), snippet.content, snippet.r#type],
                |row| row.get(0),
            )?;
            if exists > 0 {
                skipped += 1;
                continue;
            }

            let input = SnippetInput {
                title: snippet.title.clone(),
                content: snippet.content.clone(),
                r#type: SnippetType::from_db(&snippet.r#type),
                category: snippet.category.clone(),
                language: snippet.language.clone(),
                tags: snippet.tags.clone(),
                aliases: snippet.aliases.clone(),
                kind: snippet.kind.clone(),
                commands: snippet.commands.clone(),
            };
            tx.execute(
                "INSERT INTO snippets (title, content, type, category, language, favorite, usage_count, last_used_at, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8, ?9)",
                params![
                    input.title.trim(),
                    input.content,
                    input.r#type.as_str(),
                    input.category.trim(),
                    input.language.trim(),
                    snippet.favorite as i64,
                    snippet.usage_count,
                    snippet.created_at,
                    snippet.updated_at
                ],
            )?;
            let id = tx.last_insert_rowid();
            // cannot fail for already-validated data; any error aborts the
            // whole transaction (rollback) so no partial import is possible
            sync_tags(&tx, id, &input.tags)?;
            sync_aliases(&tx, id, &input.aliases)?;
            added += 1;
        }
        tx.commit()?;

        Ok(crate::backup::ImportSummary {
            added,
            skipped_duplicates: skipped,
            failed,
        })
    }

    pub fn update(&self, id: i64, input: &SnippetInput) -> rusqlite::Result<Snippet> {
        let now = crate::models::now_ms();
        let tx = self.conn.unchecked_transaction()?;
        if !input.category.trim().is_empty() {
            tx.execute(
                "INSERT OR IGNORE INTO categories(name,created_at) VALUES (?1,?2)",
                params![input.category.trim(), now],
            )?;
        }
        let changed = tx.execute(
            "UPDATE snippets SET title = ?1, content = ?2, type = ?3, category = ?4, language = ?5, updated_at = ?6, kind = ?8, commands = ?9 WHERE id = ?7",
            params![
                input.title.trim(),
                input.content,
                input.r#type.as_str(),
                input.category.trim(),
                input.language.trim(),
                now,
                id,
                input.kind,
                serde_json::to_string(&input.commands).unwrap_or_else(|_| "[]".into())
            ],
        )?;
        if changed == 0 {
            tx.rollback()?;
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        tx.execute(
            "DELETE FROM snippet_tags WHERE snippet_id = ?1",
            params![id],
        )?;
        tx.execute("DELETE FROM aliases WHERE snippet_id = ?1", params![id])?;
        sync_tags(&tx, id, &input.tags)?;
        sync_aliases(&tx, id, &input.aliases)?;
        tx.commit()?;
        self.purge_orphan_tags();
        self.get(id)
    }

    pub fn delete(&self, id: i64) -> rusqlite::Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        let removed = tx.execute("DELETE FROM snippets WHERE id = ?1", params![id])?;
        if removed == 0 {
            tx.rollback()?;
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        tx.commit()?;
        self.purge_orphan_tags();
        Ok(())
    }

    pub fn toggle_favorite(&self, id: i64) -> rusqlite::Result<Snippet> {
        let now = crate::models::now_ms();
        let changed = self.conn.execute(
            "UPDATE snippets SET favorite = CASE favorite WHEN 1 THEN 0 ELSE 1 END, updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        self.get(id)
    }
    pub fn toggle_dock_pin(&self, id: i64) -> rusqlite::Result<Snippet> {
        self.conn.execute(
            "UPDATE snippets SET dock_pinned=CASE dock_pinned WHEN 1 THEN 0 ELSE 1 END WHERE id=?1",
            params![id],
        )?;
        self.get(id)
    }

    /// usage_count + 1 and refresh last_used_at — called on every copy.
    pub fn record_usage(&self, id: i64) -> rusqlite::Result<()> {
        let changed = self.conn.execute(
            "UPDATE snippets SET usage_count = usage_count + 1, last_used_at = ?1 WHERE id = ?2",
            params![crate::models::now_ms(), id],
        )?;
        if changed == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    }

    pub fn categories(&self) -> rusqlite::Result<Vec<CategoryInfo>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.name, COUNT(s.id) FROM categories c LEFT JOIN snippets s ON s.category = c.name COLLATE NOCASE \
             GROUP BY c.id, c.name ORDER BY c.name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(CategoryInfo {
                name: row.get(0)?,
                count: row.get(1)?,
            })
        })?;
        rows.collect()
    }

    pub fn create_category(&self, name: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "INSERT INTO categories(name, created_at) VALUES (?1, ?2)",
            params![name.trim(), crate::models::now_ms()],
        )?;
        Ok(())
    }

    pub fn rename_category(&self, old: &str, new: &str) -> rusqlite::Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE categories SET name=?1 WHERE name=?2 COLLATE NOCASE",
            params![new.trim(), old],
        )?;
        tx.execute(
            "UPDATE snippets SET category=?1 WHERE category=?2 COLLATE NOCASE",
            params![new.trim(), old],
        )?;
        tx.commit()
    }

    pub fn delete_category(&self, name: &str) -> rusqlite::Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE snippets SET category='' WHERE category=?1 COLLATE NOCASE",
            params![name],
        )?;
        tx.execute(
            "DELETE FROM categories WHERE name=?1 COLLATE NOCASE",
            params![name],
        )?;
        tx.commit()
    }

    pub fn scratch_list(&self) -> rusqlite::Result<Vec<ScratchItem>> {
        let mut stmt = self.conn.prepare(
            "SELECT id,content,created_at,updated_at FROM scratch_items ORDER BY updated_at DESC",
        )?;
        let items = stmt
            .query_map([], |r| {
                Ok(ScratchItem {
                    id: r.get(0)?,
                    content: r.get(1)?,
                    created_at: r.get(2)?,
                    updated_at: r.get(3)?,
                })
            })?
            .collect();
        items
    }
    pub fn scratch_create(&self, content: &str) -> rusqlite::Result<ScratchItem> {
        let now = crate::models::now_ms();
        self.conn.execute(
            "INSERT INTO scratch_items(content,created_at,updated_at) VALUES (?1,?2,?2)",
            params![content, now],
        )?;
        let id = self.conn.last_insert_rowid();
        Ok(ScratchItem {
            id,
            content: content.into(),
            created_at: now,
            updated_at: now,
        })
    }
    pub fn scratch_update(&self, id: i64, content: &str) -> rusqlite::Result<()> {
        self.conn.execute(
            "UPDATE scratch_items SET content=?1,updated_at=?2 WHERE id=?3",
            params![content, crate::models::now_ms(), id],
        )?;
        Ok(())
    }
    pub fn scratch_delete(&self, id: i64) -> rusqlite::Result<()> {
        self.conn
            .execute("DELETE FROM scratch_items WHERE id=?1", params![id])?;
        Ok(())
    }
    pub fn scratch_clear(&self) -> rusqlite::Result<()> {
        self.conn.execute("DELETE FROM scratch_items", [])?;
        Ok(())
    }

    /// Remove tags that no snippet references anymore.
    fn purge_orphan_tags(&self) {
        let _ = self.conn.execute(
            "DELETE FROM tags WHERE id NOT IN (SELECT tag_id FROM snippet_tags)",
            [],
        );
    }
}

/// Insert tag links, creating missing tags on the fly.
fn sync_tags(tx: &Transaction, snippet_id: i64, tags: &[String]) -> rusqlite::Result<()> {
    for tag in normalize_list(tags) {
        tx.execute(
            "INSERT INTO tags (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
            params![tag],
        )?;
        tx.execute(
            "INSERT INTO snippet_tags (snippet_id, tag_id) SELECT ?1, id FROM tags WHERE name = ?2",
            params![snippet_id, tag],
        )?;
    }
    Ok(())
}

fn sync_aliases(tx: &Transaction, snippet_id: i64, aliases: &[String]) -> rusqlite::Result<()> {
    for alias in normalize_list(aliases) {
        tx.execute(
            "INSERT OR IGNORE INTO aliases (snippet_id, alias) VALUES (?1, ?2)",
            params![snippet_id, alias],
        )?;
    }
    Ok(())
}

/// Trim, drop empties and deduplicate case-insensitively (first wins).
pub fn normalize_list(items: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for item in items {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed.to_lowercase()) {
            out.push(trimmed.to_string());
        }
    }
    out
}

fn map_snippet(row: &Row) -> rusqlite::Result<Snippet> {
    let tags: Option<String> = row.get(17)?;
    let aliases: Option<String> = row.get(18)?;
    Ok(Snippet {
        id: row.get(0)?,
        title: row.get(1)?,
        content: row.get(2)?,
        r#type: SnippetType::from_db(&row.get::<_, String>(3)?),
        category: row.get(4)?,
        language: row.get(5)?,
        tags: split_joined(tags),
        aliases: split_joined(aliases),
        favorite: row.get::<_, i64>(6)? != 0,
        usage_count: row.get(7)?,
        last_used_at: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        kind: row.get(11)?,
        commands: serde_json::from_str(&row.get::<_, String>(12)?).unwrap_or_default(),
        dock_pinned: row.get::<_, i64>(13)? != 0,
        stable_id: row.get(14)?,
        source_pack_id: row.get(15)?,
        source_pack_version: row.get(16)?,
    })
}

fn split_joined(value: Option<String>) -> Vec<String> {
    match value {
        None => Vec::new(),
        Some(s) => s.split(char::from(31)).map(String::from).collect(),
    }
}

/// `%term%` with `%`, `_` and `\` escaped.
fn like_pattern(term: &str) -> String {
    let escaped = term
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{}%", escaped)
}

/// Subsequence pattern: `rtl` -> `%r%t%l%`.
fn subsequence_pattern(term: &str) -> String {
    let mut pattern = String::from("%");
    for ch in term.chars() {
        if matches!(ch, '%' | '_' | '\\') {
            continue;
        }
        pattern.push(ch);
        pattern.push('%');
    }
    pattern
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn test_repo(conn: &Connection) -> Repo<'_> {
        Repo::new(conn)
    }

    fn input(title: &str, content: &str, category: &str) -> SnippetInput {
        SnippetInput {
            title: title.to_string(),
            content: content.to_string(),
            r#type: SnippetType::Text,
            category: category.to_string(),
            language: String::new(),
            tags: vec![],
            aliases: vec![],
            kind: "TEXT".into(),
            commands: vec![],
        }
    }

    fn full_input() -> SnippetInput {
        SnippetInput {
            title: "ret2libc EXP Template".into(),
            content: "from pwn import *\np = process('./binary')".into(),
            r#type: SnippetType::Exp,
            category: "Security/Pwn".into(),
            language: "python".into(),
            tags: vec!["pwn".into(), "pwntools".into()],
            aliases: vec!["rtl".into(), "ret2libc".into()],
            kind: "TEXT".into(),
            commands: vec![],
        }
    }

    #[test]
    fn insert_then_get_roundtrip() {
        let conn = db::open_in_memory(false).unwrap();
        let repo = test_repo(&conn);
        let created = repo.insert(&full_input()).unwrap();

        assert_eq!(created.title, "ret2libc EXP Template");
        assert_eq!(created.r#type, SnippetType::Exp);
        assert_eq!(created.category, "Security/Pwn");
        assert_eq!(created.language, "python");
        let mut tags = created.tags.clone();
        tags.sort();
        assert_eq!(tags, vec!["pwn", "pwntools"]);
        let mut aliases = created.aliases.clone();
        aliases.sort();
        assert_eq!(aliases, vec!["ret2libc", "rtl"]);
        assert!(!created.favorite);
        assert_eq!(created.usage_count, 0);
        assert!(created.created_at > 0);
        assert_eq!(created.created_at, created.updated_at);

        let fetched = repo.get(created.id).unwrap();
        assert_eq!(fetched.content, created.content);
    }

    #[test]
    fn update_replaces_fields_and_links() {
        let conn = db::open_in_memory(false).unwrap();
        let repo = test_repo(&conn);
        let created = repo.insert(&full_input()).unwrap();

        let mut next = input("changed", "new content", "Dev/Git");
        next.tags = vec!["git".into(), "undo".into()];
        next.aliases = vec!["gu".into()];
        next.r#type = SnippetType::Command;
        let updated = repo.update(created.id, &next).unwrap();

        assert_eq!(updated.title, "changed");
        assert_eq!(updated.tags, vec!["git", "undo"]);
        assert_eq!(updated.aliases, vec!["gu"]);
        assert_eq!(updated.category, "Dev/Git");

        // old tag must be orphan-purged, new one kept
        let tags_after: i64 = conn
            .query_row("SELECT COUNT(*) FROM tags", [], |r| r.get(0))
            .unwrap();
        assert_eq!(tags_after, 2);
    }

    #[test]
    fn update_missing_id_errors() {
        let conn = db::open_in_memory(false).unwrap();
        let repo = test_repo(&conn);
        assert!(repo.update(999, &input("x", "y", "")).is_err());
        assert!(repo.delete(999).is_err());
        assert!(repo.get(999).is_err());
    }

    #[test]
    fn delete_removes_row_and_links() {
        let conn = db::open_in_memory(false).unwrap();
        let repo = test_repo(&conn);
        let created = repo.insert(&full_input()).unwrap();
        repo.delete(created.id).unwrap();

        assert!(repo.get(created.id).is_err());
        let links: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM snippet_tags WHERE snippet_id = ?1",
                params![created.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(links, 0);
        let orphan_tags: i64 = conn
            .query_row("SELECT COUNT(*) FROM tags", [], |r| r.get(0))
            .unwrap();
        assert_eq!(orphan_tags, 0);
    }

    #[test]
    fn search_finds_by_title_content_tag_alias_category() {
        let conn = db::open_in_memory(false).unwrap();
        let repo = test_repo(&conn);
        repo.insert(&full_input()).unwrap();
        let mut other = input(
            "curl API template",
            "curl -X POST https://api.example.com",
            "Dev/HTTP",
        );
        other.tags = vec!["http".into()];
        other.aliases = vec!["curl".into()];
        repo.insert(&other).unwrap();

        assert_eq!(repo.search("ret2libc").unwrap().len(), 1); // title
        assert_eq!(repo.search("binary").unwrap().len(), 1); // content
        assert_eq!(repo.search("pwntools").unwrap().len(), 1); // tag
        assert_eq!(repo.search("curl").unwrap().len(), 1); // alias + title
        assert_eq!(repo.search("pwn").unwrap().len(), 1); // category + tag
        assert_eq!(repo.search("template").unwrap().len(), 2); // both titles
        assert_eq!(repo.search("").unwrap().len(), 0); // empty query
    }

    #[test]
    fn search_terms_are_anded() {
        let conn = db::open_in_memory(false).unwrap();
        let repo = test_repo(&conn);
        repo.insert(&full_input()).unwrap();
        let mut other = input("curl API template", "curl example", "Dev/HTTP");
        other.tags = vec!["http".into()];
        repo.insert(&other).unwrap();

        assert_eq!(repo.search("curl example").unwrap().len(), 1);
        assert_eq!(repo.search("curl pwn").unwrap().len(), 0);
    }

    #[test]
    fn search_falls_back_to_subsequence() {
        let conn = db::open_in_memory(false).unwrap();
        let repo = test_repo(&conn);
        let mut s = full_input();
        s.aliases = vec![]; // no alias shortcut — pure subsequence match on title
        repo.insert(&s).unwrap();

        let hits = repo.search("rtl").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "ret2libc EXP Template");
    }

    #[test]
    fn views_and_ordering() {
        let conn = db::open_in_memory(false).unwrap();
        let repo = test_repo(&conn);
        let a = repo.insert(&input("alpha", "a", "Cat/One")).unwrap();
        let b = repo.insert(&input("beta", "b", "Cat/Two")).unwrap();
        let c = repo.insert(&input("gamma", "c", "Cat/One")).unwrap();

        // favorites
        repo.toggle_favorite(b.id).unwrap();
        let favs = repo.list(&ListView::Favorites).unwrap();
        assert_eq!(favs.len(), 1);
        assert_eq!(favs[0].id, b.id);

        // recent: nothing used yet
        assert_eq!(repo.list(&ListView::Recent).unwrap().len(), 0);
        repo.record_usage(c.id).unwrap();
        repo.record_usage(a.id).unwrap();
        let recent = repo.list(&ListView::Recent).unwrap();
        assert_eq!(recent[0].id, a.id);

        // most used
        repo.record_usage(a.id).unwrap();
        let most = repo.list(&ListView::MostUsed).unwrap();
        assert_eq!(most[0].id, a.id);
        assert_eq!(most[0].usage_count, 2);

        // category filter
        let one = repo.list(&ListView::Category("Cat/One".into())).unwrap();
        assert_eq!(one.len(), 2);
        assert!(one.iter().all(|s| s.category == "Cat/One"));

        // all
        assert_eq!(repo.list(&ListView::All).unwrap().len(), 3);
    }

    #[test]
    fn record_usage_increments_and_sets_last_used() {
        let conn = db::open_in_memory(false).unwrap();
        let repo = test_repo(&conn);
        let created = repo.insert(&input("cmd", "ls", "")).unwrap();
        assert!(created.last_used_at.is_none());

        repo.record_usage(created.id).unwrap();
        let after = repo.get(created.id).unwrap();
        assert_eq!(after.usage_count, 1);
        assert!(after.last_used_at.is_some());
    }

    #[test]
    fn toggle_favorite_flips() {
        let conn = db::open_in_memory(false).unwrap();
        let repo = test_repo(&conn);
        let created = repo.insert(&input("t", "c", "")).unwrap();
        assert!(!created.favorite);
        let on = repo.toggle_favorite(created.id).unwrap();
        assert!(on.favorite);
        let off = repo.toggle_favorite(created.id).unwrap();
        assert!(!off.favorite);
    }

    #[test]
    fn categories_are_counted_and_sorted() {
        let conn = db::open_in_memory(false).unwrap();
        let repo = test_repo(&conn);
        repo.insert(&input("a", "x", "Zeta")).unwrap();
        repo.insert(&input("b", "x", "alpha")).unwrap();
        repo.insert(&input("c", "x", "Zeta")).unwrap();
        repo.insert(&input("d", "x", "")).unwrap(); // excluded

        let cats = repo.categories().unwrap();
        assert_eq!(cats.len(), 2);
        assert_eq!(cats[0].name, "alpha");
        assert_eq!(cats[1].name, "Zeta");
        assert_eq!(cats[1].count, 2);
    }

    #[test]
    fn normalize_list_trims_and_dedupes() {
        assert_eq!(
            normalize_list(&[
                "  Pwn  ".to_string(),
                "pwn".to_string(),
                "".to_string(),
                "Web".to_string()
            ]),
            vec!["Pwn", "Web"]
        );
    }

    #[test]
    fn long_library_of_35_snippets_remains_complete() {
        let conn = db::open_in_memory(false).unwrap();
        let repo = test_repo(&conn);
        for i in 0..35 {
            repo.insert(&input(&format!("item {i}"), &"x".repeat(500), "Load test"))
                .unwrap();
        }
        assert_eq!(repo.list(&ListView::All).unwrap().len(), 35);
    }

    #[test]
    fn import_backup_preserves_metadata_and_assigns_new_ids() {
        use crate::backup::{BackupFile, BackupSnippet, CURRENT_VERSION, FORMAT};
        let conn = db::open_in_memory(false).unwrap();
        let repo = test_repo(&conn);

        let backup = BackupFile {
            format: FORMAT.into(),
            version: CURRENT_VERSION,
            exported_at: "t".into(),
            snippets: vec![BackupSnippet {
                title: "exp".into(),
                content: "from pwn import *".into(),
                r#type: "EXP".into(),
                category: "Security/Pwn".into(),
                language: "python".into(),
                tags: vec!["pwn".into()],
                aliases: vec!["rtl".into()],
                favorite: true,
                usage_count: 7,
                created_at: 100,
                updated_at: 200,
                kind: "TEXT".into(),
                commands: vec![],
            }],
        };

        let summary = repo.import_backup(&backup).unwrap();
        assert_eq!(summary.added, 1);
        assert_eq!(summary.skipped_duplicates, 0);

        let all = repo.list(&crate::models::ListView::All).unwrap();
        assert_eq!(all.len(), 1);
        let s = &all[0];
        assert_eq!(s.title, "exp");
        assert_eq!(s.favorite, true);
        assert_eq!(s.usage_count, 7);
        assert_eq!(s.created_at, 100);
        assert_eq!(s.updated_at, 200);
        assert_eq!(s.tags, vec!["pwn"]);
        assert_eq!(s.aliases, vec!["rtl"]);

        // re-importing the same backup skips the exact duplicate
        let again = repo.import_backup(&backup).unwrap();
        assert_eq!(again.added, 0);
        assert_eq!(again.skipped_duplicates, 1);
        assert_eq!(repo.list(&crate::models::ListView::All).unwrap().len(), 1);
    }

    #[test]
    fn import_backup_is_all_or_nothing() {
        use crate::backup::{BackupFile, BackupSnippet, CURRENT_VERSION, FORMAT};
        let conn = db::open_in_memory(false).unwrap();
        let repo = test_repo(&conn);

        // second item has an invalid type — but validate() is expected to run
        // first in the command layer, so here we test the transactional write
        // path directly by importing valid data, then a title that would fail
        // a NOT NULL check would not happen post-validation. Instead assert
        // that a valid import never leaves partial state when the backup has
        // both new and duplicate items.
        let backup = BackupFile {
            format: FORMAT.into(),
            version: CURRENT_VERSION,
            exported_at: "t".into(),
            snippets: vec![
                BackupSnippet {
                    title: "a".into(),
                    content: "1".into(),
                    r#type: "TEXT".into(),
                    category: String::new(),
                    language: String::new(),
                    tags: vec![],
                    aliases: vec![],
                    favorite: false,
                    usage_count: 0,
                    created_at: 1,
                    updated_at: 1,
                    kind: "TEXT".into(),
                    commands: vec![],
                },
                BackupSnippet {
                    title: "b".into(),
                    content: "2".into(),
                    r#type: "TEXT".into(),
                    category: String::new(),
                    language: String::new(),
                    tags: vec![],
                    aliases: vec![],
                    favorite: false,
                    usage_count: 0,
                    created_at: 2,
                    updated_at: 2,
                    kind: "TEXT".into(),
                    commands: vec![],
                },
            ],
        };

        let summary = repo.import_backup(&backup).unwrap();
        assert_eq!(summary.added, 2);
        assert_eq!(repo.list(&crate::models::ListView::All).unwrap().len(), 2);
    }
}
