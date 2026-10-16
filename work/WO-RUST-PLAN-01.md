---
id: WO-RUST-PLAN-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: ac1b0c375f8236716ac4975c381ed3fd0e7a372e08bf47ae53e45c813b1614f5
parallel_safe: true
network: true
---
# Goal
Record the owner's replacement direction: full Rust port and full product delivery. Stop former C implementation, preserve its evidence, deeply research six independent migration areas with actual parallel Luna codex exec processes, then freeze a dependency-aware parity and completion plan.

# Files allowed to touch
- `work/WO-RUST-PLAN-01.md`, `OWNER-DIRECTIVES.md`, `AGENTS.md`
- `docs/adr/0022-full-rust-port.md`
- `docs/product/rust-port-goal-and-plan.md`, `docs/product/luna-parallel-delivery-plan.md`, `docs/product/vexel-goal-and-delivery-plan.md`
- `artifacts/WO-RUST-PLAN-01/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
All current application/source/tests/fixtures/oracles and worker source trees. No C candidate integration, code deletion/reset/staging/commits, new system packages, sibling edits, broad UI/E2E/automatic test campaign. Research outputs do not constitute implementation or acceptance. No GPL editor source read/fetch/copy; no weakening of platform/license/source-preservation product locks.

# Requirements
1. All first-party application/platform/parser/raster/document/animation/MCP/UI code and in-repo development/test tooling ultimately Rust. OS frameworks remain external platform dependencies. No shipped first-party C/Objective-C implementation hidden behind Rust. Historical code preserved until parity retirement.
2. Full ADR0018 product finish line persists: modern custom UI, SVG/Inkscape interoperability, native drawing/animation/assets/recovery, large-file performance, live user-controlled Codex/Claude MCP, final acceptance and recorded character+animation example for website/README.
3. Six read-only Luna exec processes: toolchain/build/tooling; macOS/custom UI; XML/SVG/renderer; documents/animation/recovery; MCP/concurrency; migration inventory/parallel sequencing. Low/Medium/High effort scoped; existing ChatGPT allowance, Standard service, no API billing/credit purchase. Primary official sources only; exact source links/current versions/license evidence, unresolved decisions explicit. Shared manifest/build/dependency/ledger integration belongs only to coordinator; isolated one-writer module implementation after APIs freeze.
4. Preserve current main and unfinished capture candidate, identify lifecycle state honestly. Current goal tracker paused because owner stopped old work; replacement create rejected unfinished goal. Record replacement repository goal without marking prior goal achieved.
5. Complete code-file inventory and dependency seams, parallel batches and ready scopes, parity gates/default strict inputs, unsafe FFI policy, pinned provenance/dependencies, honest estimates after inventory; no Rust speed/size guarantee without measurements. Broad UI/E2E final; focused regressions during migration.

# Verification
python3 scripts/wo_validate.py WO-RUST-PLAN-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
Validate research/process/inventory JSON and source links; confirm worker outputs/exit status, no product source mutation and preserved HEAD/index. Review dependency/ownership plan for gaps. git diff --check. No application tests/builds required by planning order.

# Output
A concrete repository goal and research-backed parallel Rust migration plan. Toolchain/dependency decisions beyond the owner-approved Rust language are frozen only after research. Leave uncommitted; actual Rust implementation requires separate ready orders.
