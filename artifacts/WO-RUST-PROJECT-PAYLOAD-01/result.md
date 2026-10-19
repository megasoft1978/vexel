# WO-RUST-PROJECT-PAYLOAD-01 candidate evidence

Worktree: `verify/WO-RUST-PROJECT-PAYLOAD-02`, based on revision-2 checkpoint `0a96a711`.

## Implementation

- Added bounded owned session copies of the exact manifest and timeline bytes and every validated asset name, media type, and data payload.
- Project open now attaches those bytes to the session. New documents and SVG imports use the existing generated manifest, `{}` timeline, and no assets.
- Session save and save-as atomically publish current scene bytes with retained payload. The public source-only `write_project` retains its default metadata and empty-assets behavior.
- Added tests for opaque JSON and multiple binary assets across open, scene edit, save, save-as, and reopen, plus source-only default payload.
- The project bundle decoder validates payload sizes/counts before the session copies them; limits include 1 MiB manifest, 64 MiB timeline, 1,024 assets, 512 MiB aggregate asset data, and 1 GiB bundle size.

## Verification

Commands were run from the task worktree:

| Exact command | Result |
| --- | --- |
| `cargo +stable test -p vexel-app --locked --offline` | Exit 0; 242 unit tests and all package integration tests passed; one measurement test ignored. |
| `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app -- --check` | Exit 0; no output. |
| `git diff --check` | Exit 0; no output. |

The test command emitted existing warnings for an unnecessary `mut` in `src/session/tests_playback.rs:140` and an unused import in `tests/ui_canvas_pointer.rs:11`; no test failed. All three owned Rust files are below 800 lines.
