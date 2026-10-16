Corrected malformed fill span handling: if `XmlSpan::slice` fails, `classify_inner` now returns `Unsupported` at the supplied fill offset, matching the original behavior.

Added regression coverage for five malformed span forms and valid local URL spans across `classify_fill`, cached resolution, and owned resolution. The tests check exact error kind and offset, unchanged cache and ledger accounting, and cancellation priority.

Fresh locked offline checks passed: 23 crate tests, 3 graph tests, formatting, and WO validation. All source and test files remain under 800 lines. The result report is 310 words.

The candidate still awaits independent review and shared production delegate adoption. Mixed renderer and application gates remain pending.

- [Resolver change](/Users/megasoft78/Desktop/Freelance/vexel/.cache/WO-RUST-SVG-PAINT-01/worker/rust/crates/vexel-svg-paint/src/resolver.rs:144)
- [Regression test](/Users/megasoft78/Desktop/Freelance/vexel/.cache/WO-RUST-SVG-PAINT-01/worker/rust/crates/vexel-svg-paint/tests/resolver_parity.rs:65)
- [Repair report](/Users/megasoft78/Desktop/Freelance/vexel/.cache/WO-RUST-SVG-PAINT-01/worker/artifacts/WO-RUST-SVG-PAINT-01/span-parity-repair.md)
- [Verification evidence](/Users/megasoft78/Desktop/Freelance/vexel/.cache/WO-RUST-SVG-PAINT-01/worker/artifacts/WO-RUST-SVG-PAINT-01/verification.json)