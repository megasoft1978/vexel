# WO-RUST-UI-TIMELINE-01 result

Footer timeline for Animate: transport row (skip start, step back, play/pause, step forward, skip end, time readout,
loop, speed label, disabled Ease and "+ Key"), ruler, element/property track rows with key diamonds, accent playhead.
Code: `vexel-app/src/timeline/{mod,model,layout,paint}.rs`; wiring in `lib.rs`, `model.rs` (`Model.timeline`),
`render.rs` (`timeline::paint`), `main.rs` (pointer, keys, wheel, per-frame sync).

## Track data
Tracks come from the existing `vexel_svg_animation::parse_owned` over the document source (already a vexel-app
dependency, no new accessor or package): one element row (name = `id`, else tag) with one sub-row per animated
property, key times `begin + keyTime * dur` clamped into the loop, first period only. Element rows show the union
of their property keys. Unsupported tracks are skipped like the loop length does.

## Intentionally disabled / not done
Key editing, "+ Key" and Ease are drawn disabled; `Hit::disabled_hint()` returns the "coming soon" tooltip text but
no tooltip is painted (the shell has no tooltip painter yet). Speed "1x" is a label only. Redo history cursor not touched.

## Deviations from the order (main's real structures)
- Footer rect comes from CHROME's `Layout::footer` (resizable). Timeline state lives in `Model.timeline`.
- Old overlay strip (`animation/strip.rs`) is now only drawn/hit in Draw mode (no footer); Animate uses the footer.
  `strip.rs` and `tests_ui.rs` unchanged.
- Time readout uses the existing `strip::label` format (`0:00.80 / 0:01.00`, centiseconds), not `0:00.8`.
- Frame step is 1/24 s in `timeline::step` (pauses, wraps when looping, clamps otherwise); Draw mode keeps the
  legacy 1/30 `keys::apply`. End key added in `timeline::key` (keys.rs is outside allowed paths).
- Buttons fire on pointer release over the pressed button; ruler/lane scrub on press and drag.
- Vertical scroll is in whole rows (`Timeline::scroll_rows`, wheel via `scroll_by_points`).
- Hover/press of the new buttons is painted from `Timeline.hover/pressed` (instant, not through MotionRegistry).
- Status-bar hint text still says "drag the strip to scrub" (chrome-owned text, not changed).

## Changed expectations (pins)
- `render.rs::geometry_tests::timeline_rows_are_omitted_until_their_full_bounds_fit` removed with the placeholder
  `draw_timeline`/`timeline_content_rows`; its guarantee (rows stay inside the rect, omitted until they fit) is
  covered by `ui_timeline_model::layout_bands_never_overlap_and_stay_inside_for_all_sizes`.
- 16 `fixtures/ui/chrome/*animate*.rle` goldens (and their review PNGs under `artifacts/WO-RUST-UI-CHROME-01/review`)
  re-blessed: only the Animate footer changed (placeholder "Timeline" panel -> real timeline). All 16 Draw goldens
  are byte-identical. Candidates until a human views them.
- New first-party goldens: `fixtures/ui/timeline/footer-{playing,paused-selected,long-scrolled}-{dark,light}@{1,2}x.rle`
  (12). Tolerance: <=0.5% pixels differ, no channel > 24 (CoreText glyphs). Review PNGs in `review/`.

## Verification
Frozen commands: check, vexel-ui tests (46), vexel-app tests (339), wo_validate, ledger, diff --check all pass.
`cargo fmt --check` passes for vexel-app; it fails for vexel-ui on files this order did not touch (pre-existing).
Live: loop.svg and many.svg in Animate (temporary env hook, removed) and loop.svg in Draw, screenshots in
`docs/archive/live/2026-10-17-ui-timeline-01-*.png`. Live click injection was impossible (screen locked), so
behaviour is verified headlessly only.
