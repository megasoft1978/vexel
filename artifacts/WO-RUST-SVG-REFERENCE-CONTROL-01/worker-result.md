# Repair result

Changed trivial lookup construction so invalid spans, empty keys, and indexes without a slot table remain nonterminal until stepped. The first zero quantum checkpoints and returns `Pending { charged_units: 0 }`; the first positive step checkpoints before completing `None` without source work. `find_controlled` propagates checkpoint cancellation and limits for these cases. Completed sessions retain their stable result without additional checkpoints or charges; legacy `find` still returns `None` with unchanged probe counts.

Restored `tests/reference.rs` and the original `src/lib.rs` `#[cfg(test)]` block byte-for-byte from the accepted baseline. Verification also covers the prior build/lookup candidate: 1 unit, 7 controlled, and 10 original integration tests pass. Work-order validation, ledger validation, whitespace check, and baseline comparisons pass. Files are ready for independent review; no shared files changed.

Word count: 127
