# WO-RUST-RECOVERY-DISCOVERY-01 candidate evidence

Candidate implementation is in `rust/crates/vexel-project-io/src/recovery_discovery.rs`; focused tests are in `rust/crates/vexel-project-io/src/recovery_discovery_tests.rs`. `lib.rs` remains untouched for coordinator wiring. This is a candidate only; no commit was made.

## Verification

- `cargo +stable test -p vexel-project-io --locked --offline` — exit 0.
- Because the frozen path protects `lib.rs` and assigns wiring to the coordinator, the acceptance command cannot be relied on to compile the new private module. To verify it, I copied the root manifest/lockfile and `rust/` tree into `/private/tmp/recovery-discovery-check.4wenRU`, added `mod recovery_discovery;` to that temporary copy only, and ran `cargo +stable test --manifest-path /private/tmp/recovery-discovery-check.4wenRU/Cargo.toml -p vexel-project-io --locked --offline` — exit 0, 19 tests passed (including all three discovery tests).
- `git diff --check` — exit 0.
- `wc -l rust/crates/vexel-project-io/src/recovery_discovery.rs rust/crates/vexel-project-io/src/recovery_discovery_tests.rs` — 137 and 240 lines, respectively.

Tests cover absent, matching, and source-stamp conflict results (including independently changed size and mtime); exact returned bytes; path mismatch; corrupt checksum; unsupported capsule version; invalid embedded bundle; oversized capsule; missing and same paths; symlink and directory paths; and unchanged source/capsule contents and file associations.

## Platform limits

The implementation uses the existing Unix `MetadataExt` device, inode, `mtime`, and `mtime_nsec` APIs and `OsStrExt`, consistent with the repository's macOS-only target. The configured build target is `aarch64-apple-darwin`. No source-body reads, writes, dependency additions, network access, or external SVG resources were used. Growth is checked with a bounded `RECOVERY_MAX_BYTES + 1` read and post-read descriptor metadata; path identity is checked against the opened descriptor.

## Snapshot and report location

Frozen base SHA: `3100010042044e494736f4f2c5c98b4829adec50`. Worktree HEAD at execution: `b509b07ce715b5877a43033873e7cb0cd2a3dfa2` (different from the frozen base). Only the two allowed Rust files and this allowed evidence directory are changed.

The requested JSON report path under `/Users/megasoft78/Desktop/Freelance/vexel/artifacts/...` is outside the writable worktree. An explicit copy attempt failed with `Operation not permitted`. The JSON report is stored here as `artifacts/WO-RUST-RECOVERY-DISCOVERY-01/report.json`.
