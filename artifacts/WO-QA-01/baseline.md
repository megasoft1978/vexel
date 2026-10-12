# Full-gate baseline and post-fix confirmation

Recorded for WO-QA-01 revision 2. All post-fix records were produced at the candidate
commit with a clean working tree, after the frozen spec reached revision 2.

## Pre-fix baseline (taken on main before the repair)

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

`./dev final` runs: verify, asan, ubsan, shape-fuzz, mcp-fuzz, e2e, soak,
editor-smoke, viewer-smoke.

Not part of `./dev final`: `./dev fuzz-xml` (drives `xml_tokenizer_fuzz`) and
`build/mac-asan/bin/animation_fuzz <cases> <seed>` (no `./dev` wrapper). Both were run
for this order and both pass. `./dev fuzz-xml` currently writes its report to the
tracked path `artifacts/WO-M2-03/pr-smoke.json`, so running it dirties a prior work
order's committed evidence; that side effect is recorded here and is fixed by a
separate order.

## Post-fix confirmation (candidate, clean tree)

| # | frozen command | exit | seconds | derived |
|---|---|---:|---:|---|
| 1 | `./dev editor-smoke` | 0 | 23.6 |  |
| 2 | `python3 -m unittest tests.e2e.test_dev_final` | 0 | 0.1 |  |
| 3 | `python3 -m unittest tests.test_quick_tools` | 0 | 0.2 | python_plain=5 |
| 4 | `./dev verify` | 0 | 68.6 | ctest=100% tests passed, 0 tests failed out of 80, python=('328', '328', '0', '0', '0'), python_plain=328, ledger=476 |
| 5 | `./dev asan` | 0 | 71.9 | ctest=100% tests passed, 0 tests failed out of 80 |
| 6 | `./dev ubsan` | 0 | 53.8 | ctest=100% tests passed, 0 tests failed out of 80 |
| 7 | `./dev e2e` | 0 | 6.4 |  |
| 8 | `./dev soak --minutes 1` | 0 | 62.7 |  |
| 9 | `./dev shape-fuzz --seed 20261010 --output .cache/qa/shape-fuzz-final.json` | 0 | 58.9 |  |
| 10 | `./dev mcp-fuzz` | 0 | 1.0 |  |
| 11 | `./dev viewer-smoke` | 0 | 0.9 |  |
| 12 | `./dev fuzz-xml --duration 30 --seed 20261012` | 0 | 30.2 |  |
| 13 | `build/mac-asan/bin/animation_fuzz 1000000 20261012` | 0 | 627.3 |  |
| 14 | `python3 scripts/ledger.py validate ledger.jsonl` | 0 | 0.1 | ledger=476 |
| 15 | `python3 scripts/wo_validate.py WO-QA-01 --ready` | 0 | 0.0 | wo=('WO-QA-01', '2', '2d778c85910dbf1720f76b7937da0839e177d3f87dcce7f2ddf7dae28ba666cf') |
| 16 | `git diff --check` | 0 | 0.0 |  |

## The one pre-fix failure and its cause

`./dev editor-smoke` failed with

```
node 'Rectangle #box' has values ['Selected on canvas'], expected ''
```

The W15 accessibility lane added a `Selected on canvas` value to every layer row whose
source span lies inside the canvas selection (`src/app/viewer_chrome_structure.c`). The
smoke scenario predates that value and asserted an empty row value after re-showing and
after unlocking a row.

Selection deliberately survives hiding and locking: the scenario's own later step expects
`Unlock the selected layer to transform it` while the locked row is still selected, and
the pre-fix diagnostic itself shows the selection survived the toggle. The two stale
assertions were replaced with the specific state each step toggles, and the driver gained
a `not-contains=` condition implemented as the exact complement of `contains=`. No product
code, localization string, or other assertion changed.

## Record sequencing

The candidate tree already contains the `ready`, `run_started` and `run_finished` ledger
events, so the ledger counts recorded above are the candidate's counts. The
`verified`, `accepted` and `merged` events are appended after the verifier verdict because
`scripts/ledger.py` only permits that order; the merge commit therefore raises the ledger
count by three, and the `LEDGER.md` row and the schema-valid `executor-report.json`
(naming the verified candidate) are written there.

