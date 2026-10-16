# WO-RUST-XML-STORAGE-01 result

Restored `XmlTokenStream::new`'s preserved API behavior: zero and sub-token byte budgets immediately return `Err(StreamStatus::Failed)`. Private count-based `with_limits` behavior is unchanged.

Token growth still uses bounded exact reservations. The stream checks retained Vec capacity against the floored byte budget, releases a rejected oversized reservation, and reports `OutputExhausted` at the original token offset. Added token and open-name storage accessors report actual retained capacity. Visitor scratch and open-name stack capacities remain checked against their fixed caps.

Corrected only the new zero/sub-token assertions; exact-token, nonmultiple, odd-capacity, exact-cap progress, failure, take, cancellation, and visitor-capacity regressions remain. Existing XML assertions and parser/name-read/cancel/visitor contracts remain in place. No allocator RSS claim is made.

The rejected candidate's boundary assertions, result, parity map, and verification record are preserved as `rejected-candidate-*` evidence in this directory. This is an uncommitted candidate for separate root review and integration.
