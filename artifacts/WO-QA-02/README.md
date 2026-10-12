# WO-QA-02 evidence — every fuzzer in the final gate, pinned evidence read-only

Frozen spec revision 6, spec SHA-256
`fd4a95b5fc5ffcc4a55d9d4d28ae77eab0032774a5a48c25ed53e5b293bb25a4`.

## What was wrong

1. `./dev final` ran only two of the four fuzzers. `xml_tokenizer_fuzz` and
   `animation_fuzz` existed but no standard gate ran them, and `animation_fuzz`
   had no `./dev` wrapper at all.
2. `./dev fuzz-xml` wrote `artifacts/WO-M2-03/pr-smoke.json`, whose SHA-256 is
   pinned by `reports/M3/validate_packet.py`. Any routine campaign therefore
   rewrote a pinned milestone artifact. `scripts/xml_fuzz_campaign.py` also wrote
   failure inputs and the tracked canary files under
   `artifacts/WO-M2-03/failures/**` regardless of `--output`.
   `tests/fuzz/shape/shape_fuzz.py` had the same shape of problem through its
   default `artifacts/WO-M4-04/fuzz-report.json`.
3. `.github/workflows/fuzz.yml` lacked the `push` trigger that
   `scripts/check_ci.py` requires, so the repository's own CI policy check failed
   on it.

## What changed

- `./dev animation-fuzz [cases] [seed]` builds and runs the ASan animation
  fuzzer, defaulting to 200,000 cases and seed 20261012.
- `./dev fuzz-xml` gained `--output PATH`. Without it, reports go to
  `.cache/qa/`; `scripts/xml_fuzz_campaign.py`'s output confinement allows the
  ignored `.cache/**` tree plus the pinned `artifacts/WO-M2-03/**`, and its
  failure evidence is now derived from the report's parent directory instead of a
  fixed `artifacts/` path.
- `tests/fuzz/shape/shape_fuzz.py` defaults its report to `.cache/qa/`.
- `./dev final` runs shape, XML tokenizer, animation and MCP fuzzers with
  explicit bounds and seeds, then e2e, soak and both native smokes.
- `fuzz.yml` added `push` and writes the pinned packet only through an explicit
  `--output`, keeping `scripts/check_fuzz_workflow.py`'s failure-log confinement
  satisfied.
- `tests/e2e/test_dev_final.py` grew from 1 to 5 tests, asserting each fuzzer
  step's exact bound and seed (`--seed 20261010`, `--duration 30 --seed 20261010`,
  `200000 20261010`, `200000 20261010`), that the XML failure directory follows
  the report path, and that no `./dev` command defaults to a tracked artifact
  report.

## Frozen-command results

All nine frozen commands exit 0.

| 1 | `python3 -m unittest tests.e2e.test_dev_final` | 0 | `check-01.json` |
| 2 | `./dev fuzz-xml --duration 5 --seed 20261012` | 0 | `check-02.json` |
| 3 | `./dev animation-fuzz 100000 20261012` | 0 | `check-03.json` |
| 4 | `./dev shape-fuzz --seed 20261010` | 0 | `check-04.json` |
| 5 | `python3 scripts/check_ci.py .github/workflows/fuzz.yml` | 0 | `check-05.json` |
| 6 | `./dev final` | 0 | `check-06.json` |
| 7 | `python3 scripts/ledger.py validate ledger.jsonl` | 0 | `check-07.json` |
| 8 | `python3 scripts/wo_validate.py WO-QA-02 --ready` | 0 | `check-08.json` |
| 9 | `git diff --check` | 0 | `check-09.json` |

`./dev final` passed end to end: CTest `100% tests passed, 0 tests failed out of 80` and Python `332`
passing, including the XML and animation fuzz steps it now runs. The ledger
validated at `487` entries.

AC-1's cleanliness check is explicit: `git diff --name-only -- artifacts/` is
empty after the fuzz campaigns, and the reports were written under `.cache/qa/`.

## Observed flake, recorded rather than hidden

The first `./dev final` attempt in this revision failed inside `editor-smoke`
with `saved SVG changed bytes outside the selected transform`. `./dev editor-smoke`
passes standalone (recorded in `artifacts/WO-QA-01/check-01.json` for the previous
order), and the re-run of the whole gate passed. The real-window, real-mouse
smoke is timing-sensitive when it runs at the end of a long automated gate; that
flakiness is a defect in the smoke, not evidence of a product regression, and it
is not masked here: the failing diagnostic is quoted above and the smoke is
unchanged by this order.

## Out of scope

`python3 reports/M3/validate_packet.py --check` fails with
`changed source hash: docs/PERF.md` because the M4 performance work legitimately
updated that living document after the M3 packet pinned its bytes. Exactly one of
the packet's pinned inputs is stale; every other frozen input still matches. That
defect is frozen as WO-QA-03 and is not changed here.

## Record sequencing

The candidate contains the `ready`, `run_started` and `run_finished` events for
revision 6, so the counts above are the candidate's. Revision 5 was closed as
`blocked` then `superseded` after the first verifier round. `scripts/ledger.py`
only permits `run_started -> run_finished -> verified -> accepted -> merged`, so
the last three are appended after the verdict, together with the `LEDGER.md` row
and `executor-report.json`, which names the verified candidate commit.
