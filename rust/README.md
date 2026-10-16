# Vexel Rust workspace

Foundation scaffolding only; desktop app and full legacy parity are pending.
Rust1.99.0/2024, MPL2, std-only/internal dependencies, AppleSilicon macOS13+ only.
Existing C checkout remains migration reference.

This host has Rust1.99.0 under stable, not a version-named installed alias.
Use cargo +stable after verifying rustc +stable --version is exactly1.99.0;
this avoids a toolchain download outside the project. Set CARGO_HOME/TMPDIR
to repo-local .cache/cargo and .cache/tmp; create them first. Cargo output stays
in ignored build/rust. Work orders name focused Rust tests; no broad UI runs.
The Rust developer CLI supports info/build/check and calls Rust Cargo only.
Legacy ./dev stays reference until its complete Rust command port is accepted.
