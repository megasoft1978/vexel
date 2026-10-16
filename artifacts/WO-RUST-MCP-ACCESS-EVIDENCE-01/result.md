# MCP access evidence follow-up candidate

Added three tests to `rust/crates/vexel-mcp-access/tests/access.rs`; the accepted production source and API are unchanged. The tests prove that `from_environment()` captures both `VEXEL_MCP_ALLOWED_DIRS` and `HOME` once and retains those values after the child process changes them, that an 8191-byte allowed-directory value is accepted while 8192 bytes is rejected, and that a real 1023-byte config pathname is accepted while the next longer HOME-derived pathname is rejected.

The environment mutation runs only in a subprocess invoking this single test, so no sibling or parent test can race on global variables. The 1023-byte config case built a real nested path in the repository-local fixture directory on Darwin arm64.

The prescribed full-workspace Cargo command cannot resolve offline because the local index/cache lacks `objc2-image-io`, a dependency of another workspace member. To exercise the unmodified access crate and its two local path dependencies without network access, the checks used an isolated minimal workspace under `.cache/wo-mcp-access-minimal` and output under `.cache/target-mcp-access-evidence`; check passed and all 15 integration tests passed (including all three new tests). This is focused candidate evidence, not a substitute for the coordinator's full-workspace/integrated-main verification.

No production behavior or historical C source changed. The C adapter reads process environment on each access check; Rust deliberately snapshots values at policy construction. This test makes the Rust snapshot contract explicit. Independent review and coordinator integration remain outstanding.
