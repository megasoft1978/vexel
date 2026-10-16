---
id: WO-PRODUCT-PLAN-01
revision: 1
status: READY
milestone: PRODUCT
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 2236c6b5f0754d7d0c7de87f35fb91a6da97ce509f919a74ed41f42a8035eb9f
parallel_safe: false
network: false
---
# Goal
Replace the broad immediate-next-steps list with a dependency-aware, effort-estimated execution plan for the owner-approved Vexel product goal. Preserve the accepted product definition and distinguish estimates from verified project results.

# Context
- Follow OWNER-DIRECTIVES.md, ADR-0018 through ADR-0020, and the accepted state in docs/product/vexel-goal-and-delivery-plan.md.
- The user requested a deep review of remaining work and the quickest reliable delivery plan.
- The current worktree contains substantial uncommitted implementation and evidence. Preserve it; do not rewrite or discard existing work.
- Keep broad automated UI/E2E acceptance deferred until feature work is complete. Focused checks required by individual frozen work orders remain allowed.

# Files allowed to touch
- work/WO-PRODUCT-PLAN-01.md
- docs/product/vexel-goal-and-delivery-plan.md
- artifacts/WO-PRODUCT-PLAN-01/**
- STATE.md, LEDGER.md, ledger.jsonl

# Protected paths
- OWNER-DIRECTIVES.md, AGENTS.md, ADRs, application source, tests/oracles/goldens, existing frozen work orders and prior evidence, README, websites/deployment, and every path outside the allowlist.
- Do not run broad UI/E2E, corpus, performance, fuzz, or release suites in this documentation order.

# Required changes
1. Retain the owner-approved product goal: high-quality native macOS vector editor, animation workflows, Inkscape SVG interchange and preservation, large-document responsiveness, observable live MCP, and final Codex character-animation example on the website and in README.
2. Add a dependency-aware delivery sequence covering SVG fidelity, project recovery/migration, user-facing animation, MCP progress/cancellation and client interoperability, large-file measurement, UI/accessibility, final acceptance, and recorded showcase.
3. Give explicitly rough focused-effort ranges, state their assumptions and uncertainty, and distinguish engineering time from review/hardware waits and reruns.
4. Explain that parallel execution is safe only for orders with disjoint paths and explicit parallel approval; serialize shared state, ledger, build artifacts, and final acceptance. Do not start multiple concurrent executors against shared repository state.
5. Identify the immediate next actions from accepted evidence and blocked work orders. Keep the comprehensive UI/E2E pass as the final software gate and the video after acceptance.
6. Record an implementation summary and validation evidence in the allowed artifact path and update STATE.md plus both ledgers. No application tests are required for this docs-only order.

# Verification
- python3 scripts/wo_validate.py WO-PRODUCT-PLAN-01 --ready
- python3 scripts/ledger.py validate ledger.jsonl
- git diff --check
- Review the resulting plan for agreement with ADR-0018, current STATE.md, and open work-order statuses. Do not run application tests or suites.

# Output
Leave changes uncommitted. Report the product goal, critical path, rough schedule, parallelization limits, and exact documentation checks. Keep the full product goal active.
