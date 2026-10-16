---
id: WO-RUST-V1-RENDER-DASH-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 1d077a5539d44b64586b5416ccf7dab99f9f58f9
spec_sha256: a3ed4c89ce089976c4a20cc743db84b2bd298a2840950361879a9a1a13cef4ad
parallel_safe: true
network: false
---
# stroke-dasharray and stroke-dashoffset in vexel-render

## Goal
Render dashed strokes in `vexel-render` with pixel parity against the legacy C
renderer, so a document with `stroke-dasharray`/`stroke-dashoffset` no longer
fails with `UnsupportedKind::Dash`. Closes the "dashes" item of HANDOFF.md
"Known gaps" at the renderer level. Wiring into `vexel-app` is a follow-up the
coordinator schedules (see Non-goals).

## Dependencies
Read first: `HANDOFF.md`, `rust/crates/vexel-render/{src,tests}` (especially
`stroke.rs`, `adapter.rs`, `tests/parity.rs`, `tests/common/parity_data.rs`,
`tests/golden/manifest.json`), the legacy C sources
`git show legacy-c-final:src/raster/shape_render_dash_cache.c`,
`git show legacy-c-final:src/raster/shape_render_stroke.c` and
`...shape_render_stroke_grid.c` (copies of the last two are in
`rust/fixtures/legacy/c-raster/`), and, read-only for reference,
`rust/crates/vexel-svg-scalar-sample/src/{dash,job_dash}.rs` and
`vexel-scene` (`Stroke.dash_values`, `shape_dashes`). Provenance: ported from
project-owned C, so no external algorithm is involved.

## Assumptions
- The decoder subset already parses `stroke-dasharray`/`stroke-dashoffset` into
  `vexel_scene::Stroke` (`dash_values`, `dash_offset`) via
  `vexel-svg-decode/src/style.rs`. Verify with a one-line probe before editing;
  the minimal allowed decoder change is: only if a failing test proves a dash
  attribute is dropped, edit `vexel-svg-decode/src/style.rs` (and nothing else
  there) to pass it through, listing the proof in the audit. Expected: none.
- Existing public items stay source- and behaviour-compatible: callers in
  `vexel-app` (`compose/shapes.rs`, `animation/frame.rs`) construct
  `StrokeStyle`/`RenderShape` literals and call `from_scene_shape`, which must
  keep returning `Unsupported(Dash)` for dashed scene strokes. Dashing is
  therefore additive: new `DashedShape<'a> { shape: RenderShape<'a>, dashes:
  &'a [f64] }`, `render_dashed_shapes(...)` (same contract as `render_shapes`)
  and `from_scene_shape_with_dashes(..., dashes: &[f64])`. An empty `dashes`
  slice must render byte-identically to `render_shapes`.
- Dash semantics follow the C code: odd counts are repeated, any non-finite,
  negative or all-zero pattern means solid, the offset is taken modulo the
  period, dashes restart per subpath, closed subpaths join the seam when the
  pattern is "on" at both ends (`dash_range_joins_seam`), and caps apply to every
  dash. Bounds (max dash values 16, max segments per shape) come from the C
  constants and are enforced with `RenderError::LimitExceeded`.

## Scope
1. Dash expansion in `vexel-render/src/dash.rs` (new, <= 800 lines), consumed by
   `stroke.rs`/`render.rs` through the new entry points only.
2. Adapter `from_scene_shape_with_dashes` and the `DashedShape` type exported
   from `lib.rs`.
3. Fixtures and goldens: `rust/fixtures/render-dash/` with the SVGs, the C-made
   PPMs and `manifest.json` (fixture id, source sha256, ppm sha256, size,
   legacy tag/commit, exact command). SVGs: the four legacy fixtures
   `tests/svg/shape/fixtures/{stroke-dashes,stroke-dash-join,
   stroke-dash-closed-seam}.svg` and `tests/realworld/fixtures/special/
   dashed-cubic.svg` (via `git show legacy-c-final:<path>`), plus one new
   project-authored SVG with a round-cap dot pattern (`stroke-dasharray="0 6"`),
   a negative offset and an odd-length array on a circle and a rect.
4. Golden generation from the frozen legacy binary, outside the repo, in
   `$TMPDIR`: `mkdir legacy && git archive legacy-c-final | tar -x -C legacy`;
   `cd legacy && cmake --preset mac-debug && cmake --build --preset mac-debug
   --target svgworkbench`; per fixture
   `build/mac-debug/bin/svgworkbench --render-animation <fixture.svg> --fps 1
   --duration 1 --out <dir>` and take `frame-0000.ppm`, composited over white
   exactly as the existing manifest records. Record `git rev-parse
   legacy-c-final` and the cmake/compiler versions in the manifest. If the legacy
   build fails, stop and report; never hand-edit or Rust-generate a golden.
5. Parity test `tests/dash_parity.rs` plus `tests/common/dash_data.rs` in the
   same hand-transcribed shape-data style as `parity_data.rs`, with the
   tolerances of `parity.rs` copied unchanged (max channel delta 1, no pixel
   over 2, differing fraction <= 0.5%, exact beyond 1 px of any edge).

## Non-goals
Wiring dashes into `vexel-app` compose or animation, markers, dashed text,
`pathLength`, percentage dash lengths, changing existing goldens or tests,
GPU paths, performance work.

## Files allowed to touch
- `rust/crates/vexel-render/src/**`, `rust/crates/vexel-render/tests/**` (new
  files only, plus `common/mod.rs` if a helper is needed; existing tests and
  goldens unchanged)
- `rust/fixtures/render-dash/**` (new)
- `rust/crates/vexel-svg-decode/src/style.rs` only under the proof condition
  in Assumptions
- `artifacts/WO-RUST-V1-RENDER-DASH-01/**`

## Protected paths
Workspace/lock manifests and every `Cargo.toml`, `vexel-app`, `vexel-scene`,
`vexel-protocol`, all other crates, existing goldens and `tests/golden/**`,
`rust/fixtures/legacy/**`, C/ObjC, `tests/`, oracles, `docs/**`, `STATE.md`,
`LEDGER*.md`, `ledger.jsonl`, other work orders, git HEAD/index. No unsafe, new
dependencies, network (the legacy tag is local), installs, GUI launch (the C
`--render-animation` path is headless; if it opens a window, stop), commit,
stage, reset or deletion. Temp worktree files stay outside the repository.

## Acceptance criteria
1. Each of the five fixtures renders with `render_dashed_shapes` within the
   tolerances above against its C golden; the report prints max delta,
   differing count and outside-dilation count per fixture.
2. Unit tests for the expander: pattern repeat for odd counts, offset modulo
   (positive, negative, larger than period), zero and negative entries mean
   solid, subpath restart, seam join on a closed rect, a dot (zero-length "on")
   with round cap draws, butt cap draws nothing, 17 values returns
   `LimitExceeded`, NaN returns solid.
3. `render_dashed_shapes` with empty dashes is byte-identical to `render_shapes`
   on every existing parity case (asserted by test).
4. `from_scene_shape` still returns `Unsupported(Dash)`; the existing
   `errors.rs` assertion at that line is unchanged and passes.
5. All pre-existing `vexel-render` tests pass unmodified, counts only increase;
   goldens untouched (`git diff --stat` shows no changed existing golden).
6. `manifest.json` records tag commit, build commands, binary sha256 and per-file
   hashes; a test verifies each recorded `ppm_sha256`.
7. `cargo tree -p vexel-render` shows no new package; no `unsafe`; files < 800
   lines.
8. `result.md` (<=300 words) states the decoder-change decision, the C constants
   mirrored, and that app wiring and live use are unexecuted.

If an AC cannot be met, stop with the exact blocker; no tolerance raising, no
test weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-render --locked --offline`
- `cargo +stable test -p vexel-render --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline` (callers still compile)
- `python3 scripts/wo_validate.py WO-RUST-V1-RENDER-DASH-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`
`cargo fmt --check` is not frozen here: `vexel-render` has pre-existing drift
(see WO-RUST-V1-RELEASE-CI-01); new files must still be rustfmt-clean
(`rustfmt --check --edition 2024 <file>`).

## Evidence
`artifacts/WO-RUST-V1-RENDER-DASH-01/`: `result.md`, `verification.json`,
`ac-map.json`, `changed-path-audit.json`, `legacy-build.log` (tail only).

## Worker and review
Worker: gpt-6-luna High, one writer, existing allowance only; stop on quota
errors. Candidate only until a fresh read-only reviewer reruns the frozen
commands and checks each AC. The coordinator integrates the patch and schedules
the `vexel-app` wiring order, then updates STATE.md, LEDGER.md and ledger.jsonl.
