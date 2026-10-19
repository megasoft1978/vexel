---
id: WO-RUST-DEPENDENCY-SCOPE-01
revision: 1
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 849371835fa4cb8c3e48ff154778a3ec5d7de196
spec_sha256: 7376438f0ef7023324c1b33d34b5528e865a0d6b3548a85d5e1138a9202d1819
parallel_safe: true
network: false
---
# Goal
Remove unnecessary direct production dependency edges while preserving their use in crate integration tests.

# Evidence and contract
Read `OWNER-DIRECTIVES.md`. Source search shows `vexel-svg-engine` uses `vexel-svg-animation` only from `tests/decoder.rs`; `vexel-svg-path` uses `vexel-svg-values` only from `tests/controlled.rs`; and `vexel-svg-media` uses `vexel-svg-color` only from `tests/style_publication.rs`. Preserve these test imports by placing the edges under `[dev-dependencies]`. Do not remove workspace crates or third-party packages, alter code/tests, or change lockfile entries unless the dependency metadata requires it.

# Allowed paths
- `rust/crates/vexel-svg-engine/Cargo.toml`
- `rust/crates/vexel-svg-path/Cargo.toml`
- `rust/crates/vexel-svg-media/Cargo.toml`
- `Cargo.lock` only if required to keep `--locked` metadata consistent
- Work-order artifact directory only
- `work/WO-RUST-DEPENDENCY-SCOPE-01.md` and `ledger.jsonl` by coordinator only

# Protected paths
All Rust source/tests, other manifests, ADRs, provenance, fixtures/oracles, migration references, active implementation files outside the allowlist and external paths. Do not run builds/tests/benchmarks during implementation; final workspace verification is deferred until all implementation slices complete.

# Acceptance
- AC-1: the three edges are absent from production dependencies and present as dev-dependencies.
- AC-2: source and tests are unchanged; no third-party dependency or version changes.
- AC-3: final locked workspace check/tests confirm crate integration tests still resolve the dependencies.
- AC-4: ledger and work-order validation pass.

# Deferred verification
`cargo check --workspace --locked`, `cargo test --workspace --locked`, and required repository verification after all implementation work is complete.
