---
id: WO-MCP-ASYNC-03
revision: 1
status: READY
milestone: MCP-OBSERVABILITY
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 9a38e129242fccf4f232f0eaaeae2911eb8f84635d3fc42589dffd43d9f04253
parallel_safe: false
network: false
---
# Goal
Run native `.vexel` animation export as a bounded asynchronous MCP task with visible progress and cooperative cancellation while the macOS event loop remains responsive.

# Context pointers
- Read `OWNER-DIRECTIVES.md`, `AGENTS.md`, ADR-0018 through ADR-0021, `docs/mcp.md`, and `docs/product/vexel-goal-and-delivery-plan.md`.
- Inspect `src/app/mcp_broker_hook.c`, `src/app/viewer_runtime.c`, `src/app/viewer_editor_status.c`, `src/mcp/mcp_protocol.c`, and `src/mcp/mcp_broker_render.c` before editing.
- Protocol reference: MCP Tasks extension for `2026-07-28`, https://tasks.extensions.modelcontextprotocol.io/specification/draft/tasks (accessed 2026-10-13). Legacy `2025-11-25` progress/cancellation behavior remains supported only through that protocol's notifications.
- Preserve the shared worktree's current changes, including the tested task-store implementation from superseded WO-MCP-ASYNC-02. HEAD is `19234a036ed6a8d40c2ccdaec9b8186806d70eca`; do not stage, reset, clean, or rewrite unrelated changes.

# Existing contracts and invariants
- The app's Unix socket is private to the current user; maintain peer-UID checks, file grants, request bounds, and atomic GIF publication.
- MCP broker document/session state belongs to the app owner thread. Worker code must receive owned immutable snapshots and must not dereference `viewer_app`, `mcp_broker`, `document_edit`, or live timeline/session pointers.
- Only native `.vexel` `export_animation` in GIF format is asynchronous in this order. Other tools retain current behavior. Preserve deterministic integer-tick rendering and current frame/pixel/source limits.
- Bound active tasks to 2, retained terminal tasks to 8, task IDs to unguessable per-process values, task lifetime to 15 minutes, scene snapshot to 16 MiB, and encoded project bundle snapshot to 64 MiB. Reject over-limit work before starting it.
- For `2026-07-28`, advertise `io.modelcontextprotocol/tasks` and return `resultType:"task"` only when the request advertises that extension; implement `tasks/get` and `tasks/cancel`. Cancellation uses `tasks/cancel`, never `notifications/cancelled`, for a task.
- For legacy `2025-11-25`, honor a supplied `progressToken` with bounded `notifications/progress` messages and accept `notifications/cancelled` for the in-flight original request. Do not emit a response after a canceled legacy request.
- Show operation name, bounded progress text, and an accessible cancel action in Vexel. A task's final result/error remains available through `tasks/get` until expiry; cancellation before GIF publication must leave no partial output.

# Files allowed to touch
- `work/WO-MCP-ASYNC-03.md`
- `src/app/mcp_broker_hook.c`, `src/app/mcp_broker_hook.h`, `src/app/viewer_runtime.c`, `src/app/viewer_editor.c`, `src/app/viewer_editor.h`, `src/app/viewer_editor_internal.h`, `src/app/viewer_editor_status.c`, `src/app/viewer_events.c`, `src/app/viewer_internal.h`
- `src/mcp/mcp_protocol.c`, `src/mcp/mcp_protocol.h`, `src/mcp/mcp_broker.c`, `src/mcp/mcp_broker.h`, `src/mcp/mcp_broker_internal.h`, `src/mcp/mcp_broker_render.c`, `src/mcp/mcp_broker_render.h`, `src/mcp/mcp_broker_async_export.c`, `src/mcp/mcp_broker_async_export.h` (new), `src/mcp/mcp_task_store.c`, `src/mcp/mcp_task_store.h`
- `tests/mcp/mcp_protocol_test.c`, `tests/mcp/mcp_async_task_test.c` (new), `tests/mcp/mcp_native_timeline_test.c`, `tests/test_map.json`, `CMakeLists.txt`, `docs/mcp.md`
- `artifacts/WO-MCP-ASYNC-03/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Preserve all unrelated dirty changes and prior accepted evidence.
- Do not change SVG parser/rasterizer semantics, file-grant policy, animation schema, project format, unrelated MCP tool behavior, website, packaging, platform support, frozen oracles, or comprehensive UI/E2E tests.
- Do not claim live Codex/Claude interoperability from unit or socket tests; real-client sessions are a later acceptance gate.

# Required implementation
1. Preserve and complete the tested task-store slice from WO-MCP-ASYNC-02. Keep task state and export execution in separate focused modules. Add explicit `working`, `completed`, `cancelled`, and `failed` states, safe task-ID generation, per-task owner binding, terminal result retention, expiry, and synchronized worker-to-owner progress/cancel state.
2. Refactor only native GIF `export_animation` into prepare/run/finalize phases. Prepare immutable scene/timeline/bundle inputs on the owner thread, run rendering/encoding on a worker, and publish the final GIF atomically only after successful uncanceled completion.
3. Keep the app event loop and socket polling live while the worker runs. Route current-protocol task polling/cancellation and legacy progress/cancellation to the same task state. Reject a second conflicting long operation with a stable busy error.
4. Update the existing status strip/operator log with operation start, progress, completion, failure, and cancellation. Wire the existing cancel button in `viewer_editor.c` to a per-editor cancellation request, then route it to the active MCP task on the app thread. Preserve its accessible semantic label; do not introduce a new UI automation suite.
5. Add focused tests for task capability negotiation, response shapes, ownership, expiry/capacity, progress monotonicity/bounds, cooperative cancel, canceled legacy no-response behavior, stale result handling, atomic output cleanup, and successful deterministic GIF result retrieval.
6. Update `docs/mcp.md` and the product inventory to document both protocol-era behaviors and their limits. Keep changed C sources below 800 lines; specifically, keep `mcp_broker_render.c` below 800 by placing the worker pipeline in the new async-export module.

# Acceptance criteria
- AC-1: `cmake --build build/mac-debug --target svgworkbench mcp_protocol_test mcp_async_task_test mcp_native_timeline_test` exits 0.
- AC-2: `ctest --test-dir build/mac-debug --output-on-failure -R '^(mcp-protocol|mcp-async-task|mcp-native-timeline)$'` passes all three focused tests.
- AC-3: A test-driven export longer than one frame yields a durable task handle before completion; repeated `tasks/get` reports working then terminal state, and completed result contains the same output contract as synchronous GIF export.
- AC-4: Cancel during worker rendering is acknowledged, stops before publication at a frame boundary, produces terminal cancelled state, leaves no output file, and does not block test event-loop/socket polling for more than 100 ms per poll iteration.
- AC-5: Legacy progress is emitted only with a valid request `progressToken`, remains monotonic and bounded, and cancellation stops the original request without sending its eventual response.
- AC-6: The status strip exposes progress and cancel state semantically; canceling through the UI reaches the same task token. Unit-level event dispatch is sufficient; no broad UI/E2E run.
- AC-7: `python3 scripts/wo_validate.py WO-MCP-ASYNC-03 --ready`, test-map JSON parse, ledger validation, allowed-path audit, line-count report, and `git diff --check` pass.

# Verification
1. Confirm the frozen work order and inspect changed-path scope against its allowlist.
2. Build only the three named Debug targets.
3. Run only the three named focused CTests.
4. Validate the frozen order, test-map JSON, ledger, changed C source line limits, and `git diff --check`.
5. Do not run broad UI/E2E, full CTest/Python suites, corpus, fuzz, performance, sanitizer matrix, or release gates.

# Non-goals
- No task support for arbitrary tools, native PNG-sequence export, SVG animation export, or file-open/save in this slice.
- No remote MCP transport, task persistence across app restart, subscriptions/listen, general notification framework, demo recording, or live Codex/Claude client session.
- No change to the comprehensive UI/E2E stage ordering.

# Budget
One executor run and one focused repair run; 120 minutes hard limit; no new dependencies or network fetches. Official protocol reference above is sufficient.

# Failure and escalation
Stop if safe immutable snapshots cannot be constructed within the stated bounds or if current task/result contracts require changes outside the allowlist. Record the exact ownership/lifetime or protocol conflict and propose a follow-up work-order split; do not silently raise limits or move broker state off the owner thread.

# Output
Leave changes uncommitted. Record exact commands and results in `artifacts/WO-MCP-ASYNC-03/verification.txt`; update `STATE.md`, `LEDGER.md`, `ledger.jsonl`, and `docs/mcp.md`. Keep the full product goal active.
