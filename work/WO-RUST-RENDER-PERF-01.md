---
id: WO-RUST-RENDER-PERF-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 6e072ef8a00bee2526acf20ce858f4f8f354b631
spec_sha256: b30242ad3c1a1e2852812717ad29ea40aa184f8688493fa8794d9e51ffce0a7e
parallel_safe: true
network: false
---
# Renderer performance for large scenes, byte-exact

## Goal
Make rasterising many-shape scenes faster without changing any output byte.
Baseline: `compose_2000_rects_512` takes 3178 ms (debug, rust/ci/perf-baseline.json);
corpus `generated-use_t1` (986 KB) takes 6.7 s and `generated-giant_path_t1`
21.7 s in release. Port only techniques that fit the Rust design from the legacy
accel grid/scanline index (`git show legacy-c-final:src/raster/shape_render_stroke_grid.c`
and neighbours in `src/raster/`): active-edge/scanline bucketing and per-shape
bounding-box culling against the tile/canvas.

## Dependencies
Read first: `vexel-raster`, `vexel-render`, `vexel-spatial`, the golden/parity
tests that cover them, `vexel-app/tests/perf_gate.rs`, `rust/ci/perf-baseline.json`.

## Assumptions
- Profile before changing: `result.md` names the measured hot spots (sampling or
  timing counters via a test, no external profiler install) and only those are
  changed.
- Determinism is exact: same coverage arithmetic and accumulation order per
  pixel; optimisations only skip work that provably cannot touch a pixel.
- If exactness cannot be kept for a technique, drop that technique and report.

## Scope
1. Bounding-box and scanline-range culling of edges/shapes outside the target
   region; active-edge handling that avoids rescanning all edges per row.
2. A reusable bench/test in `vexel-raster` or `vexel-render` tests that builds
   the 2000-rect scene and a 100k-segment single-path scene and records ms.
3. Baseline update per the rule in `perf-baseline.json` (`max_ms = max(250,
   8 x baseline_ms)`, rounded up): only the `compose_2000_rects_512` entry may
   change, in `rust/ci/perf-baseline.json`, with the new measured baseline_ms on
   the same host string; the test never rewrites the file. Only tighten.

## Non-goals
New features, SIMD/unsafe/threads, public API changes, tile-cache changes,
decode changes, changing goldens or oracles, corpus-tool changes.

## Files allowed to touch
- `rust/crates/vexel-raster/src/**`, `rust/crates/vexel-raster/tests/**`
- `rust/crates/vexel-render/src/**`, `rust/crates/vexel-render/tests/**`
- `rust/crates/vexel-spatial/src/**`, `rust/crates/vexel-spatial/tests/**`
- `rust/ci/perf-baseline.json` (Scope 3 only)
- `artifacts/WO-RUST-RENDER-PERF-01/**`

## Protected paths
Workspace/lock manifests, `vexel-app` (including `tests/perf_gate.rs`),
`vexel-svg-*`, `vexel-scene`, `vexel-corpus`, other crates, C/ObjC, `tests/`,
all goldens/oracles/fixtures, `docs/**`, `STATE.md`, `LEDGER*.md`,
`ledger.jsonl`, other work orders, git HEAD/index. No unsafe, new external deps,
network, installs, GUI launch, commit, stage, reset, deletion. Files under 800 lines.

## Acceptance criteria
1. Every existing test and golden/parity test in `vexel-raster`, `vexel-render`,
   `vexel-spatial`, `vexel-svg-parity` and `vexel-app` passes unchanged with no
   golden or expectation edited (git diff shows none); counts may only increase.
2. A new differential test renders >=3 scenes (2000 rects, overlapping
   strokes/clips, a large path) through the optimised path and an in-test naive
   reference path, asserting byte equality.
3. `compose_2000_rects_512` measured baseline is at least 1.5x faster than
   3178.29 ms (debug, same host class) over 5 runs (median reported), or the
   order stops with the measured ceiling and profile; `perf_gate` passes against
   the updated file.
4. The large-path case improves measurably (before/after ms recorded); no case
   gets slower by more than 5 % (listed in `result.md`).
5. Peak memory of the 2000-rect case does not grow (RSS from `/usr/bin/time -l`
   before/after recorded).
6. `cargo tree` for the touched crates shows no new external package; no `unsafe`.
7. `result.md` (<=400 words): hot spots found, techniques kept/dropped and why,
   before/after table, what was not ported from the legacy grid.

If an AC cannot be met, stop with the exact blocker; no test weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check --workspace --locked --offline`
- `cargo +stable test -p vexel-raster -p vexel-render -p vexel-spatial -p vexel-svg-parity --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-raster --package vexel-render --package vexel-spatial -- --check`
- `python3 scripts/wo_validate.py WO-RUST-RENDER-PERF-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Evidence
`artifacts/WO-RUST-RENDER-PERF-01/`: `result.md`, `verification.json`, `ac-map.json`,
`changed-path-audit.json`.

## Worker and review
Worker: gpt-6-luna High, Standard speed, one writer, existing allowance only; stop
on quota errors. Candidate only until a fresh read-only reviewer (separate
session, Luna or Sol Medium, diff + order only) reruns the frozen commands and
checks each AC. The coordinator integrates the exact patch, reruns main-tree
checks, then updates STATE.md, LEDGER.md and ledger.jsonl.
