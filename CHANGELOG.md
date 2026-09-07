# Changelog

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
