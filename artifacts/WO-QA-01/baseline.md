# Full-gate baseline (pre-fix) and post-fix confirmation

Recorded for WO-QA-01 revision 2. Baseline was taken on main at `5894cb40acfaf0aac10127d82c30fb8be9d38cb4` before the assertion repair.

## Pre-fix baseline (`./dev final` constituents and the ungated suites)

| # | command | exit | seconds |
|---|---|---:|---:|
| 1 | `./dev verify` | 0 | 58.9 |
| 2 | `./dev asan` | 0 | 65.2 |
| 3 | `./dev ubsan` | 0 | 45.3 |
| 4 | `./dev shape-fuzz --seed 20261010 --output .cache/qa/shape-fuzz-final.json` | 0 | 45.7 |
| 5 | `./dev mcp-fuzz` | 0 | 0.9 |
| 6 | `./dev e2e` | 0 | 5.0 |
| 7 | `./dev soak --minutes 1` | 0 | 61.8 |
| 8 | `./dev viewer-smoke` | 0 | 0.7 |
| 9 | `./dev editor-smoke` | 1 | 23.2 |
| 10 | `./dev fuzz-xml --duration 30 --seed 20261012` | 0 | 30.2 |
| 11 | `build/mac-asan/bin/animation_fuzz 1000000 20261012` | 0 | 599.2 |

`./dev final` itself runs: verify, asan, ubsan, shape-fuzz, mcp-fuzz, e2e, soak, editor-smoke, viewer-smoke.

Not part of `./dev final`: `./dev fuzz-xml` (drives `xml_tokenizer_fuzz`) and `build/mac-asan/bin/animation_fuzz <cases> <seed>` (no `./dev` wrapper). Both were run manually for this baseline.

## Post-fix confirmation (candidate)

| # | frozen command | exit | seconds |
|---|---|---:|---:|
| 1 | `./dev editor-smoke` | 0 | 23.1 |
| 2 | `python3 -m unittest tests.e2e.test_dev_final` | 0 | 0.0 |
| 3 | `python3 -m unittest tests.test_quick_tools` | 0 | 0.1 |
| 4 | `./dev verify` | 0 | 61.9 |
| 5 | `./dev asan` | 0 | 64.8 |
| 6 | `./dev ubsan` | 0 | 46.7 |
| 7 | `./dev e2e` | 0 | 4.9 |
| 8 | `./dev soak --minutes 1` | 0 | 61.3 |
| 9 | `./dev shape-fuzz --seed 20261010 --output .cache/qa/shape-fuzz-final.json` | 0 | 42.9 |
| 10 | `./dev mcp-fuzz` | 0 | 0.8 |
| 11 | `./dev viewer-smoke` | 0 | 0.8 |
| 12 | `./dev fuzz-xml --duration 30 --seed 20261012` | 0 | 30.2 |
| 13 | `build/mac-asan/bin/animation_fuzz 1000000 20261012` | 0 | 596.2 |
| 14 | `python3 scripts/ledger.py validate ledger.jsonl` | 0 | 0.1 |
| 15 | `python3 scripts/wo_validate.py WO-QA-01 --ready` | 0 | 0.0 |
| 16 | `git diff --check` | 0 | 0.0 |

## The one failure and its cause

`./dev editor-smoke` failed: `node 'Rectangle #box' has values ['Selected on canvas'], expected ''`.

The W15 accessibility lane added a `Selected on canvas` state value to every layer row whose source span lies inside the canvas selection (`src/app/viewer_chrome_structure.c`). The smoke scenario was written before that value existed, and asserted an empty row value after re-showing and after unlocking the row.

Selection deliberately survives hiding and locking: the scenario's own later step expects `Unlock the selected layer to transform it` while the locked row is still selected. The two stale assertions were therefore replaced with the specific states they toggle (`not-contains=Hidden`, `not-contains=Locked`), and the driver gained a `not-contains=` condition. No product code, localization string, or other assertion changed.

