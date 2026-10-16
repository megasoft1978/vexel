# Revision 2 continuation result

Closed the two r1 acceptance gaps in the isolated worker. Added a Scene API regression that parses a real lexical document, creates a source-mapped shape in a nested descendant request, seals a static tile child, and successfully mints `StaticPatternRequest` through `PatternTickets::request_mapped_static`. The returned static view carries the nested request identity. This proves typed capability construction only; it makes no renderer claim.

Added cancellation/abort cleanup for each active request depth (outer through three nested child scopes). The existing 64-scope boundary/overflow, rollback, token misuse, parent restoration, and frozen legacy API rejection checks remain in place. `decode_publication_contracts.rs` is unchanged; its exact nested legacy rejection test passes.

## Verification

All Cargo invocations used `CARGO_HOME=/Users/megasoft78/Desktop/Freelance/vexel/.cache/cargo` and `CARGO_TARGET_DIR=$PWD/.cache/build/rust`.

- `cargo +stable test --manifest-path Cargo.toml -p vexel-scene --locked --offline --test child_request_contracts`: PASS, 6 tests.
- `cargo +stable test --manifest-path Cargo.toml -p vexel-scene --locked --offline --test decode_publication_contracts request_preserves_parent_context_with_real_nested_child_drafts -- --exact`: PASS, frozen rejection retained.
- `cargo +stable check --manifest-path Cargo.toml -p vexel-scene --locked --offline`: PASS.
- `cargo +stable test --manifest-path Cargo.toml -p vexel-scene --locked --offline`: PASS, full package suite.
- `rustfmt --check` on every changed Rust file: PASS.
- `python3 scripts/wo_validate.py WO-RUST-SCENE-CHILD-REQUESTS-01 --ready`: PASS, revision 2 frozen hash `bc66b60ceddf787ed8e1e18a2714aa781ce27aadf23ddc60f7b80103096ee863`.
- Protected hash audit: PASS, 61 accepted-main baseline files checked; zero protected hash failures; all changed source files below 800 lines. Exact path allowlist includes `rust/crates/vexel-scene/README.md`; isolated root `README.md` remains unchanged at SHA-256 `34f70610364f798bcfa9d036118eba3e4789bb241371cf84b57a775856bb84ac`.

Logs: `child-request-focused.log`, `frozen-rejection-focused.log`, `scene-check.log`, `scene-package.log`, `rustfmt.log`, and `validator.log`. Scope and hash details: `scope-audit.json`.

R1 evidence under `continuation-1/` and the earlier frozen baseline evidence remain preserved. No workspace-wide formatting was run. No nested workers were used; review is limited to these required acceptance checks.
