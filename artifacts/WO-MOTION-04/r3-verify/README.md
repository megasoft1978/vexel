# WO-MOTION-04 revision 3 runner check logs

`check-NN.json` and `verifier-build-NN.json` are the exact records the project verifier produced for the frozen verification commands in a fresh detached worktree (run `verify-231e7428-8fee-4acc-9490-b69f2c15e60d`; the runs directory is gitignored). Every command exited 0. Counts in these records, such as ledger entries, reflect the ledger when that run executed; the ledger grows by the closure events afterwards.
