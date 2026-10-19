---
id: WO-RUST-V1-NATIVE-PROPERTY-01
revision: 1
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 849371835fa4cb8c3e48ff154778a3ec5d7de196
spec_sha256: 72a75af99cb71ba56df360511aac9d1d6ca5ef35279956f794319536197ced60
parallel_safe: true
network: false
---
# Goal
Apply native timeline-v1 property samples to a temporary SVG source snapshot while preserving source and SVG identity.

# Context and contract
Read `OWNER-DIRECTIVES.md`, ADR-0020 and ADR-0032, `vexel-timeline/src/{schema.rs,access.rs}`, `vexel-xml`, and existing source-preserving edit APIs. Resolve actual SVG `id` values; never use flattened scene indexes or transient handles as identity. Apply writes in timeline order, preserve CSS cascade, RGBA color plus opacity behavior, and transform overwrite semantics. Do not reuse SMIL evaluation. Reject unsupported target/property/value behavior explicitly.

# Allowed paths
- New `rust/crates/vexel-app/src/animation/native_properties.rs`
- New `rust/crates/vexel-app/src/animation/native_properties_tests.rs`
- Work-order artifact directory only

# Protected paths
All existing source, module registries, interfaces, manifests/locks, app frame/MCP/session integration, recovery files, ADRs, work orders, ledger, C/Python evidence, and external paths. Do not run tests/builds; author focused tests only.

# Acceptance
- AC-1: creates an isolated temporary source snapshot and leaves original SVG bytes untouched.
- AC-2: resolves persistent target IDs and applies ordered supported property writes with ADR-0032 precedence/cascade behavior.
- AC-3: tests cover missing/duplicate target conditions where API allows, repeated targets, supported value types, order and cascade behavior.
- AC-4: no SMIL coupling, no dependency, source files under 800 lines.
- AC-5: verification deferred until all implementation slices complete.

# Deferred verification
`cargo test -p vexel-app --locked`
