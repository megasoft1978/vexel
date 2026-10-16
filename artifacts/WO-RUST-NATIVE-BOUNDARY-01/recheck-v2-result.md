ACCEPT — `match catch_unwind(...)` binds `_secondary_payload` in the `Err` arm and immediately calls diverging `std::process::abort()`. The payload remains alive through the abort call; it is neither dropped before abort nor forgotten.

The subprocess regression now exercises the previously missing case: the primary payload’s destructor `panic_any`s a secondary payload whose destructor also panics, and asserts the child exits with `SIGABRT`. The recorded scoped run reports 5 integration tests and 2 compile-fail doctests; success-value, ordinary panic, real Foundation exception, and spawned-thread checks remain present.

I reviewed the recorded evidence and source; I did not run tests or builds.