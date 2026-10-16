ACCEPT

The candidate satisfies the owned-animation work-order contracts. Comparing crate hashes and diffs against the main crate, `parse.rs` adds only `parse_owned`; the accepted parser and its grammar are unchanged. `eval.rs` routes both APIs through the same track-slice engine. The test diff preserves all nine prior test bodies and assertions verbatim, adding three tests.

`parse_owned` moves parsed tables and clones the source `Arc` only after success. Owned views borrow the retained source and tables; evaluation does not clone tables or reparse. Storage accounting includes source capacity once and checked capacities for track, diagnostic, key, motion-point, and spline vectors. The track-limit failure test verifies the owner count stays unchanged; the worker test verifies moved spans and evaluation.

The candidate report’s verification is backed by recorded passing check and test output: 12 integration tests passed, plus the validator, ledger, and diff checks. No build or tests were run for this review.