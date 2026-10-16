# WO-RUST-V1-APP-COMPOSE-01 result (candidate, not self-accepted)

New module `rust/crates/vexel-app/src/compose/` (window-free core plus shell glue):
`compose_svg(bytes, w, h) -> Result<Frame, ComposeError>` (Frame = RGBA8 plus
disclosure list), `compose_document` (adds cancel callback, `PointMap`, hit
test), `open_path`, `parse_launch`, a latest-wins `ComposeWorker`, and
`DocumentView`/`blit_document` for the shell. `main.rs` has a small hook
(args, per-frame poll/blit, click routing); `--mcp-preview` is unchanged.

## Verified (62 tests pass, 31 pre-existing + 31 new; no display needed)
- Frozen PPM for a rect/circle/ellipse/path(curve, evenodd)/polygon/nested
  transformed group/viewBox fixture, plus independent pixel assertions;
  repeat runs byte-identical.
- Pipeline vs C PPMs for the six shared fixtures: max channel delta 0.
- Pointer map round trip < 1e-9 for meet/slice/none; outside viewport -> None;
  render and pointer map agree on where a document rect lands; topmost hit.
- Typed errors: missing, directory, >8 MiB, NUL/non-UTF-8/over-long path,
  invalid UTF-8, malformed XML, filter/mask/text (offset + element), bad size;
  truncated prefixes never panic.
- Supersession: newest generation only, cancel, drop joins; one pending-job slot
  and one result slot.

## Disclosure behaviour
Scene-level deferrals (clip, opacity group, mask, filter, text) fail the whole
document with `Unsupported`. Per-shape deferrals (dash, gradient/pattern/image
paint, crisp edges, paint-order) skip that shape and list it
("gradient paint not rendered (<rect> at byte N)"). The status bar shows the
count and first three.

## Unexecuted / gaps
- Live window, Retina fidelity (pixels are replicated scale x scale), real
  click-to-status in the window: coordinator check.
- File > Open... menu: `vexel-platform-apple` has no open-panel API; AC5 is
  partial (CLI and `DocumentView::open` share `open_path`).
- Hit test models fill rule and stroke distance, not caps/joins.
- Stage is composited over white; the top-bar "No document open" label is
  still the shell's.
