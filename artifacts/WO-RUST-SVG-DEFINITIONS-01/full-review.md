**BLOCKER — marker dimension error parity**

[marker.rs](/Users/megasoft78/Desktop/Freelance/vexel/.cache/WO-RUST-SVG-DEFINITIONS-01/worker/rust/crates/vexel-svg-definitions/src/marker.rs:116) returns `ErrorKind::Number` when `markerWidth` or `markerHeight` is nonpositive. The original [marker.c](/Users/megasoft78/Desktop/Freelance/vexel/.cache/WO-RUST-SVG-DEFINITIONS-01/worker/src/svg/shape/marker.c:274) returns `SVG_SHAPE_ERROR_LIMIT` for those same values. The parity map lists no accepted contract limitations, and the frozen order requires preserving original error behavior. Correct this mismatch before accepting the owned-source candidate.

Read-only review only; no build or tests run.