---
id: WO-RUST-V1-APP-JOURNAL-01
revision: 2
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 849371835fa4cb8c3e48ff154778a3ec5d7de196
spec_sha256: 41d05254394c26a7c55357929956b90402769e0cadcb9b589ad53152cd369ccb
parallel_safe: true
network: false
---
# Goal
Implement a bounded, generic chronological app journal that orders source-editor states with immutable project-payload snapshots and can be safely integrated around scene mutations.

# Context and frozen admission contract
- Read `OWNER-DIRECTIVES.md`, ADR-0020, `session/mod.rs`, `project_session/mod.rs`, `project_session/payload.rs`, and `vexel-v1-document` history wrappers.
- This journal is session-only; project files continue to store current scene/payload without undo history. Model each app state as opaque scene state ID + immutable payload snapshot. Scene transitions share the payload; a drawing import plus its cel reference is one payload transition.
- Use noncloneable borrowing admission guards so all fallible allocation/capacity/accounting checks occur before scene or payload publication. Dropping an uncommitted guard preserves logical state, cursor, IDs, checkpoint and redo. Increased Vec capacity alone is harmless.
- `prepare_scene` checks app-ID advance and fallibly reserves one state slot before invoking the scene edit. Its finalization consumes the observed document state ID and a nonallocating reachability predicate. If the closure partially changes the document and returns an error, the observed state must still be committed; this module does not promise rollback of arbitrary editor closures.
- `prepare_payload(candidate)` stages the branch-truncated descriptor vector plus candidate, fallibly reserves all storage, validates app-ID advance and unique retained payload bytes, and returns an admission guard. `commit` replaces journal state without allocation or recoverable failure. The session closes the document coalescing boundary and installs the prebuilt payload immediately before/with this commit.
- Scene admission adds no unique payload bytes because it shares the existing snapshot. Payload accounting must avoid allocating HashMap: use bounded pairwise identity scans over at most 1024 states, detect conflicting retained-byte values for one identity, and checked-add unique bytes.
- Global branch truncation must remove app redo after a new scene or payload transition. Scene coalescing may replace only the current scene tail; payload transitions are hard boundaries. Filter unreachable scene IDs without allocating by querying `history_state_id_at` through the current document. Never compare opaque IDs numerically.
- Live journal cap: 1024 state descriptors. Scene admission may reserve capacity for one additional descriptor (at most 1025). Payload admission may stage at most 1025 descriptors alongside the live journal; document this peak bound. Unique payload bytes per admitted journal remain at or below `vexel_project::MAX_BYTES`.
- Initialize the journal fallibly (`try_new`) so admission does not rely on infallible `vec!` allocation. Payload owner clones used by admissions must be cheap/allocation-free (`SessionPayload` uses `Arc`).
- Keep this module independent from AppDocument/ProjectSession mutation, save, recovery, UI/MCP and lifecycle integration. Coordinator owns registrations and callers.
- Author deterministic unit tests but do not execute tests/builds; implementation must finish before final verification.

# Expected internal interfaces
Names may vary only if semantics remain equivalent and coordinator reviews before integration:
```rust
Journal::try_new(scene_state_id, payload) -> Result<Journal<_>, JournalError>
Journal::prepare_scene(&mut self) -> Result<SceneAdmission<'_, _>, JournalError>
SceneAdmission::finish(self, final_scene_id, reachable: impl FnMut(u64) -> bool) -> Option<u64>
Journal::prepare_payload(&mut self, payload) -> Result<PayloadAdmission<'_, _>, JournalError>
PayloadAdmission::commit(self) -> u64
```
`None` from scene finalization means no editor state change and preserves journal identity/redo. Finalization/commit after successful admission must not allocate or return a recoverable error.

# Allowed paths
- `rust/crates/vexel-app/src/project_session/history.rs`
- `rust/crates/vexel-app/src/project_session/history_tests.rs`
- `work/WO-RUST-V1-APP-JOURNAL-01.md`, its artifact directory, and `ledger.jsonl` only by coordinator

# Protected paths
All session/app integration, module declarations, payload adapter, recovery files, manifests/locks, ADRs, fixtures/oracles/goldens, C/Python migration references, external paths. No dependency, history rollback, test weakening, or verification execution.

# Acceptance criteria
- AC-1: each committed state has a fresh monotonic app ID and opaque scene ID + payload snapshot; save checkpoint/dirty comparison remain identity-based.
- AC-2: scene admission and finalization are safe around partial-error closures; preparation failure skips the edit, and a no-op preserves redo/identity.
- AC-3: payload admission rejection leaves journal and caller payload unchanged; commit is infallible after admission and counts shared snapshots once.
- AC-4: scene/payload order, branch truncation, coalescing boundaries, reachability filtering and 1024-state eviction are deterministic and bounded.
- AC-5: payload accounting is checked and allocation-free; admission Vec reservations are fallible; journal initialization is fallible; no file exceeds 800 lines.
- AC-6: focused tests cover scene A → payload B → scene C undo/redo, branches, coalescing, checkpoint/dirty, admission/capacity failure, no-op, partial-error observation, eviction, identity collision, and exact memory bounds. Tests are authored and left unrun.

# Deferred verification
`cargo test -p vexel-app --locked project_session::history`
