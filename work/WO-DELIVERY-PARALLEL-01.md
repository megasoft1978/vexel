---
id: WO-DELIVERY-PARALLEL-01
revision: 1
status: READY
milestone: DELIVERY
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 7727cf31536c475fca37c1f7b77d06e943fc826f7165945b6e2e2b3887e0ec96
parallel_safe: false
network: false
---
# Goal
Record an actionable Luna execution plan for the complete ADR-0018 product with safe parallel subtasks and existing subscription usage only.

# Files allowed to touch
- `work/WO-DELIVERY-PARALLEL-01.md`
- `docs/product/vexel-goal-and-delivery-plan.md`
- `docs/product/luna-parallel-delivery-plan.md`
- `artifacts/WO-DELIVERY-PARALLEL-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
No application code, tests, frozen prior orders or acceptance thresholds. Preserve all dirty changes. No system packages, paid credit purchases, global configuration changes, unsupported platforms or broad UI/E2E.

# Required implementation
1. Define task effort, ownership, dependencies, focused checks, independent review and integration policy. Retain every ADR-0018 gate and final recorded character/animation website+README example.
2. Use existing subscription allowance; Standard speed; Luna Low default, Medium ordinary modules, High ownership/concurrency, XHigh/Max only when narrowed failed attempts justify it. No automatic Sol/Astra escalation or paid purchase.
3. At most two implementation workers plus coordinator. Thin task briefs, scoped reads, compact outputs, no full-history forks. Actual usage is recorded when available; unknown account consumption stays unknown.
4. Isolated repo-local worktrees with current dirty source overlay and fingerprinted baseline. Disjoint implementation paths and build outputs. One coordinator serializes ledger, test-map and shared integration changes; validates worker deltas before copying.
5. Record current evidence and launch the next independent bounded feature subtasks under their own frozen orders.

# Acceptance criteria
Plan names owners, task effort, expected exit evidence, sequence/parallel dependencies, unresolved gates, and cost control. Product roadmap links it. Frozen metadata, ledger and git diff --check pass. No application test run required for planning prose.

# Verification
python3 scripts/wo_validate.py WO-DELIVERY-PARALLEL-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check

# Output
Leave changes uncommitted and record evidence. The full product goal remains active.
