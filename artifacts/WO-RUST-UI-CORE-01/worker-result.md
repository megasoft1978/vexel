Implemented WO-RUST-UI-CORE-01 in the owned Rust crate, with focused parity tests and artifact records. No manifests, locks, C sources, original tests, coordinator records, or unrelated paths were edited. Changes are uncommitted.

- `cargo +stable check -p vexel-ui --locked --offline` passed.
- `cargo +stable test -p vexel-ui --locked --offline` passed: 7 tests.
- The work-order validator, ledger validator, and `git diff --check` passed.
- Rustfmt and Clippy were not run.

See [parity-map.json](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust2-ui/artifacts/WO-RUST-UI-CORE-01/parity-map.json) for symbol coverage, API adaptations, and remaining scope, and [verification.txt](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust2-ui/artifacts/WO-RUST-UI-CORE-01/verification.txt) for check evidence. This covers the named core behavior; it makes no desktop UI or whole-application parity claim.