BLOCKER — the repair still drops the secondary payload before aborting.

At [native_boundary.rs:60](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust-native-boundary/rust/crates/vexel-platform-apple/src/native_boundary.rs:60), `.is_err()` consumes the `Result` and drops its `Err` payload before the `if` body calls `abort()`. If that payload’s destructor panics, the new unwind can escape the guard. The death regression only uses an original payload whose `Drop` panics with a normal string payload; it does not exercise a panicking secondary payload.

Match the caught result directly, e.g. `Err(_payload) => std::process::abort()`, so abort occurs while the secondary payload remains alive. Extend the subprocess regression so the original payload’s `Drop` uses `panic_any` with a secondary payload whose `Drop` also panics; assert the child receives `SIGABRT`.

The recorded 5 integration tests plus 2 compile-fail doctests preserve the prior checks, but they do not cover this defect. I did not run tests or builds.