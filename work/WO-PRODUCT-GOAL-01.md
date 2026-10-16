---
id: WO-PRODUCT-GOAL-01
revision: 1
status: READY
milestone: PRODUCT
owner: project-owner
executor: gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 3e166ccb5bd496c9d52d70cd5e7abfead84542c74b08e378e37c33b24b3daf9f
parallel_safe: false
network: false
---
# Goal
Record the owner's expanded product goal as a governing ADR and make the README and roadmap accurately describe the intended high-quality macOS vector drawing and 2D animation application, what already exists, measurable completion gates, and staged remaining work. This order is documentation only; it starts no feature implementation.

# Context
- Follow `OWNER-DIRECTIVES.md` and ADRs 0016/0017. This decision refines/supersedes conflicting scope statements in ADR-0016 and the old roadmap; it does not reverse the macOS, C17, custom UI, owned SVG, minimal dependency, or licensing locks.
- The existing repository already has meaningful editor, animation, workspace, and MCP functionality. Describe current capability accurately and use remaining-work evidence rather than treating the app as a blank slate.
- Research reference set (primary/official sources): Blender [workspaces](https://docs.blender.org/manual/en/latest/interface/window_system/workspaces.html) and [Dope Sheet](https://docs.blender.org/manual/en/4.4/editors/dope_sheet/introduction.html); Figma [contextual properties](https://help.figma.com/hc/en-us/articles/360039832014-Design-prototype-and-explore-layer-properties-in-the-right-sidebar); Apple [toolbars HIG](https://developer.apple.com/design/human-interface-guidelines/toolbars); Krita [animation timeline](https://docs.krita.org/en/reference_manual/dockers/animation_timeline.html) and [animation workflow](https://docs.krita.org/en/user_manual/animation.html); Blender [Grease Pencil animation/onion skin](https://docs.blender.org/manual/en/3.4/grease_pencil/animation/introduction.html); Inkscape [FAQ](https://inkscape.org/es/learn/faq/) and [SVG overview](https://inkscape.org/en/develop/about-svg/); [W3C SVG](https://www.w3.org/TR/SVG/); MCP [2026-07-28 spec announcement](https://blog.modelcontextprotocol.io/posts/2026-07-28/); Apple [document packages](https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFBundles/DocumentPackages/DocumentPackages.html) and [Metal profiling](https://developer.apple.com/documentation/xcode/analyzing-the-performance-of-your-metal-app/).
- Preserve the owner's preference to defer the comprehensive automated UI/end-to-end acceptance pass until the planned feature work is complete. Focused checks required by a feature order remain appropriate; this order runs documentation/ledger validation only.

# Files allowed to touch
- `docs/adr/0018-product-goal-and-completion-gates.md`
- `docs/product/vexel-goal-and-delivery-plan.md`
- `README.md`
- `docs/research-r4/05-revised-roadmap.md`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`
- `artifacts/WO-PRODUCT-GOAL-01/**`
- This work order is frozen; do not edit it after READY.

# Protected paths
- `OWNER-DIRECTIVES.md`, `AGENTS.md`, `docs/adr/0016-product-target-and-gates.md`, `docs/adr/0017-ui-direction-blender-aligned.md`
- `src/**`, `app/**`, `platform/**`, `tests/**`, `CMakeLists.txt`, `scripts/**`, `bench/**`, `reports/**`
- `artifacts/WO-CORPUS-04/**` and all existing artifacts; `work/**` except this newly created order
- Every path outside the allowlist

# Required content
1. Define Vexel as a high-quality, macOS-first vector drawing/editing and 2D cartoon animation application, aspiring to be an Inkscape alternative for creation/editing while providing animation workflows. State that interchange fidelity is measured and scoped, with unsupported content preserved and explicitly diagnosed; do not promise unbounded total parity.
2. Set the UI direction: original custom-drawn macOS interface drawing on Blender task workspaces, contextual inspector patterns, and established animation timeline/onion-skin workflows, with Apple HIG, keyboard, accessibility, discoverability, and visual polish. Do not clone a competitor's interface or assets.
3. Define SVG/Inkscape SVG import/export as the interoperability path, preserving Inkscape namespaces and unknown source data. Define a versioned `.vexel` native project as the place for editable scene, animation, assets, and project metadata. State that package structure/migration/compatibility is a separate design decision before implementation.
4. Include vector animation scope: timeline, timed/property keyframes, frame-by-frame drawings/exposures, onion-skin review, deterministic playback and export. Inventory existing animation and MCP features as implemented/partial, and identify feature gaps from repository evidence.
5. Define large-document quality with reproducible corpus/full-document rendering and workload benchmarks. Record prior roadmap performance targets (first visible content under 2 s for a 100 MB representative document, main-thread pauses under 100 ms, pan/zoom p95 under 33 ms) as proposed targets requiring benchmark re-baseline, not achieved results. Track memory/RSS, cancellation, and responsive editing separately from visual fidelity.
6. Define detailed MCP as a live, local-first, observable and user-controlled agent surface: tools/resources and actionable feedback for document, selection, edits, animation, save/export/render; visible live app updates; bounded progress/cancellation, undoable transactions, scoped file access, and clear error/status. Preserve current spec compatibility and stdio-first posture; remote transport requires a separate authenticated threat model. Distinguish existing server capabilities from remaining gaps based on `docs/mcp.md`.
7. Give staged delivery sequence and completion evidence, ending with comprehensive UI/end-to-end user acceptance, accessibility, recovery, performance and release checks only after feature work. Keep prior incomplete corpus candidate marked partial.
8. Update README and roadmap to point to this goal; remove obsolete cross-platform/product claims while keeping historical estimates clearly labeled as stale if retained.

# Acceptance criteria
- AC-1 ADR records the product goal, scope boundaries, compatibility/security, quality gates, open native format design, and relationship to prior decisions.
- AC-2 The delivery plan distinguishes existing features, known gaps, order/dependencies, and evidence-based completion gates; no unsupported implementation status is claimed.
- AC-3 README describes the app and links the goal; roadmap identifies this plan as the active product target and does not schedule prohibited Linux/Windows work.
- AC-4 All cited external references are primary/official and directly support the attributed design guidance.
- AC-5 `python3 scripts/wo_validate.py WO-PRODUCT-GOAL-01 --ready`, `python3 scripts/ledger.py validate ledger.jsonl`, `git diff --check` pass. Do not run automated UI tests or broad test suites in this docs-only order.
- AC-6 Record exact validation output in `artifacts/WO-PRODUCT-GOAL-01/verification.txt`; update `STATE.md` and ledgers, retaining the corpus order's partial status.

# Non-goals
- Implementing UI, document format, rendering, animation, performance, MCP, or release features.
- Rewriting old ADRs, changing current source/tests, executing full UI/end-to-end test suites, or accepting the partial corpus candidate.

# Output
One docs commit with the product decision, detailed delivery plan, concise README update, roadmap alignment, verification evidence, and ledger/state records.
