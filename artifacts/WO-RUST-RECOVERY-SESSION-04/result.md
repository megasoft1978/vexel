# WO-RUST-RECOVERY-SESSION-04 result

Coordinator-reported package and workspace verification results are recorded below. GUI behavior and deferred recovery features are not covered by these checks.

## Acceptance

See [ac-map.md](ac-map.md). Verification commands listed below passed as reported; GUI and explicitly deferred behavior are not established by these results.

## Verification record

Record exact command, exit code, and concise real output for every run. Link full logs when captured.

| Command | Exit | Observed output / log |
|---|---:|---|
| `cargo test -p vexel-app --locked --quiet` | 0 | Passed: library 257 passed; binary 1 passed; all integration targets passed; 1 ignored (`scene_limits_measure::print_limit_measurements`). |
| `cargo test -p vexel-platform-apple --locked` | 0 | Passed earlier: 51 unit tests, 17 parity tests, plus integration and doc-test targets. Exact total/output not supplied. |
| `git diff --check` | 0 | Passed. |
| `rustfmt --edition 2024 --check --config skip_children=true <changed Rust files>` | 0 | Passed across all changed Rust source and test files. |
| `python3 -m unittest tests.test_ledger tests.test_wo_runner` | 0 | 51 tests passed. |
| `python3 scripts/ledger.py validate ledger.jsonl` | 0 | Validated 2,182 entries. |
| `cargo check --workspace --locked` | 0 | Passed. |
| `cargo test --workspace --locked --quiet` | 0 | 2,093 registered; 2,091 ran and passed; 2 ignored; 0 failures. Ignored: `vexel-app/tests/scene_limits_measure.rs::print_limit_measurements`, `vexel-mcp/tests/fuzz_stdio.rs::reproducer`. |
| `python3 scripts/wo_validate.py --ready WO-RUST-RECOVERY-SESSION-04` | 0 | Revision 1 readiness passed. |

Focused app rerun passed: library 257 tests, binary 1 test, and all integration targets. The platform run passed 51 unit tests and 17 parity tests, plus integration and doc-test targets; exact aggregate output was not supplied. Workspace test summary reports no failures; two ignored tests are listed above.

## SHA-256 provenance

Required behavior: derive the store key from SHA-256 of `std::fs::canonicalize` output for the existing source path, implement without an added dependency, and keep the canonical path as capsule identity with collision/path-mismatch rejection. The digest implementation is `project_session/recovery_store.rs::sha256`; `sha256_matches_fips_known_answers` verifies the empty-string and `abc` known-answer vectors. Standard: NIST FIPS PUB 180-4, SHA-256; [official publication](https://doi.org/10.6028/NIST.FIPS.180-4). The Sol review returned CLEAN.

## Deferred scope

Not included or claimed complete: automatic/background snapshots; app-launch or restart recovery scan/presentation; missing-source recovery; retention; broad UI/E2E; performance compliance; release packaging. Native GUI behavior is not verified here and remains deferred. `AppDocument::open_path` and MCP open continue as normal-open behavior for this order.
