---
id: WO-MCP-TRANSPORT-01
revision: 4
status: READY
milestone: MCP-OBSERVABILITY
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 7030942e3972fbe7bfd0bd5eb35645539d7132b98c38ac0aa9e414d3f31a86e6
parallel_safe: true
network: false
---
# Goal
Finish the public stdio relay with bounded identity-aware request/response/cancellation accounting, preserving responsive bidirectional forwarding and existing socket/security limits.

# Context
Revision 3 is superseded after the Luna High review in artifacts/WO-MCP-TRANSPORT-01/review-r3-luna.txt. Existing 12 focused fixtures pass but do not cover the three findings. Keep all product behavior outside the relay unchanged.

# Files allowed to touch
- `work/WO-MCP-TRANSPORT-01.md`
- `app/vexel_mcp_main.c`
- `app/vexel_mcp_pending.c`, `app/vexel_mcp_pending.h` (only if cohesive split is needed)
- `tests/mcp/test_e2e.py`
- `CMakeLists.txt` (relay helper registration only, coordinator merges serially)
- `docs/mcp.md`, `src/mcp/README.md`
- `artifacts/WO-MCP-TRANSPORT-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/vexel-goal-and-delivery-plan.md` (coordinator only)

# Protected paths
Preserve dirty tree, task store, broker, project format, SVG renderer, socket grants, and prior evidence. Do not weaken any test. No broad UI/E2E, corpus sweep, full suite, live-client or async export claim.

# Required implementation
1. Broker inactivity is 30 seconds only while genuine client requests remain pending. New requests set the deadline on transition zero-to-one; later client activity cannot renew it. Only broker bytes renew inactivity.
2. Clear a matching pending request when a full broker response is framed, once, independently of stdout drain. Maintain separate existing bounded stdout/socket stalls and EOF drain. Never reorder, rewrite or synthesize protocol lines.
3. Fully forwarded valid notifications/cancelled retire only their matching requestId. Unknown, duplicate cancellation and late responses must not retire unrelated work. Track identity for accounting only; client still owns correlation. Decode equivalent JSON string IDs consistently and retain numeric/string distinction. Reject duplicate active IDs or accounting capacity overflow explicitly before forwarding the rejected request; malformed messages retain current broker validation behavior.
4. Bound accounting to 256 simultaneous request slots and 2 MiB aggregate owned ID bytes (including metadata). Do not raise message/line limits. Allocation, invalid-ID, overload and cleanup paths must be bounded and deterministic. Document the additional transport bound.
5. Add focused regressions proving later client traffic cannot delay silence timeout, broker activity renews it, complete responses clear pending independent of output progress, successful completion and cancellation leave idle stdio open beyond 30 seconds, and unknown/duplicate cancellation or late responses leave other requests pending. Prefer deterministic synthetic-time unit coverage when feasible; otherwise parallel real-time fixtures with bounded wall time. Preserve current 12 fixture assertions and original real-broker E2E suffix byte-for-byte.
6. C sources below 800 lines; C17. No backend export work here.

# Parallel execution contract
Implement only in isolated .worktrees/delivery-transport overlay. Worker owns relay source/tests and module README; coordinator owns ledger/product docs and merges CMake serially. The independent snapshot worker never edits these implementation paths. Build outputs stay in each worktree.

# Acceptance criteria
Named Debug targets vexel-mcp and mcp_headless_broker build. Focused public relay fixtures and mcp-stdio-e2e pass on host without skips. Required accounting regressions pass. Exact changed paths and baseline preservation audited. Frozen order, JSON test-map, ledger, C line report and git diff --check pass. New native async capability is not claimed.

# Verification
cmake --preset mac-debug
cmake --build build/mac-debug --target vexel-mcp mcp_headless_broker
ctest --test-dir build/mac-debug --output-on-failure -R '^mcp-stdio-(relay|e2e)$'
python3 scripts/wo_validate.py WO-MCP-TRANSPORT-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check

# Budget
One bounded implementation run and one focused repair; use Luna High, no network fetch/dependencies, no recursive agents. Additional attempts require a narrowed defect brief. Report exact checks, usage if available and remaining risks.

# Output
Uncommitted code and artifacts/WO-MCP-TRANSPORT-01/verification-r4.txt; coordinator records acceptance only after independent review and integrated focused checks.
