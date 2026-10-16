# Continuation 2 result

Candidate remains isolated; no root/main integration or acceptance claim was made. All changes are limited to the four allowed Scene files. Continuation-1 evidence is preserved.

## Repairs

- `ChargeLedger::allocation_history()` now includes each actual retained lease-registry Vec capacity increase under `AllocationTag::None`. Reserve overlap is capped using checked remaining capacity for total history, None-tag live bytes, and None-tag cumulative history before the Vec reserve can mutate storage. The payload amount tagged `None` remains included independently; released/reused registry storage does not erase prior history.
- Added accounting unit tests for exact None-tag registry/payload capacity deltas and for payload history overflow; the overflow proof sets private payload history near `usize::MAX`, verifies arithmetic failure and overflow reporting, and proves live/total/tag/history state stays unchanged apart from the overflow flag.
- Added cancellation after replacement allocation and nonempty copying. It proves the old contents and capacity remain, while replacement physical-allocation history persists. Successful replacement checks exact actual Vec capacity-based history delta.
- README semantics now describe both payload and retained registry history.

## Verification

- `rustfmt --edition 2024 --check` on the changed Rust source/test files: passed (`scoped-rustfmt.log`).
- Offline locked `vexel-scene` check and package tests: passed (`check.log`, `test.log`). Package tests include **eight** observability integration tests, in addition to the seven original tests confirmed in continuation-1 `test.log`; ten compile-fail doctests pass.
- Ready validator: passed (`ready.log`).
- Seed manifest audit: 61 files match; the three mismatches are the allowed candidate-modified `accounting.rs`, `source_context.rs`, and README. All unchanged seeded files match. Rust source files are untracked relative to HEAD, so the seed JSON—not git status—was used for baseline verification.
- Package-level format scan was not rerun; independent Sol review records failures confined to unchanged protected files. `git diff --check` remains blocked by the protected `STATE.md:1341` blank line at EOF (`diff-check.log`). Neither protected file was edited.
- Commands were headless CLI checks on macOS; no GUI or desktop state was used.

Hashes, line counts, command statuses, and seed comparisons are in `verification.json` and `seed-hash-audit.json`.
