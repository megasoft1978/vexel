REJECT

- `rust-tools/vexel-dev/src/main.rs` gives `info` before checking the platform. The work order restricts the workspace to Apple Silicon macOS 13+; this command can report support on any host. Add a platform gate or make the claim conditional.
- `build`/`check` create `.cache/cargo` and `.cache/tmp`, but Cargo’s configured `target-dir` is `build/rust`, not the work order’s required repo-local cache setup. The CLI does not redirect target output into `.cache`; align its behavior with the required local build/cache paths.

The workspace is std-only with internal path dependencies; `Span` uses checked addition and bounds-checked slicing; compiler verification requires `rustc +stable` 1.99.0 and claims no install attempt. The geometry and other domain crates are clearly labeled as scaffolding. No formatting-pass claim appears; recorded evidence says `rustfmt` was unavailable and no component was installed.