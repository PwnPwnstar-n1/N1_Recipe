use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

/// Application-wide database handle managed by Tauri.
pub struct Db(pub Mutex<Connection>);

/// Ordered migrations. Index i is applied to bring the schema from version i to i+1.
/// `PRAGMA user_version` tracks the current schema version.
const MIGRATIONS: &[&str] = &[
    // v1 — initial schema
    r#"
    CREATE TABLE snippets (
        id           INTEGER PRIMARY KEY AUTOINCREMENT,
        title        TEXT NOT NULL,
        content      TEXT NOT NULL DEFAULT '',
        type         TEXT NOT NULL DEFAULT 'TEXT'
                     CHECK (type IN ('PROMPT','PAYLOAD','EXP','COMMAND','CODE','TEXT','TEMPLATE','HTTP')),
        category     TEXT NOT NULL DEFAULT '',
        language     TEXT NOT NULL DEFAULT '',
        favorite     INTEGER NOT NULL DEFAULT 0,
        usage_count  INTEGER NOT NULL DEFAULT 0,
        last_used_at INTEGER,
        created_at   INTEGER NOT NULL,
        updated_at   INTEGER NOT NULL
    );

    CREATE TABLE tags (
        id   INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL UNIQUE COLLATE NOCASE
    );

    CREATE TABLE snippet_tags (
        snippet_id INTEGER NOT NULL REFERENCES snippets(id) ON DELETE CASCADE,
        tag_id     INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
        PRIMARY KEY (snippet_id, tag_id)
    );

    CREATE TABLE aliases (
        snippet_id INTEGER NOT NULL REFERENCES snippets(id) ON DELETE CASCADE,
        alias      TEXT NOT NULL COLLATE NOCASE,
        PRIMARY KEY (snippet_id, alias)
    );

    CREATE INDEX idx_snippets_favorite   ON snippets(favorite);
    CREATE INDEX idx_snippets_usage      ON snippets(usage_count);
    CREATE INDEX idx_snippets_last_used  ON snippets(last_used_at);
    CREATE INDEX idx_snippets_category   ON snippets(category);
    CREATE INDEX idx_aliases_alias       ON aliases(alias);
    "#,
];

/// Open (or create) the database at `path`, run pending migrations and seed
/// sample data when the database was just created.
pub fn open(path: &Path) -> rusqlite::Result<Connection> {
    let mut conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;

    let fresh = current_version(&conn)? == 0;
    migrate(&mut conn)?;
    if fresh {
        crate::seed::seed(&conn)?;
    }
    Ok(conn)
}

/// In-memory database, optionally seeded — used by tests.
pub fn open_in_memory(seed: bool) -> rusqlite::Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    let fresh = current_version(&conn)? == 0;
    migrate(&mut conn)?;
    if fresh && seed {
        crate::seed::seed(&conn)?;
    }
    Ok(conn)
}

fn current_version(conn: &Connection) -> rusqlite::Result<i64> {
    conn.query_row("PRAGMA user_version", [], |row| row.get(0))
}

fn migrate(conn: &mut Connection) -> rusqlite::Result<()> {
    let mut version = current_version(conn)? as usize;
    while version < MIGRATIONS.len() {
        let tx = conn.transaction()?;
        tx.execute_batch(MIGRATIONS[version])?;
        tx.pragma_update(None, "user_version", (version + 1) as i64)?;
        tx.commit()?;
        version += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_db_gets_seeded() {
        let conn = open(Path::new(":memory:")).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM snippets", [], |r| r.get(0))
            .unwrap();
        assert!(count > 0, "fresh database should contain seed snippets");
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, MIGRATIONS.len() as i64);
    }

    #[test]
    fn memory_db_without_seed_is_empty() {
        let conn = open_in_memory(false).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM snippets", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}
