# Changelog

## 0.2.3 - 2026-11-20

### Added / Improved

- Added independently configurable primary and backup global shortcuts with explicit registration diagnostics.
- Added Windows Mouse XButton1/XButton2 bindings, optional modifier combinations, and an opt-in matched-event consume mode.
- Expanded validated keyboard support through F24, navigation keys, Insert, Pause, Print Screen, and Windows-key combinations.
- Added shortcut recording, Launcher testing, manual re-registration, and separate input/display health indicators.
- Unified keyboard, mouse, tray, and Settings activation through one persistent Launcher service.

### Fixed

- Preserved the working shortcut and saved settings when a replacement shortcut fails to register.
- Added bounded 2/5/15-second recovery attempts for missing registrations without repeatedly churning healthy shortcuts.
- Fixed duplicate XButton delivery toggling Launcher twice with matched-event debouncing.
- Fixed preserved mouse events immediately stealing focus back after Launcher activation.
- Improved privacy-safe diagnostics for registration conflicts, input receipt, window display, focus, and latency.

### Compatibility / Known Limitations

- Retains `com.snipdeck.app`, `snipdeck.db`, existing Recipes, categories, and legacy `launcher.shortcut` settings.
- Windows cannot override another application or the system when a global shortcut is already registered.
- Physical gaming-mouse drivers, UAC secure desktop, RDP, sleep/resume, and all multi-display/DPI combinations are not comprehensively verified.

## 0.2.2 - 2026-09-10

### Added / Improved

- Recipe export now supports selecting whole categories or individual recipes.
- Category checkboxes expose selected, unselected, and indeterminate states, with select-all and clear-all actions.
- The Quick Launcher now supports per-step Command Group copying and newline-separated Copy All.
- Added privacy-safe local diagnostics for launcher creation, shortcut registration, show, hide, focus, close, destruction, and errors.

### Fixed

- Fixed the global shortcut permanently stopping after the Quick Launcher was closed on some Windows machines.
- Fixed the launcher being destroyed while the registered shortcut continued targeting its missing window.
- Hardened launcher show, hide, focus, and lifecycle error handling.

## 0.2.1 - 2026-09-08

- Fixed the main window being restored as a tiny transparent Dock-like strip.
- Kept main, Dock and launcher window roles and geometry state strictly isolated.
- Added automatic recovery for invalid, undersized, off-screen and DPI-invalid main-window geometry.
- Preserved the database, snippets, categories, scratch items, recipes and all non-window settings during recovery.

## 0.2.0 - 2026-09-07

- Renamed the user-facing product to N1_Recipe while retaining the existing bundle identifier and data directory for seamless upgrades.
- Reworked the warm plum visual system and semantic four-level typography.
- Fixed Settings scrolling in constrained windows and extra-large text.
- Added explicit Dock pins and command-group expansion with per-step and copy-all actions.
- Added the stable, human-readable N1 recipe-pack v1 format, schema, example and community authoring guidance.
- Replaced application icons with the original N1 recipe-card mark.
- Feature freeze begins after this release; the next cycle is limited to testing, interaction polish and bug fixes.

## 0.1.1 - 2026-09-07

- Fixed long snippet lists stretching the window and hiding create/copy/edit/delete actions.
- Added create, rename and safe-delete category management; deleting a category moves its snippets to Uncategorized.
- Made the Quick Launcher resizable and added separate persisted geometry for main and launcher windows, with visible-screen recovery.
- Added small, standard, large and extra-large font settings.
- Added Unicode template variables with whitespace normalization and literal-safe replacement.
- Added the persistent local Scratch area with autosave, copy, delete and confirmed clear.
- Added ordered command-group snippets, multiline import, reordering, per-command copy and copy-all.
- Added light, dark and follow-system themes using shared design tokens.
- Added an in-place SQLite v2 migration. v0.1.0 snippets remain available and default to ordinary text.
