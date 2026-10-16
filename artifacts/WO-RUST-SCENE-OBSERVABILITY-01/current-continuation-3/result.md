# Continuation 3 repair result — isolated candidate only

Status: repair implemented and focused checks pass. Independent review and coordinator integration remain required; no work-order, decoder, renderer, application or product acceptance is claimed.

Changed in this continuation: `rust/crates/vexel-scene/src/accounting.rs`, `rust/crates/vexel-scene/tests/observability_contracts.rs`, and the Scene README. `source_context.rs` retains the reviewed candidate implementation. No root/main writes, ledger/state edits, manifests, dependencies, other modules, nested agents, index or HEAD changes were made.

Registry growth now adds checked `new - old` to cumulative None history. Live None bytes still use `-old + new`; total allocation history still adds full actual replacement bytes. Caller None payload remains additive. Before reserve can transfer entries, the arithmetic proof checks None history plus any caller None payload against the largest integral Entry capacity allowed by the existing physical overlap/history/live headroom, minus old capacity. The reserve helper rejects excess actual capacity before moving entries, so every possible committed delta is representable. This deliberately conservative bound may reject when only the requested capacity would fit. None history is no longer converted into reserve-budget headroom; exhaustion returns Arithmetic through the existing overflow-flag wrapper. Lease limit and insufficient physical overlap errors retain their previous priority.

Tests now exercise second registry growth from its actual first capacity, exact net history and live deltas, full replacement total-history semantics, caller None aggregation, release, free-slot reuse, initial and second-growth arithmetic exhaustion for None and Output callers, unchanged complete ledger state and registry pointer on failure, exact proof headroom, and Nodes/ReservedBytes priority. The existing cumulative payload/count/serial overflow and nonempty replacement cancellation proofs remain and pass.

Validation was headless/CLI in the existing candidate worktree, using the existing repo-local Cargo cache:

- Offline locked `cargo +stable check --manifest-path Cargo.toml -p vexel-scene`: exit 0 (`check.log`).
- Offline locked `cargo +stable test --manifest-path Cargo.toml -p vexel-scene`: exit 0 (`test.log`). Exact test-result-line aggregate: 246 passed, 0 failed, 0 ignored, 0 measured, 0 filtered out across 20 result blocks. Includes 17 unit tests, 9 observability integration tests and 10 doctests; one existing target reports zero tests. These are reported Cargo outcomes, not fixture or product acceptance counts.
- Direct changed-file Rust formatting check: exit 0 (`changed-file-rustfmt.log`). Full package formatting was not rerun, as requested; prior review records protected pre-existing formatting failures.
- Frozen work-order ready validator: exit 0 (`ready.log`).
- Ledger validation: exit 0, 1,696 entries (`ledger.log`); ledger not edited.
- `git diff --check`: exit 2, solely `STATE.md:1341: new blank line at EOF` (`diff-check.log`). This pre-existing protected defect remains unfixed.

Rust line counts: accounting 608, source_context 331, observability_contracts 275; each is below 800. `input-hashes.json` captures continuation-start repository file hashes excluding artifacts; `scope-audit.json` verifies only the three named allowed files changed from those inputs, with no protected file changes. `seed-hash-audit.json` checks all 64 original seed entries against candidate and main: all main hashes still match the seed. `verification.json` records current hashes, exact commands/statuses, counts and review/work-order hashes. Earlier evidence was not rewritten.

Remaining limitations: the protected diff-check failure above, the previously recorded full-package formatting failures (not retested), and independent review/coordinator acceptance and fresh main-tree checks remain outstanding. No GUI/runtime, full decoder or performance checks were run or claimed.
