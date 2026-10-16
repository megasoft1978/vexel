---
id: WO-MCP-OBS-01
revision: 1
status: READY
milestone: MCP-OBSERVABILITY
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 2c668d8f13f6b18e87d4f9f39b36b0c52b8f8c73fa9da8245bbad05516be89af
parallel_safe: false
network: false
---
# Goal
Show the user which MCP tool just completed and whether it succeeded or failed, using Vexel's existing status strip and bounded operator log.

# Context
- Follow `OWNER-DIRECTIVES.md`, ADR-0018, and the active Vexel product goal.
- The AppKit hook dispatches MCP work on the owner thread. The editor already has a bounded operator log, but the current callback only shows a generic “Agent edited” status after mutations.
- The broker centralizes tool dispatch and receives both the tool name and `mcp_tool_response.is_error`.
- This is completion visibility only. Synchronous long operations still lack in-progress rendering, cancellation, streaming, and progress reporting.

# Files allowed to touch
- `work/WO-MCP-OBS-01.md`
- `src/mcp/mcp_broker.h`, `src/mcp/mcp_broker_tools.c`, `src/mcp/README.md`
- `src/app/mcp_broker_hook.c`
- `tests/mcp/mcp_native_timeline_test.c`
- `artifacts/WO-MCP-OBS-01/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Preserve all existing worktree changes. Do not change protocol schemas, tool semantics, mutation behavior, MCP permission grants, socket security, async execution, UI layout, project formats, or platform support.
- No broad UI/E2E, corpus, fuzz, or performance suite in this slice.

# Required implementation
1. Add an optional broker host callback at the centralized MCP tool dispatch boundary, receiving a bounded tool name and whether the response succeeded (`dispatch && !is_error`).
2. In the macOS app hook, add a concise sanitized `MCP <tool> completed/failed` entry to the existing bounded operator log and update the status strip/redraw state. Do not open the log automatically.
3. Test successful and failed tool calls through the real broker backend, asserting the callback receives the correct name and outcome. Existing native timeline mutation behavior remains intact.
4. Document that the log shows completed outcomes and does not provide progress/cancellation for long-running calls.

# Verification
- Build `svgworkbench` and `mcp_native_timeline_test` in Debug, ASan/UBSan, and UBSan.
- Run only the `mcp-native-timeline` CTest in those configurations. Do not run broad UI/E2E, all tests, corpus, fuzz, or performance suites.
- Validate the frozen order and ledger, parse `tests/test_map.json`, run `git diff --check`, and report source sizes.

# Output
Leave changes uncommitted. Record exact verification in `artifacts/WO-MCP-OBS-01/verification.txt`; update `STATE.md` and both ledgers. Keep the full product goal active.
