---
id: WO-RUST-V1-PAYLOAD-TIMELINE-01
revision: 1
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna-high
base_sha: 849371835fa4cb8c3e48ff154778a3ec5d7de196
spec_sha256: 907a8b0edebf48a4211fcbf8ecb93ab63c466503194fa47c8ebb719477ed9794
parallel_safe: true
network: false
---
# Goal
Bridge immutable native-timeline candidates to the project's opaque `SessionPayload` while preserving unknown manifest, timeline, and existing asset bytes exactly.

# Context and frozen API
- Read `OWNER-DIRECTIVES.md`, ADR-0020, ADR-0032, `project_session/payload.rs`, `project_session/timeline_edit.rs`, and `project_session/svg_asset.rs`.
- Add internal payload methods:
  - `timeline_bytes(&self) -> &[u8]`
  - `timeline_candidate(&self, scene_ids: Vec<Vec<u8>>, settings: EmptyTimelineSettings, limits: EditLimits) -> Result<ProjectTimelineCandidate, ProjectTimelineError>`
  - `with_timeline_candidate(&self, candidate: &ProjectTimelineCandidate) -> Result<Self, SessionError>`
- `timeline_candidate` fallibly copies existing asset names/media types/data into the candidate. Candidate initialization is read-only; callers must not dirty/publish it unless an explicit edit succeeds.
- `with_timeline_candidate` retains exact current manifest bytes and installs candidate timeline/assets in a new immutable `Arc` payload, validating asset/project bounds before return.
- Implement `history::PayloadSnapshot`: identity must be stable across cheap `Arc` clones and differ for replacement; retained bytes use checked accounting over manifest, timeline, names, media types and asset data.
- Preserve exact opaque timeline and assets when unopened/unsupported. No session path/baseline/journal mutation here. Author focused tests, do not execute tests/builds.

# Allowed paths
- `rust/crates/vexel-app/src/project_session/payload.rs`
- New `rust/crates/vexel-app/src/project_session/payload_timeline_tests.rs`
- `work/WO-RUST-V1-PAYLOAD-TIMELINE-01.md`, its artifacts, and `ledger.jsonl` only by coordinator

# Protected paths
All other source/modules, registrations, manifests/locks, ADRs, session commit boundaries, recovery code, fixtures/oracles/goldens, C/Python migration references, external directories. No dependency, deletion, test weakening, or verification execution.

# Acceptance
- AC-1: candidate round-trip keeps manifest and all untouched asset bytes exact.
- AC-2: candidate initialization does not mutate payload identity or dirty state; replacement identity changes.
- AC-3: allocation/size/asset failure returns without changing source payload; no overflow in retained-byte accounting.
- AC-4: unsupported timeline remains opaque and source files remain below 800 lines; tests authored and unrun.

# Deferred verification
`cargo test -p vexel-app --locked project_session::payload`
