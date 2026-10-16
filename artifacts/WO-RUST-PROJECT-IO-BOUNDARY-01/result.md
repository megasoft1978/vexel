# WO-RUST-PROJECT-IO-01 candidate update

Added focused recovery decode assertions for invalid embedded-project precedence and offset propagation. A mutated bundle with repaired outer checksums reaches the bundle decoder and returns `InvalidProject`, `BundleError::Corrupt`, and the frozen bundle header offset; a mismatched expected source path returns `PathMismatch` first with no partial bundle error/offset. Added a bounded maximum-total-size test for the exact recovery ceiling formula, the maximum accepted path length, and malformed declarations at the ceiling and one byte beyond the bundle limit. It rejects malformed sizes without allocating a >1 GiB test fixture.

Updated `parity-map.json` to document the successful decode `encoded_size` adaptation: Rust returns a borrowed `RecoveryView`, while the caller retains the validated input slice and obtains the exactly matching size with `encoded.len()`; no copy is needed. No production behavior changed.

Named checks passed: offline locked Cargo check; 9 tests; work-order validation; ledger validation; `git diff --check`. See `verification.txt` for exact commands and results. Candidate remains uncommitted; stale caches were not cleaned.
