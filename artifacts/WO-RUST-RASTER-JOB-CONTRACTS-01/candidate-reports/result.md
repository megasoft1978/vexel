# Shared raster job contracts implemented

Implemented the entire shared ownership/control scope in the isolated worker:
RasterJob owns the same source/job ChargeLedger, typed scalar scratch/index/row/
fill/dash/grid/output storage, actual-capacity leases and both metadata registries.
Requested bytes are actively reserved before allocation; actual capacity, local
caps and old/new overlap validate before publication. Growth/shrink/append poll
actual copy loops and preserve old contents/capacity/lease on failure. Explicit
release, release_all, finish and ordinary Drop have documented honest lifecycles.
Wrong/stale/double/source/pool/aliased tokens are rejected. Boundary's original
200M marking-work limit is preserved without inventing a byte cap.

Added complete ControlledKernelResources, ControlledSampleInput and typed lower
index/sample/flat seams. Legacy literals and required methods remain unchanged;
unmetered legacy paint has no implicit production conversion. Public fixtures
perform actual same-job row writes, child/output allocation, nested grid access,
in-loop paint cancellation and original 4x4 lattice/rounding.

Final exact commands passed:

- `cargo +stable check -p vexel-scene --locked --offline`
- `cargo +stable test -p vexel-scene --locked --offline`: 111 passed, comprising
  all 86 original tests plus 23 new integration and 2 new arithmetic unit tests.
- `python3 scripts/wo_validate.py WO-RUST-RASTER-JOB-CONTRACTS-01 --ready`

Independent Luna High review found no remaining concrete blocker after fixes.
Worker audit compared 1150 protected file hashes with zero changes. New source/
tests remain below 800 lines; no manifests, peer Rust engines, original tests,
state, ledgers, catalog or Git index were edited.

See APIcoverage.md, preservation.json, review.md and verification.json. Real
try_reserve_exact capacity failure is tested; global OOM/allocator excess fault
injection and RSS claims are excluded. Safe std reports actual excess only after
allocation, which is checked before publication. Coordinator fresh-main audit
and acceptance/peer seed remain required. Complete lower engines/root adapters,
603/157 mixed renderer oracle acceptance, full app/UI/runtime/performance/live
MCP video/final QA/release/demo remain unfinished separate scopes.
