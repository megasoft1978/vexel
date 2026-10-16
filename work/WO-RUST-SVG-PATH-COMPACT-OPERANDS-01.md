---
id: WO-RUST-SVG-PATH-COMPACT-OPERANDS-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 73173144addc6407ee1c564f1c6cc1e81f0325eca3421ef7c64f44532ba9b09a
parallel_safe: true
network: false
---
# Goal and frozen scope
Repair a real decoder compatibility failure in the Rust SVG path number scanner. The frozen C source `src/svg/shape/shape.c` treats a dot after a completed number as an operand boundary. The frozen fixture `tests/svg/shape/path_number_test.c` accepts compact path `M-.4.6L1.2.4Z`, including `1.2` followed by `.4`. Current Rust rejects the latter at absolute offset 42 in the real decoder. Restore the compact operand boundary while preserving numeric parsing, error spans, parser controls/quantum charging, and all other path behavior.

Keep the exact malformed C fixture `M.4..6` rejected at its expected absolute diagnostic offset. Preserve the prior accepted malformed case `M0 0 L1.2.3 4 5`, valid adjacent dot-leading values `.6.5`, exponent/sign rules, and every existing assertion. Add focused Rust regressions for direct parse, controlled parse and resumable session as appropriate, then run the existing real-decoder path-number test. Do not weaken either frozen oracle.

# Candidate and ownership
Coordinator seeds an isolated exact-current Rust candidate and source hash manifest. Worker may change only:
- `rust/crates/vexel-svg-path/src/reader.rs`
- `rust/crates/vexel-svg-path/src/parser.rs` only if needed for precise operand-boundary behavior
- `rust/crates/vexel-svg-path/tests/controlled.rs` and other focused path-number Rust test files
- candidate-local work-order evidence

Do not alter frozen C source/tests, shared workspace manifests/locks/interfaces, other crates, STATE, ledgers or work orders. No new dependency, unsafe code, network, GUI, broad UI/E2E. Keep each Rust source file below 800 lines. One writer; candidate remains isolated until coordinator review and integration.

# Verification
Run from the candidate workspace with candidate-local Cargo home and target dir; keep all generated state inside the repository. Run locked/offline `cargo +stable check -p vexel-svg-path` and `cargo +stable test -p vexel-svg-path`; scoped rustfmt check on changed Rust files; `python3 scripts/wo_validate.py WO-RUST-SVG-PATH-COMPACT-OPERANDS-01 --ready`; `python3 scripts/ledger.py validate ledger.jsonl`. Also run the existing focused real-decoder test `original_path_number_main_all_assertions_through_real_decoder_scene` if candidate dependency closure permits. Record any workspace resolver blocker separately from package results. Preserve exact test output, hashes, error offsets, parity behavior, and changed paths. No broad UI suite.
