# SnipDeck Design Records

Non-code design decisions for future phases. Implementation lives in the
codebase; this file records the planned shapes so later work does not
redesign what is already decided.

## Window Roles & Multi-Window Architecture

SnipDeck will have three window roles (window label = role, see
`src-tauri/src/windows.rs`):

| Role     | Label     | Purpose                                    | Status        |
| -------- | --------- | ------------------------------------------ | ------------- |
| Main     | `main`    | Full management UI (edit / categories / settings) | implemented   |
| Launcher | `launcher`| Global-shortcut quick search + Enter copy  | implemented (Phase 3) |
| Dock     | `dock`    | Edge-docked quick access panel             | implemented (Phase 4) |

Rules:

- The frontend bundle is shared: each window renders `App.vue` and picks its
  role from `getCurrentWindow().label`.
- Backend services are shared by ALL windows — no per-window duplication:
  `repo` (CRUD/search/views), `template` (variables/system values),
  `copy_snippet` (clipboard + usage_count), `settings`.
- `windows.rs` is the Window Service: role constants, `get()` by role,
  `create_launcher()`, `ensure_hidden_launcher()`, cursor-monitor positioning
  and lifecycle commands (`show_launcher` / `hide_launcher`). The launcher
  window is created hidden at startup and **never destroyed at runtime**:
  creating a webview from an IPC/sync-command thread deadlocks on Windows
  (upstream wry issue), so create happens only on the main thread during
  setup and show/hide is the only runtime transition.
- `TemplateVariableDialog` is props/emits-only and reusable from any window.
  The main window's `store.ts` (views/editor state) is main-window specific;
  the launcher uses its own `launcherStore.ts` (query/results/selection/
  dialog/copy feedback) reusing `api.ts`, `types.ts`, `i18n`, and the dialog
  component. `SnippetList`/`SearchBar` are main-window components; the
  launcher has its own compact list.
- Global shortcut (`src-tauri/src/shortcut.rs`): the plugin's `on_shortcut`
  performs the OS registration itself — calling `register()` first would
  register the same hotkey twice (the second attempt fails and the first
  registration leaks). Updates are atomic: validate → register new → save
  settings → unregister old → record state; a failed registration never
  breaks the current shortcut or overwrites settings.json.

## Settings Shape (planned, only `language` implemented)

`settings.json` in the app data directory, read/written by
`src-tauri/src/settings.rs`. Future fields (serde defaults keep old files
valid):

```jsonc
{
  "language": "zh-CN",          // "zh-CN" | "en-US" | null (null = not chosen)
  "launcher": {
    "shortcut": "Alt+Space"     // global shortcut (Phase 3)
  },
  "dock": {
    "enabled": false,
    "position": "right",        // "left" | "right"
    "autoHide": true,
    "alwaysOnTop": true,
    "collapsedWidth": 8,        // handle width in px
    "expandedWidth": 360,
    "hideDelay": 400            // ms before auto-collapse
  }
}
```

## Edge Dock Design (implemented in Phase 4)

Goal: a slim always-available panel docked to the left or right screen edge.

- **Collapsed**: an 8px handle remains visible at the screen edge (never
  fully off-screen — mouse enter/leave drives the transitions, no global
  mouse hook).
- **Expand**: mouse enters the handle → window expands to 360px
  (logical), anchored to the same edge; full primary-monitor work-area
  height (respects the taskbar; logical-pixel math handles DPI).
- **Collapse**: mouse leaves → wait `hideDelay` (default 400ms) → collapse
  back to handle width. Re-entering cancels the pending collapse; a
  generation counter invalidates stale timers. An open variable dialog
  pins the dock open. Copy success collapses after a brief "Copied"
  feedback.
- **Placement**: `dock.position` = `left` | `right`, primary monitor's
  work area; `alwaysOnTop` optional (default true); toggles apply
  immediately, no restart.
- **Content**: Search (backend search) / Favorites / Recent, Enter to copy
  (variable dialog for templates), Esc to collapse — no CRUD/editor.
- **Lifecycle**: created hidden at startup, runtime show/hide/resize/
  reposition only — NEVER destroyed (same wry constraint as the
  launcher). Geometry commands are idempotent (skip when the width
  already matches) so enter/leave bursts cannot pile up a command backlog.
- **First version explicitly does NOT use a global mouse hook**: relies on
  the always-visible edge handle plus the window's mouse enter/leave.
- **No reserved screen-edge areas**: the collapsed handle is an ordinary
  small always-on-top window; reserved edges / work-area re-layout are
  future refinements.

## i18n

- vue-i18n (`legacy: false`), locale files `src/i18n/locales/{zh-CN,en-US}.ts`.
- Language state lives in `settings.json` (backend) — persisted and shared
  across windows; the frontend applies it immediately on change.
- First launch: `navigator.language` starts with `zh` → `zh-CN`, else
  `en-US`; the detected choice is persisted via `set_language`.
- Backend errors stay stable English strings; `localizeError()` in
  `src/i18n/index.ts` maps them to localized UI text.
- User data (title/content/category/tag/alias) is NEVER translated.

## Backup Format v1 (Phase 5A)

Stable interchange format for the Snippet library. Managed by
`src-tauri/src/backup.rs` — NOT a raw SQLite dump.

```jsonc
{
  "format": "snipdeck-backup",
  "version": 1,
  "exportedAt": "2026-08-20T12:34:56+08:00",   // RFC3339
  "snippets": [
    {
      "title": "...", "content": "...", "type": "PROMPT",
      "category": "...", "language": "...",
      "tags": [], "aliases": [],
      "favorite": false, "usageCount": 0,
      "createdAt": 1234567890, "updatedAt": 1234567890
    }
  ]
}
```

- `id` is intentionally absent: database ids are not a migration identity.
- UTF-8, pretty-printed (2-space indent); Unicode / multi-line content is
  preserved verbatim.
- `version` is the compatibility gate: future versions may extend the shape
  only by adding optional fields; `version = 1` files must keep loading.

## Import Strategy (Phase 5A)

First version is **Merge only** (no Replace All):

1. Parse + validate the ENTIRE file (format, version, every snippet) before
   any database write.
2. One transaction: for each snippet —
   - exact duplicate (`title` + `content` + `type` all equal) → skip;
   - otherwise → insert with a NEW id, preserving `favorite`, `usage_count`,
     `created_at`, `updated_at`, `tags`, `aliases`.
3. Any error → whole transaction rolls back; the library is never partially
   imported.
4. Result summary: `added` / `skippedDuplicates` / `failed` (failed is
   always 0 — failures abort the whole import).

App settings are deliberately NOT part of the backup; the backup migrates
the Snippet library only.

## App Data & Release

- SQLite database: `%APPDATA%\com.snipdeck.app\snipdeck.db`
  (created/opened at startup; seeded only when brand-new).
- Settings: `%APPDATA%\com.snipdeck.app\settings.json`
  (language / launcher / dock; serde defaults keep old files valid).
- Version policy: `0.1.0` is the single source in `package.json`,
  `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` — all three must
  stay in sync.
- Product identity: `SnipDeck`, identifier `com.snipdeck.app`.
- Release artifacts (after `tauri build`):
  - installer: `src-tauri/target/release/bundle/nsis/SnipDeck_0.1.0_x64-setup.exe`
  - portable exe: `src-tauri/target/release/snipdeck.exe`
- Bundle target: NSIS only (Windows-first). The installer does NOT delete
  `%APPDATA%\com.snipdeck.app` on uninstall — user snippets are never
  removed by uninstall/reinstall (upgrade-in-place preserves data).

## Icon

A simple generated placeholder icon (dark rounded square + "SD") is used for
now (`src-tauri/icons/*`). A professionally designed icon is a non-blocking
follow-up.
