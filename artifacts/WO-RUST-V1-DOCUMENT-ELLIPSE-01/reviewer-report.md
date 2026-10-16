# Independent candidate review — WO-RUST-V1-DOCUMENT-ELLIPSE-01

**Result: BLOCKER — tiny nonzero drag dimensions can produce a zero SVG radius.**

## Finding

`add_ellipse` accepts any pair of distinct finite in-bounds endpoint coordinates, then computes each radius as `(max - min) / 2.0`. For a legal drag from `0.0` to `f64::from_bits(1)` on an axis, the input span is positive but halving the smallest positive subnormal underflows to `0.0`. `numeric_text` then serializes that zero successfully (`0e0`) and the editor commits an ellipse with `rx="0e0"` or `ry="0e0"`. This violates the required zero-area rejection / meaningful ellipse contract even though the endpoint equality check passed. Add a pre-mutation check that both calculated radii are finite and strictly positive, returning `InvalidGeometry`, plus an atomicity test using the smallest positive subnormal as a drag span. No implementation was changed during this review.

## Reviewed behavior

- For ordinary positive spans, endpoint min/max normalization gives the same center and radii for all four drag directions. The emitted attributes are `cx`, `cy`, `rx`, `ry`, then `fill`; `TemplateKind::Ellipse` delegates markup creation and generated ID to the existing `Editor` template path. The fixed expected-output test covers that markup exactly.
- Existing validation rejects non-finite coordinates, negative/out-of-document coordinates, equal endpoints on either axis, and malformed RGB fill before calling the editor. Numeric buffer and vector allocations are fallible and happen before mutation. The editor template insertion owns the one-edit/history behavior; undo/redo tests check byte-identical source and save-state transitions.
- Tests cover all four direction combinations, ordinary invalid geometry/colors, dirty/history atomicity for those errors, exact source output, and save/undo/redo. The ellipse tests do not cover underflow radii or editor-allocation failures; those omissions align with the finding.
- `lib.rs` and `tests.rs` are below 800 lines per the worker evidence. The candidate implementation edits are confined to the allowed Rust source paths. The worker report is under the candidate artifact directory. Source-seed/scope files were inspected as immutable inputs.

## Exact files inspected

- `work/WO-RUST-V1-DOCUMENT-ELLIPSE-01.md`
- `artifacts/WO-RUST-V1-DOCUMENT-ELLIPSE-01/source-seed.json`
- `artifacts/WO-RUST-V1-DOCUMENT-ELLIPSE-01/scope.json`
- `.cache/WO-RUST-V1-DOCUMENT-ELLIPSE-01/worker/rust/crates/vexel-v1-document/src/lib.rs`
- `.cache/WO-RUST-V1-DOCUMENT-ELLIPSE-01/worker/rust/crates/vexel-v1-document/src/tests.rs`
- `.cache/WO-RUST-V1-DOCUMENT-ELLIPSE-01/worker/rust/crates/vexel-document-edit/src/insert.rs` (template tag, generated ID, and insertion path)
- `.cache/WO-RUST-V1-DOCUMENT-ELLIPSE-01/worker/artifacts/WO-RUST-V1-DOCUMENT-ELLIPSE-01/worker-report.md`
