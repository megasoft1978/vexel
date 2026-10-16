---
id: WO-MCP-GIF-WORKER-01
revision: 1
status: READY
milestone: MCP-OBSERVABILITY
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: c97344a7ca2f9c2ff803a34d97bd576bf70c20adf0450b1b094b0a83d1400774
parallel_safe: true
network: false
---
# Goal
Implement the native GIF rendering/encoding worker from the accepted immutable project snapshot, with observable progress and cooperative cancellation. Preserve the full async app integration requirement as a following order.

# Files allowed to touch
- `work/WO-MCP-GIF-WORKER-01.md`
- `src/mcp/mcp_export_job.c`, `src/mcp/mcp_export_job.h`
- `tests/mcp/mcp_export_job_test.c`
- `CMakeLists.txt`, `tests/test_map.json` (only job registrations, coordinator merges serially)
- `artifacts/WO-MCP-GIF-WORKER-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/luna-parallel-delivery-plan.md`, `docs/product/vexel-goal-and-delivery-plan.md` (coordinator only)

# Protected paths
No app/broker/protocol, parser/rasterizer/project schema, existing snapshot/task store, file grant/publication behavior, old tests, dependencies or platform changes. Preserve dirty source and original evidence. No staging/commit or broad UI/E2E/full suite.

# Contract and required implementation
1. Opaque mcp_export_job create consumes mcp_export_snapshot only on success. Inputs fps and duration_seconds must be finite/positive, fps <=30, ceil(duration*fps) <=120. Reject invalid args before work. Constructor owns only snapshot/config; worker does all scene/timeline decode, validation, rendering and encoding. Never retain/dereference viewer_app, broker, document_edit or a live timeline/session. Use current stateless svg_app_media_providers and owned timeline context built from snapshot.
2. Expose READY/RUNNING/ENCODED/CANCELLED/FAILED state and bounded error enum. Thread-safe status includes monotonic progress_permille, dimensions and frame count. mcp_export_job_execute(void *job, const platform_task_cancel_token*, int64_t*) is a platform_task_callback; run exactly once. mcp_export_job_request_cancel, get_status, take_encoded and destroy must have explicit header lifetime/ownership contracts. take_encoded moves platform_animation_bytes only after ENCODED and no cancellation. Caller joins before destroy; constructor failure leaves snapshot caller-owned. Do not create another thread framework or task store.
3. Render deterministic integer ticks floor(frame_index*ticks_per_second/fps) using owned native timeline/scene/assets. Preserve native duration checks, 512px max output axis,120frames and16,000,000 cumulative frame pixels. Match synchronous native export scaling and provider behavior. Scene16MiB/bundle64MiB caps from snapshot remain. Check cancellation during validation and before/after each frame and GIF encoding; enforce15minute lifetime with monotonic time. Reuse platform_animation_encode_gif. ENCODED means bytes ready, not a published/completed task.
4. No file/network writes. Owner integration will recheck cancellation, file grants and document generation immediately before atomic publication. Cancellation before take drops encoded bytes. Failed/cancelled/partial paths free all tiles, context/session and transient bytes; destroy is safe after joined completion. Progress remains <=1000; terminal state publication and cancellation races synchronized.
5. Focused tests: real multi-frame native color/transform or cel animation produces valid GIF and matches independently sampled native frames/timing; accepted snapshot survives original edits/free; invalid params/work/native duration; real platform_task_queue dispatch and safe owner polling while worker runs; deterministic mid-work cancellation using a narrow test barrier/hook or queue coordination, cancel-before-run and after-encode-before-take; failed decode cleanup, repeat lifecycle and no output side effects. Test-only barriers must not be exposed as product behavior. Keep C <800 lines.

# Parallel execution
Only isolated .worktrees/async-gif-worker. Worker owns listed job code/tests and local registration deltas. Protocol worker does not touch these paths. Coordinator serializes shared registrations and records; separate builds. Return compact results, no recursive agents.

# Acceptance / verification
cmake --preset mac-debug
cmake --build build/mac-debug --target mcp_export_job_test vexel-mcp
ctest --test-dir build/mac-debug --output-on-failure -R '^mcp-export-job$'
python3 scripts/wo_validate.py WO-MCP-GIF-WORKER-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact worker/root baseline audit, JSON test-map, changed C <800, independent review and main integrated focused check must pass. No claim of live async app export until broker/UI integration is accepted.

# Budget and output
Luna High, one implementation plus one narrowed repair; no network/dependencies/paid usage purchases. Record artifacts/WO-MCP-GIF-WORKER-01/verification.txt and return exact changed paths/results, API and risks; leave uncommitted.
