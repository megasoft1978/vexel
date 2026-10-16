---
id: WO-RUST-SOURCE-SAVE-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: d0d3c5d17b3cd82b3c25eef4be355a53b05704479314b889bf29fcab86858818
parallel_safe: true
network: false
---
# Goal and frozen scope
Port full source_save.c/h against accepted XML tokenizer and native atomic replace/create. Safe source_save_noop accepts borrowedsource/rawbytepath and token_capacity (owned bounded tokens adapt caller array); returns exact source/parse/nativeerror andfile_synced/renamed/directory_synced state. Match argument/path/source-limit precedence,16MiBlegacy defaultsource limit, tokenize before ANYdestination calls, exact diagnostic categories/offsets and outputexhaustion. Valid input publishes byte-for-byte unchanged; replaceNotFound falls back no-replace create, forwards originalwrite gate only, preserves collision/nonregular/symlink/race/error andpre/postcommit durability outcomes. Hooks remain safe observation/fault gates and successful gate MUST actualnative op. All fixtures repo-local .cache/source-save-*; port original meaningful unchanged bytes/unknown XML/namespaces/metadata/comments, malformed/outcapacity/invalidpath-before-adapter, existing/new files/races/tempcleanup/durability. Typed Rust OOM adaptation for tokenizer allocation must be recorded without inventing deterministic systemOOM. No usergrants/appdirty state/view/read-only flags or large-source SAVE API added implicitly; later integration must close100MBworkflow andcannotclaimfullproduct fromlegacy16Msave.

# Reference paths
src/document/source_save.c/h; tests/document/test_source_save.c; accepted vexel-xml and vexel-platform-macos atomic hooks/types
Legacy provenance: original dirty-source hashes in artifacts/WO-RUST-PLAN-01/source-baseline.json. No external source fetch.

# Files allowed to touch
- `work/WO-RUST-SOURCE-SAVE-01.md` (coordinator freeze only)
- `rust/crates/vexel-document-io/src/**`, `rust/crates/vexel-document-io/tests/**`
- `artifacts/WO-RUST-SOURCE-SAVE-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths and quality
All legacy sources/fixtures/oracles, accepted APIs/behavior/assertions, all other Rust crates, all Cargo/config/lock/toolchain metadata, other worktrees, earlier artifacts/orders and coordinator records. Worker only own src/tests/artifacts; coordinator alone shared interfaces/metadata/integration. No dependencies/download/install/network, unsafe/new sys boundary, C shim, unsupported OS code, staging/commit/delete/reset, recursive agents or broad UI/E2E/fuzz/performance campaign. Mac13+ AppleSilicon only, MPL2, no GPL-editor source or external SVG fetch. Explicit ownership/lifetimes, bounded input/checked arithmetic/fallible significant allocations. Source<800lines; split before exceeding. No TODO/unimplemented/mock production paths for named scope. If needed contract is absent, stop with concrete dependency rather than narrowing scope or editing peer crate. Every named C public API/helper mapped in parity-map.json to symbols/tests/limits/ownership/errors/exact safe adaptations; preserve meaningful assertions, not only is_err checks.

# Execution and verification
Only /Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust6-source-save; Standard subscription Luna High, one writer. Read OWNER-DIRECTIVES/AGENTS/order; use accepted source contracts.
cargo +stable check -p vexel-document-io --locked --offline
cargo +stable test -p vexel-document-io --locked --offline
python3 scripts/wo_validate.py WO-RUST-SOURCE-SAVE-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact existing Rust1.99/macOS13, repo-local Cargo/cache/tmp/targets, no fmt/clippy install or claim.

# Output
Uncommitted candidate, result<=400words, parity-map.json/verification.txt with exact checks/adaptations/failures/remaining downstream work. Independent review, preservation audit and named main checks required before acceptance. Existing subscription allowance only; no purchased credits/API billing. Stop/checkpoint on allowance/rate errors, no retry loop.
