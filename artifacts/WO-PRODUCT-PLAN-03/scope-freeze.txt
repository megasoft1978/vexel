---
id: WO-PRODUCT-PLAN-03
revision: 1
status: READY
milestone: PRODUCT
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 53e59cf46effcb239b28dbc09c97aa38b97f64209be5df0250b8e251a57ee65f
parallel_safe: false
network: false
---
# Goal
Reconcile the active product plan and root MCP guide with the accepted stable-ID implementation and current MCP tool surface.

# Context
- Follow `OWNER-DIRECTIVES.md`, `AGENTS.md`, ADR-0018 through ADR-0021, and accepted evidence in `STATE.md`.
- `WO-ANIM-PROJECT-29` now adds stable IDs to newly inserted graphics roots and returns the IDs through MCP.
- `docs/mcp.md` is stale: it says text insertion is unavailable, says `save_as` only replaces, and omits fragment insertion, animation control/export, native timeline tools, and visible completion/failure status.
- `src/mcp/README.md` and `src/mcp/mcp_tools.c` describe the current broker/tool surface. Preserve the distinction between implemented behavior and missing progress/cancellation or verified live client sessions.
- Preserve all unrelated dirty repository work. Broad UI/E2E remains deferred to the final product gate.

# Files allowed to touch
- `work/WO-PRODUCT-PLAN-03.md`
- `docs/product/vexel-goal-and-delivery-plan.md`
- `docs/mcp.md`
- `artifacts/WO-PRODUCT-PLAN-03/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Do not edit application source, tests, ADRs, previous work orders/evidence, website content, or any path outside the allowlist.
- Do not run application tests, UI/E2E, corpus, performance, sanitizer, fuzz, or release suites.
- Do not shrink or change the owner-approved product goal or final acceptance gates.

# Required changes
1. Mark stable-ID creation for newly inserted graphics as implemented and focused-verified, citing `artifacts/WO-ANIM-PROJECT-29/verification.txt`. Keep full user-authored character animation incomplete until the workflow is demonstrated end to end.
2. Update `docs/mcp.md` to match the current public tools and limits in `src/mcp/README.md` and `src/mcp/mcp_tools.c`: template and fragment/text insertion, returned generated IDs, SMIL and native timeline tools, render/export, atomic new-file Save As, and visible per-tool completion/failure status.
3. Keep progress/cancellation for in-flight operations, error recovery, and real Codex/Claude/test-client verification explicitly open. Preserve local grants and same-user transport requirements.
4. Refresh the fastest delivery sequence and critical-path text to remove completed stable-ID and guide-reconciliation tasks. Keep SVG fidelity, animation workflow, project recovery/compatibility, large-file measurements, UI/accessibility, final broad acceptance, and post-acceptance showcase in dependency order.
5. Add concise verification evidence and update `STATE.md` and both ledgers. No application tests are required.

# Verification
- `python3 scripts/wo_validate.py WO-PRODUCT-PLAN-03 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`
- Manually compare the updated MCP guide with `src/mcp/README.md` and tool names/schemas in `src/mcp/mcp_tools.c`; confirm all remaining product gates and the final demo dependency are still explicit.

# Output
Leave changes uncommitted. Keep the full goal active. Report the corrected documentation and exact documentation-only checks.
