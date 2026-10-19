# WO-RUST-PROJECT-PAYLOAD-01 candidate evidence

Worktree: `verify/WO-RUST-PROJECT-PAYLOAD-02`

Frozen revision-3 work-order base SHA: `0a96a7110c9a9acf326dbf54dfc4d5de9a6e64b0`

Runner start checkpoint: `61e64ad0a432d84abeaf581148d55bfeb17899b0`
Candidate before review fix: `489a7fe3a9cb5dc0212f5b7bbb2bcdaff2869c91`

## Implementation

- Retain exact manifest, timeline, and asset name, media type, and data bytes in the session. Project opens copy each field after fallible reservation; allocation failure maps to `SessionError::OutOfMemory` before the replacement session is installed.
- New documents and SVG imports keep the generated manifest, `{}` timeline, and no assets. Session saves atomically publish the current scene with retained payload. Source-only `write_project` keeps its default metadata and empty-assets behavior.
- Added a narrow test-only reservation gate and deterministic regression that fails during asset copying, then verifies `open_replacing` reports `OutOfMemory` while the existing document source, history, dirty state, and path remain unchanged.
- Existing tests cover opaque payload bytes across open, edit, save, save-as, and reopen, plus defaults, failed opens/saves, and SVG non-overwrite.

## Verification

Commands were run from the task worktree after the review fix:

| Exact command | Result |
| --- | --- |
| `cargo +stable test -p vexel-app --locked --offline` | Exit 0; 243 library unit tests passed, 1 measurement test ignored, main and all package integration tests passed, doc tests passed. |
| `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app -- --check` | Exit 0; no output. |
| `git diff --check` | Exit 0; no output. |

The test command reported pre-existing warnings for an unnecessary `mut` in `src/session/tests_playback.rs:140` and an unused import in `tests/ui_canvas_pointer.rs:11`. No files outside the work-order allowlist were changed; the three Rust source files remain below 800 lines.
