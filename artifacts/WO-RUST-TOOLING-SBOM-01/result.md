# WO-RUST-TOOLING-SBOM-01 result

Implemented native CycloneDX SBOM generation in `vexel-dev` using `shasum -a 256` and `otool -L` through `Runner`. The generator validates the SHA-256 digest before writing, preserves empty linked-library output on a nonzero `otool` exit, sorts and deduplicates libraries, escapes JSON strings, and writes `sbom.cyclonedx.json`. Fake-runner tests cover metadata and component fields, ordering/deduplication, quote/backslash/control escaping, missing executable, malformed digest, SHA command failure, and nonzero `otool`.

## Verification

1. `cargo +stable test -p vexel-dev --locked --offline` — PASS; 28 passed, 0 failed.
2. `cargo +stable fmt --manifest-path Cargo.toml --package vexel-dev -- --check` — PASS; exit 0.
3. `git diff --check` — PASS; exit 0.
4. Fake-runner invocation audit — PASS; valid SBOM path invokes only `shasum -a 256 <binary>` and `otool -L <binary>`, and no Python command; failing `otool` writes no library components.
5. Scope audit — PASS; implementation changes are limited to `rust/tools/vexel-dev/src/release.rs` and this evidence file. No dependency, manifest, lockfile, or protected path changes.
6. Independent read-only review — CLEAN; reviewer confirmed the implementation, failure behavior, test coverage, and scope with no actionable findings.

The output fixture is generated and checked by unit tests in a temporary directory; no persistent generated fixture was retained.

The required external `report.json` path under `/Users/megasoft78/Desktop/Freelance/vexel/artifacts/runs/` is outside the writable sandbox and rejected with `operation not permitted`. A copy is saved at `artifacts/WO-RUST-TOOLING-SBOM-01/report.json` in this worktree.

## Escaped `shasum` filename correction

The SHA parser now accepts the optional leading backslash marker emitted by macOS `shasum -a 256` when the input filename contains a newline or backslash, removes only that marker, and then requires exactly 64 ASCII hexadecimal characters. A fake-runner regression uses a binary path containing both characters and returns the escaped-output form; it verifies the emitted SBOM contains the expected 64-character digest. The malformed-digest test remains unchanged and still verifies that no SBOM is written.

Revision-2 verification, run in the candidate worktree:

1. `cargo +stable test -p vexel-dev --locked --offline` — PASS; `test result: ok. 29 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` (exit 0).
2. `cargo +stable fmt --manifest-path Cargo.toml --package vexel-dev -- --check` — PASS (exit 0).
3. `git diff --check` — PASS (exit 0).
