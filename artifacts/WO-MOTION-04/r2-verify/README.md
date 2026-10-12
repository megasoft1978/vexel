# WO-MOTION-04 revision 2 runner check logs

`check-NN.json` and `verifier-build-NN.json` are the exact records the project verifier produced for the frozen verification commands in a fresh detached worktree (run `verify-40953df7-3d7b-4cea-9fe3-41c309122e7c`; the runs directory is gitignored). Every command exited 0. Counts in these records, such as ledger entries, reflect the ledger when that run executed; the ledger grows by the closure events afterwards.
