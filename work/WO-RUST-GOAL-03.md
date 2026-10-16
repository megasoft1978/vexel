---
id: WO-RUST-GOAL-03
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: eaa1d640af76c0a0a6954001111d052086a5e7db5bbdf60b66c5999d90e27c3d
parallel_safe: false
network: false
---
# Goal
Make the owner's final goal explicit: the full Rust Vexel application completed, comprehensively tested, packaged, documented and ready for real use.

# Files allowed to touch
- `work/WO-RUST-GOAL-03.md`
- `docs/product/rust-port-goal-and-plan.md`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`
- `artifacts/WO-RUST-GOAL-03/**`

# Protected paths
All product code, Rust manifests/lock, legacy code, fixtures/oracles, worker trees, prior work orders and evidence. Documentation only; no application tests/builds, installations, staging, commits or source deletion.

# Requirements
Preserve the nine outcome areas and owner platform/license/implementation/billing locks. Define full release readiness, explicit final test gates and no release-blocking defects; preserve broad UI/E2E after feature completion and focused correctness checks during implementation. Include fresh-install packaged usability, user documentation, actual live agent sessions and final recorded demos. Record tracker API restriction without falsely completing old goal. Correct only stale second-batch process checkpoint using current status; do not treat successful worker exits as acceptance.

# Verification
python3 scripts/wo_validate.py WO-RUST-GOAL-03 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Review final goal against owner requirements and ADR0018/0022. Confirm HEAD/index unchanged and order writes limited to allowed paths. No application tests or broad UI campaign for this documentation order.

# Output
A clear final product goal and measurable ready-to-use definition in the authoritative repository plan and state, with recorded documentation acceptance.
