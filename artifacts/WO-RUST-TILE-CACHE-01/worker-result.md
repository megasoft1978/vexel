Implemented the full frozen cache scope. Metadata charge uses the actual reserved slot capacity and `size_of::<Option<Entry>>()`. Payload charge uses each `Tile::allocated_byte_count()` in admission, replacement, eviction, and stats. Failed insertion returns the same tile allocation; regressions verify pixels, pointer, and capacity. Clock injection remains test-only.

Final evidence: locked offline package check passed; tests passed (3 unit, 8 integration); WO ready validation passed; ledger validation passed (1148 entries); `git diff --check` passed; parity JSON is valid. The prior consolidated rejection and repairs are preserved in `verification.txt`.

No renderer integration or full application completion is claimed. The candidate remains subject to independent narrowed recheck and main audit.
