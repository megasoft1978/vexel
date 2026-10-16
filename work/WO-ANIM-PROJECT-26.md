---
id: WO-ANIM-PROJECT-26
revision: 2
status: READY
milestone: M5
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: c2e8e67419e6171e67bfffe83d285bb87a397c7cd5b4dd25a2261f2fa99d3216
parallel_safe: false
network: false
---
# Goal
Complete a core native animation timeline editing operation by letting users and MCP agents move drawing cels and property keys to a different integer tick as one atomic, undoable edit.

# Context
- Read `OWNER-DIRECTIVES.md`, `docs/CODING_STANDARD.md`, ADR-0018 through ADR-0020, current `STATE.md`, `docs/mcp.md`, and the native timeline editing/session/UI/MCP implementations.
- Native drawing cels and property keys are visible and can be authored through the UI/MCP, but native timeline items cannot currently be retimed in place.
- Keep edits source-preserving, bounded, deterministic, and compatible with the existing undo/redo session. Do not run broad UI/E2E acceptance.

# Files allowed to touch
- `work/WO-ANIM-PROJECT-26.md`
- `src/project/project_timeline_edit.c`, `src/project/project_timeline_edit.h`, `src/project/project_timeline_move_edit.inc`, `src/project/project_timeline_session.c`, `src/project/project_timeline_session.h`
- `src/app/viewer_timeline.c`, `src/app/viewer_timeline.h`, `src/app/viewer_timeline_layout.c`, `src/app/viewer_timeline_native_move.c`, `src/app/viewer_timeline_ui.c`
- `src/mcp/mcp_broker_native_timeline.c`, `src/mcp/mcp_broker_tools.c`, `src/mcp/mcp_tools.c`, `src/mcp/README.md`
- `tests/project/project_timeline_move_test.c`, `tests/app/viewer_timeline_native_test.c`, `tests/mcp/mcp_native_timeline_test.c`
- `CMakeLists.txt`, `tests/test_map.json`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`
- `artifacts/WO-ANIM-PROJECT-26/**`

# Protected paths
- All other source, test, documentation, ADR, report, artifact, and work-order paths; preserve all pre-existing working-tree changes.

# Required implementation
1. Add a single `move_item(trackId, fromTick, toTick)` project/session operation for drawing cels and property keys. It must reject missing tracks/items, non-editable tracks, out-of-duration ticks, occupied destination ticks, and moving a tick-zero baseline. A same-tick move is a successful no-op.
2. Preserve the item's full JSON payload and surrounding unknown/root source bytes; only its tick value and position in its ordered item array may change. Revalidate the candidate and commit exactly one history transaction. Rejected operations must leave bytes and undo/redo history unchanged.
3. In the native timeline UI, let pointer users grab a visible item marker, preview its destination at integer-tick precision while dragging, and commit once on pointer-up. Do not mutate the timeline during preview. Reject collisions/baseline moves visibly and keep the original item intact. Keep the pointer hit area consistent with existing marker hit-radius behavior.
4. Add MCP `native_timeline_move_item` with `documentId`, `trackId`, `fromTick`, and `toTick`; publish the schema/description. Successful changes must use the active session, trigger existing dirty/live-preview notifications, and be undoable/redoable. Invalid requests must not mutate state.
5. Keep changed C source files under 800 lines. Put cohesive additions in focused source files or the existing project edit include fragment. Add focused project, pointer interaction, and MCP tests for both item types, source preservation, collision/bounds/baseline handling, no-op, undo/redo, preview-without-mutation, successful pointer release, and live MCP mutation.

# Verification
- Run the focused project timeline move, native timeline, and MCP native timeline tests in Debug and the repository's ASan/UBSan and UBSan presets, plus strict `svgworkbench` and relevant focused targets builds.
- Validate this frozen order and ledger; run `git diff --check` and confirm all changed C files are below 800 lines.
- Do not run the comprehensive UI/E2E suite, corpus sweep, or performance benchmarks.

# Output
Leave changes uncommitted. Record exact focused command results and any unresolved limits in `artifacts/WO-ANIM-PROJECT-26/verification.txt`, `STATE.md`, and `LEDGER.md`.
