---
id: WO-ANIM-PROJECT-08
revision: 1
status: READY
milestone: M8
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: e43a0053035b2b5e5c8f2708bd54c1f0fcaf766a962d31c34d2e430dcd10c92c
parallel_safe: false
network: false
---
# Goal
Expose the active native-v1 project timeline to connected MCP agents through bounded, undoable live operations. Agent-authored track and cel changes must update the same project session the app saves and must be visible to the user through app status/dirty state immediately.

# Context pointers
- Read `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `docs/CODING_STANDARD.md`, `docs/testing.md`, ADR-0018/0019/0020, W07 session API, MCP broker/tool schemas, and focused MCP tests.
- Existing `animation_*` tools edit SVG SMIL on broker documents. Native `.vexel` timeline sessions now persist edits but have no live MCP callers.
- This order bridges the active app-owned session only. General MCP-opened SVGs and opaque/empty native timelines have no native edit session.
- No GPL editor source, no network, no broad MCP UI/E2E sweep, no commit.

# Existing contracts
- macOS Apple Silicon only, C17, no new dependencies, explicit ownership and bounded JSON.
- MCP calls run on the viewer owner thread; do not retain session pointers beyond the call.
- Native-v1 timeline edits use `project_timeline_session`; failed edits must leave timeline bytes/history unchanged.
- Session edits participate in project dirty state and can only be saved as `.vexel`.
- MCP tool schemas are closed and arguments must be validated before mutation.

# Required implementation
1. Extend the MCP host seam so a broker can access the active app-owned native timeline session and notify the app after successful timeline mutations, without transferring session ownership.
2. Add native timeline read tools returning active project identity, version/timing metadata, tracks, target IDs, and ordered cel exposures with bounded output.
3. Add native drawing-track creation and cel set/removal tools using typed JSON fields, existing session validation, checked integer bounds, and structured success/error responses. Add native timeline undo/redo operations isolated from source-document SMIL edits.
4. Reject operations when the document is not the active app-owned `.vexel` with an editable native-v1 session. Never silently route a native tool to SVG SMIL.
5. On successful live mutation, update app status/title dirty state and request a frame refresh so the user can observe agent activity. Failed operations do not notify or change state.
6. Add focused MCP broker/schema tests for listing, edits, visibility notification, undo/redo, invalid references/types, non-project documents, and save-through of the same edited session where feasible. Preserve existing SMIL tool behavior.
7. Document native-v1 tool names, limits, routing, and user-visible behavior.

# Files allowed to touch
- `src/mcp/mcp_broker.h`, `src/mcp/mcp_broker_internal.h`, `src/mcp/mcp_broker_tools.c`, `src/mcp/mcp_tools.c`, `src/mcp/mcp_broker_animation.c`, new `src/mcp/mcp_broker_native_timeline.c`, `src/mcp/README.md`
- `src/app/mcp_broker_hook.c`, `src/app/viewer_save.c` only if required for saved-session integration
- `tests/mcp/mcp_features_test.c`, `tests/mcp/mcp_protocol_test.c`, new focused MCP native timeline test only if separable
- `CMakeLists.txt` MCP lane only, `tests/test_map.json`
- this work order, `artifacts/WO-ANIM-PROJECT-08/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
Native timeline UI/rendering, animation playback/export implementation, project schema/container, document edit internals, SVG/XML/parser/renderer, release files, scripts, presets, and paths outside allowlist. Do not run broad UI/E2E tests.

# Acceptance criteria
- AC-1: strict Debug app and MCP targets build.
- AC-2: focused MCP protocol/features tests and existing project session/save tests pass.
- AC-3: focused MCP and project tests pass under ASan/UBSan.
- AC-4: native tools mutate the active project session, serialize through project save, preserve undo/redo behavior, and reject invalid/non-native use without mutation.
- AC-5: successful edits notify the app; failed edits do not. Existing SVG SMIL tools remain unchanged.
- AC-6: independent read-only review has no blocking/high findings.
- AC-7: frozen order, ledger, changed-path allowlist, source line limits, and `git diff --check` validate.

# Verification commands
- Build `svgworkbench`, MCP feature/protocol targets, and project session/save targets in Debug.
- Run focused MCP feature/protocol and project session/save CTests.
- Run the same focused tests under ASan/UBSan.
- Run independent read-only review limited to allowed paths.
- Record exact commands and outcomes under `artifacts/WO-ANIM-PROJECT-08/`.

# Budget
One implementation pass and one independent verifier; no network, no commit.

# Failure and escalation
Do not expose a native tool that can mutate a different document than the active `.vexel` session. If the bounded tool result cannot reliably describe timeline state, return a structured limit error rather than truncated JSON. Preserve existing source-based SMIL functionality.

# Output
Leave changes uncommitted. Report exact tools and verification, and state that native timeline UI/playback/rendering remain subsequent work.
