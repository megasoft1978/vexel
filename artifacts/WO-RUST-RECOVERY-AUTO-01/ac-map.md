# WO-RUST-RECOVERY-AUTO-01 Acceptance Map

Status: implementation and verification are in progress. This map records the planned evidence locations from the frozen work order; it does not claim any check has passed.

| Criterion | Planned implementation evidence | Planned test / verification evidence | Status |
|---|---|---|---|
| AC-1: Automatic bounded capture preserves project state and baseline across edits/replacement. | `rust/crates/vexel-app/src/project_session/recovery_snapshot.rs`; `project_session/recovery.rs`; `project_session/payload.rs`; `recovery_policy.rs`; `file_ui.rs`; `main.rs` | `project_session::` tests; `recovery_policy` tests; `tests/recovery_auto.rs` round-trip and lifecycle coverage | Pending |
| AC-2: Serialization and I/O happen off the owner thread; snapshot preparation shares immutable owners and failure preserves state. | `rust/crates/vexel-v1-document/src/lib.rs` and `snapshot_tests.rs`; `project_session/recovery_snapshot.rs`; `recovery_job.rs` | Snapshot tests and `recovery_job` tests; focused integration tests | Pending |
| AC-3: Worker, pending, and result storage are bounded; injected-time policy covers eligibility, cadence, coalescing, and retry. | `recovery_job.rs`; `recovery_policy.rs` | `recovery_job_tests.rs`; `recovery_policy_tests.rs` | Pending |
| AC-4: Pause is acknowledged and clears pending work; lifecycle transactions cannot be followed by stale publication; cancel/error resumes safely. | `recovery_job.rs`; `file_flow/mod.rs`; `file_flow/recovery.rs`; `file_ui.rs`; `main.rs` | Blocking publication/barrier tests; `file_flow::` tests; `tests/recovery_auto.rs` lifecycle-ordering cases | Pending |
| AC-5: Only current epoch/revision results affect status; older success, durability uncertainty, and failures are represented accurately. | `recovery_job.rs`; `file_ui.rs`; `main.rs` | Job/coordinator tests and integration status assertions | Pending |
| AC-6: Existing save/open/recovery/MCP semantics and frozen oracles remain intact; explicit discard cleanup and close preservation are correct. | `file_flow/**`; `file_ui.rs`; `main.rs`; recovery snapshot/job integration | App and platform test suites; workspace suite; recovery integration cases | Pending |
| AC-7: Required focused and full checks, ledger validation, formatting, diff check, and WO readiness pass. | Work-order implementation and this evidence artifact | `cargo test -p vexel-app --locked`; `cargo test -p vexel-platform-apple --locked`; `cargo check --workspace --locked`; `cargo test --workspace --locked`; `python3 -m unittest tests.test_ledger tests.test_wo_runner`; `python3 scripts/ledger.py validate ledger.jsonl`; `python3 scripts/wo_validate.py --ready WO-RUST-RECOVERY-AUTO-01`; `git diff --check` | Pending |

## Verification record

No verification results are recorded yet. Add exact command output and outcome only after each command has actually run. Report any skipped or failed check explicitly; do not infer completion from planned coverage.
