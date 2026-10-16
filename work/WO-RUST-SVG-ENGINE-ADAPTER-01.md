---
id: WO-RUST-SVG-ENGINE-ADAPTER-01
revision: 2
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 1d206e37a684eaef1ddbf7121af66a2fa801994fbb250d067508d5b3ad487647
parallel_safe: false
network: false
---
# Production SVG decode composition adapter

Create a separately scoped Rust library that binds the resumable SVG Decoder to
accepted cached-paint, definition-planner, and Scene publication APIs. This is
the first production composition step for editable artwork; it is not full SVG
parity, renderer integration, app UI, or product acceptance.

## Candidate inputs

The coordinator creates `.cache/WO-RUST-SVG-ENGINE-ADAPTER-01/worker/` as a
fresh isolated workspace. It contains the current root Rust workspace/crates,
the terminal R8 decoder candidate overlaid at `rust/crates/vexel-svg-decode/`,
this exact work order, and a frozen SHA-256 source seed. Do not use or modify the
R8 decoder checkout. Peer crates, decoder, and workspace sources are immutable
worker inputs. Root `Cargo.toml`/`Cargo.lock` are not copied from later edits.
The candidate uses existing offline Cargo cache only; no network or new deps.

## Required behavior

1. Add a real `vexel-svg-engine` Rust library in
   `rust/crates/vexel-svg-engine/`. Worker edits only this crate and its new
   work-order artifacts. Coordinator may register this crate in the candidate
   workspace manifest/lock only.
2. Implement actual `vexel_scene::ChannelAwareProductionDecodeOperations`:
   - retain a `PaintCache` and delegate gradient resolution to
     `vexel_svg_paint::resolve_cached_fill`, then publish borrowed cache stops
     through the Scene cached-paint path;
   - retain and delegate definitions to the real
     `vexel_svg_definitions::DefinitionPlanner`, including explicit Fill and
     Stroke pattern channels;
   - route decoder text/image requests to an injected provider or a typed,
     source-offset `Unsupported` error. Never return success for unhandled media.
3. Expose bounded incremental execution over the actual Decoder, preserving
   source ownership, progress, cancellation/errors, pending pattern tickets and
   child results. Never treat pending pattern work as successful completion.
   Finish is allowed only after Decoder reports `Complete` and no pattern job
   remains.
4. Test the real Decoder with supported vector shapes, transforms, solid
   fills/strokes, and source preservation. Include a local-gradient case through
   the actual cached resolver. Exercise the actual definition planner where
   candidate contracts permit. If an immutable contract prevents a case, assert
   and report the exact blocker; fixture-only adapters cannot count as success.
5. No new/external dependencies, unsafe code, native bridge, unsupported OS
   implementation, or source/test file over 800 lines. Document public API and
   unsupported media behavior.

## Paths and ownership

Worker may edit only:
- `rust/crates/vexel-svg-engine/**`
- `artifacts/WO-RUST-SVG-ENGINE-ADAPTER-01/**` except frozen source seed

Coordinator only:
- candidate root `Cargo.toml` and `Cargo.lock` for workspace registration
- root work-order metadata, ledger, `STATE.md`, and `LEDGER.md`
- candidate root copies of this frozen work order and source seed

All other candidate sources are protected inputs. Main checkout sources,
manifests, lock, HEAD, index, fixtures, oracles, previous evidence and any
user-owned app state are protected. One worker, no nested workers. No broad UI,
GUI, app launch, E2E, demo/release, staging/commit, reset/delete, or network.

## Verification

Use the candidate-local existing Cargo cache, locked and offline:
- `cargo +stable check -p vexel-svg-engine --locked --offline`
- `cargo +stable test -p vexel-svg-engine --locked --offline`
- `cargo +stable check -p vexel-svg-decode --locked --offline`
- `cargo +stable test -p vexel-svg-decode --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-svg-engine -- --check`
- `python3 scripts/wo_validate.py WO-RUST-SVG-ENGINE-ADAPTER-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`

Record exact output, source hashes, source/test limits, dependencies, concrete
input/result predicates, remaining decoder contract gates, and changed-path
audit. This package is still a candidate until independently reviewed, exact
source seed audited, and fresh main-tree checks run after the decoder prerequisite
is accepted. Do not claim full application completion.
