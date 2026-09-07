# N1_Recipe

Local-first recipe, prompt, command-group and template launcher.

Windows-first, local-first desktop tool for searching, finding and copying
frequently used text: AI prompts, security payloads, EXP templates, shell
commands, curl / HTTP templates and more. All data stays on your machine.

## Core features

- Snippet management (title / content / type / category / language / tags / aliases)
- Search across title, aliases, tags, category and content
- Template variables (`{{name}}`) with system variables (`{{datetime}}`, `{{clipboard}}`, …)
- Global Quick Launcher — search and copy from anywhere
- Desktop Edge Dock — hover-to-expand panel pinned to a screen edge
- zh-CN / en-US
- JSON Backup / Restore (Settings → Data)
- System tray with close-to-tray and Windows startup support
- User-created categories with safe rename/delete (items move to Uncategorized)
- Scratch area for persistent temporary notes and pasted text
- Ordered command-group snippets with per-command and copy-all actions
- Unicode template variables such as `{{文章路径}}` and `{{项目 名称}}`
- Light, dark and system themes plus four readable font sizes
- Resizable quick launcher with per-window geometry restoration

## v0.1.1 data compatibility

The existing `snipdeck.db` in the unchanged `com.snipdeck.app` data directory is upgraded in place. Existing snippets remain
ordinary text snippets; their titles, content, categories and usage metadata
are retained. Categories are promoted to managed category records. No database
or user file is deleted or recreated.

## Default shortcut

```
Ctrl + Alt + Space
```

opens / closes the Quick Launcher.

## Data directory

```
%APPDATA%\com.snipdeck.app\
├── snipdeck.db     # your snippets (SQLite)
└── settings.json   # language / launcher / dock / startup
```

## Quitting

The Main Window's **X** hides N1_Recipe to the tray (the launcher, dock and
global shortcut keep working). To actually exit:

```
System Tray → Quit N1_Recipe
```

## Backup

```
Settings → Data → Export Data / Import Data
```

## Stack

- Tauri 2 (Rust backend) + Vue 3 + TypeScript + Vite (frontend)
- SQLite (rusqlite, bundled — no system dependency)

## Development

```sh
npm install
npm run tauri dev
```

## Build

```sh
npm run tauri build
```
