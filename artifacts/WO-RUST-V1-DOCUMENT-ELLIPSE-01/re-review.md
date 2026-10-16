# Independent re-review — WO-RUST-V1-DOCUMENT-ELLIPSE-01

**Result: PASS for the remediation logic, with one focused-test coverage gap noted below.** No implementation files were changed during this review.

## Remediation check

`add_ellipse` computes `rx` and `ry` and rejects `rx <= 0.0 || ry <= 0.0` at `worker/rust/crates/vexel-v1-document/src/lib.rs:205-209`. This guard runs before numeric formatting, root lookup, and the only mutation call, `Editor::insert_template` at lines 238-243. Coordinates have already been required finite and within the document bounds, so subtraction cannot overflow to an infinite radius. A smallest-positive-subnormal span on either axis halves to zero and is rejected before mutation.

The focused atomicity test at `worker/rust/crates/vexel-v1-document/src/tests.rs:205-250` passes `f64::from_bits(1)` as the X span, expects `InvalidGeometry`, and verifies source bytes, history count/cursor, and dirty state are unchanged. I independently ran:

- `CARGO_HOME=/Users/megasoft78/Desktop/Freelance/vexel/.cache/cargo cargo +stable test -p vexel-v1-document --locked --offline tests::invalid_ellipse_inputs_are_atomic -- --exact` — **1 passed**.

## Axis coverage gap

The focused subnormal regression exercises X (`rx`) only. The table's zero-height case is rejected by the earlier equal-endpoint check, so it does not exercise a subnormal `ry` underflow. The implementation checks both radii in one guard, and the Y behavior is symmetric, but no focused test currently demonstrates the Y-underflow branch. A Y-axis subnormal atomicity case would close this evidence gap. This is not a code defect found in the remediation.

## Scope and surrounding contract

The candidate diff against the frozen root seed adds only `add_ellipse` in `vexel-v1-document/src/lib.rs` and ellipse tests in `src/tests.rs`; rectangle code is unchanged. The worker copies of `source-seed.json` and `scope.json` compare byte-for-byte with the frozen root copies. The seeded document manifest and both `vexel-document-edit` source files match their frozen SHA-256 values. Candidate hashes: `lib.rs` `29b7f94e02db808a431015bf48dcb029814c590e3fd6539a0091cdf0d56adecb` (288 lines); `tests.rs` `6313c3d09ec702ce871c8cf3e0b381f6aedd2f8e7b259f5839676c6cb68a0a9e` (251 lines). Both remain below 800 lines.

`TemplateKind::Ellipse` maps to the `<ellipse>` tag and `HistoryAction::AddEllipse`; `insert_template` prepares markup before `insert_prepared` mutates the editor. Existing fixed-output tests cover all four drag directions, and undo/redo tests check exact source restoration. No unrelated implementation path was changed.

## Appendix — Y-axis subnormal regression re-review

The updated `invalid_ellipse_inputs_are_atomic` table adds `(sx, sy, ex, ey) = (0.0, 0.0, 1.0, f64::from_bits(1))` at candidate `src/tests.rs:240-247`. Its X span is 1.0, so `rx` is positive; the Y span is the smallest positive subnormal, so `ry = span / 2.0` underflows to zero and specifically exercises the Y-radius rejection. The shared atomicity assertions then verify unchanged SVG bytes, zero history count/cursor, and clean state. This closes the previously noted Y-axis evidence gap.

Updated test-file SHA-256: `5569631528579f35e307c39731071f3cdd432efb290b5b7c9190f08f658074b6`. The diff adds only this Y-axis tuple to the existing test; no implementation file changed. Focused rerun passed: `CARGO_HOME=/Users/megasoft78/Desktop/Freelance/vexel/.cache/cargo cargo +stable test -p vexel-v1-document --locked --offline tests::invalid_ellipse_inputs_are_atomic -- --exact` — **1 passed**.
