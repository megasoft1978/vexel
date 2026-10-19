---
id: WO-RUST-V1-SESSION-COMMANDS-01
revision: 1
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna-high
base_sha: 849371835fa4cb8c3e48ff154778a3ec5d7de196
spec_sha256: cbc43b4946ff19e93389342a38db8134784fcecac41d807966b0dd1fd69a7a5e
parallel_safe: true
network: false
---
# Goal
Add project-session commands that publish native timeline edits and SVG drawing assets as one globally undoable payload transition.

# Frozen contract
- Read ADR-0019, ADR-0020, ADR-0032/0033 and the existing project-session timeline, SVG asset, payload, and journal modules.
- Implement a new child module `native_commands.rs`; it may access private ProjectSession fields but must not change parent declarations or shared interfaces. Coordinator registers the module after this slice.
- Expose a typed command error separating session/journal admission failure, timeline candidate/validation failure, and malformed/duplicate SVG scene ids.
- Enumerate supported drawing scene ids from the current V1 SVG with the owned XML tokenizer; decode attribute values, reject duplicate IDs, and return sorted owned bytes using fallible allocation. Do not use external XML/SVG resources.
- Provide a transactional timeline-edit entry point: build candidate from the current opaque payload and explicit `EmptyTimelineSettings`/`EditLimits`; run caller operation; a semantic no-op publishes nothing; prepare replacement `SessionPayload`; obtain `Journal::prepare_payload`; only after admission succeeds establish `document.history_boundary()`, publish payload and commit journal without a recoverable failure. Any preparation/admission error leaves payload identity, document, journal cursor/IDs/checkpoint/redo unchanged.
- Keep opaque unsupported timeline payloads byte-exact and reject attempts to edit them. Drawing import and its first cel/track reference remain atomic through `ProjectTimelineCandidate`.
- Add focused tests in new `native_commands_tests.rs`; do not run tests/builds. Do not register modules or change `project_session/mod.rs`; coordinator owns those edits. No dependencies or manifest changes.

# Allowed paths
- `rust/crates/vexel-app/src/project_session/native_commands.rs` (new)
- `rust/crates/vexel-app/src/project_session/native_commands_tests.rs` (new)
- Work-order artifacts and ledger only by coordinator

# Protected paths
- Existing source files, module declarations, interfaces, payload/history/journal modules, session/app/MCP/UI wiring, recovery, manifests/locks, ADRs, fixtures/oracles, C/Python migration references, and external paths.
- Do not weaken tests, remove C references, add dependencies, or run tests/builds.

# Acceptance
- AC-1: timeline candidate reads/edits retain exact opaque manifest and existing asset data; unsupported nonempty timeline edits fail without publication.
- AC-2: payload edit + drawing asset + first cel reference is one journal transition with a hard editor coalescing boundary.
- AC-3: scene ids are decoded, unique, bounded, and sorted; malformed/duplicate ids fail before mutation.
- AC-4: admission/operation rejection preserves source payload identity, current scene, journal cursor/IDs/checkpoint/redo.
- AC-5: tests cover candidate commit/no-op/rejection, asset import atomicity, undo-ready payload state, id decoding/duplicates, and bounds; all remain unrun pending final verification.

# Deferred verification
Required workspace verification only after all implementation slices and test authoring are complete.
