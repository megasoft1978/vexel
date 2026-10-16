# WO-RUST-PROJECT-IO-01 candidate update

Added the existing-destination side of the checksum-corrupt encoded-save regression while retaining the absent-destination assertion. The test seeds known bytes, verifies exact `InvalidArgument`/`Corrupt` errors and zero-write/unpublished/unsynced/default-native-error fields, then confirms the seeded bytes and directory entry count are unchanged. No temporary file is left. No production behavior changed.

The authorized pure decoder mapping remains covered: Rust decoder `OutOfMemory` maps to `FileError::OutOfMemory` and retains `bundle_error`; `Limit` and other mappings are unchanged. All eight tests are preserved.

The named locked offline Cargo checks, work-order validation, ledger validation, and `git diff --check` were run. The candidate remains uncommitted and confined to the owned crate and work-order artifacts.
