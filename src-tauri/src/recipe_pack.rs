use crate::models::{Snippet, SnippetType};
use crate::repo::Repo;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const FORMAT: &str = "n1-recipe-pack";
pub const FORMAT_VERSION: u32 = 1;
pub const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;
const MAX_RECIPES: usize = 5000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageMeta {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub homepage: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub minimum_app_version: String,
    pub exported_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub mode: String,
    pub r#type: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub variables: Vec<String>,
    #[serde(default)]
    pub source_pack_id: String,
    #[serde(default)]
    pub source_pack_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pack {
    #[serde(rename = "$schema", default)]
    pub schema: String,
    pub format: String,
    pub format_version: u32,
    pub package: PackageMeta,
    #[serde(default)]
    pub categories: Vec<String>,
    pub recipes: Vec<Recipe>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub package: PackageMeta,
    pub category_count: usize,
    pub recipe_count: usize,
    pub text_count: usize,
    pub command_group_count: usize,
    pub types: Vec<String>,
    pub tags: Vec<String>,
    pub conflicts: usize,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub added: u32,
    pub updated: u32,
    pub skipped: u32,
    pub failed: u32,
    pub categories_added: u32,
    pub categories_reused: u32,
}

fn valid_semver(v: &str) -> bool {
    let core = v.split(['-', '+']).next().unwrap_or("");
    let p: Vec<_> = core.split('.').collect();
    p.len() == 3
        && p.iter()
            .all(|x| !x.is_empty() && x.chars().all(|c| c.is_ascii_digit()))
}
fn app_version_too_high(v: &str) -> bool {
    fn nums(v: &str) -> [u32; 3] {
        let mut o = [0; 3];
        for (i, x) in v.split('.').take(3).enumerate() {
            o[i] = x.parse().unwrap_or(u32::MAX)
        }
        o
    }
    nums(v) > nums("0.2.0")
}
pub fn parse(raw: &str) -> Result<Pack, String> {
    if raw.len() as u64 > MAX_FILE_SIZE {
        return Err("配方包超过 10 MB 限制".into());
    }
    let p: Pack = serde_json::from_str(raw)
        .map_err(|_| "配方包不是有效的 JSON，或字段类型不正确".to_string())?;
    if p.format != FORMAT {
        return Err("文件不是 N1_Recipe 配方包".into());
    }
    if p.format_version != FORMAT_VERSION {
        return Err(format!("不支持配方包格式版本 {}", p.format_version));
    }
    if p.package.id.len() < 3 || p.package.id.len() > 128 || !valid_semver(&p.package.version) {
        return Err("package ID 或版本号不合法".into());
    }
    if app_version_too_high(&p.package.minimum_app_version) {
        return Err(format!(
            "此配方包需要 N1_Recipe {} 或更高版本",
            p.package.minimum_app_version
        ));
    }
    if p.recipes.len() > MAX_RECIPES || p.categories.len() > 500 {
        return Err("配方或分类数量超过限制".into());
    }
    let mut ids = HashSet::new();
    for r in &p.recipes {
        if r.id.is_empty() || r.id.len() > 128 || !ids.insert(r.id.clone()) {
            return Err("配方 ID 为空、过长或重复".into());
        }
        if r.title.trim().is_empty() || r.title.chars().count() > 300 {
            return Err("配方标题为空或过长".into());
        }
        if !matches!(r.mode.as_str(), "TEXT" | "COMMAND_GROUP") {
            return Err("内容模式不合法".into());
        }
        if r.mode == "COMMAND_GROUP" && (r.commands.is_empty() || r.commands.len() > 1000) {
            return Err("命令组步骤为空或过多".into());
        }
        if r.content.len() > 1_000_000 || r.commands.iter().any(|x| x.len() > 100_000) {
            return Err("配方内容超过长度限制".into());
        }
        if SnippetType::from_db(&r.r#type).as_str() != r.r#type {
            return Err("用途类型不合法".into());
        }
    }
    Ok(p)
}
pub fn preview(conn: &Connection, p: &Pack) -> Result<Preview, String> {
    let mut conflicts = 0;
    for r in &p.recipes {
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM snippets WHERE source_pack_id=?1 AND stable_id=?2",
                params![p.package.id, r.id],
                |x| x.get(0),
            )
            .map_err(|_| "无法检查现有配方".to_string())?;
        conflicts += n as usize
    }
    let mut types = HashSet::new();
    let mut tags = HashSet::new();
    for r in &p.recipes {
        types.insert(r.r#type.clone());
        tags.extend(r.tags.clone())
    }
    Ok(Preview {
        package: p.package.clone(),
        category_count: p.categories.len(),
        recipe_count: p.recipes.len(),
        text_count: p.recipes.iter().filter(|r| r.mode == "TEXT").count(),
        command_group_count: p
            .recipes
            .iter()
            .filter(|r| r.mode == "COMMAND_GROUP")
            .count(),
        types: types.into_iter().collect(),
        tags: tags.into_iter().take(20).collect(),
        conflicts,
    })
}
fn stable_id(s: &Snippet) -> String {
    if s.stable_id.is_empty() {
        format!("local-{}-{}", s.created_at, s.id)
    } else {
        s.stable_id.clone()
    }
}
pub fn export(
    conn: &Connection,
    meta: PackageMeta,
    category: Option<&str>,
    ids: &[i64],
) -> Result<Pack, String> {
    let list = Repo::new(conn)
        .list(&crate::models::ListView::All)
        .map_err(|_| "无法读取配方".to_string())?;
    let selected: Vec<_> = list
        .into_iter()
        .filter(|s| {
            category.map(|c| s.category == c).unwrap_or(true)
                && (ids.is_empty() || ids.contains(&s.id))
        })
        .collect();
    let mut cats = HashSet::new();
    let recipes = selected
        .into_iter()
        .map(|s| {
            if !s.category.is_empty() {
                cats.insert(s.category.clone());
            }
            Recipe {
                id: stable_id(&s),
                title: s.title,
                description: String::new(),
                mode: s.kind,
                r#type: s.r#type.as_str().into(),
                category: s.category,
                tags: s.tags,
                aliases: s.aliases,
                content: s.content,
                commands: s.commands,
                variables: vec![],
                source_pack_id: meta.id.clone(),
                source_pack_version: meta.version.clone(),
            }
        })
        .collect();
    Ok(Pack {
        schema: "./n1-recipe-pack-v1.schema.json".into(),
        format: FORMAT.into(),
        format_version: 1,
        package: meta,
        categories: cats.into_iter().collect(),
        recipes,
    })
}
fn sync(tx: &Transaction, id: i64, r: &Recipe) -> rusqlite::Result<()> {
    tx.execute("DELETE FROM snippet_tags WHERE snippet_id=?1", params![id])?;
    tx.execute("DELETE FROM aliases WHERE snippet_id=?1", params![id])?;
    for tag in &r.tags {
        let t = tag.trim();
        if t.is_empty() {
            continue;
        }
        tx.execute("INSERT OR IGNORE INTO tags(name) VALUES(?1)", params![t])?;
        tx.execute("INSERT OR IGNORE INTO snippet_tags(snippet_id,tag_id) SELECT ?1,id FROM tags WHERE name=?2 COLLATE NOCASE",params![id,t])?;
    }
    for a in &r.aliases {
        if !a.trim().is_empty() {
            tx.execute(
                "INSERT OR IGNORE INTO aliases(snippet_id,alias) VALUES(?1,?2)",
                params![id, a.trim()],
            )?;
        }
    }
    Ok(())
}
pub fn import(conn: &Connection, p: &Pack, strategy: &str) -> Result<ImportResult, String> {
    if !matches!(strategy, "skip" | "duplicate" | "overwrite") {
        return Err("冲突策略不合法".into());
    }
    let tx = conn
        .unchecked_transaction()
        .map_err(|_| "无法开始导入事务".to_string())?;
    let mut out = ImportResult {
        added: 0,
        updated: 0,
        skipped: 0,
        failed: 0,
        categories_added: 0,
        categories_reused: 0,
    };
    for c in p
        .categories
        .iter()
        .chain(p.recipes.iter().map(|r| &r.category))
    {
        let c = c.trim();
        if c.is_empty() {
            continue;
        }
        let exists: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM categories WHERE name=?1 COLLATE NOCASE",
                params![c],
                |x| x.get(0),
            )
            .map_err(|_| "分类检查失败".to_string())?;
        if exists == 0 {
            tx.execute(
                "INSERT INTO categories(name,created_at) VALUES(?1,?2)",
                params![c, crate::models::now_ms()],
            )
            .map_err(|_| "分类写入失败".to_string())?;
            out.categories_added += 1
        } else {
            out.categories_reused += 1
        }
    }
    for r in &p.recipes {
        let existing: Option<i64> = tx
            .query_row(
                "SELECT id FROM snippets WHERE source_pack_id=?1 AND stable_id=?2",
                params![p.package.id, r.id],
                |x| x.get(0),
            )
            .optional()
            .map_err(|_| "冲突检查失败".to_string())?;
        if existing.is_some() && strategy == "skip" {
            out.skipped += 1;
            continue;
        }
        let commands =
            serde_json::to_string(&r.commands).map_err(|_| "命令序列化失败".to_string())?;
        let now = crate::models::now_ms();
        let id = if let Some(id) = existing.filter(|_| strategy == "overwrite") {
            tx.execute("UPDATE snippets SET title=?1,content=?2,type=?3,category=?4,kind=?5,commands=?6,source_pack_version=?7,updated_at=?8 WHERE id=?9",params![r.title,r.content,r.r#type,r.category,r.mode,commands,p.package.version,now,id]).map_err(|_|"覆盖配方失败".to_string())?;
            out.updated += 1;
            id
        } else {
            let title = if existing.is_some() && strategy == "duplicate" {
                format!("{}（副本）", r.title)
            } else {
                r.title.clone()
            };
            let stable = if existing.is_some() {
                format!("{}-copy-{now}", r.id)
            } else {
                r.id.clone()
            };
            tx.execute("INSERT INTO snippets(title,content,type,category,created_at,updated_at,kind,commands,stable_id,source_pack_id,source_pack_version) VALUES(?1,?2,?3,?4,?5,?5,?6,?7,?8,?9,?10)",params![title,r.content,r.r#type,r.category,now,r.mode,commands,stable,p.package.id,p.package.version]).map_err(|_|"写入配方失败".to_string())?;
            out.added += 1;
            tx.last_insert_rowid()
        };
        sync(&tx, id, r).map_err(|_| "写入标签或别名失败".to_string())?;
    }
    tx.commit().map_err(|_| "提交导入事务失败".to_string())?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    fn pack() -> Pack {
        parse(include_str!(
            "../../examples/packs/n1-recipe-starter.n1recipe.json"
        ))
        .unwrap()
    }
    #[test]
    fn roundtrip_and_strategies() {
        let c = db::open_in_memory(false).unwrap();
        let p = pack();
        let a = import(&c, &p, "skip").unwrap();
        assert_eq!(a.added, 2);
        assert_eq!(import(&c, &p, "skip").unwrap().skipped, 2);
        assert_eq!(import(&c, &p, "duplicate").unwrap().added, 2);
        assert_eq!(import(&c, &p, "overwrite").unwrap().updated, 2);
        let e = export(&c, p.package.clone(), None, &[]).unwrap();
        assert!(e.recipes.iter().any(|r| r.mode == "COMMAND_GROUP"));
    }
    #[test]
    fn rejects_bad_inputs() {
        assert!(parse("no").is_err());
        let mut p = pack();
        p.format_version = 2;
        assert!(parse(&serde_json::to_string(&p).unwrap()).is_err());
        p.format_version = 1;
        p.recipes.push(p.recipes[0].clone());
        assert!(parse(&serde_json::to_string(&p).unwrap()).is_err());
    }
    #[test]
    fn transaction_rolls_back_and_local_same_title_is_safe() {
        let c = db::open_in_memory(false).unwrap();
        let mut p = pack();
        p.recipes[1].r#type = "INVALID".into();
        assert!(import(&c, &p, "skip").is_err());
        let count: i64 = c
            .query_row("SELECT COUNT(*) FROM snippets", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
        let mut p = pack();
        p.recipes.truncate(1);
        c.execute(
            "INSERT INTO snippets(title,content,created_at,updated_at) VALUES(?1,'local',1,1)",
            params![p.recipes[0].title],
        )
        .unwrap();
        assert_eq!(import(&c, &p, "overwrite").unwrap().added, 1);
        let local: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM snippets WHERE source_pack_id=''",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(local, 1);
    }
    #[test]
    fn export_omits_local_state_and_preserves_unicode() {
        let c = db::open_in_memory(false).unwrap();
        let p = pack();
        import(&c, &p, "skip").unwrap();
        c.execute(
            "UPDATE snippets SET favorite=1,dock_pinned=1,usage_count=99,last_used_at=42",
            [],
        )
        .unwrap();
        let json =
            serde_json::to_string(&export(&c, p.package.clone(), None, &[]).unwrap()).unwrap();
        assert!(json.contains("文章标题"));
        for forbidden in ["favorite", "dockPinned", "usageCount", "lastUsedAt"] {
            assert!(!json.contains(forbidden));
        }
    }
}
