# WO-M1-01 revision 3 closure evidence (2026-10-12)

The Cocoa event/window shim (`src/platform/macos/`, `app/test_driver/`) reached
`main` through the later M1-02..M1-04 work orders and every lane since.
Revision 3 only verifies that the shipped code still meets the frozen commands
on current `main`. Per-command output is retained in `../r3-verify/`.

## Scope of the closure candidate

The closure candidate is the uncommitted evidence set under
`artifacts/WO-M1-01/r3-verify/` plus the truthful refresh of
`bench/results/WO-M1-01.json`. It changes no product code, test, golden, or
oracle.

The commit range `2b7336b..248fbfa` contains commits from other work orders, so
it is not the closure diff: `a1be0f4` is this work order's own revision-3
freeze, and `c95ad83` / `248fbfa` belong to WO-ANIM-FIX-01 (whose allowlist
contains `tests/anim/animation_authoring_edge_test.c`). The closure note's "no
test, golden or oracle may change" applies to the closure candidate, not to
unrelated work orders that landed in the same range.

## Acceptance mapping

| AC | Evidence |
|---|---|
| AC-1 Debug build | `../r3-verify/check-01.json`, `check-02.json` |
| AC-2 Release build | `../r3-verify/check-03.json`, `check-04.json` |
| AC-3 focused tests | `../r3-verify/check-05.json` — 13 tests pass; coverage in `tests/test_platform_driver.py` (lifecycle replay, resize/scale, key/pointer normalization, clean quit, malformed/missing rejection, 100 cycles) |
| AC-4 100-cycle native smoke | `../r3-verify/check-06.json` — exactly 100 cycles, events `open resize scale key pointer close`, and `.cache/WO-M1-01/native-smoke/run-0002/result.json` (the run that produced `check-06.json`; `run-0001` is the earlier baseline run) |
| AC-5 verify, asan, ubsan, ledger, WO, diff | `../r3-verify/check-07.json` … `check-12.json` |
| AC-6 binary size and smoke time | `bench/results/WO-M1-01.json` — `closure_revision_3` records the current measured Release size (1,321,920 bytes) and smoke elapsed time (3.174 s, matching `check-06.json` and `run-0002`). The revision-1 measurement (74,048 bytes) is retained under `revision_1_history` and is not presented as current. No rendering throughput or RSS claim is made. |
| AC-7 fresh verifier | `../r3-verify/verdict.json` (final round); the corrected round-1 verdict is retained as `../r3-verify/verdict-round-1.json`. |
