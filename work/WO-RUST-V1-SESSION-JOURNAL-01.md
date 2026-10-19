---
id: WO-RUST-V1-SESSION-JOURNAL-01
revision: 1
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 849371835fa4cb8c3e48ff154778a3ec5d7de196
spec_sha256: 495a485c751001c46a87df6c132d4b34fd25f7a687eb6c895d59ebad9d2d99ef
parallel_safe: true
network: false
---
# Goal
Integrate the reviewed bounded project journal into ProjectSession/AppDocument so drawing edits, payload edits, undo/redo, checkpoints, opens, and recovery share one chronological state history.

# Frozen contract
- `ProjectSession` owns `Journal<SessionPayload>` beside document and payload. Register `history` and initialize a fresh journal fallibly for every new/opened/recovered session before publishing the session.
- `ProjectSession::new(V1Document) -> Result<Self, SessionError>`; `AppDocument::{new,from_document}` propagate session initialization errors. Add a non-private session error for history admission/navigation failures; keep `JournalError` private.
- `ProjectSession::edit<R,E>` returns `Result<R, SessionEditError<E>>`. It obtains scene admission before the closure, invokes no closure on admission failure, and finalizes the observed editor state regardless of closure `Ok`/`Err`, preserving partial mutation behavior. The wrapper retains the original operation error.
- Scene reachability is checked by enumerating `history_state_id_at(0..=history_count())`; do not allocate a reachable-ID vector. Undo/redo first validate and stage the target payload, navigate the editor by opaque scene ID, then commit the matching journal cursor without a recoverable failure. Payload-only transitions must still work when the scene ID is unchanged.
- Remove mutable ProjectSession document access that can bypass the journal; route callers through session edit/undo/redo.
- `is_dirty` compares the combined journal checkpoint plus recovery-dirty state. Successful durable write marks the editor saved and checkpoints the current combined journal at the existing success boundary. Failed writes do not checkpoint.
- Open and recovery replacement prepare a fresh Journal before assigning live session fields. Preserve recovery conflict/baseline semantics and FileUi revision/barrier behavior.
- `AppDocument::revision` continues to reflect source byte changes only. Partial scene changes followed by operation error still increment it; payload-only edits do not unless they change the playback frame source.
- Do not add dependencies or persisted history. Keep source files below 800 lines. Implement production code before test/call-site migration; tests and all build/test commands remain deferred until the implementation wave is complete.

# Slice ownership
- S1 coordinator/Luna high: `rust/crates/vexel-app/src/project_session/mod.rs`, new `journal_integration.rs`, new `edit_error.rs`. Journal registration, initialization, core edit/undo/redo, checkpoint/dirty, narrow public APIs.
- S2 Luna medium after S1 API is frozen: `rust/crates/vexel-app/src/project_session/recovery.rs`. Prepare fresh journal before recovered state publication; preserve recovery semantics.
- S3 Luna high after S1: `rust/crates/vexel-app/src/session/mod.rs`, `session/hooks.rs`. AppDocument error/revision adapters and combined undo/redo availability.
- S4 Luna medium after S3: `session/mcp.rs`, `session/mcp_tools.rs`. Adapt operation/history errors without losing validation messages.
- S5 Luna medium after S3: `src/history.rs`, `src/model.rs`, `src/main.rs`, `src/toolbar_actions.rs`, `src/file_ui.rs`. Connect enabled/disabled undo/redo state and fallible constructors while preserving current visual design.
- S6 Luna high after production slices: migrate direct ProjectSession mutation and fallible constructors in existing tests/callers; add focused combined-history integration tests in new `project_session/journal_integration_tests.rs`. Do not weaken existing assertions. Coordinator owns test module registrations.

# Acceptance
- Admission rejection runs no scene closure and preserves scene, payload, journal cursor/IDs/checkpoint/redo.
- Partial-error scene mutation is undoable and increments source revision; no-op preserves redo and revision.
- Mixed scene/payload undo/redo restores exact paired states; payload-only navigation does not rerender scene.
- Scene coalescing, payload boundaries, branches, editor eviction, dirty/checkpoint/save, open, recovery restore, and failed replacement are covered.
- No raw mutation path bypasses ProjectSession history; no persisted history or new dependency.

# Deferred verification
After all implementation and test authoring is complete, run the required workspace checks from AGENTS.md, including `cargo check --workspace --locked`, `cargo test --workspace --locked`, Python ledger/WO tests, ledger validation, relevant WO validation, and `git diff --check`.
