# WO-RUST-SVG-DEFINITION-VIEWS-01 candidate report

## Changes

- Definition planner reads for owner shapes and viewport data now use the active `traversal_scene(ctx.scene)` view. Optional viewport data is checked and reported through the planner's existing diagnostic path.
- Expansion helper also uses the active traversal view for viewport dimensions.
- The engine's real decoder clip test runs the shared decode helper, requiring successful `step()` completion and `finish()`, then asserts one published clip and a valid clip reference on the owner shape.
- Source lengths: planner.rs 767 lines; planner_support.rs 357 lines; decoder.rs 178 lines.

## Verification

All commands below were rerun with `CARGO_HOME=.cache/cargo` and `CARGO_TARGET_DIR=.cache/WO-RUST-SVG-ENGINE-ADAPTER-01/current/target` (the prepared candidate target path). Exact output is in `final-verification.log`.

| Command | Exit | Result |
|---|---:|---|
| `cargo +stable check -p vexel-svg-definitions --locked --offline` | 0 | Passed. |
| `cargo +stable test -p vexel-svg-definitions --locked --offline` | 0 | Passed. |
| `cargo +stable check -p vexel-svg-engine --locked --offline` | 0 | Passed. |
| `cargo +stable test -p vexel-svg-engine --locked --offline` | 0 | Passed; `actual_clip_planner_path_completes_and_publishes_owner_clip` passed. |
| `cargo +stable test -p vexel-svg-decode --locked --offline` | 0 | Passed. |
| `cargo +stable fmt --manifest-path Cargo.toml --package vexel-svg-definitions --package vexel-svg-engine -- --check` | 0 | Passed. |
| `python3 scripts/wo_validate.py WO-RUST-SVG-DEFINITION-VIEWS-01 --ready` | 0 | Passed; frozen work-order spec hash validated. |
| `python3 scripts/ledger.py validate ledger.jsonl` | 0 | Passed; ledger valid (2 entries). |

The initial attempt without the candidate Cargo environment failed before compile due to missing offline index entry `objc2-image-io`. This was resolved by using the provided candidate cache and target directory; all final commands passed.

## Hashes and scope

SHA-256 after final edits:

- `rust/crates/vexel-svg-definitions/src/planner.rs`: `ab605f181d878d3f76919b980315d0375994de99440557ec2746b1541ce39693`
- `rust/crates/vexel-svg-definitions/src/planner_support.rs`: `8059e9988a9126cdaa92a3a0e60ed2df70bb8540c97b9b53deba7ceb133b4bc2`
- `rust/crates/vexel-svg-engine/tests/decoder.rs`: `6617a9af0d9c572d2a7b36c4e9aeef802a23f5cc5b0eaef7a5faa5675811f3e4`
- Frozen seed copied from root: `artifacts/WO-RUST-SVG-DEFINITION-VIEWS-01/source-seed.json`, SHA-256 `895b301ae535bf2288290639522b6514030d84eb13fc68318a8234d4fa54fe5f`.

Worker-edited source paths are exactly those three allowed files. Candidate evidence files are `source-seed.json`, `final-verification.log`, and this report. No manifest, lockfile, fixture, decoder source, or peer crate was changed.

## Immutable-input audit and behavior evidence

Audited all 436 immutable entries in the frozen seed manifest, excluding exactly the three allowed worker source paths. Every entry matched its frozen size and SHA-256; zero missing or mismatched files. The copied frozen seed was not modified.

On the clip route, the decoder emitted `Progress` events (the shared helper asserts every progress event carries `charged_units > 0`), then `Complete`; `finish()` returned `DecodedScene::Drawable`. The passing engine test observed one owner shape, one published clip, and that the owner's `clip` ID resolves to a clip in the scene. Full SVG parity and full decoder acceptance are not claimed.
