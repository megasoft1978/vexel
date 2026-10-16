---
id: WO-RUST-ATOMIC-FILES-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 0aa3256814c8c5e084f2bd453bb6b0e2656cfea39cfec9996ce087c1aa4cd717
parallel_safe: true
network: false
---
# Goal and frozen scope
Extend accepted native crate with complete atomic existing-file replace and atomic no-replace new-file publication from both C files/headers. Safe APIs accept bounded byte-path/bytes and return exact error class plus file_synced/renamed-or-published/directory_synced state. Preserve1023byte/NUL/path-component bounds, lstat regular/non-symlink rules, target mode/identity rules, exclusive sibling temp creation, short-write/EINTR loops, sync before commit, no-replace collision, atomic commit point, cleanup and correct postcommit durability reporting. Use std filesystem/File/MetadataExt/OsStrExt and safe typed operation hooks (borrowed File/paths), actual native success path; no Cshim/externaldeps/new unsafe or accepted sys.rs changes. If a std-only route cannot meet named guarantee stop with exact dependency rather than weaken it. Explicit safe temporary naming/owned-hook API adaptations must preserve security/race/error behavior. Tests only repo-local .cache/atomic-* paths; port deterministic short write/interrupt/failure/publication/race/durability cases, zero bytes, symlink/nonregular/existing destination and temp cleanup. Preserve8accepted native-service tests/API. Caller file-grants/application/project wiring later.

# Reference paths
src/platform/macos/platform_atomic_file.c, platform_atomic_new_file.c, src/platform/platform_atomic_file.h, platform_atomic_new_file.h; focused existing atomic filesystem fixtures in tests/platform and tests/project
Legacy reference provenance: original dirty-source hashes in artifacts/WO-RUST-PLAN-01/source-baseline.json; no external algorithms/source fetch.

# Files allowed to touch
- `work/WO-RUST-ATOMIC-FILES-01.md`
- `rust/crates/vexel-platform-macos/src/**`, `rust/crates/vexel-platform-macos/tests/**`
- `artifacts/WO-RUST-ATOMIC-FILES-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths and quality
All legacy source/fixtures/oracles, accepted other-crate code, all Cargo/config/lock/toolchain metadata, other worker trees, coordinator records/docs. Existing same-crate accepted behavior/assertions preserved. Worker only own src/tests/artifacts; coordinator alone shared integration. No manifest edit, code outside scope, dependency/download/install, unsupported OS code, Cshim, staging/commit/delete/reset, network/recursive agents, broad UI/E2E/C/performance campaign. Safe owned/lifetime-bound data, bounded inputs/checked arithmetic/fallible significant allocations, no unsafe in new modules; preserve accepted platform sys boundary. Source<800lines. No TODO/unimplemented/production stub for named scope. Map every named C API/helper to Rust symbols, focused tests, source/diagnostic spans, ownership/limits/cleanup and exact safe adaptations in parity-map.json. If dependency or contract cannot meet full scope stop with concrete issue; do not narrow deliverable to convenient subset.

# Execution and verification
Only .worktrees/rust3-atomic-files; Standard subscription LunaHigh, thin scope/context, one writer and focused independent review/repair. Read OWNER-DIRECTIVES/AGENTS and this order; consolidated goal governs full finish line. Preserve accepted underlying interfaces. Root serializes shared metadata/integration.
cargo +stable check -p vexel-platform-macos --locked --offline
cargo +stable test -p vexel-platform-macos --locked --offline
python3 scripts/wo_validate.py WO-RUST-ATOMIC-FILES-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact existing1.99/macOS13, repo-local Cargo/cache/tmp/targets. No rustfmt/clippy installation or unsupported claims. Original fixtures/assertions unchanged; add focused Rust regressions with exact outputs, not only is_err. Run no extra broad suite.

# Output
Leave uncommitted; result<=400words, parity-map.json/verification.txt with actual checks, failures/repairs/adaptations and remaining app scope. Candidate only until independent review, exact scope/main audit and named integrated checks. Existing allowance only; no API billing/credits; stop/checkpoint on rate/allowance errors rather than retryloop.
