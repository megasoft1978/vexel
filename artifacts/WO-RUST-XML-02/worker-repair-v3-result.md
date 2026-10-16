Name parsing now preserves the partial scalar decoder state and caches QName classification, eliminating repeated full-scalar decoding.

The measured regressions cover 2/3/4-byte start, attribute, PI, and end names: each budget-one parse step performs **at most 1 actual name-source read and 1 decoder input**. Cancellation at all **6 partial-scalar boundaries** performs no further name reads.

All 23 existing tests and assertions remain; **2 tests added, 25 passed, 0 failed; 0 doc tests**. Named Cargo checks passed with existing Rust **1.99.0**. WO, ledger (**1,096 entries**), diff, and JSON validation passed.

Updated [parity-map.json](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust2-xml-tokenizer/artifacts/WO-RUST-XML-02/parity-map.json) and [verification.txt](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust2-xml-tokenizer/artifacts/WO-RUST-XML-02/verification.txt); prior evidence is preserved.

Two known nonblocking coverage gaps remain: the original C fuzz loop and exhaustive span combinations. Independent final narrow recheck is required; acceptance is not declared.