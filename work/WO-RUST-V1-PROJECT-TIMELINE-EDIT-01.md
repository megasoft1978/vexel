---
id: WO-RUST-V1-PROJECT-TIMELINE-EDIT-01
revision: 1
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna-high
base_sha: 849371835fa4cb8c3e48ff154778a3ec5d7de196
spec_sha256: fe0ae638e5bbe8c0b76be73f37362bf14d9cb29e1cc7dbe73e2c3029cc809073
parallel_safe: true
network: false
---
# Goal
Add a bounded, transactional app-layer API for native timeline edits and SVG drawing asset validation, ready for integration with the project session journal.

# Context and contract
Read `OWNER-DIRECTIVES.md`, ADR-0019, ADR-0020, ADR-0032, ADR-0033, and the current public `vexel-timeline::{TimelineEdit,TimelineContext,EmptyTimelineSettings}` APIs. Keep zero-length timeline payloads distinct from unsupported nonempty payloads. Initialize only from caller-provided settings. A transaction owns candidate state and publishes no bytes/assets until every requested operation succeeds. Existing asset bytes remain exact. Drawing assets are complete `image/svg+xml`; use the app's coordinator-registered `native_drawing::validate_drawing_svg` as the single asset security/geometry validator. New assets and their first timeline reference must be one atomic result. Do not change SessionPayload or shared project-session declarations because recovery ownership is still active there.

# Allowed paths
- New `rust/crates/vexel-app/src/project_session/timeline_edit.rs`
- New `rust/crates/vexel-app/src/project_session/timeline_edit_tests.rs`
- New `rust/crates/vexel-app/src/project_session/svg_asset.rs`
- New `rust/crates/vexel-app/src/project_session/svg_asset_tests.rs`
- Work-order artifact directory only

# Protected paths
All existing source, including `project_session/{mod.rs,payload.rs}` and recovery files; module registries, protocols/MCP, manifests/locks, ADRs, fixtures/oracles, work orders, ledger, C/Python references, and external paths. Do not run tests/builds/benchmarks; author focused tests only.

# Acceptance
- AC-1: source wrapper handles empty payload by explicit settings and strictly rejects unsupported nonempty timelines without rewriting them.
- AC-2: supported edit helpers use the public `TimelineEdit` API and caller context; invalid operations or allocation/limit failures expose no partial candidate state.
- AC-3: imported drawing assets validate name/type/size/scene geometry and safety, retain exact source bytes, and can be committed atomically with the first cel/track reference.
- AC-4: timeline and asset state is returned as one owned candidate for the later global app journal; no parallel private undo system is introduced.
- AC-5: focused tests cover initializer passthrough/rejection, edits, atomic failure, exact asset bytes, unsupported payload preservation, and bounds; tests remain unrun and no dependency is added.

# Deferred verification
`cargo test -p vexel-app --locked`
