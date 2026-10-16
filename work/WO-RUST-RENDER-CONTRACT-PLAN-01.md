---
id: WO-RUST-RENDER-CONTRACT-PLAN-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 596baaba56cc119c521cc75d2561f209b31d888188b0e8945561926201aa8725
parallel_safe: true
network: false
---
# Goal
Produce actionable complete Rust interfaces and dependency-ready port partitions
for ALL remaining owned SVG scene/decoder/paint/clip/effects/media and scalar
raster kernels, preserving the full vector/cartoon/MCP/large-file product goal.
This is a contract plan only; it must unblock code, not select an external SVG
renderer, omit supported legacy behaviors or shrink the application.

# Files allowed to touch
- `work/WO-RUST-RENDER-CONTRACT-PLAN-01.md` (coordinator freeze only)
- `artifacts/WO-RUST-RENDER-CONTRACT-PLAN-01/**`
- `docs/product/rust-render-contract-plan.md`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Required work and bounded research
Read OWNER-DIRECTIVES/AGENTS/ADR0022/23/24/25/coding standard, accepted Rust
contracts (geometry/path/values/color/CSS/reference/XML/animation/raster/spatial/
tile/task/document) and existing rendering research. Source inventory is
src/svg/shape (including .h/internal/alloc/media/custom/selection/document),
svg/paint/clip, remaining src/raster shape_* and injected macOS media headers.
Discover every named source file; inspect public/internal headers, include
edges and function signatures, then only implementation excerpts needed to
resolve ownership/dependencies/errors/work/memory contracts. Do not dump/read
whole trees, old logs or whole huge sources gratuitously.
Map every remaining C implementation to exactly one full port or explain an
already accepted replacement with symbol evidence. Resolve shape/paint/clip/
pattern/mask recursive dependencies through owned data and explicit operations,
not stub functions, fake providers, duplicate shadow parsers or self-references.
Propose minimal crate/module layout, complete shared type/API signatures,
source Arc/spans/cached tables, bound/capacity/error/diagnostic contracts,
media ownership/drop/thread requirements, cancellation/work charging and frozen
read/write interfaces. Preserve exact supported feature breadth, unsupported
reports, paint order, fill/stroke/effects/math operation order, source/transform/
path metadata and scalar rendering fidelity. No new unsafe except later audited
native boundary, no new dependencies, external SVG fetch, GPL editor source,
unsupported OS or speculative SIMD/GPU rewrite. C kernels are the migration
reference; paper directions remain testable inferences.

Identify hard accepted-contract gaps (for example owned animation views without
cloning/reparsing) separately with minimal additive API changes; do not consume
unaccepted editor/native candidates. Provide viable disjoint writers after
shared contracts land, one owner per module and coordinator-only composition.
Any implementation partition must contain its entire named C/API scope and
original meaningful assertions; do not create convenient smaller renderer.
Map required original focused suites/fixtures/limits to each port and preserve
24 frozen authored goldens and existing raster/oracle tests for their respective
acceptance stage. Final broad UI/E2E/corpus/performance/release/live demos remain
explicit. Large100MB editing/rendering/save and budgets remain full product
requirements even where old parity entry points have16MiB defaults.

# Protected paths and verification
All product/peer code, other docs/orders/artifacts/manifests/locks and index/HEAD/
original603legacy. Own report/evidence only; no source, Cargo, ledger edits by
worker, dependencies/network/install/builds/tests/recursive agents/commit/stage/
reset/delete. Actual Standard subscription Sol6.1Medium, narrow cross-file
architecture scope; no purchasedcredits/APIbilling, stop allowanceerrors.
python3 scripts/wo_validate.py WO-RUST-RENDER-CONTRACT-PLAN-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Root preservation audit and independent focused contract review required.

# Output
Concise report<=2000words, machine-readable complete per-file/symbol ownership
matrix, dependency DAG, signatures/contracts/limits/error/source/thread data,
original assertion assignments, implementation launch order and exact proof
commands. Result<=350words. Every omitted or unresolved feature is explicit;
no application, renderer or performance acceptance claim. Coordinator freezes
implementation scopes and registration after review.
