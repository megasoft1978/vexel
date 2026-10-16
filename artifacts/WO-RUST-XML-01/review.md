**REJECT — test coverage gaps; no production-code defect found.**

The Rust reader and character helpers closely match the named C behavior. The reader borrows immutable source, caps step work, and uses bounded state; the UTF-8 decoder rejects malformed scalars. I found no lifetime, checked-arithmetic, or byte-span parity issue in the reviewed paths.

Two acceptance behaviors lack meaningful assertions:

- In `reader_reports_stable_malformed_categories_offsets_and_callback_failure` (`rust/crates/vexel-xml/src/tests.rs`), malformed cases check only that diagnostic offsets are within the source. They do not assert the expected byte offsets, despite byte-accurate diagnostics being part of the order.
- The 4096-byte lexical name limit in `lib.rs` is not tested at the boundary or when exceeded.

Please add focused assertions for those behaviors before acceptance. No builds or tests were run, as requested.