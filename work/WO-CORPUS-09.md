---
id: WO-CORPUS-09
revision: 1
status: READY
milestone: SVG-FIDELITY
owner: product-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
parallel_safe: false
network: false
spec_sha256: d3c8bb45d2fc1fbfd4c10ca3dc166c806b1497ddecdce00bbf1b5e79ac5e7ddc
---
# Goal
Identify the exact internal return branch and allocation sizes causing
`raster_fill_scanline_cache_create` to fail for manifested generated T1 in the
current worktree.

# Context
- WO-CORPUS-08 reports T1 reaches `raster_fill_scanline_cache_create` in the
  grouped raster path and returns false with LIMIT before boundary/preflight.
- WO-CORPUS-07 keeps dash entries governed by 16 MiB total cache bytes and
  1,000,000 build steps; no further caps may be changed from this diagnostic.

# Files allowed to touch
- `work/WO-CORPUS-09.md`
- `artifacts/WO-CORPUS-09/**`
- `.cache/WO-CORPUS-09/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Shared source tree byte-identical; snapshot, instrumentation, builds, outputs
  stay under ignored `.cache/WO-CORPUS-09/`.
- No production source changes, cap changes, tests/oracle edits, external corpus
  modifications, dependency/platform changes, or commits.
- No UI/E2E, full corpus, fuzz, sanitizer or broad benchmark.

# Required implementation
1. Snapshot current worktree with untracked files, excluding `.git/`, `build/`,
   `.cache/`, and external corpus; verify selected hashes before instrumentation.
2. Instrument only `src/raster/shape_scanline_fill.c` in the scratch snapshot to
   distinguish validation, arithmetic, and each allocation failure, and report
   document width/height/shape count, eligible maximum edge count, requested
   bytes, and diagnostic.
3. Build only Release `svgworkbench` in the snapshot.
4. Run exactly one T1 128x128 `--render-frame` probe under `/usr/bin/time -l`.
5. Recommend a bounded fix based on the observed branch; change no limits.

# Verification
- T1 must match WO-CORPUS-06/07 identity: 979,796 bytes and SHA-256
  3578c89a2a55b487de739846aeac4fe84ba3bf0d71b2b03387afe3c822cf33b5.
- Verify pre-instrument hashes for `src/raster/shape_scanline_fill.c`,
  `src/raster/shape_render.c`, and `src/raster/shape_render_group.c`.
- Confirm shared files still match those hashes; validate order/ledger and run
  `git diff --check`.

# Output
Record exact findings in `artifacts/WO-CORPUS-09/verification.txt`, update
state and ledgers. Keep full objective active.
