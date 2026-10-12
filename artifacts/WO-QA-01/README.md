# WO-QA-01 evidence — full-gate baseline and native editor smoke repair

Frozen spec revision 2, spec SHA-256
`2d778c85910dbf1720f76b7937da0839e177d3f87dcce7f2ddf7dae28ba666cf`.

## What was wrong

`./dev editor-smoke` failed with:

```
TimeoutError: timed out waiting for layout node:
node 'Rectangle #box' has values ['Selected on canvas'], expected ''
```

The W15 accessibility lane added a `Selected on canvas` value to every layer row
whose source span lies inside the canvas selection
(`src/app/viewer_chrome_structure.c`). The smoke scenario predates that value and
asserted an empty row value after re-showing the row (line 164) and after
unlocking it (line 181).

Selection deliberately survives hiding and locking: the scenario's own later step
expects the message `Unlock the selected layer to transform it` while the locked
row is still selected. The two stale assertions were therefore replaced with the
specific state each step toggles, and the driver gained a `not-contains=`
condition implemented symmetrically to `contains=`.

No product code, localization string, or other assertion changed.

## Frozen-command results

All sixteen frozen commands exit 0 on the candidate.

| # | frozen command | exit | record |
|---|---|---:|---|
| 1 | `./dev editor-smoke` | 0 | `check-01.json` |
| 2 | `python3 -m unittest tests.e2e.test_dev_final` | 0 | `check-02.json` |
| 3 | `python3 -m unittest tests.test_quick_tools` | 0 | `check-03.json` |
| 4 | `./dev verify` | 0 | `check-04.json` (CTest 80/80, Python 328/328) |
| 5 | `./dev asan` | 0 | `check-05.json` |
| 6 | `./dev ubsan` | 0 | `check-06.json` |
| 7 | `./dev e2e` | 0 | `check-07.json` |
| 8 | `./dev soak --minutes 1` | 0 | `check-08.json` |
| 9 | `./dev shape-fuzz …` | 0 | `check-09.json` |
| 10 | `./dev mcp-fuzz` | 0 | `check-10.json` |
| 11 | `./dev viewer-smoke` | 0 | `check-11.json` |
| 12 | `./dev fuzz-xml --duration 30 --seed 20261012` | 0 | `check-12.json` |
| 13 | `build/mac-asan/bin/animation_fuzz 1000000 20261012` | 0 | `check-13.json` |
| 14 | `python3 scripts/ledger.py validate ledger.jsonl` | 0 | `check-14.json` |
| 15 | `python3 scripts/wo_validate.py WO-QA-01 --ready` | 0 | `check-15.json` |
| 16 | `git diff --check` | 0 | `check-16.json` |

`baseline.md` records the pre-fix run of every gate, the exact `./dev final`
membership, and which suites `./dev final` does not cover.

## Not covered by `./dev final`

`./dev fuzz-xml` (which drives `xml_tokenizer_fuzz`) and
`build/mac-asan/bin/animation_fuzz <cases> <seed>` (which has no `./dev`
wrapper) are not part of the standard final gate. Both were run for this order
and both pass; bringing them into the standard gate is a separate work order.

## Record sequencing

`STATE.md` and the `ready`, `run_started` and `run_finished` ledger events are
written before verification. `scripts/ledger.py` only permits
`run_started -> run_finished -> verified -> accepted -> merged`, so the
`verified`, `accepted` and `merged` entries cannot precede the verdict. The
post-verification evidence commit appends those three events, adds the
`LEDGER.md` row, and writes `executor-report.json` naming the verified candidate
commit — a file cannot name the commit that contains it.
