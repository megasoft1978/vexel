# Candidate result

Ported the assigned bounds, boundary mask, scanline AVL index, uniform fill intervals/runs, and render work preflight into safe Rust. The port preserves the 200M boundary cap, 4096-edge scalar fallback, five scanline samples, event tie ordering, fill winding rules, and frozen work budget. Vectors use fallible reservation; variable loops poll `WorkControl`.

Eleven pure algorithm tests pass. They cover exact budget numbers, bounds margins, conservative mask reads, overflow/cancellation, AVL ordering/removal, crossing ordering, half-open intervals, and row capacity. The complete mixed C/Python render, decode, fixture-hash, and performance suites remain unexecuted and are mapped with original hashes in `coverage.json`.

**Acceptance blocker:** accepted `RasterScratch` and scalar operation signatures provide no `ChargeLedger` reservation gate. Retrospective vector charging cannot account for transient buffers or old/new capacity overlap. The coordinator must repair the shared contract; this candidate does not claim capacity-accounting completion or renderer acceptance.

## Shared-contract implementation in this isolated tree

`RasterJob` now owns the original job ledger, typed scalar buffers and their
actual-capacity leases. `JobRasterScratch` covers bounds/boundary/AVL/rows/fill
and local sample storage. `ControlledIndexKernel::build_index` receives that
same owner; a new fixture performs real per-edge row writes alongside child
scratch and output. No post-hoc-only scratch admission is required. Growth,
physical shrink, cancellation and early release are transactional; metadata
capacity/history remain honest. See APIcoverage.md and verification.json.

This repairs the shared seam only. The complete INDEX candidate algorithms are
not present in the registration-only index crate in this tree. Coordinator
fresh-main audit and preserved mixed-renderer/oracle acceptance remain open.
