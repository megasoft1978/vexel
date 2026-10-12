# WO-M1-01 revision 3 closure verification sweep (2026-10-12)

Closure revision 3 verifies that the shipped Cocoa window shim still meets the
frozen commands on current `main` (`248fbfa`). All twelve frozen commands
exited 0.

Ordering detail: the sweep ran first, on a clean tree; these record files were
written immediately afterwards. `git diff --check` therefore rejected the first
artifact state, because writing the evidence READMEs introduced a trailing
blank line at EOF. The blank line was removed and the frozen command was
re-executed against the final artifact state; `check-12.json` carries the
re-execution fields `re_executed_utc` and `re_executed_note`. Round 1 of the
verification found this and two further evidence defects; all three are
retained in `verdict-round-1.json` and are not hidden.

The first closure attempt
`artifacts/runs/WO-M1-01/verify-9831f120-ddbc-4523-980b-9cd5f3554e26/` stopped
at `./dev asan`: CTest 77 `animation-authoring-edge` aborted with an
AddressSanitizer heap-use-after-free in `authoring.c:113`, caused by a test that
freed the borrowed animation source before `svg_anim_authoring_add_key`.
WO-ANIM-FIX-01 corrected the test lifetime; `./dev asan` and `./dev ubsan` now
pass 80/80 (records 08 and 09).

| # | frozen command | exit | evidence record |
|---|---|---:|---|
| 1 | `cmake --preset mac-debug` | 0 | `check-01.json` |
| 2 | `cmake --build --preset mac-debug` | 0 | `check-02.json` |
| 3 | `cmake --preset mac-release` | 0 | `check-03.json` |
| 4 | `cmake --build --preset mac-release` | 0 | `check-04.json` |
| 5 | `python3 -m unittest tests.test_platform_driver` | 0 | `check-05.json` |
| 6 | `python3 app/test_driver/driver.py --native-platform-smoke --cycles 100 --output-dir .cache/WO-M1-01/native-smoke` | 0 | `check-06.json` |
| 7 | `./dev verify` | 0 | `check-07.json` |
| 8 | `./dev asan` | 0 | `check-08.json` |
| 9 | `./dev ubsan` | 0 | `check-09.json` |
| 10 | `python3 scripts/ledger.py validate ledger.jsonl` | 0 | `check-10.json` |
| 11 | `python3 scripts/wo_validate.py WO-M1-01 --ready` | 0 | `check-11.json` |
| 12 | `git diff --check` | 0 | `check-12.json` (re-executed on the final artifact state) |

Observed: `{"python": ["13", "302"], "ctest": "100% tests passed, 0 tests failed out of 80", "ledger": "459"}`

Verifier verdicts: `verdict-round-1.json` (fail, corrected), `verdict.json` (final round).
