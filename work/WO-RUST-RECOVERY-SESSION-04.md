---
id: WO-RUST-RECOVERY-SESSION-04
revision: 1
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 76898badf27d3b1ee774786b1214c63a42b18a7c
spec_sha256: dc20523ade296daa48088dc379ff675b6f1c9e085557b3f194883eea4b7942e2
parallel_safe: true
network: false
---
# Goal
Connect the accepted recovery capsule codec and discovery primitive to app session state so a native `.vexel` project can produce a bounded manual recovery snapshot and restore it through an explicit user choice without losing source/payload data or silently overwriting a source changed after editing began.

# Context and limits
- Read `OWNER-DIRECTIVES.md`, ADR-0021, the recovery codec/discovery implementation, `project_session`, `file_flow`, `file_ui`, and `AppDocument` open routes.
- Reuse `project_recovery_save`, `project_recovery_discover`, and `project_recovery_decode`; do not add dependencies or change the bundle/capsule formats.
- Scope includes the per-user recovery store resolver and explicit File > Open recovery workflow for an existing source. It does not claim automatic periodic/background capture, crash-launch recovery presentation, retention, missing-source restoration, broad UI/E2E, or performance compliance. Those remain explicit follow-up work orders.
- Keep source under 800 lines, use fallible bounded allocations, preserve unknown manifest/timeline/assets bytes, and follow existing app/platform error contracts.

# Frozen contract
- Recovery snapshots are written only for a saved native project path. Source SVG imports and untitled documents have no recovery target. `ProjectSession` exposes `capture_recovery(captured_time_ns: u64) -> Result<(), SessionError>`, `recovery_status() -> Result<RecoveryStatus, SessionError>`, `restore_recovery() -> Result<(), SessionError>`, and `discard_recovery() -> Result<(), SessionError>`. Public `RecoveryStatus` has `Absent`, `Matching`, and `Conflict`; invalid discovery returns a typed error. `is_dirty()` includes restored recovery state. `PanelBackend::confirm_recovery(&RecoveryText) -> RecoveryChoice` returns Restore, Discard, or Cancel; the caller supplies distinct matching/conflict wording.
- Store capsules below `~/Library/Application Support/Vexel/Recovery/` with owner-only directory permissions. Derive a stable SHA-256 key from `std::fs::canonicalize` of the existing source path; keep that canonical path as the capsule identity and reject a key collision/path mismatch. Implement the digest from FIPS 180-4 without a new dependency and record this provenance in the report. If the source cannot be standardized or the private store cannot be secured, disable recovery for that source and surface an actionable error; never fall back to a sibling or shared directory.
- Capture the native source size, modification seconds/nanoseconds, device, and inode when a `.vexel` file is opened. Compare metadata immediately before and after `project_file_load`; reject the open if the source identity/stamp changed during loading. Keep this baseline in the session and use it for all recovery captures, so a source change after editing began is reported as `Conflict`, including a change that precedes the first capsule. Refresh baseline only after a successful save/publication. When restoring, retain the capsule baseline. `Matching` and `Conflict` remain distinct; conflict restore must never acquire permission to overwrite the source via Save.
- Recovery adoption prepares and validates the complete replacement before mutating live session state. Restored sessions have empty undo history and are unsaved. Failure/cancel leaves session document, payload, path, dirty state, and capsule bytes unchanged.
- Successful normal save and explicit recovery discard may remove the capsule only after the operation outcome is known; post-publication durability errors must not be misreported as if no file changed. Save As from a conflict recovery adopts the new path and clears the old conflict only after durable publication.
- File > Open keeps the existing unsaved-changes prompt semantics, then checks a capsule for the chosen native project and offers explicit Restore / Discard Recovery / Cancel. Cancel leaves the current session untouched. Conflict restore is clearly identified and requires explicit Restore; it remains dirty and cannot Save over the source (Save As is required).
- Existing `AppDocument::open_path` / MCP open behavior remains normal-open for this order; do not imply that these routes show a native dialog.

# Parallel slices and file ownership
1. `SESSION` (`luna-high`): own new `rust/crates/vexel-app/src/project_session/recovery.rs`, new `project_session/recovery_tests.rs`, new `project_session/recovery_store.rs`, and `project_session/payload.rs`. Implement bounded capture/decode/adoption, secure per-user store resolution, and stable SHA-256 key derivation against this contract. Do not edit module declarations or shared interfaces.
2. `FLOW` (`luna-medium`): own new `rust/crates/vexel-app/src/file_flow/recovery.rs` and new `file_flow/tests_recovery.rs`. Implement pure choice and failure handling against the session/backend interface frozen by the coordinator. Do not edit existing module declarations or platform interfaces.
3. `PROMPT` (`claude:design`): own only the native AppKit recovery alert implementation in `rust/crates/vexel-platform-apple/src/panels/native.rs` and focused cases in `panels/tests.rs`. Use the coordinator-frozen recovery choice/text types; never call dialogs off the main thread, and do not alter shared traits/re-exports.
4. `EVIDENCE` (`luna-low`): own only `artifacts/WO-RUST-RECOVERY-SESSION-04/**`; create an acceptance matrix and later record exact integrated test results and deferred boundaries. No code/ledger/work-order edits.
5. Coordinator owns interface/module declarations and registrations in `project_session/mod.rs`, `file_flow/mod.rs`, `file_ui.rs`, `vexel-platform-apple/src/panels/mod.rs`, `vexel-platform-apple/src/lib.rs`, updating all `PanelBackend` test implementations as required, and shared ledger/state updates.
6. Integration test ownership is coordinator: add a focused app integration test under `rust/crates/vexel-app/tests/recovery_session.rs` after slices integrate.

# Acceptance criteria
- AC-1: A dirty saved `.vexel` session can create a bounded recovery capsule using the complete scene and retained opaque payload. Invalid input, allocation failure, or failed publication leaves live session state unchanged.
- AC-2: Matching and conflicting capsules are discovered and decoded using the canonical source path and the metadata baseline captured at open/edit start. A changed source from before the first snapshot is reported as Conflict. Corrupt, unsupported, mismatched, and invalid capsules return actionable typed failures without partial adoption.
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
- `artifacts/WO-RUST-RECOVERY-SESSION-04/**`
- `work/WO-RUST-RECOVERY-SESSION-04.md` and `ledger.jsonl` only by coordinator

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
Automatic periodic/background snapshot scheduling, app-launch recovery scan, missing-source recovery, retention, broad UI/E2E, and release packaging. Do not claim these are complete from this order.
