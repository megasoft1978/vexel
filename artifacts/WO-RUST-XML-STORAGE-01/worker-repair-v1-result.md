Restored the public API contract: `XmlTokenStream::new` immediately returns `Err(StreamStatus::Failed)` for zero and sub-token budgets. The bounded capacity growth, storage accessors, and valid capacity/lifecycle regressions remain unchanged. The original XML assertions are preserved.

Archived the rejected boundary assertions and parity evidence under `artifacts/WO-RUST-XML-STORAGE-01/rejected-candidate-*`. Updated the current result, parity map, and verification record.

All named checks passed: Rust 1.99 offline check; 31 tests passed; work-order and ledger validation passed; `git diff --check` passed. This remains an uncommitted candidate for root review.