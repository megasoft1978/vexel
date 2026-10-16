---
id: WO-MCP-SNAPSHOT-01
revision: 1
status: READY
milestone: MCP-OBSERVABILITY
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: dfa79a7f942253c4c1d76a9d07729b8c0e1946a934c4ee0090c074911ea25ccc
parallel_safe: true
network: false
---
# Goal
Provide an owned immutable current-native-project snapshot for subsequent async GIF export, including dirty scene/timeline and overlay assets without borrowing app/session/edit/bundle memory.

# Files allowed to touch
- `work/WO-MCP-SNAPSHOT-01.md`
- `src/mcp/mcp_export_snapshot.c`, `src/mcp/mcp_export_snapshot.h`
- `tests/mcp/mcp_export_snapshot_test.c`
- `CMakeLists.txt`, `tests/test_map.json` (snapshot registration only; coordinator merges serially)
- `artifacts/WO-MCP-SNAPSHOT-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/vexel-goal-and-delivery-plan.md` (coordinator only)

# Protected paths
No live broker/app/UI/transport, existing task store, project schema, renderer or parser changes; preserve all dirty source and acceptance tests. No broad tests, packages or network fetches.

# Required implementation
1. Expose an opaque mcp_export_snapshot; create from borrowed const project_bundle_view*, current scene SVG bytes, current timeline JSON bytes on owner thread. Return project_bundle_result with null output on failure. Accessor returns a const decoded project_bundle_view borrowing only snapshot-owned encoded storage; destroy releases it. Callers retain no live pointers inside the result.
2. Validate native v1 input/view and bounded slices. Enforce scene <=16 MiB and total encoded snapshot <=64 MiB before allocating/copying large input. Reject zero scene, invalid pointers/count/limits deterministically. Use existing project_bundle_asset_at to include base+overlay assets and existing bounded container encode/decode. Preserve metadata and all provided scene/timeline/asset bytes exactly. Snapshot module is a container copy, not an SVG/schema acceptance claim.
3. Preserve the existing pthread-mutex-protected task store unchanged. Subsequent async worker will decode/render only the owned snapshot and join before task-store/snapshot destruction; document lifetime in header. Source inputs must be stable throughout create.
4. Tests verify independence after modifying/freeing every source allocation, unsaved current scene/timeline replacing original bundle values, base+overlay assets, unknown metadata, exact byte preservation, invalid args/output initialization, checked size/limit rejection before data access, repeated create/destroy. No implementation-mirroring tests.
5. Register mcp-export-snapshot target/test and source in vexel_mcp plus focused test-map. Keep changed C below 800 lines and use C17/checked arithmetic.

# Parallel execution contract
Implement only isolated .worktrees/delivery-snapshot overlay with independent build outputs. Worker edits only snapshot code/tests and snapshot CMake/test-map entries. Coordinator merges shared registrations/ledgers serially. Transport worker never edits snapshot files.

# Acceptance criteria
mcp_export_snapshot_test and vexel-mcp build in Debug. Focused mcp-export-snapshot passes; snapshot ownership/bounds regressions pass. Frozen metadata, exact paths, baseline/index preservation, C lines, test-map and ledger checks pass. No integrated async export claim until subsequent worker/protocol/UI orders complete.

# Verification
cmake --preset mac-debug
cmake --build build/mac-debug --target mcp_export_snapshot_test vexel-mcp
ctest --test-dir build/mac-debug --output-on-failure -R '^mcp-export-snapshot$'
python3 scripts/wo_validate.py WO-MCP-SNAPSHOT-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check

# Budget
One scoped Luna High run plus one focused repair, no recursive agents or broad audit. No paid purchases or dependencies.

# Output
Uncommitted code and artifacts/WO-MCP-SNAPSHOT-01/verification.txt; coordinator reviews/integrates and records actual evidence.
