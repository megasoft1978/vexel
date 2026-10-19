# WO-RUST-RECOVERY-DISCOVERY-01 candidate evidence

The candidate adds bounded, read-only capsule discovery in `rust/crates/vexel-project-io/src/recovery_discovery.rs` and focused fixture coverage in `rust/crates/vexel-project-io/src/recovery_discovery_tests.rs`. The private module declaration is present in `rust/crates/vexel-project-io/src/lib.rs`; no public re-export was added.

The source descriptor metadata is refreshed after capsule reading. The refreshed descriptor must still be a regular file, the supplied path must still identify that descriptor, and the capsule's saved size, modification seconds, and modification nanoseconds are compared against that refreshed metadata. A private test seam deterministically edits the source in place after capsule reading and verifies a `Conflict` result with the original capsule bytes.

The opened capsule descriptor is compared with the opened source descriptor by device and inode before any capsule read. A hard-link alias returns `Invalid { error: RecoveryError::InvalidArgument }` without invoking the capsule-read hook. The regression also checks that both file associations and the source bytes remain unchanged. The source-stamp regression above is retained.

## Reproduction

`cargo +stable test -p vexel-project-io --locked --offline rejects_hard_link_alias_before_reading_source_body` before the fix exited 101:

```text
test recovery_discovery::tests::rejects_hard_link_alias_before_reading_source_body ... FAILED
discovery read the aliased source body
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 20 filtered out; finished in 0.00s
```

## Verification

- `cargo +stable test -p vexel-project-io --locked --offline` — exit 0; 21 passed, 0 failed, 0 ignored; doc tests 0 passed. The test build reports six `dead_code` warnings for the private module items pending coordinator public re-exports.
- `rustfmt --check rust/crates/vexel-project-io/src/recovery_discovery.rs rust/crates/vexel-project-io/src/recovery_discovery_tests.rs` — exit 0.
- `git diff --check` — exit 0.
- `wc -l rust/crates/vexel-project-io/src/recovery_discovery.rs rust/crates/vexel-project-io/src/recovery_discovery_tests.rs` — 153 and 300 lines, respectively.

The focused tests cover absent, matching and conflict results; independent size, seconds and nanoseconds stamp changes; source edit during capsule reading; exact returned bytes; wrong source identity; corrupt checksum; unsupported capsule version; invalid embedded bundle; oversized capsule; missing and same paths; hard-link source/capsule aliases rejected before reading; symlink and nonregular paths; and unchanged source/capsule associations for read-only discovery.

## Platform limits

The implementation uses Unix `MetadataExt` device, inode, `mtime`, and `mtime_nsec` APIs and `OsStrExt`, consistent with the macOS-only target. No dependencies, network access, source-body reads, file writes or deletes, or external SVG resources were used.

## Snapshot

- Work order revision: 3.
- Frozen base SHA: `62d2d99937cb989a2d8ad989a6f52218aeea4542`.
- Task branch: `wo/WO-RUST-RECOVERY-DISCOVERY-01`.
- Pre-fix task branch snapshot: `494f505bce3a4d0142931a648e8d01cc231b2649`.
- Verification ran against the working tree based on the pre-fix task branch snapshot above; the final commit SHA is returned with the task handoff.
- Changes remain limited to the assigned discovery source, its focused tests, and this evidence directory.
