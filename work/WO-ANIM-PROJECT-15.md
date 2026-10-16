---
id: WO-ANIM-PROJECT-15
revision: 1
status: READY
milestone: M8
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 0cc639a3556e867f1f8adef1c90fa174c533759b75bf398f48fd2180ef26c84c
parallel_safe: false
network: false
---
# Goal
Let Codex and other authorized MCP clients create native property animation tracks and author their key values in the active Vexel project.

# Context
- Read owner directives, ADR-0020, timeline decoder/edit/session APIs, native timeline MCP tools, and focused project/MCP tests.
- Native frame rendering now evaluates validated property tracks, but the native timeline edit model and MCP tools only author drawing cels.
- `project_timeline_edit.c` is already near the 800-line cap. Keep it as a thin includer of the focused property-edit implementation fragment so all file and source-size constraints remain satisfied.

# Contracts
- Preserve source bytes except targeted timeline insertions; validate each candidate before atomic history commit.
- Keep operation bounded by existing timeline track/key/payload limits. Key values are raw JSON values accepted only by full native-v1 timeline validation.
- Property track metadata must use ADR-0020 supported property/type/interpolation combinations; keys stay sorted, unique, finite, and in duration.
- New MCP operations are undoable/redoable, trigger live timeline refresh, and never mutate the open SVG source.
- Keep every C and `.inc` source under 800 lines. Do not run broad UI/E2E tests; no commit.

# Files allowed to touch
- `src/project/project_timeline_edit.h`, `src/project/project_timeline_edit.c`, `src/project/project_timeline_property_edit.inc`
- `src/project/project_timeline_session.h`, `src/project/project_timeline_session.c`
- `src/mcp/mcp_broker_native_timeline.c`, `src/mcp/mcp_tools.c`, `src/mcp/README.md`
- `tests/project/project_timeline_edit_test.c`, `tests/project/project_timeline_session_test.c`
- `tests/mcp/mcp_native_timeline_test.c`, `tests/mcp/mcp_protocol_test.c`
- `src/project/README.md`, `CMakeLists.txt`, `work/WO-ANIM-PROJECT-15.md`
- `artifacts/WO-ANIM-PROJECT-15/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Required behavior
1. Add an atomic undoable API to add a property track with a valid initial tick-zero key, target ID, property, value type, interpolation, visibility, and optional name.
2. Add an API to upsert a property's key at an integer tick while preserving all other source spans and sorted key order.
3. Expose MCP tools for adding a property track and setting a key. Accept number, boolean, or bounded numeric-array JSON values; candidate decode remains the schema authority.
4. Return clear errors for invalid property/type/interpolation/value combinations, duplicate track IDs, invalid targets, ticks outside duration, and track/key/payload caps.
5. Exercise add, upsert, source preservation, undo/redo, MCP live notification, and schema-rejection paths with focused tests.
6. Document that property tracks can now be authored live through MCP and are evaluated by frame rendering.

# Verification
- Build strict Debug app, focused project edit/session, and native timeline MCP/protocol targets.
- Run matching focused tests in Debug, ASan, and configured UBSan.
- Validate work order, ledger, changed-path allowlist, source line counts, and `git diff --check`.
- Do not run broad UI/E2E suites.

# Output
Leave changes uncommitted. Report live native property-track authoring and remaining app UI authoring, export, showcase recording, and final broad acceptance work.
