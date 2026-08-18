# SnipDeck

Personal Snippet / Prompt / Payload / Template Quick Launcher.

Windows-first, local-first desktop tool for searching, finding and copying
frequently used text: AI prompts, security payloads, EXP templates, shell
commands, curl / HTTP templates and more.

## Stack

- Tauri 2 (Rust backend)
- Vue 3 + TypeScript + Vite (frontend)
- SQLite (rusqlite, bundled — no system dependency)

## Features (Phase 1)

- Snippet CRUD with title / content / type / category / language / tags / aliases
- Search across title, aliases, tags, category and content (substring + subsequence fallback, so `rtl` finds `ret2libc`)
- Views: All, Favorites, Recent, Most Used, per-category
- One-click copy to system clipboard (usage_count +1, last-used updated)
- Keyboard driven: `↑↓` navigate, `Enter` copy, `Ctrl+E` edit, `Ctrl+N` new, `Ctrl+F` search

## Development

```sh
npm install
npm run tauri dev
```

## Build

```sh
npm run tauri build
```

## Data

SQLite database lives in the app data directory
(`%APPDATA%\com.snipdeck.app\snipdeck.db` on Windows).
