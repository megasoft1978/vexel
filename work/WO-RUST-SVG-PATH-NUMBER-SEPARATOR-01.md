---
id: WO-RUST-SVG-PATH-NUMBER-SEPARATOR-01
revision: 2
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 6168dcd502bbd53b4d0acb71ccc55b18b5bbf9ffc6ef6d7dbca821e96c914ad1
parallel_safe: true
network: false
---
# Repair SVG path numeric token boundary

## Goal and frozen scope
Fix the observed compatibility failure blocking the complete SVG scalar-index
candidate: the accepted `vexel-svg-path` parser currently accepts
`M0 0 L1.2.3 4 5`, while the frozen legacy SVG fixture explicitly rejects the
equivalent `M0 0 L1.2.3 4 5`. Preserve supported compact SVG number syntax,
including adjacent dot-leading numbers such as `.6.5` where already accepted,
plus signed adjacency, exponent behavior, exact error precedence, source spans,
and controlled parser charging/cancellation.

Add a regression that drives `parse_path`, `parse_path_controlled`, and the
resumable `PathParseSession` at small quanta, verifies the malformed decimal
pair is rejected with the correct absolute offset for a nonzero source span,
and confirms valid compact adjacent values remain accepted. Preserve all
existing accepted path behavior and assertions. Do not modify the scalar-index
candidate or change its oracle. Do not change parser limits or public APIs.

## Candidate and ownership
Coordinator seeds an isolated copy of the current Rust workspace and records
SHA-256 values for the path parser, reader, and existing path tests. Candidate
only until review and exact main-tree integration. Worker may edit only:
- `rust/crates/vexel-svg-path/src/parser.rs`
- `rust/crates/vexel-svg-path/src/reader.rs`
- `rust/crates/vexel-svg-path/tests/**` (focused regression only)
- `artifacts/WO-RUST-SVG-PATH-NUMBER-SEPARATOR-01/**` except frozen seed

No changes to peer crates, manifests/lockfiles, shared code, legacy source,
fixtures, original oracles, STATE, docs, work orders, or ledgers. Keep source
files below 800 lines. No new dependency, unsafe code, network, GUI or broad
UI/E2E suite. Coordinator alone performs shared integration and ledger updates.

## Verification
Run from the candidate workspace root and record. Set `CARGO_HOME` to the
absolute candidate `.cargo-home` path and `CARGO_TARGET_DIR` to a candidate
local target directory; do not derive either from a nested package directory.
The candidate must include the complete root `work/` directory so the copied
ledger can resolve all frozen orders. Run:
- `cargo +stable check -p vexel-svg-path --locked --offline`
- `cargo +stable test -p vexel-svg-path --locked --offline`
- `rustfmt --edition 2024 --check rust/crates/vexel-svg-path/src/reader.rs rust/crates/vexel-svg-path/tests/controlled.rs`
- `python3 scripts/wo_validate.py WO-RUST-SVG-PATH-NUMBER-SEPARATOR-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`

Report exact hashes, commands, exit codes, targeted test behavior and changed
paths. The malformed source must fail via each parser entry point with the
expected absolute diagnostic; valid compact grammar must stay successful.
Candidate result is not integrated or product-complete until independent
review, scope/hash audit and fresh named main-tree checks pass.
