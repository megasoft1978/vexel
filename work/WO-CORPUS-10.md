---
id: WO-CORPUS-10
revision: 1
status: READY
milestone: SVG-FIDELITY
owner: product-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
parallel_safe: false
network: false
spec_sha256: 0b1a125819ed61a95f5c86e737a920793fdb0fee119ccb4469b8e3743b73b58e
---
# Goal
Identify the exact internal dash-cache branch and resource measurement that
rejects the 34,714-shape grouped subrange in generated T1 after WO-CORPUS-07.

# Context
- WO-CORPUS-06 proved the prior count refusal at 65,537 / 65,536 entries.
- WO-CORPUS-07 removed that separate count ceiling while retaining the 16 MiB
  total cache-byte bound, 1,000,000 construction-step bound, grid limits, and
  all other resource limits.
- Scratch diagnostics in WO-CORPUS-08/09 show the grouped subrange fails in
  `raster_shape_dash_cache_create` before scanline-fill creation, boundaries,
  or raster work preflight. No caps may be changed by this order.

# Files allowed to touch
- `work/WO-CORPUS-10.md`
- `artifacts/WO-CORPUS-10/**`
- `.cache/WO-CORPUS-10/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Shared sources remain byte-identical. Snapshot and temporary instrumentation,
  build products and output remain in ignored `.cache/WO-CORPUS-10/`.
- No product-source edits, resource-limit edits, tests/oracles, corpus edits,
  dependencies, platform work or commits.
- No UI/E2E, full corpus, fuzz, sanitizer or broad benchmark.

# Required implementation
1. Snapshot current worktree (including untracked files; exclude `.git/`,
   `build/`, `.cache/`, external corpus) and verify relevant hashes.
2. Instrument only scratch `src/raster/shape_render_dash_cache.c` to report
   which branch rejects: segment/join capacity overflow, combined-byte refusal,
   construction steps, malformed range, allocation failure, or later grid build.
   Record shape index, current/requested entries, capacities, combined bytes and
   step count where available.
3. Build only Release `svgworkbench` in the scratch snapshot.
4. Run exactly one T1 full-document probe at 128x128 under `/usr/bin/time -l`.
5. Recommend a bounded fix or optimization; change no production budget.

# Verification
- Verify T1 identity: 979,796 bytes and SHA-256
  3578c89a2a55b487de739846aeac4fe84ba3bf0d71b2b03387afe3c822cf33b5.
- Verify pre-instrument hashes for `src/raster/shape_render_dash_cache.c`,
  `src/raster/shape_render_stroke_grid.c`, and `src/raster/shape_render.c`.
- Verify shared files still match the recorded pre-instrument hashes; validate
  frozen order and ledger; run `git diff --check`.

# Output
Record `artifacts/WO-CORPUS-10/verification.txt`; update state and ledgers.
Keep the product objective active.
