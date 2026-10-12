# WO-QA-02 evidence — every fuzzer in the final gate, pinned evidence read-only

Frozen spec revision 5, spec SHA-256
`09fdffe1d9622cb61d96faf88d635097f98a838e6678168492a3d1cabf604a59`.

## What was wrong

1. `./dev final` ran only two of the four fuzzers. `xml_tokenizer_fuzz` and
   `animation_fuzz` existed but no standard gate ran them, and `animation_fuzz`
   had no `./dev` wrapper at all.
2. `./dev fuzz-xml` wrote `artifacts/WO-M2-03/pr-smoke.json`, whose SHA-256 is
   pinned by `reports/M3/validate_packet.py`. Any routine campaign therefore
   rewrote a pinned milestone artifact. `./dev shape-fuzz` had the same shape of
   problem through its default `artifacts/WO-M4-04/fuzz-report.json`.
3. `.github/workflows/fuzz.yml` lacked the `push` trigger that
   `scripts/check_ci.py` requires, so the repository's own CI policy check failed
   on it.

## What changed

- `./dev animation-fuzz [cases] [seed]` builds and runs the ASan animation
  fuzzer, defaulting to 200,000 cases and seed 20261012.
- `./dev fuzz-xml` gained `--output PATH`. Without it, reports go to
  `.cache/qa/`; `scripts/xml_fuzz_campaign.py`'s output confinement now allows
  the ignored `.cache/**` tree plus the pinned `artifacts/WO-M2-03/**`.
- `tests/fuzz/shape/shape_fuzz.py` defaults its report to `.cache/qa/`.
- `./dev final` now runs shape, XML tokenizer, animation and MCP fuzzers, then
  e2e, soak and both native smokes.
- `fuzz.yml` added `push` and writes the pinned packet only through an explicit
  `--output`, keeping `scripts/check_fuzz_workflow.py`'s failure-log confinement
  satisfied.
- `tests/e2e/test_dev_final.py` grew from 1 to 4 tests, asserting that every
  fuzzer runs in the final gate with a fixed seed, that `animation-fuzz` defaults
  stay bounded, and that no `./dev` command defaults to a tracked artifact report.

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

`./dev final` passed end to end: CTest `100% tests passed, 0 tests failed out of 80` and Python `331`
passing, including the XML and animation fuzz steps it now runs. The ledger
validated at `479` entries.

AC-1's cleanliness check is explicit: `git diff --name-only -- artifacts/` is
empty after the fuzz campaigns, and the reports were written under `.cache/qa/`.

## Out of scope

`python3 reports/M3/validate_packet.py --check` fails with
`changed source hash: docs/PERF.md` because the M4 performance work legitimately
updated that living document after the M3 packet pinned its bytes. Exactly one of
the packet's pinned inputs is stale; that defect is handled by WO-QA-03 and is
not changed here.

## Record sequencing

The candidate contains the `ready`, `run_started` and `run_finished` ledger
events, so the counts above are the candidate's. `scripts/ledger.py` only permits
`run_started -> run_finished -> verified -> accepted -> merged`, so the last three
are appended after the verdict, together with the `LEDGER.md` row and
`executor-report.json`, which names the verified candidate commit.
