---
id: WO-RUST-V1-HISTORY-BRIDGE-01
revision: 1
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 849371835fa4cb8c3e48ff154778a3ec5d7de196
spec_sha256: 908341dea690b6cb0df1f711223601ee31904ff349d213e7965a4b1bd7ea25b0
parallel_safe: true
network: false
---
# Goal
Add a narrow, opaque history bridge to the existing V1 document editor so an app-owned chronological journal can order scene commands with project-payload transactions. Preserve the existing document history and identity allocation; do not reconstruct documents from SVG.

# Context and contract
- Read `OWNER-DIRECTIVES.md`, `docs/adr/0020-native-animation-schema-v1.md`, `rust/crates/vexel-document-edit/src/history.rs`, `rust/crates/vexel-document-edit/src/piece_table.rs`, and `rust/crates/vexel-v1-document/src/lib.rs`.
- Existing editor history is authoritative for scene transitions. Expose stable opaque state identities and bounded navigation between reachable states, coalescing boundaries, redo invalidation, and eviction information needed by the app journal.
- An app journal must not record a scene transition it cannot later execute. Navigation failure leaves editor state unchanged. Branching after undo invalidates scene redo. Payload transactions must be able to close a coalescing group so later scene edits cannot merge across them.
- Preserve existing public editing and undo/redo behavior. Do not change SVG serialization, identity allocation, frozen oracles, manifests, locks, or dependency records.
- The app `V1Document` wrapper currently has recovery-auto ownership in this working tree. Do not edit `vexel-v1-document/src/lib.rs` or any recovery files. Provide document-edit crate primitives; coordinator will add wrapper exposure after recovery ownership is released.
- Write focused unit tests but do not execute tests/builds. The user requested implementation first and final verification after all slices are complete.

# Allowed paths
- `rust/crates/vexel-document-edit/src/history.rs`
- `rust/crates/vexel-document-edit/src/piece_table.rs`
- New focused tests within `rust/crates/vexel-document-edit/src/` only if needed.
- `work/WO-RUST-V1-HISTORY-BRIDGE-01.md`, its artifact directory, and `ledger.jsonl` are coordinator-only.

# Protected paths
All other application/source files, `vexel-v1-document/src/lib.rs` until coordinator releases it, manifests/locks, ADRs, interfaces outside the named crate, recovery work, fixtures/oracles/goldens, C/Python migration references, `OWNER-DIRECTIVES.md`, and external directories. No dependency, platform expansion, deletion, test weakening, or verification execution.

# Acceptance criteria
- AC-1: Opaque stable state identity identifies the current editor history state without exposing piece-table internals or materializing full SVG bytes.
- AC-2: The bridge can report whether a requested prior/next reachable state is available and navigate to it atomically; rejected navigation leaves state unchanged.
- AC-3: An explicit history boundary prevents subsequent commands from coalescing with commands before the boundary.
- AC-4: A new edit after undo truncates redo and provides enough information for the app journal to discard its corresponding branch.
- AC-5: Bounded history eviction is observable so the app journal can trim entries no longer reachable in editor history.
- AC-6: Existing edit APIs/semantics and all accepted source behavior remain intact; no file exceeds 800 lines; tests are authored and left unrun.

# Deferred verification
After all implementation slices and integration complete:
`cargo test -p vexel-document-edit -p vexel-v1-document --locked`
