---
id: WO-MCP-TASK-PROTOCOL-01
revision: 1
status: READY
milestone: MCP-OBSERVABILITY
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: ae319b6d1e1d22771e750ac3870e6137f63fea90ff6c6768bca07f218ab8fdcd
parallel_safe: true
network: false
---
# Goal
Add an opt-in async MCP protocol adapter for current Tasks and legacy deferred tools/cancellation, preserving existing synchronous protocol behavior when no async backend is installed.

# Files allowed to touch
- `work/WO-MCP-TASK-PROTOCOL-01.md`
- `src/mcp/mcp_protocol.c`, `src/mcp/mcp_protocol.h`
- `src/mcp/mcp_protocol_async.c`, `src/mcp/mcp_protocol_async.h`
- `tests/mcp/mcp_protocol_async_test.c`
- `CMakeLists.txt`, `tests/test_map.json` (async protocol registrations only, coordinator merges serially)
- `artifacts/WO-MCP-TASK-PROTOCOL-01/**`
- `docs/mcp.md`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/luna-parallel-delivery-plan.md`, `docs/product/vexel-goal-and-delivery-plan.md` (coordinator only)

# Protected paths
Do not change existing mcp_protocol_backend or mcp_tool_response struct layouts/initializers, old test assertions, broker/app/worker/snapshot/task store/parser/project semantics, security/grants, dependencies/platforms. No live client or broad UI/E2E/full suite runs. Preserve dirty source, no commit/staging.

# Frozen API contract
Introduce mcp_protocol_async_backend in new header with void*context, bool tasks_available and callbacks call_tool, get_task, cancel_task, cancel_request. Add mcp_protocol_handle_async(session,line,length,backend,async_backend,response); old mcp_protocol_handle wraps it with NULL async. New header defines dispositions UNHANDLED/RESULT/ERROR/DEFERRED/FAILED. Callback call_tool gets (context, const mcp_json_doc *request, size_t arguments, const char *tool_name, bool modern, bool request_supports_tasks, mcp_json_buffer *payload); get_task/cancel_task get(context, const mcp_json_doc *request, const char *task_id, mcp_json_buffer *payload) and same disposition. cancel_request gets(context,const mcp_json_doc *request,size_t request_id_token), returns void. Every callback is owner-thread and may not retain borrowed input slices. RESULT payload is a complete JSON result object, ERROR payload a JSON-RPC error object; both must be validated and bounded before wrapping with original id. UNHANDLED tool falls back to current sync path; DEFERRED emits no response and is legacy-only; FAILED yields bounded protocol error.

# Required implementation
1. Keep envelope/version/init/rate/tool validation consistent with current handler. Sync wrapper/no async backend preserves existing responses/capabilities and all old behavior tests. Only advertise io.modelcontextprotocol/tasks on server/discover when tasks_available and required callbacks are installed. Never advertise unfinished app support with NULL async.
2. For2026-07-28, inspect params._meta.io.modelcontextprotocol/clientCapabilities.extensions.io.modelcontextprotocol/tasks per request, requiring an object. Never retain support from earlier calls. Only return resultType:task with valid Task fields if that call advertises support. If backend attempts task response without support return-32021 with requiredCapabilities rather than leaking a task handle. Current tool fallback remains possible without Tasks. Modern DEFERRED is invalid.
3. Task creation RESULT is flat resultType:task plus taskId/status/createdAt/lastUpdatedAt/ttlMs/pollIntervalMs; callback must create durable store entry before returning. tasks/get requires current per-request capability and emits resultType:complete DetailedTask with result for completed or JSON-RPC error for failed. Tool isError results are completed, not failed. tasks/cancel acknowledges with resultType:complete (no forced terminal state), and uses only dedicated callback. Unknown/foreign/expired IDs map to stable error without leaking state. Validate bounded nonempty UTF-8 task IDs and no embedded NUL, payload object/discriminator/status fields and terminal result shape. Unsupported tasks/update/input_required/subscriptions stay explicit method-not-found; no input requests authored.
4. Legacy2025-11-25 tools may return DEFERRED with empty immediate response. Valid notifications/cancelled goes to cancel_request only in initialized legacy session, never initialize cancellation, and produces no synthetic response. Malformed/unknown notifications are ignored. The owner bridge will suppress eventual cancelled legacy response; tests must prove callback routing/response-free disposition, not claim live cancellation yet.
5. Provide bounded legacy progress JSON helper accepting raw string/integer token JSON, progress_permille, prior-value pointer, message, output buffer. Reject invalid/fractional/over-limit tokens and non-increasing/out-of-range progress without altering prior state. Emit unchanged token, total1000, escaped bounded message; never emit progress for current Tasks. Header documents caller binds token to active original request and stops after terminal state. Use <=1024-byte raw token and <=127-byte message caps; reject embedded-NUL task IDs.
6. Focused real dispatcher tests with fake/store-backed async callbacks: optional capability advertisement, strict per-request negotiation and reset, durable task create->working->completed/isError/failed/cancelled result envelopes, unknown/foreign/expired IDs, cancel ack then cooperative transition, sync fallback, legacy deferred/cancel/no response, invalid/oversized backend payload, scalar request IDs, invalid tokens/progress monotonicity, unchanged existing synchronous protocol suite. No app integration claim. C <800; split helper module cohesively.

# Sources and semantics
Primary protocol pages fetched2026-10-14: https://tasks.extensions.modelcontextprotocol.io/specification/draft/tasks ; https://modelcontextprotocol.io/specification/2025-11-25/basic/utilities/progress ; https://modelcontextprotocol.io/specification/2025-11-25/basic/utilities/cancellation . Current Tasks uses resultType:complete for get/cancel; cancellation ack is eventually consistent, not terminal cancellation. Document captured rules in evidence; no external editor source.

# Parallel execution
Only isolated .worktrees/async-task-protocol. Worker owns listed protocol code/test and local registration deltas. GIF worker is disjoint. Coordinator merges shared files/records serially; separate build outputs. No recursive delegation.

# Acceptance / verification
cmake --preset mac-debug
cmake --build build/mac-debug --target mcp_protocol_async_test mcp_protocol_test vexel-mcp
ctest --test-dir build/mac-debug --output-on-failure -R '^mcp-(protocol|protocol-async)$'
python3 scripts/wo_validate.py WO-MCP-TASK-PROTOCOL-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact baseline/path/registration/JSON/line audits, separate code review and integrated focused checks pass. Existing protocol backend and broker remain compatible. No Tasks capability advertised by production until owner bridge installs working backend.

# Budget and output
Luna High for callback/protocol ownership, one implementation plus one narrowed repair, scoped inputs/no full history/recursive agents/paid purchases. Record artifacts/WO-MCP-TASK-PROTOCOL-01/verification.txt, leave uncommitted and report exact checks and missing bridge/UI evidence.
