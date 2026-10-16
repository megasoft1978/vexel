---
id: WO-MCP-SOCKET-IO-01
revision: 1
status: READY
milestone: MCP-OBSERVABILITY
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 71c0f2cee0289c3cb4d2b8f888393d2e2d9b3b23cd16e861d02b006048c92d43
parallel_safe: true
network: false
---
# Goal
Keep the app owner event loop responsive to slow/flooding local MCP clients through bounded nonblocking socket I/O. Supply the stable connection ownership and progress/terminal queue seams required by the later async task bridge.

# Files allowed to touch
- `work/WO-MCP-SOCKET-IO-01.md`
- `src/app/mcp_broker_io.c`, `src/app/mcp_broker_io.h` (new)
- `src/app/mcp_broker_hook.c`
- `tests/mcp/mcp_broker_io_test.c` (new)
- `CMakeLists.txt`, `tests/test_map.json` (IO source/target/test registrations only; coordinator merges)
- `artifacts/WO-MCP-SOCKET-IO-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/luna-parallel-delivery-plan.md`, `docs/mcp.md` (coordinator only)

# Protected paths
No protocol/task/job/snapshot/parser/raster/schema/grant/platform implementations, unrelated app behavior, old tests, dependencies, unsupported OS, broad UI/E2E, staging/commit. Root's accepted protocol/GIF changes and preexisting dirty source preserved. Retain private socket/peer UID checks in hook.

# Contract
1. New single-owner IO pool supports<=8 clients; attach consumes fd on success only, enables nonblocking/SO_NOSIGPIPE or equivalent macOS-safe send policy, and returns monotonic nonzero uint64 owner identity never reused within process, even pool recreation (reject overflow). Disconnect callback includes owner/reason; callbacks and buffers are borrowed only for call duration. No broker/viewer pointer in module. Hook retains broker/protocol sessions and listener/security; maps protocol by owner and routes line callbacks to current sync handler, enqueuing replies. No async advertisement yet.
2. No blocking poll/sleep/send_all. Per IO poll global read+write bytes<=65536, dispatched lines<=8, bounded syscalls<=128; per-client service<=8192bytes and1line before fairness rotation. Process already-buffered complete lines even when recv returns EAGAIN. Reject input beyond MCP_JSON_MAX_BYTES plus delimiter allowance, retain CRLF support, fail closed oversized/unframeable input. EINTR consumes syscall budget. IO counters exposed to focused harness; callback execution time excluded from transport timing claim and remains later async-preparation gate.
3. Enqueue copies complete JSON payload plus newline, bounded<=MCP_JSON_MAX_BYTES. Per-client queued byte cap<=4MiB plus fixed metadata, <=64 messages, pool aggregate accounted; progress<=64KiB per client and coalesced by bounded task key ONLY if unsent. Never rewrite partially sent frame. Reserve terminal capacity separate from progress. Terminal/control replies are non-coalescible: accepted queue entries retain until fully sent or explicit connection close. On terminal enqueue exhaustion explicitly close/reject connection with reason; never silently drop and claim sent. Progress under pressure may coalesce or report skipped. Enqueue result communicates queued/skipped/closed; future bridge can react. Expose owner-connected/close/enqueue seams.
4. EOF/hard socket errors/protocol callback failure close once and release all owned fd/buffers/queue metadata; no SIGPIPE crash, no recursive disconnect/double-free if callback closes client. Shutdown safely closes clients and callback owner notifications before hook destroys broker. Future task cancellation/join is a separate order.
5. Focused socketpair tests: fragmented and CRLF input, buffered input across message budgets, exact total/perclient budgets and fairness across clients, oversized rejection, peer EOF/closed reader, forced EAGAIN nonblocking output while peer doesn't read, exact complete-frame ordering/newline after partial sends, progress coalescing and partially-sent frame preservation, terminal reservation/cap overflow explicit close, ownership ID differs across slot/pool reuse, shutdown cleanup. Stalled-peer IO poll measured<100ms in harness (no expensive callbacks), report exact count/timing; no assertion about arbitrary tool duration. Repo-local evidence; C<800lines, split cohesive new module further only via revision if required.

# Execution and verification
Only isolated .worktrees/mcp-socket-io. Luna High for bounded framing/ownership; one implementation plus one narrowed review repair. Coordinator owns shared integration/ledgers. No recursive agents/network/new billing.
cmake --preset mac-debug
cmake --build build/mac-debug --target svgworkbench mcp_broker_io_test
ctest --test-dir build/mac-debug --output-on-failure -R '^mcp-broker-io$'
python3 scripts/wo_validate.py WO-MCP-SOCKET-IO-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact baseline/path/registration audits, test-map JSON, C line limits, independent scoped review and main integrated named checks required. No full suite/UI/E2E required here.

# Output
Leave uncommitted; verification.txt with exact checks/observed timing in artifacts/WO-MCP-SOCKET-IO-01. App sync callback still may be expensive; no live async/Tasks claim. Preserve final broad UI/E2E ordering.
