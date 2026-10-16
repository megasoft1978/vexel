---
id: WO-RUST-V1-DOCUMENT-SESSION-01
revision: 1
status: READY
milestone: V1-DRAWING
owner: project-owner
executor: gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 1614e1795c76c86614aa936cbe0886c496fbec7ab292967025639742cea7ca8c
parallel_safe: true
network: false
---
# Goal and frozen scope

Implement the first reusable V1 artwork-session model as a new, safe Rust
library `vexel-v1-document`. This is an application-domain component, not a UI
or renderer. It creates a bounded blank SVG document, inserts one validated
rectangle from document-space drag bounds through the accepted
`vexel-document-edit::Editor` API, exposes exact editable source bytes and
dirty/save state, and supports undo/redo. The module must preserve the
source-edit API's atomic history behavior and must not keep a second shape
model that can diverge from the SVG bytes.

## Required behavior

- `V1Document::new(width, height)` accepts dimensions in 1..=4096 and creates a
  valid SVG root with matching dimensions/viewBox and an initialized editor.
- Rectangle construction normalizes all drag directions; rejects nonfinite,
  zero-area, out-of-document or out-of-range values without changing source or
  history; formats coordinates deterministically; restricts fill to a valid
  six-digit RGB hex color; inserts one `<rect>` with the editor's bounded
  template API and generated ID as exactly one undoable edit.
- Expose dimensions, dirty state, explicit mark-saved, source materialization,
  undo/redo, and history counts as narrow safe APIs. Significant output
  allocation is fallible and size arithmetic checked. Errors retain actionable
  categories and do not mutate document state.
- Keep modules cohesive and under 800 lines. No `unsafe`, external dependency,
  UI/native code, renderer mock, placeholder or TODO.

## Acceptance evidence

Focused tests must prove blank SVG validity; drags in all four directions map
to the same normalized geometry; exact rectangle markup/attributes and one
history entry; undo/redo restore byte-identical source; save-state transitions;
and nonfinite, zero-area, out-of-bounds, malformed color and edit/allocation
failures leave bytes/history/dirty state unchanged. Use fixed expected values,
not test expectations computed from the implementation under test.

## Paths and ownership

Worker may edit only:
- `rust/crates/vexel-v1-document/**`
- `artifacts/WO-RUST-V1-DOCUMENT-SESSION-01/**` except immutable source-seed

Coordinator owns workspace membership/Cargo.lock, source seed, work order,
ledger, `STATE.md`, and `LEDGER.md`. All existing Rust source, fixtures,
oracles, manifests/locks, interfaces, previous evidence, original C sources,
and user data are protected. Candidate is isolated in
`.cache/WO-RUST-V1-DOCUMENT-SESSION-01/worker`; it may use a minimal offline
verification workspace with exact dependency sources to avoid unrelated
workspace resolution failures, but must hash-match the implementation under
test. Do not launch the app or run GUI/E2E. Do not create nested workers.

## Verification

Run focused locked/offline package check and tests, scoped rustfmt check,
`python3 scripts/wo_validate.py WO-RUST-V1-DOCUMENT-SESSION-01 --ready`,
`python3 scripts/ledger.py validate ledger.jsonl`, source-line and exact path/hash
audits, and `git diff --check`. Record exact results and any root-workspace
resolver limitation without treating isolated evidence as main integration.
Candidate acceptance requires independent review and later coordinator
registration/fresh main checks. No V1 UI, renderer, native project, animation,
MCP, or full-product completion claim follows from this order.
