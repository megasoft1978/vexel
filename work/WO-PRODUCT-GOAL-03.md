---
id: WO-PRODUCT-GOAL-03
revision: 1
status: READY
milestone: M0
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 218565817cf8f6495efa162f93e6d370cf853335c0b4e5e8df7267ccb889eb57
parallel_safe: false
network: false
---
# Goal
Reconcile the owner-approved product delivery plan and README with the native project, animation, UI, MCP, corpus, and performance capabilities actually present in the current worktree and accepted work-order evidence.

# Context
- Read `OWNER-DIRECTIVES.md`, ADR-0018 through ADR-0020, current `STATE.md`, the animation work-order evidence through WO-ANIM-PROJECT-24, corpus diagnostics, and MCP documentation.
- The existing plan still describes the native timeline as opaque and uneditable, while project animation schema, authoring, playback, rendering, export, and MCP capabilities now exist in the working tree.
- Do not promote the partial `WO-CORPUS-04` candidate or unverified performance figures to accepted results. Distinguish the last recorded corpus baseline from the partial candidate.
- The requested final broad UI/E2E acceptance and Codex/Vexel recording remain deferred. This order changes documentation only.

# Files allowed to touch
- `docs/product/vexel-goal-and-delivery-plan.md`
- `README.md`
- `work/WO-PRODUCT-GOAL-03.md`
- `artifacts/WO-PRODUCT-GOAL-03/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- ADRs, application/source/tests, scripts, reports, frozen prior work orders/artifacts, website/deployment, and paths outside the allowlist.

# Required changes
1. Describe the current editor, `.vexel` v1 container and editable animation timeline, available native drawing/property authoring, playback/onion preview, native/SVG animation export, and live MCP capabilities at the evidence-supported level.
2. Mark native project timeline semantics as implemented in part; identify remaining migration/compatibility and crash-recovery behavior.
3. State the exact recorded full-render corpus baseline and the partial candidate separately. Preserve the candidate's unaccepted status and note that 19 of 58 corpus files were not rendered because the recorded report capped input size at 32 MiB.
4. Report large-document performance targets and 100 MiB preview status as unmet/unproven according to current reports. Do not turn proposed targets or open/index ratios into full-render claims.
5. Separate implemented user/agent MCP workflows from remaining long-operation progress/cancellation, streamed output, recovery, and real Codex/Claude interoperability evidence.
6. Retain the approved modern UI direction and recorded showcase completion requirement. State the final order: complete feature work, run broad UI/E2E and release acceptance, then capture/accessibility-review/link the Codex/MCP video on the website and README.
7. Reorder immediate next steps to match current status, keeping broad UI/E2E deferred and the partial corpus candidate unaccepted.
8. Update README's status summary so it accurately names native project animation as present but incomplete and links to the reconciled delivery plan. Do not claim the showcase exists.

# Verification
- Validate frozen order and ledger; run `git diff --check`.
- Review plan and README for contradictions and unsupported claims.
- Do not run application, UI, E2E, corpus, or performance tests; do not change application code or publish a video/site.

# Output
Leave changes uncommitted. Report the corrected implemented/partial/planned inventory and the next evidence-based product gaps.
