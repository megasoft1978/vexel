---
id: WO-RUST-RECOVERY-AUTO-01
revision: 3
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 849371835fa4cb8c3e48ff154778a3ec5d7de196
spec_sha256: ac57eb30f217857a28045e297e273940a1699c01484cbc07ccca2a6603f4541c
parallel_safe: true
network: false
---
# Goal
Add automatic, bounded, off-UI-thread recovery snapshots for dirty saved native `.vexel` projects. Reuse the accepted capsule format, private store, open-time baseline, and explicit restore/discard/cancel flow. Never mutate the live project from the worker. Prevent a stale or queued snapshot from publishing after Save, Save As, explicit recovery discard, project open/replacement, or close has crossed its lifecycle barrier.

# Context and locks
- Read `OWNER-DIRECTIVES.md`, `docs/adr/0021-native-project-recovery-capsules.md`, `work/WO-RUST-RECOVERY-SESSION-04.md`, and `work/WO-RUST-RECOVERY-MCP-ERROR-01.md` first.
- Accepted foundation: explicit recovery session and MCP mappings are on main at `d4da6068` and `b9ae84d9`; current main is `84937183` before this order is frozen.
- Keep the existing bundle/capsule bytes, canonical identity, source baseline, path-collision checks, conflict Save As rule, and error mapping. No new dependency or format/ADR change.
- Project supports dirty native sessions only. Untitled documents and imported SVGs remain ineligible. No periodic UI/E2E campaign here.
- Use immutable source snapshots that share the document editor's `EditSnapshot` owners and an `Arc` to the immutable project payload. The worker receives owned path, baseline, capture time, session epoch and revision. It never borrows `ProjectSession`, `AppDocument`, AppKit, or UI state.

# Frozen policy and concurrency contract
- First automatic capture is eligible 2 seconds after an edit to a dirty saved native project. Thereafter, while dirty, request a snapshot at most once per 15 seconds per session. A newer eligible snapshot replaces the single pending one.
- Exactly one app-owned worker and one bounded pending snapshot are allowed. No unbounded queue, one thread per capture, or copied mutable session. Use monotonic time for scheduling and Unix-epoch nanoseconds for the capsule timestamp.
- Snapshot serialization, encoding, secure-store resolution and disk publication run on the worker. Snapshot creation on the owner thread may clone bounded immutable piece descriptors and `Arc` owners; it must not materialize/encode the SVG or project bytes there. Honor existing project/document/capsule limits and use fallible allocations.
- The worker is a serial actor with an acknowledged pause barrier. Pause clears pending work, prevents selection of another snapshot, and acknowledges only after any active publication has fully completed (including durability reporting). Save, Save As, capsule discard/discovery/restore, every project replacement route (native File Open, startup open, and MCP Open), and Close must hold this barrier across their entire existing transaction. Resume on success, error, or cancel. Cancellation/generation checks without an acknowledged barrier are insufficient.
- Lifecycle epoch advances across each barrier. Results include epoch and document revision; the owner applies only current-epoch results. Do not claim current edits are covered by an older snapshot. Active work is serialized before following captures; pending work is latest-wins.
- Closing stops the worker, drops a pending snapshot, and joins any active task before app teardown. It does not force a final potentially large write. Preserve the last successfully published capsule on close; only successful normal Save or explicit capsule discard removes it.
- Visible status reports capture success, unsaved newer edits, store/publication failure, and uncertain durability through the existing app status surface. Do not show a modal alert for every periodic retry. Recovery failure must not block editing or falsely mark the document saved.
- Lifecycle barrier may wait for the currently active write. The app's existing synchronous file operations remain synchronous; no unbounded background work may continue after their barrier. Record this latency risk; do not claim responsiveness targets from headless tests.

# Frozen internal interfaces
- `V1Document::edit_snapshot(&self) -> Result<EditSnapshot, V1DocumentError>` returns the document editor's immutable, share-owner snapshot; it must not materialize bytes.
- `ProjectSession::prepare_recovery_snapshot(&self, epoch: u64, revision: u64, captured_time_ns: u64) -> Result<Option<RecoverySnapshot>, SessionError>` returns `None` for clean/untitled/imported sessions and otherwise captures the exact open-time baseline, canonical native path, immutable scene snapshot, shared opaque payload, timestamp, epoch and revision. Allocation or validation failure leaves the session and existing capsule unchanged.
- `RecoverySnapshot::publish(self) -> RecoveryJobResult` materializes and encodes only on the worker, then uses the existing secure store and atomic capsule writer. `RecoveryJobResult` carries epoch, document revision, publication/durability outcome and a typed `SessionError` on failure.
- `RecoveryJob::spawn() -> Result<RecoveryJob, SessionError>`, `submit(RecoverySnapshot) -> bool`, `pause() -> RecoveryPause`, `poll_result() -> Option<RecoveryJobResult>`, and `shutdown()` form the internal worker API. `RecoveryPause` is an RAII guard: acquiring it drops pending work and blocks until the active publication finishes; dropping it resumes the worker. Failed worker startup disables automatic capture but does not prevent the app from opening; it exposes a visible recovery-unavailable status.
- `RecoveryCoordinator` is owned by `FileUi`. It exposes `with_lifecycle<T>(&mut self, document: &mut AppDocument, action: impl FnOnce(&mut AppDocument) -> T) -> T` to serialize lifecycle actions with recovery, and `tick(&mut self, document: &AppDocument, now: Instant) -> Option<RecoveryStatusMessage>` to consume results and schedule bounded work. `RecoveryStatusMessage` distinguishes saved-current, saved-older-revision, preparation/publication failure, and durability uncertainty; conversion to user-facing text stays stable and explicit.
- Coordinator owns registrations and these shared declarations in `vexel-v1-document/src/lib.rs`, `vexel-app/src/project_session/mod.rs`, and `vexel-app/src/lib.rs`. Workers implement only their explicitly owned bodies and tests.

# Acceptance criteria
- AC-1: A dirty saved `.vexel` automatically schedules a complete capsule after the frozen debounce/cadence. The capsule preserves scene, manifest, timeline, assets, canonical identity and the open-time source baseline; worker snapshots outlive subsequent edits/session replacement safely.
- AC-2: Worker encoding and file I/O run off the owner thread. Owner-thread preparation only creates a bounded immutable snapshot without materializing project bytes. A failed snapshot leaves live session and any prior complete capsule intact.
- AC-3: Worker/pending/result storage is bounded (one active, one replaceable pending snapshot, one bounded latest result); injected-clock tests cover debounce, 15-second rate bound, dirty/untitled eligibility, coalescing and retry cadence.
- AC-4: Deterministic blocking-publication tests prove Pause waits for an active publish and drops pending work. For Save, Save As, explicit discard, native File Open/recovery choice, startup/MCP replacement, and Close, no pre-barrier job can publish after the operation completes. Cancel/error preserves the live session and can resume scheduling.
- AC-5: Only matching epoch/revision results update status. Success for an older revision explicitly leaves newer edits pending; durability uncertainty and store failure are visible without marking saved.
- AC-6: Existing save/open/recovery/MCP behavior and frozen oracles remain unchanged except the explicit lifecycle barrier and expected recovery cleanup on explicit discard. Close preserves the last valid recovery capsule. No new dependency, file format, broad UI claim, or unsupported-platform code.
- AC-7: Focused app and integration checks, workspace check/test, Python ledger/runner tests, ledger validation, formatting, diff check and this WO readiness validation pass.

# Parallel slices and ownership
Coordinator freezes shared type contracts/module registration and owns final integration/ledger. Do not change Cargo manifests/locks.

1. `SNAPSHOT` (`luna-high`) — own `rust/crates/vexel-app/src/project_session/recovery.rs`, `project_session/payload.rs`, new `project_session/recovery_snapshot.rs`, and new `project_session/recovery_snapshot_tests.rs`. Implement immutable snapshot preparation and worker-side capsule serialization against this contract. Keep manual capture API behavior; do not alter shared declarations outside the owned module. Verify `cargo test -p vexel-app --locked project_session::`.
2. `JOB` (`luna-high`) — own new `rust/crates/vexel-app/src/recovery_job.rs` and `recovery_job_tests.rs`. Implement the single serial worker, bounded latest-wins mailbox, pause acknowledgement, resume/shutdown and bounded result delivery. Provide injected publisher/clock gates for race tests; no UI or session borrowing. Verify `cargo test -p vexel-app --locked recovery_job`.
3. `POLICY` (`luna-medium`) — own new `rust/crates/vexel-app/src/recovery_policy.rs` and `recovery_policy_tests.rs`. Implement the pure injected-time eligibility/coalescing/retry policy from frozen values. No filesystem, sleeping, threads or UI. Verify `cargo test -p vexel-app --locked recovery_policy`.
4. `FLOW` (`luna-medium`) — own `rust/crates/vexel-app/src/file_flow/mod.rs`, `file_flow/recovery.rs`, and `file_flow/tests_recovery.rs`. Wire the barrier around candidate discovery/Restore/Discard/Cancel and implement the frozen explicit-discard semantics without losing the current document on failure/cancel. Do not own `FileUi` or main-loop APIs. Verify `cargo test -p vexel-app --locked file_flow::`.
5. `LOOP` (`sol-worker`, after SNAPSHOT/JOB/POLICY/FLOW) — own `rust/crates/vexel-app/src/main.rs`, `file_ui.rs`, and new child module `file_ui/lifecycle_routes.rs`. Connect scheduling/polling/status and use shared route helpers for Save/Save As, native Open, Close, startup replacement, and MCP Save/Save As/Open. Keep status typed until display conversion and source files below 800 lines. Verify `cargo check -p vexel-app --locked` and `cargo test -p vexel-app --locked --bin vexel`.
6. `INTEGRATION` (`sol-worker`, after LOOP) — own `rust/crates/vexel-app/tests/recovery_auto.rs`; deterministic coverage for publication/lifecycle ordering, actual startup and MCP lifecycle entry points, recovery round trip, and preserved source. Route helper unit tests live with `file_ui/lifecycle_routes.rs`. Verify `cargo test -p vexel-app --locked --lib file_ui::lifecycle_routes::tests` and `cargo test -p vexel-app --locked --test recovery_auto`.
7. `EVIDENCE` (`luna-low`) — own only `artifacts/WO-RUST-RECOVERY-AUTO-01/**`; prepare AC map and record actual checks after integration. No source, work-order, shared-doc or ledger changes.

# Files allowed to touch
- `rust/crates/vexel-app/src/project_session/**`
- `rust/crates/vexel-app/src/recovery_job.rs` and `recovery_job_tests.rs`
- `rust/crates/vexel-app/src/recovery_policy.rs` and `recovery_policy_tests.rs`
- `rust/crates/vexel-app/src/file_flow/**`
- `rust/crates/vexel-app/src/file_ui.rs`
- `rust/crates/vexel-app/src/file_ui/lifecycle_routes.rs`
- `rust/crates/vexel-app/src/main.rs`
- `rust/crates/vexel-app/src/lib.rs`
- `rust/crates/vexel-app/tests/recovery_auto.rs`
- `rust/crates/vexel-v1-document/src/lib.rs` and focused new `snapshot_tests.rs` (coordinator-owned snapshot API exposing the existing immutable EditSnapshot)
- `work/WO-RUST-RECOVERY-AUTO-01.md` and `ledger.jsonl` only by coordinator
- `artifacts/WO-RUST-RECOVERY-AUTO-01/**`

# Protected paths
All other source/crates, manifests/locks, ADRs, fixtures/oracles/goldens, previous work orders/evidence, `OWNER-DIRECTIVES.md`, external apps and non-macOS platform code. No GPL editor source, network fetch, system package install, destructive cleanup, GUI takeover, dependency, force push or weakening of frozen acceptance.

# Verification commands
```text
cargo test -p vexel-app --locked
cargo test -p vexel-platform-apple --locked
cargo check --workspace --locked
cargo test --workspace --locked
python3 -m unittest tests.test_ledger tests.test_wo_runner
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py --ready WO-RUST-RECOVERY-AUTO-01
git diff --check
```

# Non-goals and follow-up
This order does not add app-launch scanning/presentation, missing-source restore, capsule retention/rotation, cross-process arbitration, manual visual GUI acceptance, broad UI/E2E, performance acceptance, release, or migration closure. Those remain required or explicit follow-ups under the full goal.
