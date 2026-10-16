Repaired the cached flow: hits borrow the actual stop slice, cold buffers move into the shared cache with accounting, and public fill/stroke methods use real SceneBuilder publication.

All 16 prior tests remain. **22 Cargo tests and 3 graph tests pass**, including 40,000 instances sharing two stops, capacity accounting, cancellation and rollback. Revision 2 validation passes.

One shared gate remains: owned-only `DecodeOperations::resolve_paint` cannot return borrowed stops. Root must adopt the cached handoff or repair that contract. The compatible owned path now charges its copies explicitly.

Evidence: [result.md](/Users/megasoft78/Desktop/Freelance/vexel/.cache/WO-RUST-SVG-PAINT-01/worker/artifacts/WO-RUST-SVG-PAINT-01/result.md), [verification.json](/Users/megasoft78/Desktop/Freelance/vexel/.cache/WO-RUST-SVG-PAINT-01/worker/artifacts/WO-RUST-SVG-PAINT-01/verification.json), and [exact shared gap](/Users/megasoft78/Desktop/Freelance/vexel/.cache/WO-RUST-SVG-PAINT-01/worker/artifacts/WO-RUST-SVG-PAINT-01/owned-interface-gap.md). Candidate acceptance remains pending independent root review.