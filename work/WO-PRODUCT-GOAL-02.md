---
id: WO-PRODUCT-GOAL-02
revision: 1
status: READY
milestone: M0
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 58cfb1924ce5cf1abf0b5db9955b35f4ba6012522a58b46afb7f7272351a32e1
parallel_safe: false
network: false
---
# Goal
Make the final product goal explicitly include a recorded live Codex/Vexel showcase: Codex creates a character and animates it through MCP while Vexel visibly updates, and the video is used to demonstrate capabilities on the product website and in the repository README.

# Context
- Read `OWNER-DIRECTIVES.md`, ADR-0018, the Vexel goal/delivery plan, README, and the user's product clarification.
- This is a goal clarification, not a claim that recording or website publication already exists.

# Files allowed to touch
- `docs/adr/0018-product-goal-and-completion-gates.md`
- `docs/product/vexel-goal-and-delivery-plan.md`
- `README.md`
- `work/WO-PRODUCT-GOAL-02.md`
- `artifacts/WO-PRODUCT-GOAL-02/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Application source, website/application deployment, other ADRs, existing frozen work orders, tests/oracles, scripts, and paths outside the allowlist.

# Required changes
1. State that completion includes a recorded end-to-end demonstration of Codex driving live character creation and animation through the detailed MCP integration while the user can see Vexel update.
2. Require editing, captioning/accessibility review, and placement/linking of the resulting demonstration on the product website and in README as product evidence.
3. Distinguish this future deliverable from current implemented/verified status.
4. Keep final broad automated UI/E2E acceptance as the last software verification stage; capture/publish the showcase after the demonstrated capabilities work.

# Verification
- Validate the frozen work order, ledger, and `git diff --check`.
- Review all goal language for consistency and for no assertion that the planned videos or website are already complete.
- Do not run UI tests; do not create/publish a recording in this documentation order.

# Output
Leave changes uncommitted. Report the exact added product completion requirement and the remaining implementation/demo work.
