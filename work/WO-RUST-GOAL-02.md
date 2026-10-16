---
id: WO-RUST-GOAL-02
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 2ef0ce3af8d133a17e6095555a0db38302603f1a83b34c68d21c1acf3a5df27e
parallel_safe: false
network: false
---
# Goal
Detail the owner's full Rust migration and finished Vexel product goal, with deliverables, measurable acceptance, dependency waves, parallel execution and truthful checkpoints.

# Files allowed to touch
- `work/WO-RUST-GOAL-02.md`
- `docs/product/rust-port-goal-and-plan.md`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`
- `artifacts/WO-RUST-GOAL-02/**`

# Protected paths
All application sources, Rust implementations/manifests/lock, legacy code, immutable fixtures/oracles, worker trees and other ready work orders. No code edits, installations, staging, commits, source deletion or application tests in this documentation order.

# Requirements
Preserve owner locks and ADR0018 product gates, superseding its historical C17 clause through ADR0022. Define all-first-party Rust scope and source inventory accounting; vector editing, SVG/Inkscape source preservation, native projects/recovery, animation, custom modern UI, measurable performance, live user-controlled MCP and real recorded demo deliverables. Preserve focused work-order checks during implementation and defer broad UI/E2E until feature completion. Specify useful isolated parallel Luna workers and coordinator integration, subscription-only usage and escalation policy. Separate accepted code from candidate worker completion; retain legacy evidence. Attempt the requested goal registration but record the unfinished-goal API restriction without falsely completing the old goal. No new dependency approval, performance promise or revised schedule guarantee.

# Verification
python3 scripts/wo_validate.py WO-RUST-GOAL-02 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Review goal against OWNER-DIRECTIVES.md, ADR0018, ADR0022 and actual process status. Audit this order's edits against allowed paths and unchanged HEAD/index. No application tests or broad UI checks.

# Output
Detailed authoritative repository goal with an explicit release definition and current next steps, recorded in STATE and ledgers; tracker limitation stated accurately.
