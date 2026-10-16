---
id: WO-RUST-SVG-COLOR-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 69d657ed27f89784601df339feca1601854a32d15d8090df7f2f1d2a63f7ce4d
parallel_safe: true
network: false
---
# Goal and frozen scope
Port full color.c/h owned CSS/SVG color parser. Expose parse_color(source:&[u8],span:vexel_base::Span)->Option<[u8;4]>; valid original containing source may be large, only checked requested span is scanned. Preserve every original named color value/alias, transparent, case folding, CSS whitespace, all named hex/rgb/rgba/hsl/hsla syntaxes, comma vs space/slash grammar, percentages/mixed components, alpha/clamping/rounding, hue units/wrapping, strict number suffix/exponent and malformed rejection. currentColor stays context-dependent and unsupported here. No external lookup/dependency or replacement SVGvalue number parser with different whitespace rules. Port exact color fixture outputs (including nontrivial rounding/hue/alpha), rejection/trailing/span-overflow/no-partial-output behavior; frozen named-color table fingerprint or exhaustive table assertions. Safe code no unsafe, unnecessary allocation or whole-source copy. CSS cascade/computed paint is next separate scope, no full SVG rendering claim.

# Reference paths
src/svg/css/color.c/h; tests/svg/css/test_css.c and focused color assertions in tests/svg/shape
Legacy provenance: original dirty-source hashes in artifacts/WO-RUST-PLAN-01/source-baseline.json. No external source fetch.

# Files allowed to touch
- `work/WO-RUST-SVG-COLOR-01.md` (coordinator freeze only)
- `rust/crates/vexel-svg-color/src/**`, `rust/crates/vexel-svg-color/tests/**`
- `artifacts/WO-RUST-SVG-COLOR-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths and quality
All legacy sources/fixtures/oracles, accepted APIs/behavior/assertions, all other Rust crates, all Cargo/config/lock/toolchain metadata, other worktrees, earlier artifacts/orders and coordinator records. Worker only own src/tests/artifacts; coordinator alone shared interfaces/metadata/integration. No dependencies/download/install/network, unsafe/new sys boundary, C shim, unsupported OS code, staging/commit/delete/reset, recursive agents or broad UI/E2E/fuzz/performance campaign. Mac13+ AppleSilicon only, MPL2, no GPL-editor source or external SVG fetch. Explicit ownership/lifetimes, bounded input/checked arithmetic/fallible significant allocations. Source<800lines; split before exceeding. No TODO/unimplemented/mock production paths for named scope. If needed contract is absent, stop with concrete dependency rather than narrowing scope or editing peer crate. Every named C public API/helper mapped in parity-map.json to symbols/tests/limits/ownership/errors/exact safe adaptations; preserve meaningful assertions, not only is_err checks.

# Execution and verification
Only /Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust4-colors; Standard subscription Luna Medium, one writer. Read OWNER-DIRECTIVES/AGENTS/order; use accepted source contracts.
cargo +stable check -p vexel-svg-color --locked --offline
cargo +stable test -p vexel-svg-color --locked --offline
python3 scripts/wo_validate.py WO-RUST-SVG-COLOR-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact existing Rust1.99/macOS13, repo-local Cargo/cache/tmp/targets, no fmt/clippy install or claim.

# Output
Uncommitted candidate, result<=400words, parity-map.json/verification.txt with exact checks/adaptations/failures/remaining downstream work. Independent review, preservation audit and named main checks required before acceptance. Existing subscription allowance only; no purchased credits/API billing. Stop/checkpoint on allowance/rate errors, no retry loop.
