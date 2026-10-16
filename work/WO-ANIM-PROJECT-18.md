---
id: WO-ANIM-PROJECT-18
revision: 1
status: READY
milestone: M8
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 8c206ed83fc8e2c564050b8381b1e799dc6adca565a7ec7d7babca5cd6b64aeb
parallel_safe: false
network: false
---
# Goal
Allow an authorized live MCP client to add a validated native SVG drawing asset to the active `.vexel` project, then reference it from native drawing tracks/cels and save it without losing asset data.

# Context
- Follow ADR-0018/0019/0020 and owner directives. `.vexel` assets are immutable named SVG documents; timeline cels reference asset names.
- Existing MCP timeline tools can create tracks/cels only when their asset is already in the bundle. There is no project asset edit path.
- Reuse `project_drawing_asset_validate`; never fetch external resources or mutate the asset SVG source.
- Keep all C and `.inc` files under 800 lines. Preserve unrelated pre-existing worktree changes.

# Contracts
- A bounded owned asset overlay extends the decoded bundle view while leaving original encoded bytes borrowed and unchanged. It supports stable lookup, duplicate/limit rejection, and save enumeration through `project_bundle_asset_at`.
- New assets use a safe unique name, `image/svg+xml`, and a complete SVG document validated by the owned drawing-asset validator before insertion. Store exact source bytes.
- Overlay lifetime is tied to `viewer_loaded_document`; view pointers remain valid until document destruction. Asset-only edits count as unsaved, are included in native project saves, and are cleared from dirty state only after successful atomic save. Saving as SVG while unsaved native assets exist must fail clearly instead of discarding them.
- Add one MCP tool `native_timeline_add_drawing_asset(documentId, assetName, svg)` with a bounded source size, user permission scope inherited from current native timeline tools, explicit success/error responses, and visible live status/window dirty state.
- Once inserted, existing add-track/set-cel MCP operations can reference the asset after their normal current-context refresh; frame rendering uses the same live bundle view.
- Focused unit/project/MCP coverage only. Do not run broad UI/E2E or release acceptance; no commit.

# Files allowed to touch
- `CMakeLists.txt`
- `src/project/project_bundle.h`, `src/project/project_bundle.c`, `src/project/project_bundle_asset_overlay.h`, `src/project/project_bundle_asset_overlay.c`
- `src/app/viewer_load.h`, `src/app/viewer_load.c`, `src/app/viewer_save.c`, `src/app/viewer_display.c`, `src/app/viewer.c`, `src/app/mcp_broker_hook.c`
- `src/mcp/mcp_broker.h`, `src/mcp/mcp_broker_native_timeline.c`, `src/mcp/mcp_broker_tools.c`, `src/mcp/mcp_tools.c`, `src/mcp/README.md`
- `tests/project/project_bundle_test.c`, `tests/app/project_save_test.c`, `tests/mcp/mcp_native_timeline_test.c`, `tests/mcp/mcp_protocol_test.c`
- `src/project/README.md`, `work/WO-ANIM-PROJECT-18.md`
- `artifacts/WO-ANIM-PROJECT-18/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Required behavior
1. Implement an owned validated SVG asset overlay over a decoded bundle view, with bounded name/data/count/aggregate bytes, duplicate rejection, exact source retention, deterministic destruction, and mark-saved/dirty accessors.
2. Extend decoded bundle lookup so base assets and overlay assets share one indexed accessor; app project save enumerates both.
3. Initialize/destroy overlay with loaded-document lifetime. Include overlay dirtiness in window dirty indication, unsaved confirmation, external file-change protection, SVG-save rejection, and mark saved only after a successful project publish.
4. Expose `native_timeline_add_drawing_asset` through MCP schemas, broker dispatch, app host, bounded JSON-string extraction, SVG validation, and clear error handling. The live editor reports the action and marks the project dirty.
5. Verify adding/looking up the asset, duplicate/bounds/invalid SVG rejection, overlay lifetime, save/reload retention, asset-only dirty lifecycle, and MCP schema/call behavior. Verify a following drawing track/cel can reference the new name and render from the live bundle.
6. Update project/MCP documentation with the exact operation and size limits.

# Verification
- Freeze this order before implementation; verify the frozen content against the SHA-256 recorded in front matter.
- Strict Debug app/project/MCP builds and focused project bundle, project save, MCP native timeline, and MCP protocol tests.
- Run focused tests in Debug, ASan, and configured UBSan where supported.
- Validate order and ledger, allowed paths, all changed C/`.inc` source line counts, and `git diff --check`.
- Do not run the comprehensive UI/E2E or release suite.

# Output
Leave changes uncommitted. Report the MCP asset-add workflow, save/dirty semantics, renderability, and remaining native UI cel authoring, animation export, final acceptance, and recorded Codex/Vexel website/README showcase.
