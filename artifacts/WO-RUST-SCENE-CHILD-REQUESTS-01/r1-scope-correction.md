# R1 scope correction

R1's workspace-wide `cargo fmt --all -- --check` emitted formatting differences in unchanged protected files such as `rust/crates/vexel-base/src/lib.rs`. The changed Scene Rust files passed their scoped `rustfmt --check`. R2 narrows formatting verification to changed Rust files and retains the R1 log as evidence.

R1's worker also appended a short note to its isolated root README outside the allowlist. The coordinator removed exactly those four lines in that detached worktree before the revision-2 run. The shared main README was not modified by this cleanup. The Scene README copied into the worker matches its recorded accepted-main hash and contains no candidate changes.

R1 tests passed, but its new test omitted mapped static-pattern ticket creation and cancellation at each request depth. R2 closes those proof gaps and corrects the path audit.
