# SVG engine adapter worker report — revision 2

## Implementation

Added `vexel-svg-engine` with no external dependencies. `SvgEngineOperations` owns a `PaintCache`, calls `vexel_svg_paint::resolve_cached_fill`, and returns borrowed stops through Scene's `CachedPaint` publication contract. Both definition interfaces delegate to the actual `vexel_svg_definitions::DefinitionPlanner`, including the required Fill/Stroke pattern channel. Media requests return a typed `Unsupported` diagnostic at the actual request element token's source offset, indexed within the document token slice. `SvgEngineSession` wraps the actual resumable `Decoder`, exposes bounded stepping, exact-ticket pattern resume, state, and guarded finish.

## Real decoder evidence

The engine integration tests use the actual `Decoder` and assert:

- 2 vector shapes plus transform and solid fill/stroke channel data; source bytes and source-owner identity survive finish.
- A local linear gradient resolves through the actual cached paint resolver and appears as a two-stop scene fill.
- Actual text and image requests prefixed by earlier document elements each return `Unsupported` at the nonzero source offset of the requested element. These use the real Decoder route.
- An actual clip-definition request returns `InvalidArgument` at offset 0 and `blocked_contract()` remains `None`. The test does not establish the failure stage or cause. Actual planner success through this adapter remains unproven.

Text and images are explicitly unsupported by the adapter; they are never reported as successfully decoded.

## Verification

All commands used candidate-local Cargo cache and target directory, offline and locked:

- `cargo +stable check -p vexel-svg-engine --locked --offline` — PASS.
- `cargo +stable test -p vexel-svg-engine --locked --offline` — PASS, 5 integration tests, including text and image errors at token source offsets 32 and 32 for the tested documents.
- `cargo +stable check -p vexel-svg-decode --locked --offline` — PASS.
- `cargo +stable test -p vexel-svg-decode --locked --offline` — PASS, all test binaries passed.
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-svg-engine -- --check` — PASS.
- `python3 scripts/wo_validate.py WO-RUST-SVG-ENGINE-ADAPTER-01 --ready` — PASS; revision 2 digest `1d206e37a684eaef1ddbf7121af66a2fa801994fbb250d067508d5b3ad487647`.
- `python3 scripts/ledger.py validate ledger.jsonl` — PASS, 2 entries.

## Scope and hashes

Worker-owned changed paths:

- `rust/crates/vexel-svg-engine/Cargo.toml` — 17 lines; SHA-256 `83d2a281bf2aa1ed35607228cb55aadb28be2824505af5dddb984551cdadc367`.
- `rust/crates/vexel-svg-engine/src/lib.rs` — 225 lines; SHA-256 `ded6da2152dd647ec7f55d45b2a3fab6b0a6f86dae82437bcb2ef780cb20705e`.
- `rust/crates/vexel-svg-engine/tests/decoder.rs` — 205 lines; SHA-256 `2c17105731a8d63674eec6cf49f26b0ce5d73c372b8a8ab79e3cea8a0397f573`.
- `artifacts/WO-RUST-SVG-ENGINE-ADAPTER-01/engine-worker-report.md` — this report.

Every engine Rust source/test file is below 800 lines. New dependency list is path-only: `vexel-scene`, `vexel-svg-paint`, `vexel-svg-definitions`, `vexel-svg-decode`, `vexel-base`, `vexel-geometry`, `vexel-svg-css`, and `vexel-xml`. The frozen setup seed file hash observed was `254b7865dba8eaf5ffb2a64cf3404c78ab941545d7d93a5087ba2489d13eac89` (`source-seed-r2.json`); it was not modified. Candidate root manifest/lock registration was coordinator-owned and is not included in worker edits.

## Focused follow-up verification and final hashes

After the media-offset correction: `cargo +stable test -p vexel-svg-engine --locked --offline` — PASS (all 5 integration tests). The text/image regressions supply a MediaProvider to reach the adapter callbacks and compare error offsets against the actual lexical start-token `source.offset`; both expected offsets are nonzero (32).

The hashes in the Scope and hashes section are post-fix hashes and include the media offset correction plus both actual Decoder media regressions.

## Remaining gates

The clip planner test only records the observed `InvalidArgument` result; it does not establish the planner call boundary or root cause. Decoder-wide frozen parity and remaining production composition gates remain separate; this crate is a candidate only, pending independent review and fresh main-tree checks after prerequisite acceptance.
