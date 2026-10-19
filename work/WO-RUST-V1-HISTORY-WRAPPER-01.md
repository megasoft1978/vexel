---
id: WO-RUST-V1-HISTORY-WRAPPER-01
revision: 1
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna-medium
base_sha: 849371835fa4cb8c3e48ff154778a3ec5d7de196
spec_sha256: 2848156e029ab55b96dff56c79fffcc307dab3359df19d14e53d04360a1efb22
parallel_safe: true
network: false
---
# Goal
Expose the already implemented `vexel-document-edit` opaque history bridge through `V1Document` for app-owned chronological scene/payload history.

# Context and frozen API
- Read `OWNER-DIRECTIVES.md`, `docs/adr/0020-native-animation-schema-v1.md`, `vexel-document-edit/src/history.rs`, and the existing `V1Document` wrapper.
- Add only these wrapper methods, each delegating directly to the existing editor primitive:
  - `history_state_id(&self) -> u64`
  - `history_state_id_at(&self, cursor: usize) -> Option<u64>`
  - `history_navigate(&mut self, state_id: u64) -> Result<(), V1DocumentError>`
  - `history_boundary(&mut self)`
- Navigation must preserve existing error mapping and atomicity. Do not expose piece-table internals or materialize SVG bytes. The app journal recreates on document replacement and uses reachable IDs to trim stale entries.
- Author focused wrapper tests; do not execute tests/builds until all implementation slices are complete.

# Allowed paths
- `rust/crates/vexel-v1-document/src/lib.rs`
- New `rust/crates/vexel-v1-document/src/history_bridge_tests.rs`
- `work/WO-RUST-V1-HISTORY-WRAPPER-01.md`, its artifacts, and `ledger.jsonl` only by coordinator

# Protected paths
All other source/modules, manifests/locks, ADRs, interfaces outside this wrapper, recovery files, fixtures/oracles/goldens, C/Python migration references, external directories. No dependency, deletion, test weakening, unsupported-platform work, or verification execution.

# Acceptance
- AC-1: exact four methods above delegate to existing editor APIs.
- AC-2: tests cover current identity, reachable state lookup, boundary, successful navigation and rejected navigation without mutation.
- AC-3: preserve all existing V1Document behavior; source remains below 800 lines; tests authored and unrun.

# Deferred verification
`cargo test -p vexel-v1-document --locked`
