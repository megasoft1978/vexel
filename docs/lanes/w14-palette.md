# W14 palette and undo history

## Changes

- Added a bounded command descriptor table shared by the Cmd+K palette, native
  menus, Help > Keyboard Shortcuts, and toolbar shortcut tooltips. The palette
  searches names, groups, and shortcuts, shows disabled reasons, caps results
  at eight visible rows, and keeps each row label below 64 bytes.
- Added the centered palette overlay with keyboard and pointer focus, arrow
  navigation, Return to run, and Escape to close. Commands that need a
  selection remain disabled until their requirements are met.
- Added named document edit history with current and saved state markers,
  exact cursor jumps, a Start row, and a virtualized 20-row Undo History
  popover. The existing 1,000-command and 64 MiB history bounds remain in
  effect. Move, resize, rotate, duplicate, and delete actions receive distinct
  labels.
- Added headless interaction tests and light/dark palette and history goldens.
  Extracted multi-selection edits into `viewer_editor_multi_edit.c` to keep
  source files below 800 lines.

## Verification

- `./dev build`: passed.
- `./dev t --all`: 58/60 CTests passed; all 69 Python tests passed. The two
  CTest failures were `index-replay` (offscreen Metal device/queue unavailable)
  and `native-viewer-smoke`.
- `editor-command-palette`, `document-edit-history`, `editor-chrome`, and
  `ui-editor-chrome` CTests passed. Palette/history light and dark goldens and
  both chrome golden checks passed.
- Negative controls confirmed the new tests detect regressions: disabling the
  Cmd+K open path failed `editor_command_palette_test`; reversing the history
  jump direction failed `document_edit_history_test`.
- `./dev shape-fuzz --duration 2 --cases 1000000 --seed 20261011`: passed
  48,128 cases under ASan+UBSan, with zero crashes, sanitizer reports, or
  invariant failures.
- `./dev shot tests/realworld/fixtures/material/edit.svg --size 512x512
  --out /private/tmp/w14-palette-shot-final.png`: passed; produced a 512×512
  PNG. `pixdiff` reported identical palette-dark and history-light renders
  against their goldens at tolerance 0.
- `python3 scripts/ledger.py validate ledger.jsonl`: 406 entries valid.
  No lifecycle event was added because UX-05 and UX-08 are supplied from the
  research scratchpad and are not frozen work orders under `work/`.
- `git diff --check`: passed.

## Maintainer real-window verification

The real macOS window was not driven or captured in this lane. Please verify
Cmd+K open/search/keyboard and pointer operation, disabled command behavior,
Help and menu shortcut consistency, toolbar tooltips, and the history popover's
current/saved markers, Start row, older-row scrolling, exact restore, and redo
branch clearing. Check both light and dark appearance. `native-viewer-smoke`
also needs a successful run in the maintainer's normal GUI session.
