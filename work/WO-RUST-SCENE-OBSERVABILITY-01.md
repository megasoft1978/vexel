---
id: WO-RUST-SCENE-OBSERVABILITY-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 8335e3a0a8d7297750e1e824e7ec1445f5834e25adac895559496ab0822c2f5d
parallel_safe: false
network: false
---
# Actual SVG decoder reference and allocation observability

Add narrowly scoped, production-backed observability required by the frozen
SVG decoder parity predicates. This is a shared Scene API prerequisite; it does
not claim decoder, renderer, or product acceptance.

## Required behavior

1. Add `DecodeSource::reference_lookup_probes()` forwarding the actual
   `ReferenceIndex::lookup_probes()` counter. It must report only probes the
   owned index really performed; do not build another index or count fixture
   calls. Prove a `DecodeSource::find_reference` hit and miss change the same
   observable counter consistently with direct index semantics.
2. Add bounded, checked, cumulative allocation-history observability by
   `AllocationTag` to `ChargeLedger` without adding fields to the public
   `RenderMemoryStats` struct. Document semantics against its existing live
   byte counters. History records admitted nonzero bytes associated with the
   tag and bytes actually reserved by `allocate`; it remains monotonic across
   release, rollback, cancellation after allocation, and buffer replacement.
   Failed admission and failed physical allocation must not fabricate payload
   history. Real retained lease-registry growth is attributed to `None`, not
   to the caller's payload tag. Preserve existing total-history and error
   priority behavior; check all arithmetic before mutating observable state.
3. Prove observing history/counters does not change decode results, allocations,
   limits, cancellation, or source bytes. Keep public struct literals and
   compile-fail ownership contracts unchanged. No unsafe code, dependency, or
   unbounded event log; fixed per-tag counters are sufficient.

## Focused proof

- Add `rust/crates/vexel-scene/tests/observability_contracts.rs` for real
  reference lookup probe deltas; admitted/released/reused leases; allocation
  through `allocate_buffer`; successful replacement; reserve failure and
  cancellation before versus after physical allocation; checked overflow; and
  history independence from release/rollback. Verify `None` registry growth
  and caller tags remain separately accounted.
- Keep tests deterministic and bounded. Do not alter frozen existing tests.
- Keep each changed Rust source/test under 800 lines.

## Allowed paths

- `rust/crates/vexel-scene/src/accounting.rs`
- `rust/crates/vexel-scene/src/source_context.rs`
- `rust/crates/vexel-scene/tests/observability_contracts.rs` (new)
- `rust/crates/vexel-scene/README.md`
- `artifacts/WO-RUST-SCENE-OBSERVABILITY-01/**`
- Coordinator only: this order's frozen metadata, `STATE.md`, `LEDGER.md`,
  and `ledger.jsonl`.

All other files, accepted APIs/tests, source oracles, manifests/lock, prior
orders/evidence, HEAD and index are protected. Preserve all existing dirty and
untracked work. One isolated Scene writer only; coordinator alone integrates.
No staging/commit/reset/delete, GUI, broad UI/E2E, unsupported OS, network,
dependency, or GPL editor source. Use the existing offline Cargo cache and
subscription allowance only.

## Verification and acceptance

- `CARGO_HOME=<repo-local-cache> cargo +stable check --manifest-path Cargo.toml -p vexel-scene --locked --offline`
- `CARGO_HOME=<repo-local-cache> cargo +stable test --manifest-path Cargo.toml -p vexel-scene --locked --offline`
- `CARGO_HOME=<repo-local-cache> cargo +stable fmt --manifest-path Cargo.toml --package vexel-scene -- --check`
- `python3 scripts/wo_validate.py WO-RUST-SCENE-OBSERVABILITY-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

Record exact command logs, input/protected hashes, source line counts, and
semantics in this order's evidence directory. Candidate remains isolated until
an independent review accepts it; then coordinator checks exact scope and
protected hashes, integrates the allowed files, and reruns fresh main-tree
package commands. The next decoder parity harness consumes this accepted API.
No full decoder/renderer/app/UI, performance, live MCP, release, or demo claim.
