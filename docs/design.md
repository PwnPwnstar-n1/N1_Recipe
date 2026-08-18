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
| Launcher | `launcher`| Global-shortcut quick search + Enter copy  | hidden prototype (Phase 3) |
| Dock     | `dock`    | Edge-docked quick access panel             | planned only  |

Rules:

- The frontend bundle is shared: each window renders `App.vue` and picks its
  role from `getCurrentWindow().label`.
- Backend services are shared by ALL windows — no per-window duplication:
  `repo` (CRUD/search/views), `template` (variables/system values),
  `copy_snippet` (clipboard + usage_count), `settings`.
- `windows.rs` is the Window Service: role constants, `get()` by role,
  `create_launcher()`, `ensure_hidden_launcher()`, and lifecycle commands
  (`show_launcher` / `hide_launcher`). The launcher window is created hidden
  at startup and **never destroyed at runtime**: creating a webview from an
  IPC/sync-command thread deadlocks on Windows (upstream wry issue), so
  create happens only on the main thread during setup and show/hide is the
  only runtime transition.
- `TemplateVariableDialog` is props/emits-only and reusable from any window.
  The main window's `store.ts` (views/editor state) is main-window specific;
  the launcher will get its own lightweight composable reusing `api.ts`,
  `types.ts`, `i18n`, and the dialog component. `SnippetList`/`SearchBar`
  are main-window components; the launcher will use a compact list of its
  own (no premature extraction).

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

## Edge Dock Design (future, not implemented)

Goal: a slim always-available panel docked to the left or right screen edge.

- **Collapsed**: only a ~6–10px handle remains visible at the screen edge.
- **Expand**: mouse enters the handle → window expands to ~320–400px.
- **Collapse**: mouse leaves → wait `hideDelay` (~300–500ms) → collapse back
  to handle width.
- **Placement**: `dock.position` = `left` | `right`; use the display's work
  area (respect taskbar) for full-height positioning; `alwaysOnTop` keeps it
  above other windows.
- **Content**: Search / Favorites / Recent lists, sharing the same backend
  services and the variable dialog as the main window.
- **First version explicitly does NOT use a global mouse hook**: rely on the
  always-visible edge handle plus the Tauri window's mouse enter/leave
  events (`on_window_event`, `MouseEnter` / `MouseLeave` + a hide timer).
  A global hook is a later optimization only if real-world usage shows the
  enter/leave approach is insufficient.
- **First version also does not use reserved screen-edge areas**: the
  collapsed handle is an ordinary small always-on-top window; reserved
  edges / work-area re-layout are future refinements.

## i18n

- vue-i18n (`legacy: false`), locale files `src/i18n/locales/{zh-CN,en-US}.ts`.
- Language state lives in `settings.json` (backend) — persisted and shared
  across windows; the frontend applies it immediately on change.
- First launch: `navigator.language` starts with `zh` → `zh-CN`, else
  `en-US`; the detected choice is persisted via `set_language`.
- Backend errors stay stable English strings; `localizeError()` in
  `src/i18n/index.ts` maps them to localized UI text.
- User data (title/content/category/tag/alias) is NEVER translated.
