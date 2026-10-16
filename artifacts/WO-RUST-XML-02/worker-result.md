Implemented the bounded XML tokenizer and Rust APIs for tokenization, DOCTYPE scanning, attribute decoding, cancellation, limits, diagnostics, and streaming. Added focused parity tests while preserving the accepted reader tests.

- `cargo +stable check -p vexel-xml --locked --offline`: passed.
- `cargo +stable test -p vexel-xml --locked --offline`: 17 passed.
- Work-order and ledger validators passed; `git diff --check` passed.
- Parity mappings and evidence: [parity-map.json](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust2-xml-tokenizer/artifacts/WO-RUST-XML-02/parity-map.json) and [verification.txt](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust2-xml-tokenizer/artifacts/WO-RUST-XML-02/verification.txt).

The parity map records a streaming adaptation: input scanning and token visibility advance in bounded steps, while grammar parsing runs after the scan completes. Visitor mode retains tokens within its configured storage bound, rather than using only the C scanner’s active-tag scratch space. Rustfmt and Clippy were unavailable and were not installed or claimed. No shared records, manifests, locks, original assertions, or other crates were edited; changes remain uncommitted.