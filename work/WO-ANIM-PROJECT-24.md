---
id: WO-ANIM-PROJECT-24
revision: 1
status: READY
milestone: M8
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 5865f77438293b97b0529157d683910c8bf4fafbb90a6ee5e0133ee3dc6c6376
parallel_safe: false
network: false
---
# Goal
Make native animation playback mode (`once`, `loop`, `ping_pong`) a persisted, undoable project setting that users and MCP agents can read and control.

# Context
- Follow `OWNER-DIRECTIVES.md`, ADR-0018, ADR-0019, and ADR-0020.
- The native timeline decoder already validates the root `loop` value, and ADR-0020 includes it in native project timing metadata. The decoded view currently discards it.
- The timeline UI already cycles one-shot, loop, and ping-pong playback, but these choices remain transient and reload as one-shot. MCP can read and edit native tracks/cels/keys but has no playback-mode setter.
- Keep existing SVG SMIL playback behavior unchanged. Native mode controls native project playback metadata only.
- Preserve unrelated worktree changes. No commit or broad UI/E2E suite.

# Files allowed to touch
- `src/project/project_timeline.h`, `src/project/project_timeline.c`
- `src/project/project_timeline_edit.h`, `src/project/project_timeline_edit.c`, `src/project/project_timeline_loop_edit.inc`
- `src/project/project_timeline_session.h`, `src/project/project_timeline_session.c`
- `src/app/viewer_timeline.c`, `src/app/viewer_timeline_layout.c`
- `src/mcp/mcp_broker_native_timeline.c`, `src/mcp/mcp_broker_tools.c`, `src/mcp/mcp_tools.c`, `src/mcp/README.md`
- `tests/project/project_timeline_test.c`, `tests/project/project_timeline_edit_test.c`, `tests/project/project_timeline_session_test.c`
- `tests/app/viewer_timeline_native_test.c`
- `tests/mcp/mcp_native_timeline_test.c`, `tests/mcp/mcp_protocol_test.c`
- `work/WO-ANIM-PROJECT-24.md`, `artifacts/WO-ANIM-PROJECT-24/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- `OWNER-DIRECTIVES.md`, `AGENTS.md`, ADRs, other frozen work orders, scripts, build configuration, test oracles, UI/E2E suites, and all paths outside the allowlist.

# Required behavior
1. Decode and expose the exact native playback mode in `project_timeline_view`, with a typed enum for `once`, `loop`, and `ping_pong`.
2. Add source-preserving undoable edit and session APIs to set playback mode; unchanged values are no-ops, invalid values fail atomically, and undo/redo restore mode and source bytes.
3. Initialize and refresh the native viewer mode from project metadata. The existing mode button must persist changes through the session and mark the project dirty; transient playback direction must reset coherently when mode changes. SVG SMIL mode remains unchanged.
4. Add MCP `native_timeline_set_loop` accepting exactly `once`, `loop`, or `ping_pong`, routed through the active app-owned native-v1 session, with normal mutation notification and undo/redo integration. `native_timeline_get` continues to expose the setting in timeline JSON.
5. Cover decode, source preservation, dirty/save, undo/redo, reload/viewer mode mapping, MCP schema/dispatch/invalid values and mutation notification using focused tests. Do not run broad UI/E2E.
6. Update MCP documentation and status/evidence with the implemented behavior and remaining limits.

# Verification
- Freeze and validate this work order SHA before implementation.
- Run strict Debug, ASan/UBSan, and configured UBSan builds for affected focused project, native timeline viewer, and MCP tests plus `svgworkbench`; run only those focused binaries.
- Validate frozen order, ledger, changed paths, C source line caps, and `git diff --check`.

# Output
Leave changes uncommitted. Report persisted playback behavior, MCP usage, focused verification, and remaining animation/showcase/final UI acceptance work.
