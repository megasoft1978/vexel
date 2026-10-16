Added the narrowed repair. The Rust tests now assert exact malformed-input offsets and verify that 4,096-byte names are accepted while 4,097-byte names fail at `name_offset` (byte 1). No implementation defect was found, so no Rust source fix was needed.

Updated [parity-map.json](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust-xml/artifacts/WO-RUST-XML-01/parity-map.json) and added [verification-repair.txt](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust-xml/artifacts/WO-RUST-XML-01/verification-repair.txt). Initial worker logs remain unchanged.

All named checks passed: Cargo check, 10 focused tests, work-order validation, ledger validation (1,058 entries), and `git diff --check`. Changes remain uncommitted.