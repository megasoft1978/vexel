---
id: WO-RUST-TILE-CACHE-ORACLE-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 8740e3596bdc569a3a0ea589d7b7535708ef522806829188274122f0a359659c
parallel_safe: true
network: false
---
# Exact tile-cache C-oracle follow-up

Close the exact assigned `tests/raster/cache/cache_contract.c` parity gap in
the accepted `vexel-tile-cache` crate. The immutable catalog
`artifacts/WO-RUST-TILE-CACHE-ORACLE-01/scope.json` records all 29 original
`CHECK` assertions and the source SHA-256. Preserve the original test and all
accepted Rust cache behavior. Add focused Rust tests for the full source contract:
invalid and oversized caps, exact 8192/500 rejection and 1024/2 creation,
17x17 rejected-tile caller ownership, 1x1 insertion and ownership transfer,
deterministic LRU touch/eviction, UINT64 clock rollover order, generation and
scale invalidation, 10,000 successful insertions with every iteration's
ownership and bounded stats, and an equivalent deterministic JSON summary.

## Ownership and paths

Worker may edit only `rust/crates/vexel-tile-cache/src/**`,
`rust/crates/vexel-tile-cache/tests/**`, and
`artifacts/WO-RUST-TILE-CACHE-ORACLE-01/**` except frozen `scope.json`.
Coordinator alone changes this order, its ledger, `STATE.md`, `LEDGER.md`, and
the product plan. One isolated writer; no nested workers. Seed the current
accepted Rust workspace and crate into `.cache/WO-RUST-TILE-CACHE-ORACLE-01/worker/`;
keep build output candidate-local and use the existing repository offline Cargo
cache. No manifest/lock/config changes, dependencies, network, unsafe code,
legacy-source edits, broad UI/E2E, performance campaigns, staging or commits.
Do not weaken the original 29 assertions or claim renderer/product completion.

Production fixes are allowed only when an exact new regression proves a cache
contract mismatch; keep them within this crate and preserve all accepted tests.
On an unmet contract, stop with exact source and evidence rather than substituting
different cache semantics. Source and test files must remain below 800 lines.

## Verification

Run exact commands in the isolated candidate with `--locked --offline`:

- `cargo +stable check -p vexel-tile-cache --locked --offline`
- `cargo +stable test -p vexel-tile-cache --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-tile-cache -- --check`
- `python3 scripts/wo_validate.py WO-RUST-TILE-CACHE-ORACLE-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`

Record raw outputs, hashes, all 29 assertion mappings and outcomes, expected
and actual JSON summary, changed-path audit, source/test line counts, and any
failures. Candidate acceptance still requires independent review, exact
protected-path audit and fresh main-tree package checks. The full decoder,
renderer, app, UI and release gates remain separate.
