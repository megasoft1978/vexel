---
id: WO-ANIM-PROJECT-16
revision: 1
status: READY
milestone: M8
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: aa29e674fa11f8e94d0378d6cec1bb77ba2ca758b25e67e14ab5d26c600de891
parallel_safe: false
network: false
---
# Goal
Complete native property-track and key authoring through live MCP so Codex and authorized clients can animate project nodes in Vexel.

# Context
- Follow ADR-0020 and the source-preserving timeline/session contracts.
- W15 implementation added edit/session APIs and MCP tool handlers, but focused MCP execution showed the central broker dispatch table does not yet route the new tool names.
- Keep every C and `.inc` source under 800 lines.

# Contracts
- Preserve source bytes except targeted timeline insertions; validate candidates before atomic history commits.
- Key values are bounded raw JSON number, boolean, or flat numeric array and validated by native-v1 schema decode.
- MCP mutations remain undoable/redoable, notify the viewer, and leave the scene SVG unchanged.
- Do not run broad UI/E2E tests; no commit.

# Files allowed to touch
- `src/project/project_timeline_edit.h`, `src/project/project_timeline_edit.c`, `src/project/project_timeline_property_edit.inc`
- `src/project/project_timeline_session.h`, `src/project/project_timeline_session.c`
- `src/mcp/mcp_broker_native_timeline.c`, `src/mcp/mcp_broker_tools.c`, `src/mcp/mcp_tools.c`, `src/mcp/README.md`
- `tests/project/project_timeline_edit_test.c`, `tests/project/project_timeline_session_test.c`
- `tests/mcp/mcp_native_timeline_test.c`, `tests/mcp/mcp_protocol_test.c`
- `src/project/README.md`, `work/WO-ANIM-PROJECT-16.md`
- `artifacts/WO-ANIM-PROJECT-16/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Required behavior
1. Add a property track with a validated tick-zero initial key and metadata; atomically upsert ordered keys.
2. Route both new tools through central broker dispatch and validate number, boolean, or bounded flat numeric-array values.
3. Return clear failures for invalid metadata, target, value, tick, duplicates, and existing timeline limits.
4. Verify source preservation, ordering/upsert, undo/redo, live refresh notification, and protocol schemas.
5. Document native property key authoring and rendered interpolation.

# Verification
- Strict Debug build for app, project timeline edit/session tests, and MCP native timeline/protocol tests.
- Run the focused tests in Debug, ASan, and configured UBSan.
- Validate this order, ledger, changed-path allowlist, all source line counts, and `git diff --check`.
- Keep broad UI/E2E acceptance deferred.

# Output
Leave changes uncommitted. Report MCP authoring behavior and remaining native UI authoring, export, showcase recording, and final broad acceptance work.
