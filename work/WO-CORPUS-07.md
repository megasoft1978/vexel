---
id: WO-CORPUS-07
revision: 1
status: READY
milestone: SVG-FIDELITY
owner: product-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
parallel_safe: false
network: false
spec_sha256: fbb22dda2dceb1ffc0a782afa883a8759346744f9dddae677be079a5c02cb932
---
# Goal
Allow stroke dash-cache growth beyond 65,536 entries when and only when the
existing checked 16 MiB allocation budget and 1,000,000 construction-step
budget permit it. Re-probe only manifested generated Inkscape T1 and preserve
all other finite limits.

# Context
- WO-CORPUS-06 traced generated T1 to `dash_cache_reserve_segments`, which
  requests 65,537 entries against a 65,536 count cap before raster preflight.
  The cache has an existing checked 16 MiB total allocation budget and dash
  construction has a 1,000,000-step cap.
- Generated T2 independently reaches the 65,536-shape decode cap at 65,537
  paths. This order does not change that limit.
- Preserve the exact-source import contract, arithmetic checks, bounded growth,
  and clear `SVG_SHAPE_ERROR_LIMIT` on budget exhaustion.

# Files allowed to touch
- `work/WO-CORPUS-07.md`
- `src/raster/shape_render_dash_cache.c`
- `src/raster/shape_render_internal.h`
- `docs/lanes/decode2.md`
- `artifacts/WO-CORPUS-07/**`
- `.cache/WO-CORPUS-07/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Preserve unrelated worktree changes. Do not change SVG shape/edge/command,
  dash-value, grid cell/reference, boundary-mark, pixel, or raster work limits.
- Do not change generated corpus files or frozen tests/oracles. Do not run the
  full corpus, broad UI/E2E, fuzz, broad benchmarks, sanitizers, or unrelated
  suites. Do not claim corpus-wide coverage or performance improvement.
- No dependencies, platform changes, source format changes, or commits.

# Required implementation
1. Remove the independent 65,536 segment/join entry cutoff. Grow capacity with
   checked size arithmetic; rely on the existing combined-byte preflight,
   16 MiB total cache bound, construction-step limit, and grid bounds.
2. Ensure capacity rounding itself cannot overflow and that no allocation is
   attempted before byte checks pass.
3. Document the actual retained limits in `docs/lanes/decode2.md`.
4. Build Release `svgworkbench` and the focused dash-storage target.
5. Run the focused `svg-stroke-dash-storage` and `shape-render` CTests only.
6. Probe exactly generated T1 at 128x128. Record exit, diagnostic, wall time,
   RSS, and whether it reaches raster preflight or renders. A later limit is a
   useful diagnostic, not a pass on render fidelity.

# Verification
- Verify WO-CORPUS-06 T1 SHA-256 and size before probe.
- Use only the named Release targets and two focused CTests above.
- Confirm size/cap arithmetic, source files under 800 lines, frozen order and
  ledger validity, and `git diff --check`.
- Keep T2 shape count and broad UI/E2E untouched and deferred.

# Output
Record exact evidence in `artifacts/WO-CORPUS-07/verification.txt`, update
`STATE.md` and `LEDGER.md`, and leave changes uncommitted.
