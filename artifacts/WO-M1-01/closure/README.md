# WO-M1-01 revision 3 closure evidence (2026-10-12)

The Cocoa event/window shim (`src/platform/macos/`, `app/test_driver/`) reached `main` through the later M1-02..M1-04 work orders and every lane since. Revision 3 only verifies that the shipped code still meets the frozen commands on current `main`. Per-command output is retained in `../r3-verify/` (exact runner records).

| AC | Evidence |
|---|---|
| AC-1 Debug build | runner records for `cmake --preset mac-debug` and `cmake --build --preset mac-debug` |
| AC-2 Release build | runner records for `cmake --preset mac-release` and `cmake --build --preset mac-release` |
| AC-3 focused tests | runner record for `python3 -m unittest tests.test_platform_driver` |
| AC-4 100-cycle native smoke | runner record for `python3 app/test_driver/driver.py --native-platform-smoke --cycles 100 ...` |
| AC-5 verify, asan, ubsan, ledger, WO, diff | runner records for `./dev verify`, `./dev asan`, `./dev ubsan`, and the three validators |
| AC-6 binary size and smoke time | `bench/results/WO-M1-01.json` (revision-1 measurement, history; this closure makes no new performance claim and no rendering throughput or RSS claim) |
| AC-7 fresh verifier | the revision-3 verdict in `../r3-verify/verdict.json` |

The files directly under `artifacts/WO-M1-01/` are revision-1 and revision-2 executor evidence and are history.
