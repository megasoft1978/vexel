# WO-RUST-SVG-PATH-COMPACT-OPERANDS-01 candidate result

Candidate: `.cache/WO-RUST-SVG-PATH-COMPACT-OPERANDS-01/worker`

## Changes

- `rust/crates/vexel-svg-path/src/reader.rs`: a second decimal point terminates an integer-led fractional number, allowing the following dot-leading operand to be parsed separately. Scanner state identifies that boundary.
- `rust/crates/vexel-svg-path/src/parser.rs`: tracks newly admitted non-arc compact decimal boundaries and non-endpoint arc boundaries, rejecting an additional implicit group after them. The pre-existing special case is retained only for arc endpoint operand 5.
- `rust/crates/vexel-svg-path/tests/number_separator.rs`: added direct, controlled, resumable, and full-XML-source span coverage for `M-.4.6L1.2.4Z`; exact number spans and values; rejection checks for `M.4..6` and `M0 0 L1.2.3 4 5`; accepted repeated arc after compact endpoint 5 (16 numbers and same edge count as explicit `A`); and rejection of a repeated arc after compact radius `M0 0 A2.3.4 0 0 1 4 5 2 3 0 0 1 6 7` at offset 6.
- `artifacts/WO-RUST-SVG-PATH-CONTROL-01/baseline-golden.txt`: candidate-local copy of the existing frozen path-control golden file, needed because the seeded candidate contains the parity test but not its included fixture.

## Verification

- Locked/offline `cargo +stable check --manifest-path "$PWD/Cargo.toml" -p vexel-svg-path`: PASS.
- Locked/offline `cargo +stable test --manifest-path "$PWD/Cargo.toml" -p vexel-svg-path`: PASS; 40 tests passed across unit, controlled, number-separator, parity, and path suites; 0 failed.
- Locked/offline real decoder regression `cargo +stable test --manifest-path "$PWD/Cargo.toml" -p vexel-svg-decode --test original_number_scene original_path_number_main_all_assertions_through_real_decoder_scene`: PASS.
- Scoped `rustfmt +stable --check` on changed Rust files: PASS.
- `git diff --check`: PASS.
- `python3 scripts/wo_validate.py WO-RUST-SVG-PATH-COMPACT-OPERANDS-01 --ready`: PASS; frozen spec SHA-256 `73173144addc6407ee1c564f1c6cc1e81f0325eca3421ef7c64f44532ba9b09a`.
- `python3 scripts/ledger.py validate ledger.jsonl`: PASS; 1767 entries.

Raw command output is in sibling `WO-RUST-SVG-PATH-COMPACT-OPERANDS-01-*.log` files in this directory. The decoder regression confirms original source offsets are preserved through the full XML source and real decoder scene. Its malformed dot-leading assertion passed.

## Changed Rust file hashes (SHA-256)

- `reader.rs`: `8ed5c38fd11b9f27e7da38e74d1fab3d47aa33fa93f2fe3fa646bf232a924596`
- `parser.rs`: `026800ac9920cd6c4fe6de21f71ba2669d561fbbf1185dac6710b43835ecf717`
- `number_separator.rs`: `b6e6f2f7b8e2c565cffd217bf31be2a8bc002283939dc6e2ceaf09ea19c30a68`
