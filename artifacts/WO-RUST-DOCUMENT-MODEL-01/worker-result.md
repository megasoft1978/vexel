# WO-RUST-DOCUMENT-MODEL-01 parity correction

Root finding: C initializes the result to zero, then sets only `InputLimit` when source exceeds the default 16 MiB cap; the diagnostic offset remains **0**. Corrected the Rust source-limit diagnostic offset and its candidate assertion to 0. The 16 MiB source cap and all other eight test assertions remain unchanged.

Archived the prior candidate result, parity map, verification, and test source in `archive/prior-candidate/`; `SHA256SUMS.txt` records their hashes. The separate large-file editable API is a later feature gap, outside this correction.

Named checks pass: check, 9 tests, work-order validation, ledger validation, and `git diff --check`. Candidate remains subject to review/integration.
