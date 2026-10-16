# Candidate scope audit — WO-RUST-APPLE-PRESENTATION-01

Compared isolated candidate file hashes with `artifacts/WO-RUST-APPLE-PRESENTATION-01/source-seed.json` and verified the frozen work order against its recorded source-seed hash.

- Frozen work order full-file SHA-256: `c6eca6dd74616c4e099f84b889e1bbe2dd029b424b9e2559ece316c4f27fdae0` (unchanged).
- Changed seeded files (all permitted Apple crate paths):
  - `rust/crates/vexel-platform-apple/src/lib.rs`
  - `rust/crates/vexel-platform-apple/src/native_window/app.rs`
  - `rust/crates/vexel-platform-apple/src/native_window/mod.rs`
  - `rust/crates/vexel-platform-apple/src/native_window/types.rs`
  - `rust/crates/vexel-platform-apple/src/native_window/window.rs`
- New candidate files (both permitted Apple crate paths):
  - `rust/crates/vexel-platform-apple/README.md`
  - `rust/crates/vexel-platform-apple/src/native_window/frame.rs`
- Shared workspace manifest, lockfile, and Apple crate manifest are unchanged from the source seed.
- No candidate changes were found outside `rust/crates/vexel-platform-apple/**`.
- This is a source-seed hash scope audit; it does not claim a Git diff against the owner’s full working tree or runtime AppKit validation.
