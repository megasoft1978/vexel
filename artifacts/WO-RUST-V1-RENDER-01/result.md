# WO-RUST-V1-RENDER-01 result (candidate, not self-accepted)

New crate `rust/crates/vexel-render`: direct port of the C scalar solid-colour shape rasterizer
(4x4 grid at 0.125/0.375/0.625/0.875, coverage averaging, nonzero/evenodd fill, stroke with
width, caps, joins, miter limit, open/closed and subpath handling, element transforms with the
stroke evaluated in local space, per-shape source-over). Safe Rust 2024, no new dependencies
(vexel-geometry, vexel-raster, vexel-scene only).

- Parity: byte-exact (max delta 0) on the six in-scope fixtures plus one crate-local extra
  (caps, joins, miter fallback, rotate/scale transforms, evenodd subpaths, letterboxed scaled
  view). Negative control (+0.5 px shift) fails the harness. See tolerance-report.txt.
- Errors: typed `Unsupported` per deferred feature (dash, gradient, pattern, image, clip, mask,
  filter, opacity group, text, animation overlay, SMIL, crisp edges, paint order),
  `InvalidGeometry`, `TileNotEmpty`, `LimitExceeded`, `Cancelled`; no panics.
- Backing scale: `offscreen_tile_size` / `render_offscreen` take no scale and never reference the
  host; resolves BLOCKERS.md defect 3 for the Rust path only. native_ui_render.m and its goldens
  are untouched.
- API adaptations (also in parity-map.json): RenderShape adds `subpath_closed` and
  `stroke_closed`; StrokeStyle keeps `dash_offset` and `crisp_edges`; `from_scene_shape` takes the
  paint tables, colour and subpath data; `ViewMap::from_viewbox` takes `ViewBox` and
  `Option<PreserveAspect>`; `SceneFeatures` reports scene-wide deferrals.
- Limits: edges/shapes/subpaths 4 Mi (C), stroke steps 1e6 and 16 MiB (C dash-cache budget,
  modelled without the grid).
- Known gaps: stroke acceleration grid, scanline index and center-only fast path are not ported
  (result-identical, slower); the per-shape clip branch of `from_scene_shape` has no test because
  `ClipId` cannot be constructed outside vexel-scene (clips are covered via
  `SceneFeatures.clips`); no performance, full-renderer or UI claim; goldens use
  `--render-animation` output, not `--render-frame` (viewer checkerboard).
