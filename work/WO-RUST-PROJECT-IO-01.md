---
id: WO-RUST-PROJECT-IO-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 3c2dbfa4e7603b652411c38e1296a06e99f6c5e7ff4ec1297ae3b5041618c297
parallel_safe: true
network: false
---
# Goal and frozen scope
Port full project_file.c/h AND project_recovery.c/h using accepted native atomic replace/create and project bundle codec, std safe bytes/path adapters. Save_as encodes/validates fully before publication; save_encoded checksum-validates caller bytes; preserve original existing regular vs no-replace new path, symlink/nonregular/collision/error mapping/bytes_written/published/directory-sync outcomes and exact pre/postcommit failures. Recovery v1 full72byte binary header/magic/endianness/CRC/source identity/path/size/mtime/capture timestamp, measured size/encode/decode borrowed views and atomic save distinct from source. Retain file versions/field offsets/zero reserved bytes/checksum exactness/path mismatch/nanosecond constraints and original error/offset precedence. Checked arithmetic/fallible allocations, no whole input hidden clones; no public stubs or weakened filesystem behavior. Frozen focused tests port known original encoded bytes/header/checksum, malformed/truncated/corrupt/version/path mismatch/limits; local filesystem new/replace/reopen recovery/save/symlink/nonregular/source unchanged/tempcleanup, postcommit durability via safe gates preserving real native success. All fixtures repo-local .cache/project-io-*; no app UI tests. Native recovery directory/store scanning and editor dirty state wiring later.

# Reference paths
src/project/project_file.c/h, project_recovery.c/h; tests/project/project_file_test.c, project_recovery_test.c; docs/adr/0019-vexel-project-container-v1.md, docs/adr/0021-native-project-recovery-capsules.md; accepted project/platform-macos APIs
Legacy provenance: original dirty-source hashes in artifacts/WO-RUST-PLAN-01/source-baseline.json. No external source fetch.

# Files allowed to touch
- `work/WO-RUST-PROJECT-IO-01.md` (coordinator freeze only)
- `rust/crates/vexel-project-io/src/**`, `rust/crates/vexel-project-io/tests/**`
- `artifacts/WO-RUST-PROJECT-IO-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths and quality
All legacy sources/fixtures/oracles, accepted APIs/behavior/assertions, all other Rust crates, all Cargo/config/lock/toolchain metadata, other worktrees, earlier artifacts/orders and coordinator records. Worker only own src/tests/artifacts; coordinator alone shared interfaces/metadata/integration. No dependencies/download/install/network, unsafe/new sys boundary, C shim, unsupported OS code, staging/commit/delete/reset, recursive agents or broad UI/E2E/fuzz/performance campaign. Mac13+ AppleSilicon only, MPL2, no GPL-editor source or external SVG fetch. Explicit ownership/lifetimes, bounded input/checked arithmetic/fallible significant allocations. Source<800lines; split before exceeding. No TODO/unimplemented/mock production paths for named scope. If needed contract is absent, stop with concrete dependency rather than narrowing scope or editing peer crate. Every named C public API/helper mapped in parity-map.json to symbols/tests/limits/ownership/errors/exact safe adaptations; preserve meaningful assertions, not only is_err checks.

# Execution and verification
Only /Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust4-project-io; Standard subscription Luna High, one writer. Read OWNER-DIRECTIVES/AGENTS/order; use accepted source contracts.
cargo +stable check -p vexel-project-io --locked --offline
cargo +stable test -p vexel-project-io --locked --offline
python3 scripts/wo_validate.py WO-RUST-PROJECT-IO-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact existing Rust1.99/macOS13, repo-local Cargo/cache/tmp/targets, no fmt/clippy install or claim.

# Output
Uncommitted candidate, result<=400words, parity-map.json/verification.txt with exact checks/adaptations/failures/remaining downstream work. Independent review, preservation audit and named main checks required before acceptance. Existing subscription allowance only; no purchased credits/API billing. Stop/checkpoint on allowance/rate errors, no retry loop.
