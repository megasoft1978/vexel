ACCEPT

- The Rust model preserves borrowed source bytes, raw name and attribute spans, complete element spans, tree queries, stale and foreign handle rejection, subtree live counts, and root removal. Removal leaves source unchanged and advances each removed live node’s generation.
- Production parsing enforces the 16 MiB input limit, 256 depth limit, and a 64 MiB tokenizer storage cap checked against actual retained capacity. Diagnostics retain XML categories and offsets; tokenizer exhaustion maps to `ResourceExhausted`.
- Atomic IDs reject 0 and `u64::MAX` before wraparound. The isolated test seam uses the same transition function as the production allocator. The crate forbids unsafe code and adds no dependencies.
- The local XML stream and test hashes match the coordinator delta’s authorized post-change hashes: `888a62d7…` and `5e871b68…`. The coordinator work-order and ledger hashes also match.
- The parity map covers the document model APIs and helpers. The original fixture’s view-range pagination assertion is outside this model’s API and remains a downstream view concern.

Nonblocking verification note: the candidate’s verification artifact reports the required checks passed; I did not rerun tests or builds, as requested.