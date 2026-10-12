# WO-MOTION-04 revision 3 runner check logs

`check-NN.json` and `verifier-build-NN.json` are the exact records the project verifier produced for the frozen verification commands in a fresh detached worktree (run `verify-ef4fd00a-ab80-4897-a69f-0f4b7f39e40d`; the runs directory is gitignored). Every command exited 0. Counts in these records, such as ledger entries, reflect the ledger when that run executed; the ledger grows by the closure events afterwards.
