---
id: WO-CORPUS-11
revision: 1
status: READY
milestone: SVG-FIDELITY
owner: product-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
parallel_safe: false
network: false
spec_sha256: 1d726cb1f5dfdaae633c0569a3b4151453c5eefcdf601e01429c2217a8b45fc6
---
# Goal
Change dash-cache geometric capacity growth from power-of-two doubling to
checked 1.125x growth so T1's 131,073-segment request can fit under the existing
combined 16 MiB limit. Add a focused >65,536-entry regression and reprobe T1.

# Context
- WO-CORPUS-10 measured T1's grouped subrange at 131,072 segments and joins,
  with 131,072 capacity each and 262,145 build steps. The next segment request
  (131,073) rounded to 262,144 capacity; total reserved cache bytes became
  20,109,760, above 16,777,216.
- Growth rounding, not the actual segment request or the 1,000,000-step bound,
  triggered this refusal. Keep all combined-byte and other finite bounds.

# Files allowed to touch
- `work/WO-CORPUS-11.md`
- `src/raster/shape_render_dash_cache.c`
- `tests/svg/shape/stroke_dash_storage_test.c`
- `docs/lanes/decode2.md`
- `artifacts/WO-CORPUS-11/**`
- `.cache/WO-CORPUS-11/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Keep 16 MiB combined dash-cache bytes, 1,000,000 dash build steps, grid,
  source-size, shape/edge/command, boundary, pixel, and raster work limits.
- Preserve source bytes and current dash semantics. No unrelated production or
  UI changes, oracle weakening, dependencies, platform changes, or commit.
- No broad UI/E2E, full corpus, fuzz, sanitizers or broad benchmarks.

# Required implementation
1. Use checked 1.125x capacity growth (minimum +1) for segment and join arrays.
   Continue to preflight total bytes before allocation; return LIMIT on overflow
   or when the existing budget cannot accommodate growth.
2. Add a focused dash-cache test that decodes a simple dashed line producing
   more than 65,536 dash segments, constructs the cache, and verifies the
   combined reserved bytes remain within 16 MiB and steps remain within 1M.
3. Update the lane note to describe bounded incremental capacity growth.
4. Build Release `svgworkbench` and `stroke_dash_storage_test`; run only
   `svg-stroke-dash-storage` and `shape-render` CTests.
5. Probe exactly manifested T1 at 128x128 under `/usr/bin/time -l`. Record
   whether it renders or the next exact failure and resource metrics.

# Verification
- T1 size/hash must match WO-CORPUS-10.
- Validate capacity arithmetic and the new stress regression; source files below
  800 lines; work order/ledger valid; `git diff --check` clean.
- No broad suites or corpus sweep.

# Output
Write `artifacts/WO-CORPUS-11/verification.txt`, update STATE and ledgers, and
keep the full product objective active.
