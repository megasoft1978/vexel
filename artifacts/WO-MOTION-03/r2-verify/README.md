# WO-MOTION-03 revision 2 runner check logs

`check-NN.json` and `verifier-build-NN.json` are the exact records the project verifier produced for the frozen verification commands in a fresh detached worktree (run `verify-b8f7974c-ce07-4def-a66e-abf8b353e33e`; the runs directory is gitignored). Every command exited 0. Counts in these records, such as ledger entries, reflect the ledger when that run executed; the ledger grows by the closure events afterwards.
