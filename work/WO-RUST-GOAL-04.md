---
id: WO-RUST-GOAL-04
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: fffad6f8b74be3f3b47226671e660b1e90bdeaa0c6fa40d7d639f598d029129d
parallel_safe: false
network: false
---
# Goal
Consolidate actual Rust worker execution and accepted code with the full completed/tested/ready-to-use application goal into one dependency-aware execution plan.

# Files allowed to touch
- `work/WO-RUST-GOAL-04.md`
- `docs/product/rust-port-goal-and-plan.md`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`
- `artifacts/WO-RUST-GOAL-04/**`

# Protected paths
All application code, Rust manifests/lock, legacy code, fixtures/oracles, worker trees, frozen prior work orders and evidence. Do not copy unreviewed candidate code, run application tests/builds, stage, commit, install or delete source in this documentation order.

# Requirements
Preserve full Rust/product final outcome and owner locks. Reconcile the main accepted foundation ledgers with all six current worker statuses, results, verification records and parity maps. Create a consolidated machine-readable checkpoint and link module scopes/evidence, review risks, remaining dependencies and final product requirements. Separate worker-reported checks from main-tree acceptance and never sum overlapping candidate tests into a unique app test count. Define next integration gate and parallel downstream lanes, full final testing/repair/release/demo gates and scope inventory closure. Repository plan is authoritative because replacement of unfinished paused tracker was rejected; do not falsely complete old goal or repeat a known failing registration loop. Existing subscription-only Luna parallel policy remains. No new dependency decision or completion/performance/allowance promise.

# Verification
python3 scripts/wo_validate.py WO-RUST-GOAL-04 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Validate checkpoint JSON against current accepted ledgers and worker files; review plan coverage of nine product outcomes, source preservation and final readiness gates. Confirm HEAD/index unchanged and order writes restricted to allowed paths. No application tests/builds.

# Output
One consolidated repository goal and execution plan incorporating work already run, accepted evidence, pending candidate integration and all work through tested release and recorded demos.
