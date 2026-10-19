---
id: WO-RUST-V1-DEFAULT-TIMELINE-01
revision: 1
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 849371835fa4cb8c3e48ff154778a3ec5d7de196
spec_sha256: 689699b9d874f42a14cc0567dd89ea5e38d724b4fcc7d09818c016c68666e8d0
parallel_safe: false
network: false
---
# Goal
Make newly created project sessions use the schema-defined empty native timeline representation.

# Contract
- ADR-0020 defines only zero-length bytes as the legacy empty timeline; nonempty `{}` and unknown schemas remain opaque and unsupported.
- Change the application default payload from `{}` to zero-length timeline bytes. Preserve all loaded nonempty timeline bytes exactly.
- Update the source-only default payload assertion and author one focused default test if useful. Do not weaken other assertions or run tests/builds.
- No dependency, container, ADR, or migration-reference changes.

# Allowed paths
- `rust/crates/vexel-app/src/project_session/payload.rs`
- `rust/crates/vexel-app/src/project_session/tests.rs`
- `rust/crates/vexel-app/src/project_session/payload_timeline_tests.rs`
- `work/WO-RUST-V1-DEFAULT-TIMELINE-01.md` and `ledger.jsonl` by coordinator only

# Protected paths
All other source, timeline schema/decoder, project format, manifests/locks, ADRs, fixtures/oracles, C/Python migration references, and external paths.

# Acceptance
- Newly created sessions and source-only project writes use empty timeline bytes.
- Existing nonempty `{}` and unknown timeline payloads are preserved and remain unsupported for editing.
- Source files remain under 800 lines; tests are authored and left unrun.

# Deferred verification
Final workspace verification only after all implementation slices complete.
