---
id: WO-PRODUCT-PLAN-02
revision: 2
status: READY
milestone: PRODUCT
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: c69c0250e84e126060390720b0eff064fdab78900d1fc0ae45556d130c1cbd41
parallel_safe: false
network: false
---
# Goal
Refresh the active Vexel delivery plan against accepted work and current evidence, and state the shortest truthful next sequence without shrinking the owner-approved product goal.

# Context
- Follow `OWNER-DIRECTIVES.md`, `AGENTS.md`, ADR-0018 through ADR-0021, the current `STATE.md`, and all accepted work-order evidence referenced by the product plan.
- WO-SVG-PAINT-02 materially improves one T1 full-document comparison but does not establish corpus acceptance.
- Native project recovery now includes background capture and restore for existing source files; missing-source recovery, retention, migration, and compatibility behavior remain open.
- Animate-workspace Undo/Redo integration is implemented; broader user-authored character animation remains unfinished.
- Preserve the final comprehensive UI/E2E acceptance and post-acceptance recorded showcase sequence. Broad UI/E2E remains deferred during feature implementation.
- `src/mcp/README.md` and the current broker implement an empty text template plus `text_set` insertion. The root `docs/mcp.md` incorrectly claims text insertion is unavailable and omits newer tools. Progress/cancellation remains unwired.
- The worktree is substantially modified and uncommitted. Preserve all implementation and prior evidence.

# Files allowed to touch
- `work/WO-PRODUCT-PLAN-02.md`
- `docs/product/vexel-goal-and-delivery-plan.md`
- `artifacts/WO-PRODUCT-PLAN-02/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Do not edit product/architecture ADRs, application source, tests/oracles/goldens, prior work orders/evidence, README, website/deployment files, or any path outside the allowlist.
- Do not run application tests, broad UI/E2E, corpus, performance, fuzz, sanitizer, or release suites in this documentation-only order.
- No changes to product scope, platform locks, or completion criteria.

# Required changes
1. Reconcile implemented/partial/unverified claims with current STATE and accepted work-order evidence, including WO-SVG-PAINT-02, WO-ANIM-PROJECT-28, and WO-PROJECT-05 through WO-PROJECT-08.
2. Remove completed work from the immediate next-action list. Reconcile root MCP documentation with the implemented text template/text_set behavior and current tool surface; distinguish this documentation drift from the still-missing progress/cancellation and real-client interoperability evidence. Name the next exact steps for SVG residual/corpus fidelity, the user-authored character workflow, project recovery/compatibility, large-file measurements, and UI/accessibility closure.
3. Keep the 3/39 accepted corpus result distinct from the 5/39 unaccepted candidate, and report the T1 12.099% to 0.783% tolerance-2 improvement as single-document candidate evidence only.
4. Preserve uncertainty and rough effort bands, the worktree isolation limits for parallel work, focused checks during implementation, and broad UI/E2E as the final software gate before the recorded/published Codex example.
5. Record a concise implementation summary and verification evidence; update STATE.md and both ledgers. No application code or application tests are required.

# Verification
- `python3 scripts/wo_validate.py WO-PRODUCT-PLAN-02 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`
- Manually review plan claims against ADR-0018, current STATE.md, and cited accepted evidence. Confirm the goal, platform and engineering locks, estimates, work order dependencies, and final acceptance/showcase order remain consistent.

# Output
Leave changes uncommitted. Report the refreshed critical path and exact documentation checks. Keep the full product goal active.
