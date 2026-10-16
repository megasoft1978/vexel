**REJECT.**

**Blockers**

- At [source_save.rs](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust6-source-save/rust/crates/vexel-document-io/src/source_save.rs:123), the over-16-MiB branch returns an `InputLimit` parse diagnostic. The C wrapper returns `INPUT_LIMIT` before tokenizing, leaving the diagnostic cleared. The test at [source_save.rs](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust6-source-save/rust/crates/vexel-document-io/tests/source_save.rs:168) currently asserts the mismatched diagnostic.
- The frozen [test_source_save.c](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust6-source-save/tests/document/test_source_save.c) is modified, despite being a protected path. The diff adds tests and changes the fixture location; it does not appear to weaken assertions, but still violates the work order’s path rules.

**Nonblocking gaps and checks**

- Empty source reaches the tokenizer, so the implementation does not reject it as an argument. The Rust tests do not explicitly pin the expected tokenizer error for that case.
- Reviewed the other requested paths: token-cap exhaustion is asserted; NUL paths reach the native adapter after successful tokenization; create fallback forwards only the write hook; native success flags come from adapter results; and source bytes are passed through unchanged.
- Source-save implementation is under 800 lines and contains no unsafe code. The recorded check results were read from the artifact; I ran no tests, builds, or other verification commands. This is not a full-app, fidelity, or performance certification.