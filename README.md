# SnipDeck

Local-first Snippet / Prompt / Payload / Template Quick Launcher.

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

The Main Window's **X** hides SnipDeck to the tray (the launcher, dock and
global shortcut keep working). To actually exit:

```
System Tray → Quit SnipDeck
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
