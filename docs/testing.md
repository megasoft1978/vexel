# Testing: quick loop, precise failures, final gate

Day to day, use the fast tiers. The expensive checks (e2e, soak, fuzz, native window smokes, sanitizer
suites) belong to the final gate only.

| Tier | Command | Typical time | What it runs |
| --- | --- | --- | --- |
| Inner loop | `./dev t` | 0.3–3 s | Incremental Debug build, then only the CTest and Python tests mapped to files changed vs `main` (plus uncommitted). Slow Python (librsvg comparisons, corpus, e2e) is skipped and listed. |
| Targeted | `./dev t -k PATTERN` | 0.2–14 s | CTest `-R PATTERN` and Python `-k PATTERN`. |
| Wider | `./dev t --full` / `--all` | ~15 s / ~25 s | Adds the slow Python modules / runs every CTest in parallel. |
| Pre-merge | `./dev verify` | ~1 min | Debug + Release build, all CTest, all Python. |
| Final gate | `./dev final` | several minutes | verify + ASan + UBSan + shape/MCP fuzz + e2e + soak + real-window editor and viewer smokes. Run once before merging a milestone. |

The mapping from paths to tests lives in `tests/test_map.json`. A changed source file with no mapping is
reported as `unmapped sources` so the map cannot silently rot. Add a prefix when you add a module.

## Precise failures

* `./dev t` prints the first failing test's full output, not a wall of logs.
* `./dev shot file.svg [--size WxH] [--zoom N] [--out f.png]` renders exactly what the viewer draws and prints
  the PNG path, for looking at results immediately.
* `./dev pixdiff A B [--out diff.png] [--tolerance N]` compares PPM/PNG images: differing pixel count, bounding
  box, max/mean channel delta, the first differing pixels with both colours, and a red-highlight image.
  Exit status 0 means identical within tolerance. Use it for golden failures and before/after UI work.

## Rules of thumb

* Write the regression test first and confirm it fails before the fix (`./dev t -k name`).
* Do not run e2e, soak, or the native smokes in the inner loop.
* After touching the decoder or rasterizer, run `./dev t --full` once before committing.
