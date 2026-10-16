---
id: WO-RUST-SVG-REFERENCE-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: eb5bde203919cbec9bbf54333041ee903742a42a89e63fabfbcdd4c4ea78e0a6
parallel_safe: true
network: false
---
# Goal and frozen scope
Port EVERY function/helper in src/svg/reference/reference.c/h. Safe ReferenceIndex borrows immutable source/tokens; fixed open-addressed slot table for nonemptyIDs,65536ID/131072slot caps, exactFNV hash/rawspan duplicate/lookup collision rules, lookup probe counters and actualfixedstorage bytes, emptyindex noalloc, atomic failurecleanup. Complete definition_end, attributes_supported(root/kind), xlink_is_bound, use_attributes_supported helpers and all original attribute allowlists. Define local typed ReferenceError/Diagnostic mirroring used svg_shape_error categories/offsets; later shape crate converts them, no dependency on unfinished shape scene. Preserve original raw-ID/entity behavior with exact adaptation notes; do not silently decode/rewrite reference keys while claimingCparity. Checked token indices/spans and malformed callerarrays safe rejection, no16MiBwhole-sourcecap or network resources. Full named reference funcs/tests including missing/empty/duplicate/collisions/excessIDs/definition-depth/useattrs/boundnamespace/lookupstats/failureownership; source<800/safe/std-only/fallible significant allocations. This provides resolution primitives, not fulluse traversal/render/cycle handling outside namedCfile.

# Reference paths
src/svg/reference/reference.c/h; src/svg/shape/shape.h error/diagnostic declarations ONLY; shape_alloc.h allocation contract; focused reference/use/xlink cases in tests/svg/shape; accepted vexel-xml token/spans
Legacy provenance: original dirty-source hashes in artifacts/WO-RUST-PLAN-01/source-baseline.json. No external source fetch.

# Files allowed to touch
- `work/WO-RUST-SVG-REFERENCE-01.md` (coordinator freeze only)
- `rust/crates/vexel-svg-reference/src/**`, `rust/crates/vexel-svg-reference/tests/**`
- `artifacts/WO-RUST-SVG-REFERENCE-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths and quality
All legacy sources/fixtures/oracles, accepted APIs/behavior/assertions, all other Rust crates, all Cargo/config/lock/toolchain metadata, other worktrees, earlier artifacts/orders and coordinator records. Worker only own src/tests/artifacts; coordinator alone shared interfaces/metadata/integration. No dependencies/download/install/network, unsafe/new sys boundary, C shim, unsupported OS code, staging/commit/delete/reset, recursive agents or broad UI/E2E/fuzz/performance campaign. Mac13+ AppleSilicon only, MPL2, no GPL-editor source or external SVG fetch. Explicit ownership/lifetimes, bounded input/checked arithmetic/fallible significant allocations. Source<800lines; split before exceeding. No TODO/unimplemented/mock production paths for named scope. If needed contract is absent, stop with concrete dependency rather than narrowing scope or editing peer crate. Every named C public API/helper mapped in parity-map.json to symbols/tests/limits/ownership/errors/exact safe adaptations; preserve meaningful assertions, not only is_err checks.

# Execution and verification
Only /Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust6-references; Standard subscription Luna High, one writer. Read OWNER-DIRECTIVES/AGENTS/order; use accepted source contracts.
cargo +stable check -p vexel-svg-reference --locked --offline
cargo +stable test -p vexel-svg-reference --locked --offline
python3 scripts/wo_validate.py WO-RUST-SVG-REFERENCE-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact existing Rust1.99/macOS13, repo-local Cargo/cache/tmp/targets, no fmt/clippy install or claim.

# Output
Uncommitted candidate, result<=400words, parity-map.json/verification.txt with exact checks/adaptations/failures/remaining downstream work. Independent review, preservation audit and named main checks required before acceptance. Existing subscription allowance only; no purchased credits/API billing. Stop/checkpoint on allowance/rate errors, no retry loop.
