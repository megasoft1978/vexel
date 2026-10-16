---
id: WO-RUST-SVG-INDEX-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 710311f2f5d816abc33894a96fcd53951bc6506ed89c0be1ff9ba007341cac39
parallel_safe: true
network: false
---
# Goal and frozen scope
Port EVERY function/helper in src/svg/index/index.c/h against accepted XmlReader, XmlTokenStream and TaskQueue. Safe IndexSession borrows immutable caller Arc<Vec<u8>> for lifetime: reader borrows caller bytes while validation worker clones Arc, avoiding self-referential struct, unsafe borrowed thread pointers or unconditional entire-source copy. Source bytes unchanged; no smaller source cap. Full owner lexical scan plus actual independent background bounded validation, start/step/progress/generation/preview/currentness/cancel/elements/tokens/synthetic preview/drop. Preserve1GiBsource/64KiBmaxstep/1Melements/1Mattributes/256depth/256MiBstorage with128MiBcomponent budgets; charge actual reserved metadata/token storage. Preserve original support subset/root/attribute gate, first completed supported rect/path preview and validation frontier gate, exact diagnostics/error precedence, no tokens until full validation, generation invalidation and owned <=16MiB synthetic preview bytes. Shutdown cancels/joins worker before shared buffers release; successfulworker result ownership/atomic visibility correct and error storage not partially exposed. Accepted taskqueue callbacks always run after Running even canceled: validator polls token and cannot publish canceled/stale results. No placeholder synchronous validation/thread-blocking join per step or scan-then-whole-parse substitute. Port meaningful original index cases: incremental budget/progress, previews versus malformed prefix/tail, unsupported root/attrs, generation, cancellation, full token source spans, nested parent/source spans, resource bounds and teardown. Deterministic bounded worker wait seams allowed, no timing-only flaky tests/huge perf campaign. UI/index progress and renderer integration later; index alone does not prove large-file performance.

# Reference paths
src/svg/index/index.c/h; tests/svg/index/test_index.c, test_token_stream.c; accepted vexel-xml and vexel-platform-tasks APIs
Legacy provenance: original dirty-source hashes in artifacts/WO-RUST-PLAN-01/source-baseline.json. No external source fetch.

# Files allowed to touch
- `work/WO-RUST-SVG-INDEX-01.md` (coordinator freeze only)
- `rust/crates/vexel-svg-index/src/**`, `rust/crates/vexel-svg-index/tests/**`
- `artifacts/WO-RUST-SVG-INDEX-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths and quality
All legacy sources/fixtures/oracles, accepted APIs/behavior/assertions, all other Rust crates, all Cargo/config/lock/toolchain metadata, other worktrees, earlier artifacts/orders and coordinator records. Worker only own src/tests/artifacts; coordinator alone shared interfaces/metadata/integration. No dependencies/download/install/network, unsafe/new sys boundary, C shim, unsupported OS code, staging/commit/delete/reset, recursive agents or broad UI/E2E/fuzz/performance campaign. Mac13+ AppleSilicon only, MPL2, no GPL-editor source or external SVG fetch. Explicit ownership/lifetimes, bounded input/checked arithmetic/fallible significant allocations. Source<800lines; split before exceeding. No TODO/unimplemented/mock production paths for named scope. If needed contract is absent, stop with concrete dependency rather than narrowing scope or editing peer crate. Every named C public API/helper mapped in parity-map.json to symbols/tests/limits/ownership/errors/exact safe adaptations; preserve meaningful assertions, not only is_err checks.

# Execution and verification
Only /Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust5-index; Standard subscription Luna High, one writer. Read OWNER-DIRECTIVES/AGENTS/order; use accepted source contracts.
cargo +stable check -p vexel-svg-index --locked --offline
cargo +stable test -p vexel-svg-index --locked --offline
python3 scripts/wo_validate.py WO-RUST-SVG-INDEX-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact existing Rust1.99/macOS13, repo-local Cargo/cache/tmp/targets, no fmt/clippy install or claim.

# Output
Uncommitted candidate, result<=400words, parity-map.json/verification.txt with exact checks/adaptations/failures/remaining downstream work. Independent review, preservation audit and named main checks required before acceptance. Existing subscription allowance only; no purchased credits/API billing. Stop/checkpoint on allowance/rate errors, no retry loop.
