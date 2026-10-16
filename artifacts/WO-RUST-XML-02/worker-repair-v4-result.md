Implemented the coherent name-budget repair. PI and entity classification now use cached scan data. End-name and duplicate-attribute equality use resumable comparison state, charging each operand read separately without allocating name copies.

Instrumentation now reports **scanner reads + comparison reads**. Budget-one regressions confirm **at most 1 actual variable token-name source read per step**, including comparison phases. Coverage includes 21 cancellation boundaries and a 4,096-byte name with unchanged visitor scratch capacity.

All prior 25 tests/assertions remain unchanged; **4 added, 29 passed, 0 failed; 0 doc tests**. Existing Rust **1.99.0** package check/test and WO, ledger (**1,096 entries**), diff, and JSON validators passed.

Updated [parity-map.json](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust2-xml-tokenizer/artifacts/WO-RUST-XML-02/parity-map.json) and [verification.txt](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust2-xml-tokenizer/artifacts/WO-RUST-XML-02/verification.txt); prior evidence is preserved.

Two known nonblocking coverage gaps remain: the original C fuzz loop and exhaustive span combinations. Independent final review remains required; acceptance is not declared.