# WO-ANIM-FIX-01 revision 1 runner check logs

`check-NN.json` are the exact records the project verifier produced for the frozen verification commands in a fresh detached worktree (run `verify-c49e1762-3053-4204-8d6f-447bc3ef1f83`; the runs directory is gitignored). All 9 commands exited 0, including `./dev asan`, `./dev ubsan` and `./dev t --all`. Counts such as ledger entries reflect the ledger when that run executed.
