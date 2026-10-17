---
id: WO-RUST-PIPELINE-PERF-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: d0ae379c12411e7f3e9965e637f00bc2d1acacef
spec_sha256: c17f6e941037ed294df9164c02b4557ddaebbec51bfcfd49838bd12ebc771d61
parallel_safe: false
network: false
---
# Pipeline speed: decode, scene validation and compose, byte-exact

## Goal
Remove the measured hot spots between "bytes" and "render()" so large documents
stop timing out. Output must not change by one byte. Profile (coordinator,
2026-10-17, release, 128x128, `compose_svg` harness over copies of
`compose/*` in /private/tmp; the macOS text provider was not linked, so text
documents were not measured):
- Render is not the cost. 8000 rects: lex 5 ms, session setup 2 ms, decode run
  1519 ms, render 11 ms. Louisiana (16 MB): lex 61 ms, decode run 68.0 s, render
  43 ms. 1 MB `cnc_t1` and `inkscape_t1` take 97 s and 143 s (60 s timeout).
- Cause: `SceneBuilder::commit_request` (`vexel-scene/src/request.rs:149`)
  re-runs `validate::scene` over the WHOLE scene so far on every request commit,
  so N shapes cost O(N^2): 2846 of 2920 sampled ms (97%) on 16000 rects. Two
  decode paths commit per shape: (a) `batch_root_shapes` (128 per request,
  `vexel-svg-decode/src/decode.rs:372`) is off for the whole document if any of
  `g defs use switch symbol clipPath pattern mask marker filter style`, an `href`
  attribute or `url(#` appears anywhere; every shape then commits itself;
  (b) in batched mode the whitespace `Task::Text` between elements is a
  `shape_request_boundary` (`traversal_boundary.rs`) and flushes the batch on
  every shape (`validate::scene` runs once per shape on a plain rect file).
- Ceiling probe (skip the per-request validation, scratch copy only):
  8000 rects 1.4 s -> 44 ms, 8000 paths 5.8 s -> 0.20 s, `cnc_t1` 97 s -> 0.15 s,
  `inkscape_t1` 143 s -> 0.15 s. Removing the `Task::Text` boundary arm alone
  made rects16000 177 ms but returned `Malformed(invalid value, offset 0)` on
  2000/8000 rects: a flush-order dependency exists, do not drop it blindly.

Top hot spots (self samples at 1 ms, 16000-rect file unless noted):
1. whole-scene revalidation per request commit (above): 97% of decode.
2. `validate::shape_attributes` rebuilding a `SceneView` per shape
   (`SceneData::view` 729 + shape_attributes 593 of 3297).
3. `validate::edge` 581 and `validate::transform` 352: per-element loops rerun
   on every commit.
4. `WorkControl::charge` chain (`JobWorkMeter` 290 + `CancelControl` 162 + cancel
   closure 75; on 8000 paths 628 + 547 + 239): dyn calls per charged unit,
   amplified by 1-3.
5. `path_topology::scene_segments` 489 ms and `validate::segment_valid` 253 on
   8000 paths (1406 samples on `cnc_t1`): per-shape binary search with a charge
   per probe, rerun on every commit.

## Dependencies
Read first: `vexel-scene/src/{request,validate,builder,decode_initialization}.rs`,
`vexel-svg-decode/src/{decode,traversal,traversal_boundary,requests,session}.rs`,
`vexel-app/src/compose/decode.rs`, `vexel-app/tests/perf_gate.rs`,
`rust/ci/perf-baseline.json`. `vexel-render` is finished and untouched.

## Assumptions
- Final acceptance stays strict: `validate::scene` still runs over the whole
  scene at `seal` and at decode finalization, unchanged. If `finalize_decode` /
  `freeze_decode` does not run it on the root, add exactly that call first so
  the incremental commit cannot weaken what is accepted.
- Only the commit-time check becomes incremental (shapes, edges, subpaths,
  segments, numbers appended since the request began; small tables such as
  clips, patterns, masks, filters stay whole). Error kind and offset for a bad
  scene must equal the base behaviour (asserted by differential tests).
- Cancellation stays at least as responsive (existing cancel tests pass).
- No new external crate, no `unsafe`, no output change, no test weakening.

## Scope
1. Benchmark first: `vexel-app/tests/pipeline_perf.rs` (new) builds the named
   synthetic scenes below, prints ms per phase with `VEXEL_PERF_PRINT=1`, and
   asserts the speedup ACs. Commit the failing measurement before the fix.
2. `vexel-scene`: incremental commit-time validation (watermarks taken at
   request begin); hoist the per-shape `SceneView` out of the loop.
3. `vexel-svg-decode`: stop the whitespace-only `Task::Text` from forcing a
   flush; enable root batching when boundary tags exist but no shape request is
   nested in them (or batch per scene), only where a differential test shows
   identical scenes. Smallest change that meets the ACs.
4. `vexel-app/src/compose`: cheaper charge/cancel path (single cancel poll per
   N units) only if the profile after steps 2-3 still shows it above 10%.
5. `rust/ci/perf-baseline.json`: update rules below.

## Non-goals
Raising any limit (WO-RUST-SCENE-LIMITS-01), renderer changes, streaming decode,
new SVG features, UI, multithreading, caching across documents.

## Files allowed to touch
- `rust/crates/vexel-app/src/compose/**`, `rust/crates/vexel-app/tests/**`,
  `rust/ci/perf-baseline.json`, `artifacts/WO-RUST-PIPELINE-PERF-01/**`
- PROTECTED crates, minimal edits only, each justified in the audit by a failing
  benchmark or differential test that this order adds first:
  - `rust/crates/vexel-scene/src/{validate,request,builder}.rs` and
    `rust/crates/vexel-scene/tests/**`
  - `rust/crates/vexel-svg-decode/src/{traversal_boundary,decode,requests}.rs`
    and `rust/crates/vexel-svg-decode/tests/**`
- `vexel-svg-engine/src/**` only if the post-fix profile names it above 10% and
  a benchmark test proves it (list in the audit).

## Protected paths
`vexel-render`, `vexel-raster`, every other crate, workspace/lock manifests,
`rust/tools/**`, goldens, oracles, `docs/**`, `STATE.md`, `LEDGER*.md`,
`ledger.jsonl`, other work orders, git HEAD/index, `rust/fixtures/**` (read
only), the external corpus (read only). No unsafe, new deps, network, commit,
stage, reset or deletion. Files under 800 lines. Not parallel with
WO-RUST-SCENE-LIMITS-01 (both edit `vexel-scene`).

## Acceptance criteria
Before any edit, build base_sha in release and record, in
`artifacts/WO-RUST-PIPELINE-PERF-01/before.json`: the frame SHA-256 and verdict
of every `*.svg` under `rust/fixtures/**` and `bench/**` composed at 128x128 and
512x512, plus the synthetic scenes below.
1. Byte identity: after the change every recorded hash and verdict is equal
   (zero differences). All existing golden, parity and fixture tests pass; test
   counts only increase.
2. Synthetic scenes (new bench, release; 512x512 viewbox, composed at 128x128,
   best of 3): R8000 = 8000 whitespace-separated rects at the root; P8000 = 8000
   stroked 4-segment paths; S8000 = R8000 plus a `<style>` block (disables
   batching); G8000 = 8000 rects inside one `<g>`. Base measured on this host:
   R8000 1.5 s, P8000 5.9 s; S8000 and G8000 measured by the worker. Required:
   R8000 <= 300 ms, P8000 <= 1200 ms, S8000 and G8000 <= 5x faster than base and
   <= 1500 ms. Scaling: R16000 time <= 2.6x R8000 time (no quadratic growth).
3. Differential: for at least 6 malformed-scene mutations (bad stroke width,
   clip index out of range, edge out of shape range, unordered segment, non-finite
   path number, overlapping subpath) the incremental commit rejects with the same
   `ErrorKind` and offset as base (expected values captured at base_sha and kept
   as constants); a scene built in several requests equals the one-shot scene.
4. Differential for decode batching: for R200, R200 + style, nested g, use,
   clipPath and pattern fixtures the frozen `SceneGraph` shape/edge counts and
   frame hash are identical with and without the batching change.
5. Sweep: `vexel-corpus run` (release, 128x128, 60 s, 2048 MiB, external
   manifest, same flags as WO-RUST-CORPUS-UNSUPPORTED-01) over the 12 files that
   timed out: at most 4 still `timeout`. Record per-file ms and phase split.
   If more remain, STOP with the measured ceiling and the next named hot spot
   (profile per remaining file); do not raise limits. No `ok` file regresses; no
   new `rss_exceeded` or `crash`; in-repo sweep verdicts unchanged.
6. perf-baseline.json: add `compose_4000_rects_512` (debug cargo test, same
   shape as the 2000 case) and update `compose_2000_rects_512` baseline_ms to
   the new best-of-three debug value; `max_ms = max(250, 8 x baseline_ms)`
   rounded up per the file's rule; values may only go down or be added; the test
   never rewrites the file; the coordinator confirms on the main tree.
7. Cancellation: existing cancel tests pass and a new test cancels R8000 within
   one poll interval of the cancel flag (report the interval).
8. `cargo tree -p vexel-app` shows no new package; no `unsafe`.
9. `result.md` (<=400 words): the five hot spots re-measured after the fix, every
   protected-crate edit with its proving test, before/after table, unexecuted
   items (live app, text documents needing the CoreText provider).

If an AC cannot be met, stop with the exact blocker; no test or threshold
weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-app -p vexel-scene -p vexel-svg-decode --locked --offline`
- `cargo +stable test -p vexel-app -p vexel-scene -p vexel-svg-decode -p vexel-svg-engine --locked --offline`
- `cargo +stable test -p vexel-app --release --test pipeline_perf --locked --offline`
- `cargo +stable test -p vexel-render --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app --package vexel-scene --package vexel-svg-decode -- --check`
- `cargo +stable run -p vexel-corpus --release --locked --offline -- run --manifest bench/corpus/manifest.json --pins rust/tools/vexel-corpus/pins.json --out artifacts/WO-RUST-PIPELINE-PERF-01/in-repo`
- the same `run` with `--manifest ~/Desktop/Freelance/vexel-corpus-real/manifest.json --out artifacts/WO-RUST-PIPELINE-PERF-01/external` (owner-local; if absent, stop and report)
- `python3 scripts/wo_validate.py WO-RUST-PIPELINE-PERF-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Evidence
`artifacts/WO-RUST-PIPELINE-PERF-01/`: `result.md`, `before.json`, `after.json`,
`verification.json`, `ac-map.json`, `changed-path-audit.json`, `profile.md`.

## Worker and review
Worker: gpt-6-luna High, Standard speed, one writer, existing allowance only; stop
on quota errors. Candidate only until a fresh read-only reviewer (separate
session, Luna or Sol Medium, diff + order only) reruns the frozen commands and
checks each AC, including the byte-identity hashes. The coordinator integrates
the exact patch, reruns main-tree checks and the sweep, then updates STATE.md,
LEDGER.md, ledger.jsonl and perf-baseline.json.
