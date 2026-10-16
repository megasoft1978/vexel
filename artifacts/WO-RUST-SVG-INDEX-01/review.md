**REJECT**

**Blockers**

- **The metadata budget can be exceeded and the excess allocation retained.** In [lib.rs](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust5-index/rust/crates/vexel-svg-index/src/lib.rs:416), `try_reserve_exact` may provide capacity beyond the requested `next`. The code notices that actual capacity exceeds the component budget at line 427, but leaves that oversized `Vec` allocated in the session. The validator also assumes the XML parser’s open-name stack has exactly 256 entries at lines 328–331; [stream.rs](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust5-index/rust/crates/vexel-xml/src/stream.rs:157) reserves that amount without exposing or checking actual capacity. This does not establish the required bound on **actual allocated capacity**.

- **Malformed status and progress timing differ from the C API.** Once validation completes with an error, legacy `svg_index_step` harvests the diagnostic and returns `SVG_INDEX_MALFORMED`. Rust [step](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust5-index/rust/crates/vexel-svg-index/src/lib.rs:145) waits until the lexical reader reaches the validator’s error offset before returning `Malformed`. Thus the same completed background error can produce additional reader progress and `Ok` steps in Rust.

**Nonblocking gaps**

- The parity map describes storage accounting but does not resolve the capacity issues above.
- I did not run tests or builds, as requested. The verification artifact records passing scoped checks; this review does not independently confirm them.
- No full application or performance certification was performed.