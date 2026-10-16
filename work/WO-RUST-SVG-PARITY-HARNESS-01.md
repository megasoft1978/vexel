---
id: WO-RUST-SVG-PARITY-HARNESS-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 0a4b5ce91305e84e3f6d6d3796688ac9cfc9553842fa522741cb24f7140f51de
parallel_safe: false
network: false
---
# Production paint and clip parity harness

Create a small standalone Rust integration harness that exercises the accepted
production paint and definition-planning crates together against the frozen
reference predicates. This supplies cross-crate evidence for the incomplete
SVG decoder candidate; it does not accept that decoder or claim renderer/app
integration.

## Required proof

Add `rust/tools/vexel-svg-parity/` as a **nested, standalone workspace** with
its own pinned `Cargo.lock`. It must stay out of the root workspace and use only
path dependencies on already accepted local crates (`vexel-base`, geometry,
Scene, XML/CSS/reference, SVG paint, and SVG definitions). Do not add a root
workspace member or alter root manifests/lockfiles. The tool must run offline
and require no external or dev dependencies.

Retain the original C predicates and inputs from
`tests/svg/shape/test_reference.c` as Rust test names/comments and execute the
real accepted package APIs:

1. **Cached gradient scan:** build the exact 4,096-comment / 512-rectangle / two
   gradient-stop document, parse with accepted XML/Scene APIs, and resolve the
   fills through `vexel-svg-paint`'s real cached resolver. Assert 512 rectangle
   elements are tokenized, exactly two stops are published, actual `PaintCache`
   `token_visits` is at least 4,096 and at most
   `8 * (4,096 + 512 + 64) == 37,376`, and source bytes remain unchanged.
2. **Clip source and aggregate bounds:** use `DefinitionPlanner::plan` on actual
   accepted Scene/DecodeSource owners to prove a 65,536-byte clip child span
   succeeds and +1 fails; derive the exact wrapper/child aggregate whose eight
   declarations equal 512 KiB and prove eight succeed while nine fail. Assert
   exact typed errors/offset bounds and source preservation. Do not count a
   local arithmetic reimplementation as planner proof.
3. Tests must be deterministic and bounded. Keep each Rust source/test below
   800 lines. Document command usage and the tested package versions/paths in
   the harness README.

Keep whole-decoder three-input allocation/output, one-versus-sixteen resolved
group clip history, renderer-produced 256-byte RGBA output, and tile-cache
rollover/LRU as explicit later gates. This harness must not synthesize them or
claim `WO-RUST-SVG-DECODE-01` complete.

## Allowed paths

- `rust/tools/vexel-svg-parity/**` (new standalone harness only)
- `artifacts/WO-RUST-SVG-PARITY-HARNESS-01/**`
- Coordinator only: this work order's frozen metadata, `STATE.md`, `LEDGER.md`,
  and `ledger.jsonl`.

All other code, existing tests/oracles, root Cargo manifests/lock, accepted
crates, previous orders/evidence, `HEAD` and index are protected. Preserve all
dirty/untracked content. One isolated harness writer; coordinator alone owns
shared integration and records. No C edits, unsafe code, new/external
dependencies, network, package installs, unsupported OS, GPL source, stage,
commit, reset, delete, GUI or broad UI/E2E. Use existing subscription
allowance only.

## Verification

- `CARGO_HOME=<repo-local-cache> CARGO_TARGET_DIR=<repo-local-target> cargo +stable check --manifest-path rust/tools/vexel-svg-parity/Cargo.toml --locked --offline`
- `CARGO_HOME=<repo-local-cache> CARGO_TARGET_DIR=<repo-local-target> cargo +stable test --manifest-path rust/tools/vexel-svg-parity/Cargo.toml --locked --offline`
- `cargo +stable fmt --manifest-path rust/tools/vexel-svg-parity/Cargo.toml -- --check`
- `python3 scripts/wo_validate.py WO-RUST-SVG-PARITY-HARNESS-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

Record exact inputs and accepted peer hashes, commands/logs, expected and actual
predicate values, errors/offsets, changed paths and source lengths. Run the
work-order commands in isolation. An independent review and exact protected
path audit are required before considering the harness accepted. No decoder,
renderer, native UI, product, performance, MCP, release or demo completion
claim follows from these component parity tests.
