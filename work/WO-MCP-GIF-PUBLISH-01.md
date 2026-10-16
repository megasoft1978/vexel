---
id: WO-MCP-GIF-PUBLISH-01
revision: 1
status: READY
milestone: MCP-OBSERVABILITY
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 5f6c722ce4453e8442732fc3fb7ee9ce93e960fa450322144c033ada24003a8a
parallel_safe: true
network: false
---
# Goal
Publish synchronous native and SVG GIF exports with atomic visibility using the existing no-replace platform primitive, preserving grants and truthful post-publication results. This prepares the subsequent async owner bridge.

# Files allowed to touch
- `work/WO-MCP-GIF-PUBLISH-01.md`
- `src/mcp/mcp_broker_render.c`
- `tests/mcp/mcp_native_timeline_test.c`, `tests/document/test_source_save.c` (only additive focused regressions; preserve original assertions)
- `artifacts/WO-MCP-GIF-PUBLISH-01/**`
- `docs/mcp.md`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/luna-parallel-delivery-plan.md` (coordinator only)

# Protected paths
No protocol, job, snapshot, task store, app socket/UI, schema, renderer semantics, grant policy, new dependencies, unsupported platforms, broad UI/E2E, staging/commit. Preserve all unrelated dirty source and original evidence. Platform primitive implementation unchanged; use its published/durability result correctly.

# Implementation
1. Replace direct final-file GIF write helper in mcp_broker_render.c with platform_atomic_file_create_new using the exact previously authorized canonical path; retain/recheck grant resolution immediately before publication. Preserve no-replace behavior, output contract and PNG sequence route. Do not claim bytes absent when result.published is true even if error reports cleanup/durability failure.
2. Both native and ordinary SVG GIF routes must distinguish unpublished failure (existing animation_export_failed domain error) from published output with uncertain durability/cleanup. Published output returns its path and clear structured published/durable status or error-with-published/path details; never unlink final output after commit or report simple precommit failure. Successful ordinary outputs remain compatible with existing callers.
3. Remove dead path splitting only when no other caller uses it. Keep focused C modules below800lines.
4. Add broker native GIF duplicate/no-replace regression that proves existing bytes unchanged and no temporary siblings remain, plus no-grant rejection without output. Add platform primitive write-hook regressions in source-save: destination absent throughout partial write, injected write failure leaves no destination/temp, publication duplicate preserves existing bytes. Preserve original tests; use repo-local temporary test dirs. Post-publication classification must be source-reviewed against the primitive's commit-point result fields; no fake filesystem outcome claims.

# Execution
Only isolated .worktrees/atomic-gif-publish. Luna Medium; one scoped implementation plus at most one review repair, compact context, no recursive agents. Coordinator serializes integration/ledgers. Return compact paths/evidence/results.

# Verification
cmake --preset mac-debug
cmake --build build/mac-debug --target mcp_native_timeline_test source_save_test vexel-mcp
ctest --test-dir build/mac-debug --output-on-failure -R '^(mcp-native-timeline|source-save)$'
python3 scripts/wo_validate.py WO-MCP-GIF-PUBLISH-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact overlay/root conflict audit, preserved original assertions, changed C<800, independent scoped review and main integrated checks required. No whole-suite/UI run or live async claim.

# Output
Leave uncommitted, record exact commands/results in artifacts/WO-MCP-GIF-PUBLISH-01/verification.txt. Owner snapshot preparation, bounded socket output, generation/cancellation, and live progress/UI remain follow-on gates.
