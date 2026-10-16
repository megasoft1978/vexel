---
id: WO-RUST-V1-RENDER-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 2898e150bc65d81f48d02341ba77e611fd1a6172
spec_sha256: 3e5c9db849e4e742d27698576b2b66d2c83e48833dad5b769a448b00c0f6eabc
parallel_safe: true
network: false
---
# Goal and frozen scope
New crate `rust/crates/vexel-render`: port the C scalar solid-colour shape
rasterizer (src/raster/shape_render.c, shape_render_fill.c,
shape_render_stroke.c, shape_render_stroke_grid.c) for the ADR-0029 V1 slice
into `vexel_raster::Tile`s. In scope: nonzero and evenodd anti-aliased fill;
stroke with width, butt/square/round caps, miter/bevel/round joins, miter
limit, open and closed subpaths, solid literal RGBA colours, element
transforms (stroke width in local space via the inverse transform, as in C),
source-over ordering per shape in document order, plus the offscreen
backing-scale rule below. Preserve the C sampling model exactly: 4x4 grid at
offsets {0.125,0.375,0.625,0.875}, coverage = round(covered/16), RGB averaged
over non-transparent samples, alpha averaged over all 16, opaque full-cover
pixels keep exact colour, empty pixels stay transparent black.
Deferred and must return an explicit typed `Unsupported` error (never silent
skip, never TODO/stub): dash arrays, gradient/pattern/image paint, clips,
masks, filters, opacity groups, text, animation overlay, SMIL, crisp-edges.
No performance, full-renderer or UI claim.

# Reference paths
src/raster/README.md, shape_render.c/.h, shape_render_fill.c,
shape_render_stroke.c, shape_render_stroke_grid.c, shape_render_internal.h
(C is first-party code; a direct port is permitted). Rust: vexel-raster (Tile),
vexel-tile-cache (key/epoch usage only), vexel-scene (Shape/Stroke/Linecap/
Linejoin types, read-only), vexel-svg-paint (solid classification, read-only),
vexel-geometry (Affine), vexel-platform-apple (presentation path, read-only).
Docs: ADR-0029, docs/product/rust-port-goal-and-plan.md, BLOCKERS.md defect 3,
work/WO-UI-GOLDEN-01.md (context only).

# Public API (adaptable only with a documented reason in parity-map.json)
- `FlatEdge { x0, y0, x1, y1: f64 }` and `RenderShape<'a> { edges: &'a [FlatEdge],
  subpaths: &'a [Range<usize>] (edge index ranges), evenodd, filled, fill_rgba,
  stroke: Option<StrokeStyle>, transform: Affine }`; `StrokeStyle` mirrors
  vexel_scene::Stroke without `dash_values`, `paint_server`, `paint_order`.
- `render_shapes(shapes: &[RenderShape], view: ViewMap, tile: &mut Tile,
  control: &dyn Fn() -> bool) -> Result<(), RenderError>`; `control` returning
  true cancels at row granularity (error `Cancelled`, tile contents then
  unspecified and documented).
- `ViewMap::from_viewbox(vb, width, height, preserve_aspect)` matching the C
  meet/slice/none mapping.
- `RenderError`: Unsupported(kind), InvalidGeometry, TileNotEmpty, LimitExceeded,
  Cancelled. Output tile must be empty on entry (C contract).
- Adapter `from_scene_shape(&vexel_scene::Shape, edge_slice) -> Result<RenderShape,
  RenderError>` returning Unsupported for non-solid paint or non-empty dash.
- Offscreen presentation: `pub const OFFSCREEN_BACKING_SCALE: f64 = 1.0;`
  `offscreen_tile_size(points_w, points_h) -> Result<(u32,u32), RenderError>`
  which never consults a host scale and returns exactly points rounded as C
  `render-frame` does. Offscreen/export/golden paths must call it; the live
  window path keeps the host backing scale from vexel-platform-apple unchanged.

# Requirements
1. Checked arithmetic, bounded inputs (tile area, edge count, subpath count)
   with explicit limits equal to the C caps; fallible allocation for scratch.
2. Safe Rust 2024, `#![forbid(unsafe_code)]`, no new external dependencies;
   only path dependencies already in the workspace.
3. Source files < 800 lines, cohesive modules (view map, fill, stroke, joins/
   caps, composite, presentation).
4. Existing C behaviour is the oracle; do not alter C sources or goldens.
5. Crate tests use hand-built `RenderShape` values only. No dependency on
   WO-RUST-V1-DECODE-SUBSET-01 anywhere in this crate's tests.

# C reference pixels (parity)
Build once from repo root: `./dev build`. Then for each parity case:
`build/<preset>/svgworkbench --render-frame <fixture.svg> --out <name>.ppm`
(options per src/app/render_frame_cli.c: --out, --zoom, --time; use defaults
at the fixture's native size). Fixtures are the first-party project-authored
SVGs under bench/corpus/fixtures/ whose features are within scope:
shapes-basic, shapes-rounded, shapes-lines-polygons, path-line-curves,
path-relative-commands, style-presentation-attributes (opacity/gradient/clip
fixtures are out of scope). The coordinator (not the worker) generates the PPMs
and commits them to `rust/crates/vexel-render/tests/golden/<id>.ppm` with
`manifest.json` {fixture_id, source_sha256 from tests/golden/manifest.lock,
ppm_sha256, width, height, svgworkbench git sha}. Worker reads them as frozen
inputs; if absent, stop and report (do not self-generate or edit). For each
case the test hand-builds the equivalent `RenderShape` list (flattened edges
copied as literals with a comment naming the source element) and compares.

# Tolerance (explicit, numeric)
Same 4x4 grid and same arithmetic order are required, so target is exact.
Allowed difference per channel per pixel, because of f64 vs C double
flattening and `fma` contraction: |delta| <= 1 on RGBA channels. Aggregate:
at most 0.5% of pixels differ at all, and zero pixels differ by more than 2.
Any pixel outside 1-pixel dilation of a shape edge must be exactly equal.
Tests print max delta and differing-pixel count; thresholds are constants in
one test module and may not be raised by the worker.

# Acceptance criteria
- AC-1 fill: nonzero vs evenodd on a self-intersecting star and a nested-ring
  case; exact coverage values for axis-aligned half-pixel rectangle (hand-
  derived: 8/16 samples -> alpha 128 of 255).
- AC-2 stroke: width, each cap, each join, miter limit crossing (miter falls
  back to bevel above limit), closed vs open subpath, zero-width/zero-length
  degenerate handling per C, with exact expected values or C PPM parity.
- AC-3 parity: every in-scope fixture above within the tolerance section; one
  negative test proves the harness fails on a deliberately shifted shape.
- AC-4 errors: each deferred feature returns its specific Unsupported kind;
  non-empty tile, non-finite or overflowing geometry, over-limit input and
  cancellation return their typed errors with no panic.
- AC-5 backing scale: `offscreen_tile_size` returns identical sizes when the
  test simulates host scales 1.0, 2.0 and 3.0 (scale is not a parameter); a
  grep-style test asserts no call to the host backing scale in the offscreen
  module. Document that this resolves BLOCKERS.md defect 3 only for the Rust
  path; native_ui_render.m and its goldens stay untouched.
- AC-6 determinism: two renders of the same input are byte-identical.
- AC-7 parity-map.json maps each C function in the four reference files to a
  Rust symbol and test, or to a named Unsupported/deferred entry.

# Files allowed to touch
- `work/WO-RUST-V1-RENDER-01.md`
- `rust/crates/vexel-render/Cargo.toml`, `rust/crates/vexel-render/src/**`,
  `rust/crates/vexel-render/tests/**` (excluding `tests/golden/**`, read-only)
- `artifacts/WO-RUST-V1-RENDER-01/**`
- Coordinator only: `rust/Cargo.toml` workspace members, `Cargo.lock`,
  `rust/crates/vexel-render/tests/golden/**`, `STATE.md`, `LEDGER.md`,
  `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md`,
  `docs/dependencies/render-fixtures.lock.json`.
The coordinator adds `vexel-render` to the workspace manifest and its local path
dependency pins before the worker starts; the worker never edits a manifest
outside its own crate Cargo.toml (path deps only: vexel-base, vexel-geometry,
vexel-raster, vexel-scene).

# Protected paths
All C/ObjC sources, tests, goldens, oracles, bench corpus, other Rust crates,
workspace Cargo.toml/lock/toolchain, other work orders, docs/records. No
dependencies/downloads/install/network/unsafe/Cshim, no staging/commit/reset/
clean/delete, no recursive agents, no GUI, no broad UI/E2E/C suites, no `./dev`
builds by the worker. Preserve all dirty baseline files.

# Execution and verification
Only `.worktrees/rust-render`; Luna High, one writer, then one fresh read-only
reviewer (separate session, no worker transcript, given only the diff, this
order and the goldens) and at most one narrowed repair. Repo-local
CARGO_HOME/TMPDIR/CARGO_TARGET_DIR, MACOSX_DEPLOYMENT_TARGET=13, rustc +stable
1.99.0. Rustfmt/clippy are not installed; do not install or claim them.
cargo +stable check -p vexel-render --locked --offline
cargo +stable test -p vexel-render --locked --offline
python3 scripts/wo_validate.py WO-RUST-V1-RENDER-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check

# Dependencies and sequencing
Crate work is independent of WO-RUST-V1-DECODE-SUBSET-01. Only the end-to-end
check (SVG file -> decode -> scene -> render_shapes -> compare with the same
PPMs) depends on it; that check belongs to the coordinator's integration order
after both are accepted and is not an AC here.

# Non-goals
Dash, gradients, clips, text, animation, GIF export, window presentation code,
changing native_ui_render.m or its goldens, performance targets, any
visual-parity claim beyond the listed fixtures.

# Output
Leave uncommitted; `result.md` <= 400 words, `verification.txt`,
`parity-map.json`, tolerance report (max delta, differing-pixel counts per
fixture) under own artifact. No self-acceptance; candidate until independent
review, exact path audit and named main checks. Existing allowance only; no API
billing/credits; checkpoint on rate errors, no retry loop.
