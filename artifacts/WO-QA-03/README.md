# WO-QA-03 evidence — frozen and living M3 packet inputs

Frozen spec revision 3, spec SHA-256
`5165c21af5a911f6d13ff37896e767e352274e5fe41807bad04ded92985ac14b`.

## What was wrong

`python3 reports/M3/validate_packet.py --check` failed with
`changed source hash: docs/PERF.md`. The M3 owner packet pinned `docs/PERF.md`
byte-for-byte, but that file is living narrative that every milestone's
performance work appends to; the M4 renderer work updated it, so the
repository's own milestone validator could never pass again. An inventory of
every pinned input found exactly one stale entry — all other frozen measurement
inputs still matched.

## What changed

- `reports/M3/validate_packet.py` now separates **frozen** inputs (enforced
  byte-for-byte, unchanged) from **living** inputs. `docs/PERF.md` moved to the
  living set and is recorded in the packet's `sources` as
  `{"path": "docs/PERF.md", "living": true}` with no hash.
- `validate_schema` and `render_markdown` handle the living entry, printing
  `living input, not byte-frozen` in the source table.
- Regenerating the packet changed exactly two lines: the `docs/PERF.md` entry in
  `reports/M3/evidence.json` and its row in `reports/M3/decision.md`. No M3
  claim, measurement, threshold or frozen hash changed.
- NEW `tests/test_m3_packet.py` (5 tests) covers: the current tree passing the
  frozen check and schema; a mutated frozen input failing; a mutated living input
  not failing; the packet recording the living input without a hash while frozen
  entries keep theirs; and the committed packet byte-matching the generator.

## Frozen-command results

All seven frozen commands exit 0.

| 1 | `python3 reports/M3/validate_packet.py --check` | 0 | `check-01.json` |
| 2 | `python3 -m unittest tests.test_m3_packet` | 0 | `check-02.json` |
| 3 | `./dev verify` | 0 | `check-03.json` |
| 4 | `./dev t --all` | 0 | `check-04.json` |
| 5 | `python3 scripts/ledger.py validate ledger.jsonl` | 0 | `check-05.json` |
| 6 | `python3 scripts/wo_validate.py WO-QA-03 --ready` | 0 | `check-06.json` |
| 7 | `git diff --check` | 0 | `check-07.json` |

`./dev verify` passes at CTest `100% tests passed, 0 tests failed out of 80` and Python `337` passing.
The validator's `--check` reports 41 sources — the same count as before the
change.

## Record sequencing

Revision 2 was closed as `blocked` then `superseded` when AC-3's wording turned
out to be unachievable as written (it included this order's own evidence
directory). Revision 3 corrected it. The candidate contains revision 3's
`ready`, `run_started` and `run_finished`; `scripts/ledger.py` only permits
`run_started -> run_finished -> verified -> accepted -> merged`, so the last three
are appended after the verdict with the `LEDGER.md` row and the schema-valid
`executor-report.json`, which names the verified candidate commit.
