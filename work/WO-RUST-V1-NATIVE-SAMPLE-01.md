---
id: WO-RUST-V1-NATIVE-SAMPLE-01
revision: 1
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 849371835fa4cb8c3e48ff154778a3ec5d7de196
spec_sha256: 8d5dd5f77c1b87678a3b4b058c08fa337b719b3da8a3d7c0e192eca3b503975d
parallel_safe: true
network: false
---
# Goal
Implement deterministic native timeline-v1 sampling for validated timeline views.

# Context and contract
Read `OWNER-DIRECTIVES.md`, ADR-0020 and ADR-0032, and `vexel-timeline/src/{schema.rs,access.rs}`. Preserve integer ticks, endpoint hold, `step`/`linear`/smoothstep cubic (`3t^2-2t^3`), and serialized track order. Skip hidden property tracks. Keep SVG SMIL and playback/loop policy out of this crate. Return typed errors for unsupported combinations; do not guess.

# Allowed paths
- New `rust/crates/vexel-timeline/src/sample.rs`
- New `rust/crates/vexel-timeline/src/sample_tests.rs`
- Work-order artifact directory only

# Protected paths
All existing source except coordinator-owned module registration, tests, interfaces, manifests/locks, ADRs, work orders, ledger, recovery files, C/Python migration evidence, and external paths. Do not run tests/builds; author focused tests only.

# Acceptance
- AC-1: samples validated v1 keys at integer ticks with endpoint holding and all three specified interpolation modes.
- AC-2: preserves serialized ordering and hidden-track behavior; preserves typed property values without coercion.
- AC-3: invalid/unsupported values return explicit errors without partial output.
- AC-4: tests are authored, no dependencies added, each source file stays below 800 lines.
- AC-5: full workspace verification is deferred until all implementation slices complete.

# Deferred verification
`cargo test -p vexel-timeline --locked`
