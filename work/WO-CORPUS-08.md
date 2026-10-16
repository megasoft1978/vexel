---
id: WO-CORPUS-08
revision: 1
status: READY
milestone: SVG-FIDELITY
owner: product-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
parallel_safe: false
network: false
spec_sha256: d59ca93c79c54ae7333d2bd70f6533fe8c560ed4c3a366872d83c7f55a55bc87
---
# Goal
Using a scratch snapshot of the current worktree after WO-CORPUS-07, identify
the exact next raster-stage limit that rejects manifested generated T1.

# Context
- WO-CORPUS-06 found T1 exceeded dash-cache segments at 65,537 / 65,536.
- WO-CORPUS-07 removed that separate entry count while retaining checked
  16 MiB combined cache bytes, 1,000,000 dash steps, grid limits, and all other
  finite limits. T1 now reports `limit at byte 0` later in raster processing.
- Do not adjust any budgets. Distinguish dash/cache allocation, boundary work,
  scaled raster work preflight, grouped scratch, and other setup failures.

# Files allowed to touch
- `work/WO-CORPUS-08.md`
- `artifacts/WO-CORPUS-08/**`
- `.cache/WO-CORPUS-08/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Shared source tree byte-identical; all copies, instrumentation, build outputs,
  and outputs remain in ignored `.cache/WO-CORPUS-08/`.
- Do not change production code, any resource cap, tests, oracle, corpus, docs,
  dependencies, platform code, or product behavior.
- No UI/E2E, full corpus, fuzz, sanitizer, or broad benchmark run.

# Required implementation
1. Snapshot current dirty and untracked source tree, excluding `.git/`, `build/`,
   `.cache/`, and external corpus; verify selected file hashes before edits.
2. Instrument only the snapshot to report the exact failing raster pipeline stage,
   source line or branch, accumulated work/requested amount/cap where relevant.
3. Build only Release `svgworkbench` in the scratch snapshot.
4. Run exactly one T1 `--render-frame` probe at 128x128 with `/usr/bin/time -l`.
5. Record all observed stages and recommend the next narrowly scoped product fix,
   preserving finite memory/work protections.

# Verification
- Verify T1 manifest identity: 979,796 bytes, SHA-256
  3578c89a2a55b487de739846aeac4fe84ba3bf0d71b2b03387afe3c822cf33b5.
- Verify snapshot hashes for `src/raster/shape_render.c`,
  `src/raster/shape_render_group.c`, `src/raster/shape_bounds.c`,
  `src/raster/shape_render_preflight.c` before instrumentation.
- Ensure every shared file hash remains unchanged after the probe.
- Validate the frozen order and ledger; run `git diff --check`.

# Output
Record `artifacts/WO-CORPUS-08/verification.txt`, update state and ledgers.
No commit and keep the full product goal active.
