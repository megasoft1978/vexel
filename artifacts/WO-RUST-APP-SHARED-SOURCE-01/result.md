# WO-RUST-APP-SHARED-SOURCE-01 result

## Outcome

Candidate implementation verified on revision-2 checkpoint `0a96a7110c9a9acf326dbf54dfc4d5de9a6e64b0`. All frozen commands passed. The package suite completed with zero failures; one measurement-only integration test was ignored by its existing configuration. No acceptance claim is made about the 100 MiB first-visible-content target or the complete large-document goal.

## Changed paths

- `rust/crates/vexel-app/src/compose/mod.rs`
- `rust/crates/vexel-app/src/compose/decode.rs`
- `rust/crates/vexel-app/src/compose/worker.rs`
- `rust/crates/vexel-app/src/compose/tests_open.rs`
- `artifacts/WO-RUST-APP-SHARED-SOURCE-01/result.md`
- `artifacts/WO-RUST-APP-SHARED-SOURCE-01/report.json`

The four changed source paths and two evidence paths are within the assigned allowlist. The four source files contain 185, 205, 133, and 357 lines respectively (each under 800 lines).

## Implementation evidence

- `ComposeWorker` passes its existing `Arc<Vec<u8>>` to `compose_document_owned`.
- The owned compose route passes that allocation to `decode_owned`, which gives an Arc clone of the same allocation to `LexicalDocument::parse`; it makes no source-sized copy.
- The borrowed public compose APIs remain available and adapt at the boundary with a single owned copy. Source cap, UTF-8 mapping, token budget, charge ledger, cancellation control, diagnostics, and render flow remain in place.
- The regression test compares borrowed and owned routes' complete frame including disclosures, shape debug output and hit behavior, and checks source allocation identity.
- Existing latest-generation and cancellation worker tests remain unchanged.

## Frozen verification

| Command | Result |
|---|---|
| `cargo +stable test -p vexel-app --locked --offline` | exit 0; 241 library tests passed, all integration targets passed, 1 measurement-only test ignored; doc tests passed |
| `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app -- --check` | exit 0 |
| `git diff --check` | exit 0 |

The test command emitted two existing unused-code warnings in protected files: an unnecessary `mut` in `src/session/tests_playback.rs` and an unused import in `tests/ui_canvas_pointer.rs`. No protected files were edited.

## Review constraints

Changed-path scope was checked against the revision-2 allowlist. No manifests, lockfiles, shared interfaces, fixtures, goldens, work orders, ledgers, or protected source/tests were changed. No claim is made that this single-copy reduction meets the 100 MiB first-visible-content target or the full large-document goal.
