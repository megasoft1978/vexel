# WO-RUST-V1-SAVE-01 result

Added a window-free `project_session` module to `vexel-app`. It saves and
opens `.vexel` projects through `vexel-project-io` (atomic publish, no direct
fs writes). The scene payload is the exact `V1Document::source()` bytes.

## History persistence
Undo/redo history is session-only. The v1 bundle has no history section, so
after reopen the document equals the saved state, is clean, and has
`history_count() == 0` and `history_cursor() == 0`. Saving does not reset the
stack: undo, save, redo works and `history_cursor` is unchanged by a save.
Dirty state follows the editor's saved-state marker (undo past the saved point
is dirty; returning to it is clean).

## Changes
- `vexel-app/src/project_session/{mod,actions,tests}.rs`: `ProjectSession`
  (`save`, `save_as`, `open`, `open_replacing`), `write_project` (also used by
  the MCP preview state), typed `SessionError` with `user_message()`,
  `shortcut_action` (Cmd+S, Shift+Cmd+S, Cmd+O), `unsaved_changes_prompt`.
- `vexel-v1-document`: additive `V1Document::from_source` (needed because the
  document could only be created blank; two tests). Root must be `svg` with
  integer width/height 1..=4096.
- `vexel-app/Cargo.toml`: path deps `vexel-project`, `vexel-project-io`.
  `Cargo.lock` needed the matching two-line edit for `--locked` (protected
  path; separate commit for the coordinator to keep or redo).
- `rust/fixtures/v1/save/foreign.svg`: unknown elements, foreign-namespace
  attribute, editor-metadata element.

## Unexecuted / gaps
- Live Save/Open panels and menu items are NOT wired: no native open/save
  panel or menu API exists in `vexel-platform-apple` (a protected crate).
  Only the pure shortcut mapping and prompt text are implemented and tested.
- Mid-write failure injection: the `vexel-project-io` hooks are private to
  that crate, so AC5/6 use a read-only directory (replace and create both
  fail, existing bytes unchanged, no temp file left).
- MCP save/open tools not added (out of scope); preview state tested with
  `write_project`/`open`.
- Non-integer or missing root width/height cannot be opened (typed
  `InvalidScene`).
