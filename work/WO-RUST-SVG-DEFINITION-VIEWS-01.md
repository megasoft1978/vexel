---
id: WO-RUST-SVG-DEFINITION-VIEWS-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 35f53a26cb26704c237528c4e93cfbbbef1a355d6ef255ba16cc89485c58e9ac
parallel_safe: false
network: false
---
# Use active traversal views in SVG definition planning

## Goal
Fix the observed `InvalidArgument` in real decoder clip planning. During
incremental decode the scene is not finalized; definition planners must query
its active traversal view, not the finalized-only `SceneBuilder::scene` API.

## Required behavior
1. In `vexel-svg-definitions`, replace planner-time finalized-scene reads with
   `traversal_scene` reads in the owned production planner and expansion helper.
   Preserve all calculations and public contracts.
2. In the isolated engine integration candidate, update the actual Decoder clip
   test to require successful completion, finish, and a published clip attached
   to its owner shape. Remove the prior expectation that the candidate returns
   `InvalidArgument`.
3. Keep peer crates, decoder source, manifests/lockfiles, fixtures, and tests
   outside the explicitly allowed paths unchanged. Add no dependencies or unsafe
   code. Keep Rust source files below 800 lines.
4. If the clip path succeeds, record the concrete decoder events and resulting
   scene predicates. Do not claim full SVG parity or full decoder acceptance.

## Candidate and ownership
Coordinator prepares a fresh isolated workspace from current accepted Rust
sources, plus the terminal decoder and engine adapter candidates. The candidate
contains the exact work order and a hash manifest. Worker may edit only:
- `rust/crates/vexel-svg-definitions/src/planner.rs`
- `rust/crates/vexel-svg-definitions/src/planner_support.rs`
- `rust/crates/vexel-svg-engine/tests/decoder.rs`
- `artifacts/WO-RUST-SVG-DEFINITION-VIEWS-01/**` except frozen seed

Coordinator alone may register candidate workspace members/lock entries and
later integrate an accepted exact patch into main. No edits to the terminal
`vexel-svg-decode` candidate, other peer crates, or user source files.

## Verification
Run locked/offline focused checks in the candidate:
- `cargo +stable check -p vexel-svg-definitions --locked --offline`
- `cargo +stable test -p vexel-svg-definitions --locked --offline`
- `cargo +stable check -p vexel-svg-engine --locked --offline`
- `cargo +stable test -p vexel-svg-engine --locked --offline`
- `cargo +stable test -p vexel-svg-decode --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-svg-definitions --package vexel-svg-engine -- --check`
- `python3 scripts/wo_validate.py WO-RUST-SVG-DEFINITION-VIEWS-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`

Record each command and exit code, changed paths, exact source hashes, and
immutable-input audit. Coordinator integrates only after candidate acceptance,
then runs fresh main-tree definition tests and focused engine/decoder checks.
No GUI, broad UI/E2E, browser, benchmark, or release test.

## Output
Preserve the full product goal. Keep the worktree uncommitted and report the
precise clip route outcome and any remaining decoder/composition acceptance
blockers.
