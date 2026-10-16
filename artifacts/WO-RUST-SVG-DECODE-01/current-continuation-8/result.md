# WO-RUST-SVG-DECODE-01 — continuation 8

Status: **candidate only; full decoder order remains incomplete.** No decoder source or test files changed.

Verified all 35 seeded `vexel-svg-paint` and `vexel-svg-definitions` files against the read-only main-checkout seed manifest; hashes all match. Fresh decoder package verification passed: 89 tests, 0 failures, 0 ignored. Seeded peer component suites passed 66 tests, 0 failures, 0 ignored. Formatting and the ready-order validator passed. Peer tests prove the components in isolation, not composition with decoder traversal.

The immutable scope catalog assigns the original tile-cache contract and whole original oracle mapping to DECODE. The decoder crate has no `vexel-tile-cache`, paint, or definitions dependency; its manifest and lock are protected. `vexel_scene::DefinitionPlanner` supports definition plans but exposes no group-clip allocation-history operation. Fixture adapters cannot substitute for accepted production peers. These are concrete unresolved decoder gates; no API or support was fabricated. All 19 original suites, 50 assigned function names, and 138 original function/helper bodies remain mapped in `oracle-map.json`, with prior partial and unported statuses intact.

The exact `output_bytes == 256` predicate belongs to mixed renderer execution: the catalog says COMPOSITION owns mixed end-to-end runs, and decoder consumes render output rather than rasterizing it.

Changed paths are confined to `artifacts/WO-RUST-SVG-DECODE-01/current-continuation-8/` (reports and logs); no source paths changed. Exact commands and results are in `verification.json`. Candidate completion, review, integration, and product completion are not claimed.
