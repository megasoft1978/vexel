---
id: WO-RUST-RECOVERY-SESSION-01
revision: 1
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 76898badf27d3b1ee774786b1214c63a42b18a7c
spec_sha256: 9f0ff64c658bb965fc9eb3bbb9a40ae99968079feaf04a6b40ab333ddc1a78b6
parallel_safe: true
network: false
---
# Goal
Connect the accepted recovery capsule codec and discovery primitive to app session state so a native `.vexel` project can produce a bounded manual recovery snapshot and restore it through an explicit user choice without losing source/payload data or silently overwriting a changed source.

# Context and limits
- Read `OWNER-DIRECTIVES.md`, ADR-0021, the recovery codec/discovery implementation, `project_session`, `file_flow`, `file_ui`, and `AppDocument` open routes.
- Reuse `project_recovery_save`, `project_recovery_discover`, and `project_recovery_decode`; do not add dependencies or change the bundle/capsule formats.
- Scope is the recoverable session and explicit open workflow. It does not claim automatic periodic/background capture, crash-launch recovery presentation, retention, missing-source restoration, broad UI/E2E, or performance compliance. Those remain explicit follow-up work orders.
- Keep source under 800 lines, use fallible bounded allocations, preserve unknown manifest/timeline/assets bytes, and follow existing app/platform error contracts.

# Frozen contract
- Recovery snapshots are written only for a saved native project path. Source SVG imports and untitled documents have no recovery target.
- The capsule path is supplied to the session API by the caller; path/store derivation is intentionally outside this order.
- The snapshot records the source file stamp at capture. `Matching` and `Conflict` remain distinct; conflict restore must never acquire permission to overwrite the source via Save.
- Recovery adoption prepares and validates the complete replacement before mutating live session state. Restored sessions have empty undo history and are unsaved. Failure/cancel leaves session document, payload, path, dirty state, and capsule bytes unchanged.
- Successful normal save and explicit recovery discard may remove the capsule only after the operation outcome is known; post-publication durability errors must not be misreported as if no file changed.
- File > Open keeps the existing unsaved-changes prompt semantics, then checks a capsule for the chosen native project and offers explicit Restore / Discard Recovery / Cancel. Cancel leaves the current session untouched. Conflict restore is clearly identified and requires explicit Restore; it remains dirty and cannot Save over the source (Save As is required).
- Existing `AppDocument::open_path` / MCP open behavior remains normal-open for this order; do not imply that these routes show a native dialog.

# Parallel slices and file ownership
1. `SESSION` (`luna-high`): own new `rust/crates/vexel-app/src/project_session/recovery.rs`, new `project_session/recovery_tests.rs`, and `project_session/payload.rs`. Implement bounded capture/decode/adoption helpers against this contract. Do not edit module declarations or shared interfaces.
2. `FLOW` (`luna-medium`): own new `rust/crates/vexel-app/src/file_flow/recovery.rs` and new `file_flow/tests_recovery.rs`. Implement pure choice and failure handling against the session/backend interface frozen by the coordinator. Do not edit existing module declarations or platform interfaces.
3. `PROMPT` (`claude:design`): own only the native AppKit recovery alert implementation in `rust/crates/vexel-platform-apple/src/panels/native.rs` and focused cases in `panels/tests.rs`. Use the coordinator-frozen recovery choice/text types; never call dialogs off the main thread, and do not alter shared traits/re-exports.
4. `EVIDENCE` (`luna-low`): own only `artifacts/WO-RUST-RECOVERY-SESSION-01/**`; create an acceptance matrix and later record exact integrated test results and deferred boundaries. No code/ledger/work-order edits.
5. Coordinator owns interface/module declarations and registrations in `project_session/mod.rs`, `file_flow/mod.rs`, `file_ui.rs`, `vexel-platform-apple/src/panels/mod.rs`, `vexel-platform-apple/src/lib.rs`, updating all `PanelBackend` test implementations as required, and shared ledger/state updates.
6. Integration test ownership is coordinator: add a focused app integration test under `rust/crates/vexel-app/tests/recovery_session.rs` after slices integrate.

# Acceptance criteria
- AC-1: A dirty saved `.vexel` session can create a bounded recovery capsule using the complete scene and retained opaque payload. Invalid input, allocation failure, or failed publication leaves live session state unchanged.
- AC-2: Matching and conflicting capsules are discovered and decoded using the exact source path. Corrupt, unsupported, mismatched, and invalid capsules return actionable typed failures without partial adoption.
- AC-3: Explicit Restore / Discard Recovery / Cancel behavior is covered by fake-backend flow tests. Cancel is a no-op; conflict Restore cannot overwrite the changed source; Discard opens the normal source; failed opens preserve the current session.
- AC-4: Restored scene, timeline, manifest, and asset bytes round-trip exactly; restored state is dirty, has no stale undo history, and preserves original source bytes.
- AC-5: Successful normal save and explicit discard apply capsule cleanup according to publication/durability outcomes, with tests for failure states.
- AC-6: All new Rust source files are under 800 lines; no dependency, format change, GPL source, platform expansion, false automatic-capture claim, or weakened frozen oracle is introduced.
- AC-7: Focused package/integration checks, workspace check/test, Python ledger/runner tests, ledger validation, `git diff --check`, and WO readiness validation pass.

# Files allowed to touch
- `rust/crates/vexel-app/src/project_session/**`
- `rust/crates/vexel-app/src/file_flow/**`
- `rust/crates/vexel-app/src/file_ui.rs`
- `rust/crates/vexel-app/src/session/mod.rs`
- `rust/crates/vexel-app/tests/recovery_session.rs`
- `rust/crates/vexel-platform-apple/src/panels/**`
- `rust/crates/vexel-platform-apple/src/lib.rs`
- `artifacts/WO-RUST-RECOVERY-SESSION-01/**`
- `work/WO-RUST-RECOVERY-SESSION-01.md` and `ledger.jsonl` only by coordinator

# Protected paths
All manifests/locks, `OWNER-DIRECTIVES.md`, other work orders and artifacts, other crates, shared interfaces beyond coordinator assignments, state/ledger summaries beyond coordinator assignment, C/Objective-C sources, fixtures/oracles/goldens, and any path not explicitly allowed above. No new dependency, network fetch, install, GUI takeover, reset, deletion, weakening, or force push.

# Verification commands
```text
cargo test -p vexel-app --locked
cargo test -p vexel-platform-apple --locked
git diff --check
python3 -m unittest tests.test_ledger tests.test_wo_runner
python3 scripts/ledger.py validate ledger.jsonl
cargo check --workspace --locked
cargo test --workspace --locked
```

# Non-goals
Automatic periodic/background snapshot scheduling, app-launch recovery scan, recovery-store path/key policy, missing-source recovery, retention, UI/E2E, and release packaging. Do not claim these are complete from this order.
