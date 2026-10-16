---
id: WO-RUST-SVG-PATTERN-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 842d6ce0373b6da3adf9799a6783da01d326efb5
spec_sha256: 0d835695c20c03fa42a38cb2d9a5c4d8e1facbfee931dd6c09ff621c55c21740
parallel_safe: true
network: false
---
# pattern paint rendering in vexel-render
## Goal
Render `fill`/`stroke` `url(#pattern)` in `vexel-render` with pixel parity
against the legacy C renderer, so a pattern-painted shape no longer fails with
`UnsupportedKind::PatternPaint`. Today a referenced pattern suspends the decoder
with `NeedPattern` and no renderer completes it. Umbrella: opening documents that contain this feature in `vexel-app` without the
deferral error is NOT part of this order. It is the separate follow-up
`WO-RUST-SVG-WIRE-01` (depends on MASK, PATTERN, FILTER and TEXT).

## Dependencies
Read first: `HANDOFF.md`, `artifacts/WO-RUST-V1-DECODE-SUBSET-01/deferred-map.json`,
`vexel-render/{src,tests}` (`adapter.rs` `classify`, `render.rs`),
`vexel-scene/src/{pattern_ticket,pattern_publication,static_pattern}.rs` and
`vexel-svg-decode/src/session.rs` (how `NeedPattern` is raised and completed),
`vexel-svg-definitions/src/pattern.rs`, the legacy C
`git show legacy-c-final:src/svg/shape/pattern.c` and
`...:src/raster/shape_render.h` (pattern tile entry points).

## Assumptions
- The decoder already hands a `PatternTicket`/`PatternDescriptor` with the
  mapped inverse (`PatternMapping::pixel_inverse`) and child shapes; this order
  only adds the renderer side: rasterize one tile of the child shapes with the
  existing scalar rasterizer, then sample it with the inverse transform.
- Tile sampling, `patternUnits`, `patternContentUnits`, `viewBox`,
  `patternTransform`, tile-edge rounding and the sampling filter follow the C
  code. C bounds are mirrored: `SHAPE_PATTERN_MAX_INSTANCES` 1024,
  `SHAPE_PATTERN_MAX_PIXELS` 1048576, `SHAPE_PATTERN_MAX_AXIS` 4096, each failing
  with `LimitExceeded`.
- Existing public items stay compatible: `from_scene_shape` keeps returning
  `Unsupported(PatternPaint)`; pattern support is additive
  (`PatternPaintShape`, `render_pattern_shapes`, `from_scene_shape_with_pattern`).
- Completing a suspended `NeedPattern` request from the app is done in
  WO-RUST-SVG-WIRE-01. This order must not edit the decoder or scene; if the
  renderer needs data the scene does not expose, stop and report the gap.

## Scope
1. `vexel-render/src/pattern.rs` (new, <= 800 lines): tile rasterization into
   bounded scratch, tile sampling for fill and stroke paint channels, nested
   solid-colour children, tile-count and pixel limits, per-row cancellation.
2. Additive adapter and entry points above, exported from `lib.rs`.
3. Fixtures `rust/fixtures/render-pattern/` with C goldens: the legacy pattern
   fixtures under `tests/svg/shape/fixtures` (list them with `git ls-tree`;
   take those with solid children only), a `patternUnits=userSpaceOnUse` stripe,
   an `objectBoundingBox` dot grid, one with `patternTransform` rotate and one
   used as `stroke`. Golden generation from the frozen legacy binary, outside the repo, in `$TMPDIR`:
`mkdir legacy && git archive legacy-c-final | tar -x -C legacy`; `cd legacy &&
cmake --preset mac-debug && cmake --build --preset mac-debug --target
svgworkbench`; per fixture `build/mac-debug/bin/svgworkbench --render-animation
<fixture.svg> --fps 1 --duration 1 --out <dir>`, take `frame-0000.ppm`, composited
over white exactly as `vexel-render/tests/golden/manifest.json` records. Record
`git rev-parse legacy-c-final`, cmake/compiler/macOS versions and the binary
sha256 in the fixture `manifest.json` (fixture id, source sha256, ppm sha256,
size, exact command). If the legacy build or the headless render fails (a window
opens, a missing feature), stop and report; never hand-edit or Rust-generate a
golden.
4. Parity test `tests/pattern_parity.rs` + `tests/common/pattern_data.rs`, tolerances copied unchanged from `vexel-render/tests/parity.rs` (max channel
delta 1, no pixel over 2, differing fraction <= 0.5%, exact beyond 1 px of any
edge).

## Non-goals
Decoder or scene changes, `vexel-app` wiring, gradients or images inside
patterns, pattern animation, `href` chains beyond the fixtures, performance,
GPU paths.

## Files allowed to touch
- `rust/crates/vexel-render/src/**`, `rust/crates/vexel-render/tests/**` (new
  files only, existing tests and goldens unchanged; `lib.rs` additive lines
  only)
- `rust/fixtures/render-pattern/**` (new)
- `artifacts/WO-RUST-SVG-PATTERN-01/**`

## Protected paths
Workspace/lock manifests and every `Cargo.toml` (coordinator only),
`vexel-app`, `vexel-protocol`, all crates not listed above, existing goldens and
`tests/golden/**`, `rust/fixtures/legacy/**`, `rust/fixtures/v1/**`, C/ObjC,
`tests/`, oracles, `docs/**`, `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other
work orders, git HEAD/index. No new dependencies, network (the legacy tag is
local), installs, GUI launch, commit, stage, reset or deletion. Temp worktree
files stay outside the repository. Files under 800 lines. Also protected: `vexel-scene`, `vexel-svg-decode`, `vexel-svg-engine`,
`vexel-svg-definitions`.

## Acceptance criteria
1. Each fixture renders with `render_pattern_shapes` within the tolerances
   against its C golden; the report prints max delta, differing count and
   outside-dilation count per fixture.
2. Unit tests: tile boundary alignment at integer and fractional tile sizes,
   `patternTransform` rotate/scale, objectBoundingBox vs userSpaceOnUse, zero or
   negative tile size paints nothing, 1025 instances, over-axis and over-pixel
   tiles return `LimitExceeded`, cancellation returns `Cancelled`.
3. A shape with no pattern renders byte-identically through
   `render_pattern_shapes` and `render_shapes` on every existing parity case.
4. `from_scene_shape` still returns `Unsupported(PatternPaint)` (existing
   `errors.rs` assertion unchanged and passing).
5. All pre-existing `vexel-render` tests pass unmodified; no existing golden
   changes.
6. Fixture `manifest.json` records tag commit, build commands, binary sha256 and
   per-file hashes; a test verifies each `ppm_sha256`.
7. `cargo tree -p vexel-render` shows no new package; no `unsafe`.
8. `result.md` (<=300 words) states the pattern constants mirrored, which legacy
   fixtures were excluded and why, and that app wiring is unexecuted.

If an AC cannot be met, stop with the exact blocker; no tolerance raising, no
test weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-render --locked --offline`
- `cargo +stable test -p vexel-render --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline` (callers still compile)
- `python3 scripts/wo_validate.py WO-RUST-SVG-PATTERN-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`
`cargo fmt --check` is not frozen for `vexel-render` (pre-existing drift, see
WO-RUST-V1-RELEASE-CI-01); new files must be rustfmt-clean
(`rustfmt --check --edition 2024 <file>`).

## Evidence
`artifacts/WO-RUST-SVG-PATTERN-01/`: `result.md`, `verification.json`, `ac-map.json`,
`changed-path-audit.json`, `legacy-build.log` (tail only).

## Worker and review
Worker: gpt-6-luna High, one writer, existing allowance only; stop on quota
errors. Candidate only until a fresh read-only reviewer reruns the frozen
commands and checks each AC. The coordinator integrates the patch (textual
merges in `lib.rs` are the coordinator's), schedules WO-RUST-SVG-WIRE-01, then
updates STATE.md, LEDGER.md and ledger.jsonl.
