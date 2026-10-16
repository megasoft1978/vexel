---
id: WO-RUST-SVG-CSS-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 16a7efed9c3ac031b416a1ce8fd65f0fd98f2e181c9288e4d3d7a91d44ead878
parallel_safe: true
network: false
---
# Goal and frozen scope
Port EVERY function/helper in src/svg/css/css.c/h, including header inline keyword/value access, against accepted XmlToken source spans. Safe borrowing CssSheet/Paint/Property/Error types for full39property subset, styleelement extraction, bounded selector/compound/descendant matching, specificity/source order/presentation attributes/inline!important/inheritance/initial values and stateful rule-match quota. Preserve1MiBaggregateCSS/4096rules/16384declarations/16classes/4selector terms/4194304match bounds, exact Unsupported/Limit/OOM distinctions, unknownstyle outcomes, comment/quote/semicolon grammar, local XML prefix rules and source spans. No16MiBcap on containing SVG, no whole-source clone/rewrite, no externalURLfetch. Keep raw source preserves unsupported content; do not implement a convenient CSS subset beyond frozenCscope. Significant allocations fallible/bounded with actualcapacity charges where specified. Test meaningful original CSS cases (all39property extraction; specificity/inheritance/important/inline/presentation; selectorchains/compoundclasses/namespaced/styleCDATA; unsupportedselector/media/import/malformed/limits and matches). Do not weaken original oracles. Full scene/render/font/effects integration later; no fullSVGclaim.

# Reference paths
src/svg/css/css.c/h; tests/svg/css/test_css.c and named CSS assertions in tests/svg/shape; accepted vexel-xml token/span APIs
Legacy provenance: original dirty-source hashes in artifacts/WO-RUST-PLAN-01/source-baseline.json. No external source fetch.

# Files allowed to touch
- `work/WO-RUST-SVG-CSS-01.md` (coordinator freeze only)
- `rust/crates/vexel-svg-css/src/**`, `rust/crates/vexel-svg-css/tests/**`
- `artifacts/WO-RUST-SVG-CSS-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths and quality
All legacy sources/fixtures/oracles, accepted APIs/behavior/assertions, all other Rust crates, all Cargo/config/lock/toolchain metadata, other worktrees, earlier artifacts/orders and coordinator records. Worker only own src/tests/artifacts; coordinator alone shared interfaces/metadata/integration. No dependencies/download/install/network, unsafe/new sys boundary, C shim, unsupported OS code, staging/commit/delete/reset, recursive agents or broad UI/E2E/fuzz/performance campaign. Mac13+ AppleSilicon only, MPL2, no GPL-editor source or external SVG fetch. Explicit ownership/lifetimes, bounded input/checked arithmetic/fallible significant allocations. Source<800lines; split before exceeding. No TODO/unimplemented/mock production paths for named scope. If needed contract is absent, stop with concrete dependency rather than narrowing scope or editing peer crate. Every named C public API/helper mapped in parity-map.json to symbols/tests/limits/ownership/errors/exact safe adaptations; preserve meaningful assertions, not only is_err checks.

# Execution and verification
Only /Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust6-css; Standard subscription Luna High, one writer. Read OWNER-DIRECTIVES/AGENTS/order; use accepted source contracts.
cargo +stable check -p vexel-svg-css --locked --offline
cargo +stable test -p vexel-svg-css --locked --offline
python3 scripts/wo_validate.py WO-RUST-SVG-CSS-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact existing Rust1.99/macOS13, repo-local Cargo/cache/tmp/targets, no fmt/clippy install or claim.

# Output
Uncommitted candidate, result<=400words, parity-map.json/verification.txt with exact checks/adaptations/failures/remaining downstream work. Independent review, preservation audit and named main checks required before acceptance. Existing subscription allowance only; no purchased credits/API billing. Stop/checkpoint on allowance/rate errors, no retry loop.
