---
id: WO-MCP-TRANSPORT-01
revision: 3
status: READY
milestone: MCP-OBSERVABILITY
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 8925bb9e6f9f92d5143e1a6ac67e44980c6e9f0a873ab114ee025449a1b3a03a
parallel_safe: false
network: false
---
# Goal
Make the public stdio MCP relay multiplex client input and broker output so a long-running request can emit progress, receive cancellation, and finish without blocking later client messages or losing coalesced socket lines.

# Context
- Follow `OWNER-DIRECTIVES.md`, `AGENTS.md`, ADR-0018, `docs/mcp.md`, and the product goal.
- `WO-MCP-ASYNC-03` revision 2 is blocked before product edits. Its source audit is `artifacts/WO-MCP-ASYNC-03/scope-block-execution-r2.txt`.
- `app/vexel_mcp_main.c` currently calls `relay_line` synchronously. It waits for one socket line per request, cannot read a later `notifications/cancelled` during that wait, treats a progress notification as the response, and discards bytes after the first newline from one `recv`.
- The app's same-user private Unix socket and the `mcp-stdio-e2e` Python harness are the supported public path. Correct broker-side asynchronous work is not observable or cancelable through clients until this relay is fixed.
- Revision 2 implementation and host evidence are in `artifacts/WO-MCP-TRANSPORT-01/`. A focused review found that a silent broker can leave `poll` unbounded while a request is outstanding and that the E2E tool-inventory expectation was intentionally expanded but called byte-identical by its audit. Revision 3 closes these review findings; r2 remains historical and is superseded before further implementation.

# Existing contracts and invariants
- Keep stdio MCP framing: one JSON-RPC message per line, all protocol messages on stdout, diagnostics only on stderr.
- Keep the private socket path checks, same-user ownership/mode validation, file-grant policy, JSON maximum line size, and existing 30-second socket I/O bounds.
- Preserve request line order when forwarding to the broker. Forward client notifications without waiting for a response.
- Preserve every complete broker line, including notifications and responses, even when reads are partial or combine multiple lines.
- Do not infer response ordering from request ordering. JSON-RPC response IDs provide correlation to the client.
- Preserve the existing 30-second bound while a request is awaiting broker activity. Progress/partial response bytes renew inactivity; a complete response clears one outstanding request without changing forwarding order or matching IDs.
- After stdin EOF, half-close broker input and continue draining broker output until closure or the existing bounded timeout.

# Files allowed to touch
- `work/WO-MCP-TRANSPORT-01.md`
- `app/vexel_mcp_main.c`
- `tests/mcp/test_e2e.py`
- `tests/test_map.json`
- `CMakeLists.txt`
- `docs/mcp.md`, `src/mcp/README.md`
- `docs/product/vexel-goal-and-delivery-plan.md`
- `artifacts/WO-MCP-TRANSPORT-01/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Preserve all shared dirty content, accepted evidence, and existing task-store source/tests.
- Do not change the app-side MCP protocol, broker semantics, task handling, UI, export behavior, socket security policy, project format, SVG behavior, platform support, or broad UI/E2E policy.
- Do not claim async export or legacy cancellation is implemented by the relay alone; the following async-export order must integrate those behaviors.
- Do not run broad UI/E2E, full CTest/Python suites, corpus, fuzz, performance, sanitizer matrix, or release gates.

# Required implementation
1. Replace the blocking one-request/one-response relay with a bounded `poll`-based loop that can read stdin and the broker socket while preserving partial line state in both directions.
2. Forward each complete client JSON-RPC line immediately and in order. Do not wait for its response. Forward notifications such as `notifications/cancelled` while an earlier request is still outstanding.
3. Forward every complete broker line to stdout, including `notifications/progress`, responses, and multiple lines received in one socket read. Do not mistake notifications for responses and do not discard coalesced tails.
4. Keep input and output line buffers bounded by `MCP_JSON_MAX_BYTES`. Preserve current overlong-stdio behavior with one bounded parse error and discard through the next newline. Treat an overlong broker line as a transport failure without emitting a partial JSON message.
5. Apply a 30-second broker-inactivity deadline only while client requests are outstanding; renew it on broker bytes (including progress) and clear one pending request when a complete JSON-RPC response is received. Keep idle stdio with no pending request open indefinitely and do not correlate or reorder response IDs.
6. Add a deterministic silent-broker fixture case proving the pending-request bound. Retain coverage of partial/coalesced writes, progress, cancellation forwarding, response-ID preservation, EOF drain, overflow and backpressure.
7. Update the existing real-broker E2E expected tool inventory to include the eleven already-published native timeline tools. Preserve all other existing E2E behavior assertions byte-for-byte, and make the scope audit report this exact inventory update truthfully.
8. Update `docs/mcp.md`, `src/mcp/README.md`, and the product inventory to describe the pending-request inactivity bound, idle behavior, runtime evidence, and distinction from async task integration.
9. Keep changed C files below 800 lines and preserve all current local security and size limits.

# Acceptance criteria
- AC-1: `cmake --build build/mac-debug --target vexel-mcp mcp_headless_broker` exits 0.
- AC-2: The focused relay fixture tests prove progress and responses are independently forwarded, cancellation is forwarded during an outstanding call, canceled request IDs receive no synthetic response, later responses retain their IDs, partial/coalesced lines are lossless, and a silent broker exits within the 30-second pending-request bound.
- AC-3: Existing `mcp-stdio-e2e` passes (or reports the existing documented sandbox socket-bind skip); new relay tests run without app UI automation.
- AC-4: `python3 scripts/wo_validate.py WO-MCP-TRANSPORT-01 --ready`, test-map JSON parse/mapping, ledger validation, exact changed-path audit, line-count report, and `git diff --check` pass.
- AC-5: No broad UI/E2E, full suite, corpus, performance, fuzz, sanitizer matrix, release gate, live client session, async task, or export claim is made.

# Verification
1. Confirm this frozen revision and audit changed paths against its allowlist.
2. Build only `vexel-mcp` and `mcp_headless_broker` in `build/mac-debug`.
3. Run only the new focused stdio relay tests and `ctest --test-dir build/mac-debug --output-on-failure -R '^mcp-stdio-e2e$'`.
4. Validate the frozen order, test-map mapping/JSON, ledger, C source line limits, changed-path scope, truthful preservation-script assertions, and `git diff --check`.
5. Do not run broad UI/E2E, full CTest/Python suites, corpus, fuzz, performance, sanitizer matrix, or release gates.

# Budget
One executor run and one focused repair run; 60 minutes hard limit; no new dependencies or network fetches.

# Failure and escalation
Stop if the bounded relay needs a new transport, socket-security, or MCP-protocol contract outside the allowlist. Record the exact failure and propose a narrow follow-on order; do not weaken framing, size bounds, or cancellation semantics.

# Output
Leave changes uncommitted. Record exact evidence in `artifacts/WO-MCP-TRANSPORT-01/verification.txt`; update `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/mcp.md`, `src/mcp/README.md`, and the product inventory. Keep the complete product goal active.
