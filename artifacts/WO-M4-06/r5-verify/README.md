# WO-M4-06 revision 5 runner evidence

`scripts/wo-verify.sh WO-M4-06 d120cd8045a08c7d8dc5ac662131386d46c7b381` executed these
records in a fresh detached worktree (run `verify-a42e8e4e-0aec-424f-b2c2-bec67f1b8faf`; the runs directory is gitignored).
All 18 frozen verification commands exited 0. That includes Debug CTest at 79/79, with
`native-shell-goldens` passing after WO-UI-GOLDEN-01, plus `./dev asan`, `./dev ubsan` and
`./dev verify`. Check 17 validates WO-M4-06 revision 5.

The candidate that follows `d120cd8` changes only `artifacts/WO-M4-06/**` to add this
evidence. Before every review, the runner re-applies the path guard and re-runs all 18
commands on the exact candidate.
