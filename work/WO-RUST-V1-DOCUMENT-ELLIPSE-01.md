---
id: WO-RUST-V1-DOCUMENT-ELLIPSE-01
revision: 1
status: READY
milestone: V1-DRAWING
owner: project-owner
executor: gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 923827f43ddcba3de170c968fce815b46db439951bdc15cfb6b7d63f48a268fd
parallel_safe: true
network: false
---
# Add ellipse creation to the V1 source-owning document

Extend the accepted `vexel-v1-document` model with one bounded ellipse drag operation. This gives the V1 ellipse tool the same source-preserving, undoable edit contract as rectangles. Do not change the accepted rectangle behavior.

## Required behavior

- Add `V1Document::add_ellipse(start_x, start_y, end_x, end_y, fill)` using document-space drag endpoints and the existing `vexel-document-edit::Editor` template API.
- Normalize all four drag directions and emit one `<ellipse>` with deterministic `cx`, `cy`, `rx`, and `ry` values plus the validated six-digit RGB fill. Reuse bounded/fallible coordinate formatting; significant heap growth must remain fallible.
- Reject nonfinite, zero-area, out-of-document coordinates and invalid colors before mutation. Failure leaves source, history, cursor and dirty state unchanged.
- A successful insertion is exactly one undoable edit. Undo and redo restore byte-identical SVG source. Keep the editor source as the only shape model.
- Add fixed expected-output tests for exact ellipse markup and all four normalized drag directions, plus save/undo/redo and invalid-input atomicity.
- Keep all changed Rust source/tests below 800 lines. No unsafe code, new dependency, UI/native code, renderer, selection/move editing, path authoring or placeholder behavior.

## Ownership and paths

Worker edits only `rust/crates/vexel-v1-document/src/**` and `artifacts/WO-RUST-V1-DOCUMENT-ELLIPSE-01/**` except immutable `source-seed.json` and `scope.json`. Coordinator owns the work order, seed/scope files, workspace manifests/lock, `STATE.md`, `LEDGER.md`, and ledger. Use a fresh isolated candidate in `.cache/WO-RUST-V1-DOCUMENT-ELLIPSE-01/worker`; do not modify root Rust sources. Seed accepted root package/dependency inputs exactly, build candidate-locally, and use the existing `.cache/cargo` offline cache. No nested workers, source reset/delete, network, broad UI/E2E, desktop automation, commit or staging.

## Verification

Run and record exact output for candidate-local:
- `cargo +stable check -p vexel-v1-document --locked --offline`
- `cargo +stable test -p vexel-v1-document --locked --offline`
- `cargo +stable fmt --package vexel-v1-document -- --check`
- `python3 scripts/wo_validate.py WO-RUST-V1-DOCUMENT-ELLIPSE-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

Record source hashes, path/scope audit, source line limits, fixed expected test outputs, and any actual limitation. Candidate is not accepted until an independent review and fresh root package checks pass. This model-only order does not claim visible canvas drawing or V1 completion.
