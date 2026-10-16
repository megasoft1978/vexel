Implemented revision 2’s atomic replace and no-replace publication APIs in the macOS crate. Checked close consumes `OwnedFd` once in the private `sys.rs` boundary; safe typed hooks cover injected failures and race cases. The original eight tests remain unchanged.

On rustc 1.99.0, the locked offline crate check passed. Tests passed: 17 unit tests, including 15 new atomic-file tests, and all 6 `native_system` integration tests. The work-order validator, ledger validation (1149 entries), and `git diff --check` also passed.

The 182-word result and full parity map are in [result.md](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust3-atomic-files/artifacts/WO-RUST-ATOMIC-FILES-01/result.md) and [parity-map.json](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust3-atomic-files/artifacts/WO-RUST-ATOMIC-FILES-01/parity-map.json). Revision 1 blocker evidence is preserved in the `r1-*` artifacts.

This uncommitted crate candidate awaits coordinator review and integration checks. Caller file grants and application/project wiring remain; the full application is not complete.