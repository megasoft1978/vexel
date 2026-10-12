# WO-ANIM-FIX-01 revision 1 verification sweep (2026-10-12)

Executed by the planner session on `main` at `248fbfa`, starting from a clean tree
(`git status --porcelain` empty), after the fix commit `c95ad83`. All nine frozen
commands exited 0.

Ordering detail: the sweep ran first, on the clean tree; these record files were
written immediately afterwards. `git diff --check` therefore rejected the first
artifact state, because writing `README.md` introduced a trailing blank line at
EOF. The blank line was removed and the frozen command was re-executed against
the final artifact state; `check-09.json` carries the re-execution fields
`re_executed_utc` and `re_executed_note`. This defect and its correction are
retained in `verdict-round-1.json` and are not hidden.

Context: the earlier verifier run
`artifacts/runs/WO-ANIM-FIX-01/verify-c49e1762-3053-4204-8d6f-447bc3ef1f83`
returned `fail` because `./dev t --all` evidence was believed missing. That
reason was mistaken: the committed `check-06.json` in that run already recorded
`./dev t --all` exit 0. The implementation change was found correct in that
verdict and in round 1 here, and was not redone.

The pre-fix AddressSanitizer heap-use-after-free that motivated this order is
preserved for audit in
`artifacts/runs/WO-M1-01/verify-9831f120-ddbc-4523-980b-9cd5f3554e26/check-08.json`.

| # | frozen command | exit | evidence record |
|---|---|---:|---|
| 1 | `cmake --preset mac-debug` | 0 | `check-01.json` |
| 2 | `cmake --build --preset mac-debug` | 0 | `check-02.json` |
| 3 | `ctest --test-dir build/mac-debug --output-on-failure` | 0 | `check-03.json` |
| 4 | `./dev asan` | 0 | `check-04.json` |
| 5 | `./dev ubsan` | 0 | `check-05.json` |
| 6 | `./dev t --all` | 0 | `check-06.json` |
| 7 | `python3 scripts/ledger.py validate ledger.jsonl` | 0 | `check-07.json` |
| 8 | `python3 scripts/wo_validate.py WO-ANIM-FIX-01 --ready` | 0 | `check-08.json` |
| 9 | `git diff --check` | 0 | `check-09.json` (re-executed on the final artifact state) |

Observed: `{"ctest": "100% tests passed, 0 tests failed out of 80", "python": ["69"], "ledger": "459"}`

Verifier verdicts: `verdict-round-1.json` (fail, corrected), `verdict.json` (final round).
