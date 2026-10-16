**WO-RUST-SVG-VALUES-01 is implemented** in the isolated worktree and left uncommitted. The matching work order was at `work/WO-RUST-SVG-VALUES-01.md`; this checkout had no `frozenwork/` copy.

The crate now implements affine inverse and transform parsing, SVG number and length readers, preserveAspectRatio and viewBox math, and root, nested, and symbol mappings. Its bounded `AttributeList` adapts source-backed spans without XML decoding or a C shim. Focused tests cover defaults, limits, units, composition, and diagnostic offsets.

- `cargo +stable check -p vexel-svg-values --locked --offline` — passed.
- `cargo +stable test -p vexel-svg-values --locked --offline` — 12 passed.
- Work-order validation and ledger validation passed; `git diff --check` was clean.
- Rust 1.99.0 was used. No dependencies were added.

The behavior map and verification record are in [parity-map.json](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust2-svg-values/artifacts/WO-RUST-SVG-VALUES-01/parity-map.json) and [verification.txt](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust2-svg-values/artifacts/WO-RUST-SVG-VALUES-01/verification.txt). Full XML/SVG decoding, shapes, paths, scene traversal, and rendering remain outside this order.