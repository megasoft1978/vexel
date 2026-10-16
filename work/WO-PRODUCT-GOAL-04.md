---
id: WO-PRODUCT-GOAL-04
revision: 1
status: READY
milestone: M0
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 7f04aa92ea7fe3279b594e2ed8a50355034fb86844ddb1e40c1ae906617b28c5
parallel_safe: false
network: false
---
# Goal
Make the product showcase deliverable explicit as example content for prospective Vexel users: recorded footage must demonstrate both Codex-driven character creation and animation through live MCP, with the visible Vexel application updating as the work happens, for use on the website and in README.

# Context
- Read `OWNER-DIRECTIVES.md`, ADR-0018, the current product delivery plan and README, and WO-PRODUCT-GOAL-03 evidence.
- ADR-0018 already makes a reviewed live Codex/MCP character-and-animation recording a post-acceptance completion deliverable. This order clarifies the expected example content and distribution; it does not change feature acceptance gates or claim footage exists.
- The requested recording stays after feature completion and comprehensive UI/E2E and release acceptance.

# Files allowed to touch
- `docs/product/vexel-goal-and-delivery-plan.md`
- `README.md`
- `work/WO-PRODUCT-GOAL-04.md`
- `artifacts/WO-PRODUCT-GOAL-04/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Accepted ADRs, application/source/tests, scripts, reports, frozen prior work orders/artifacts, website/deployment, and paths outside the allowlist.

# Required changes
1. Specify a user-facing example video (or coordinated short clips) showing a sample character created and animated through Codex using live MCP, with visible live updates in Vexel.
2. State that the reviewed example footage is intended to demonstrate product capabilities on the website and in README, and that it is not yet recorded or published.
3. Preserve the order: finish feature work, run broad UI/E2E and release acceptance, then capture, review for accessibility and accuracy, and publish/link the example footage.
4. Keep claims limited to capabilities actually verified; do not publish or modify the website/video in this order.

# Verification
- Validate this frozen order and ledger; run `git diff --check`.
- Review the goal and README wording for completeness and for claims that footage exists.
- Do not run application, UI, E2E, corpus, or performance tests; do not change application code or publish a video/site.

# Output
Leave changes uncommitted. Report the clarified showcase goal and confirm the final-stage sequence remains intact.
