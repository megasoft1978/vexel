# WO-QA-01 evidence — full-gate baseline and native editor smoke repair

Frozen spec revision 2, spec SHA-256
`2d778c85910dbf1720f76b7937da0839e177d3f87dcce7f2ddf7dae28ba666cf`.

## What was wrong

`./dev editor-smoke` failed with

```
node 'Rectangle #box' has values ['Selected on canvas'], expected ''
```

The W15 accessibility lane added a `Selected on canvas` value to every layer row
whose source span lies inside the canvas selection
(`src/app/viewer_chrome_structure.c`). The smoke scenario predates that value and
asserted an empty row value after re-showing the row and after unlocking it.

Selection deliberately survives hiding and locking: the scenario's own later step
expects the message `Unlock the selected layer to transform it` while the locked
row is still selected, and the pre-fix diagnostic itself shows the selection
survived the toggle. The two stale assertions were replaced with the specific
state each step toggles, and the driver gained a `not-contains=` condition
implemented as the exact complement of `contains=`. No product code,
localization string, or other assertion changed.

## Frozen-command results

All sixteen frozen commands exit 0 on the candidate, with a clean working tree.

| # | frozen command | exit | record |
|---|---|---:|---|
| 1 | `./dev editor-smoke` | 0 | `check-01.json` |
| 2 | `python3 -m unittest tests.e2e.test_dev_final` | 0 | `check-02.json` |
| 3 | `python3 -m unittest tests.test_quick_tools` | 0 | `check-03.json` |
| 4 | `./dev verify` | 0 | `check-04.json` |
| 5 | `./dev asan` | 0 | `check-05.json` |
| 6 | `./dev ubsan` | 0 | `check-06.json` |
| 7 | `./dev e2e` | 0 | `check-07.json` |
| 8 | `./dev soak --minutes 1` | 0 | `check-08.json` |
| 9 | `./dev shape-fuzz --seed 20261010 --output .cache/qa/shape-fuzz-final.json` | 0 | `check-09.json` |
| 10 | `./dev mcp-fuzz` | 0 | `check-10.json` |
| 11 | `./dev viewer-smoke` | 0 | `check-11.json` |
| 12 | `./dev fuzz-xml --duration 30 --seed 20261012` | 0 | `check-12.json` |
| 13 | `build/mac-asan/bin/animation_fuzz 1000000 20261012` | 0 | `check-13.json` |
| 14 | `python3 scripts/ledger.py validate ledger.jsonl` | 0 | `check-14.json` |
| 15 | `python3 scripts/wo_validate.py WO-QA-01 --ready` | 0 | `check-15.json` |
| 16 | `git diff --check` | 0 | `check-16.json` |

Derived from those records: `./dev verify` reports CTest `100% tests passed, 0 tests failed out of 80` and Python
`328` passing; the ledger validated at record 14 contains `476` entries.

`baseline.md` records the pre-fix run of every gate, the exact `./dev final`
membership, and which suites `./dev final` does not cover.

## Not covered by `./dev final`

`./dev fuzz-xml` (which drives `xml_tokenizer_fuzz`) and
`build/mac-asan/bin/animation_fuzz <cases> <seed>` (which has no `./dev`
wrapper) are not part of the standard final gate. Both were run for this order
and both pass; bringing them into the standard gate is a separate work order.
Running `./dev fuzz-xml` currently rewrites the tracked report
`artifacts/WO-M2-03/pr-smoke.json`, which dirties a prior work order's committed
evidence; that is recorded in `baseline.md` and fixed by a separate order.

## Record sequencing

The candidate already contains the `ready`, `run_started` and `run_finished`
ledger events, so the ledger counts recorded above are the candidate's counts.
`scripts/ledger.py` only permits `run_started -> run_finished -> verified ->
accepted -> merged`, so the last three events are appended after the verdict.
The merge commit therefore raises the ledger count by three and writes the
`LEDGER.md` row plus `executor-report.json`, which names the verified candidate
commit — a file cannot name the commit that contains it.
