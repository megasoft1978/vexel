---
id: WO-MCP-CAPTURE-01
revision: 1
status: READY
milestone: MCP-OBSERVABILITY
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 26042a5513871de1182c3649f6a8f741b17b9eaa66549770a20c689f974901c0
parallel_safe: true
network: false
---
# Goal
Move snapshot CRC/encoding work into the native GIF worker while retaining an immutable owned capture and existing strict snapshot behavior. Avoid scanning a large original saved scene during owner capture.

# Files allowed to touch
- `work/WO-MCP-CAPTURE-01.md`
- `src/mcp/mcp_export_capture.c`, `src/mcp/mcp_export_capture.h` (new)
- `src/mcp/mcp_export_snapshot.c`, `src/mcp/mcp_export_snapshot.h`
- `src/mcp/mcp_export_job.c`, `src/mcp/mcp_export_job.h`
- `src/project/project_bundle.c`, `src/project/project_bundle.h` (add prevalidated asset accessor only, preserve strict path)
- `tests/mcp/mcp_export_capture_test.c` (new), `tests/mcp/mcp_export_job_test.c` (additive capture execution only)
- `CMakeLists.txt`, `tests/test_map.json` (capture registrations only; coordinator merges)
- `artifacts/WO-MCP-CAPTURE-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/luna-parallel-delivery-plan.md`, `docs/mcp.md` (coordinator only)

# Protected paths
Original snapshot/protocol/project tests and strict validation behavior, all app/broker/socket/grants/publication/UI, schema/parser/raster/dependencies/platforms/old evidence. No broad UI/E2E/fullsuite, staging/commit/network. Preserve accepted worker/snapshot and unrelated dirty source.

# APIs and ownership
Opaque mcp_export_capture captures a previously successfully decoded, stable owner-thread project_bundle_view, current scene/timeline, including metadata and base+overlay assets. Suggested mcp_export_capture_create_prevalidated(..., **out), mcp_export_capture_finalize(capture, **out_snapshot) and destroy. Finalize borrows capture, creates independent owned snapshot, runs worker-side only. Add snapshot-from-input helper to reuse encoding/strict decode without re-decoding original source. Existing snapshot_create retains strict original validation and tests unchanged. Add job_create_from_capture with same integralFPS/duration/config/lifetime bounds; consumes capture only on success. Execute finalizes owned capture before existing render path, checks cancellation/timeout before/after, releases capture/finalized snapshot on all paths. No raw application, broker, live edit/session or original input pointers retained.

# Required behavior
1. Owner capture only preflights structure/limits and copies owned bytes; no full original project_bundle_decode, asset payload CRC, project_bundle_encode or SVG/timeline decode. Source view provenance is explicit: already validated by load/decode and stable for capture; this trusted route is never substituted for untrusted/default parse/access. Caller remains responsible for provenance.
2. Add clearly named project_bundle_asset_at_prevalidated (or equivalent minimal internal helper) checking header/length/count/slice/offset/metadata bounds without scanning asset data CRC. Existing project_bundle_asset_at stays strict and unchanged by default. Do not trust arbitrary header lengths, mismatched view slices/version/counts, overlay NULL or invalid checked-size arithmetic even on prevalidated route. Keep shared decoder default checksum checks intact.
3. Reject scene>16MiB and measured new encoded bundle>64MiB before allocating/copying data; original saved scene may be larger since current edited scene is captured. Owned capture data<=64MiB plus bounded1024asset descriptors/fixed metadata. Worker encoded output<=64MiB; free raw capture after finalize in job to avoid unnecessary retention. No raising limits/dependencies.
4. Focused new tests: original view valid with base+overlay assets, current dirty scene/timeline captured; mutate/free all original inputs then finalize and verify owned bytes; invalid counts/version/header/slices/NULL/overflow/bounds fail without borrowed dangling state. Distinct strict API still detects corrupt CRC while prevalidated API is explicitly provenance-bound. New job route runs through real queue and produces GIF, cancellation-before-finalize drops work, repeat failures/lifetime paths clean. Preserve existing strict snapshot test SHA and existing job assertions.
5. Add non-gating timing probe for owner capture on representative small/16MiB/near64MiB inputs and large saved scene with small edited scene; fixture construction/strict load/finalize excluded from capture measurements. Record actual wall time/copied bytes/original size separately; no invented100ms guarantee or full large-document performance claim. Default focused test may use smaller fixtures; probe explicit flag/command named in evidence. C<800lines.

# Execution and verification
Only isolated .worktrees/mcp-owned-capture, Luna High for immutable ownership/trusted boundary, one implementation plus one narrowed review repair; no recursive agents. Root integrates shared registrations/records.
cmake --preset mac-debug
cmake --build build/mac-debug --target mcp_export_capture_test mcp_export_snapshot_test mcp_export_job_test vexel-mcp
ctest --test-dir build/mac-debug --output-on-failure -R '^mcp-export-(capture|snapshot|job)$'
./build/mac-debug/bin/mcp_export_capture_test --probe
python3 scripts/wo_validate.py WO-MCP-CAPTURE-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exact baseline/scope/conflict audit, original snapshot test SHA, JSON registrations, C limits, independent scoped review and main integrated named checks required. No whole UI/corpus/performance campaign.

# Output
Leave uncommitted; artifacts/WO-MCP-CAPTURE-01/verification.txt captures exact tests/probe measurements. Live owner bridge/publish I/O/UI cancel/generation/client evidence remains required; no production async claim.
