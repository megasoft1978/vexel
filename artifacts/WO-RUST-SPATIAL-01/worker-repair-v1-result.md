Moved combined extent overflow validation ahead of duplicate-ID validation, matching C status precedence. Added the exact duplicate-ID regression with records at `x = -f64::MAX` and `x = +f64::MAX`; it expects `SpatialError::Overflow`. Updated the parity map to clarify that IDs are validated in temporary storage and not retained.

Checks passed with rustc 1.99.0: named Cargo check and test, work-order validation, ledger validation, and `git diff --check`. Tests: 8 integration and 1 sizing unit test passed.

Independent narrowed recheck and main integration remain outstanding; this repair does not declare acceptance. Uncommitted; edits stayed within owned Rust tests/source and artifacts.