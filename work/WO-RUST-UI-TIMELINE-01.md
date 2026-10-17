---
id: WO-RUST-UI-TIMELINE-01
revision: 1
status: READY
milestone: UI-MODERN-2026-10
owner: project-owner
executor: gpt-6-luna
base_sha: 76dc7bef0762fac199d292cb96d2e51ba33dbcc3
spec_sha256: f6658b66d561b57744ef6800a34172a1d1b56fe2d16b6f6872d8735af99facd1
parallel_safe: False
network: false
---
# Animate workspace timeline: tracks, keyframes, playhead, transport, ruler

## Goal
Replace the placeholder Timeline panel with the real footer timeline of
`docs/ui/design-direction-2026-10.md` 5.2: transport row, time ruler, track rows with keyframe diamonds, and a
draggable playhead, driven by the open document's animation state. Evidence:
the Animate screenshots in `docs/archive/live/` (empty panel).

## Dependencies
Requires WO-RUST-V1-ANIMATE-FIX-01 merged (truthful Timeline state and stage
fix) and WO-RUST-UI-TOKENS-01 and WO-RUST-UI-ICONS-01 merged (colours, play/
pause/step/loop/keyframe icons). Read first: `docs/ui/design-direction-2026-10.md` 5.2; `docs/ui/animation-timeline.md`;
`vexel-app/src/animation/{driver,strip,playback,keys}.rs` (loop time, scrub,
labels), `crates/vexel-timeline`, `render.rs::draw_timeline`.

## Assumptions
- Tracks are derived read-only from the document's SMIL/animation fragments via
  the existing driver/timeline crates (one track per animated element, one
  sub-row per animated property, keyframes at their key times). If track data
  is not exposed, add the smallest read-only accessor in `vexel-app` (no edits
  to other crates) or stop and report.
- Editing keyframes (dragging diamonds, adding keys) is out of scope: diamonds
  are selectable and the playhead is scrubbable; "+ key" and easing menu are
  drawn disabled with a tooltip stating they are coming, unless an existing
  edit API already covers them.
- Time model: loop seconds from `Driver::loop_seconds()`, 24 fps ruler ticks
  labelled with the existing `strip::label` style.

## Scope
1. `vexel-app/src/timeline/`: pure `TimelineModel` built from driver state
   (tracks, keys, loop, playhead, playing, selection), `TimelineLayout::new(rect)`
   (transport 32 px, ruler 24 px, track rows 28 px, name column 160 px,
   vertical scroll when tracks overflow), hit-testing, time/x mapping.
2. Painting: transport buttons (skip start, step back, play/pause, step forward,
   skip end, loop toggle, speed label), time readout `0:00.8 / 2:00` with tabular
   figures, ruler ticks/labels (major per second, minor per 0.25 s, density
   adapts to width), tracks with indent and chevrons, keyframe diamonds
   (selected = accent), playhead line and handle in accent over the ruler and
   tracks, empty states (no document, static document, no tracks).
3. Interaction: click/drag on ruler or tracks scrubs; Space toggles play;
   Left/Right step one frame; Home/End jump; loop toggle; all through the
   existing driver API (no playback-engine changes).
4. `render.rs::draw_timeline` delegates to `timeline::paint`; the footer
   height follows CHROME's resizable footer when present, otherwise the existing
   timeline rect; panel never needs more than the rect it is given.

## Non-goals
Keyframe editing, easing editor, graph editor, state machine, export, playback
engine changes, canvas, chrome layout, tokens/icons.

## Files allowed to touch
- `rust/crates/vexel-app/src/timeline/**` (new), `src/animation/strip.rs`,
  `src/animation/tests_ui.rs`
- `rust/crates/vexel-app/tests/ui_timeline_*.rs` (new), `rust/fixtures/ui/timeline/**`
  (new), `artifacts/WO-RUST-UI-TIMELINE-01/**`
- lib.rs registration; `main.rs`/`render.rs` call-site wiring (see below)

Shared registration files: `rust/crates/vexel-ui/src/lib.rs`,
`rust/crates/vexel-app/src/lib.rs` may receive single-line `mod`/`pub use`
additions from any UI order; `rust/crates/vexel-app/src/main.rs` and
`render.rs` may receive call-site wiring of at most 30 changed lines per order
(CHROME owns `render.rs` otherwise). The coordinator merges UI orders
sequentially and resolves these lines.

## Protected paths
Workspace/lock manifests and every `Cargo.toml` (coordinator only), all other
crates, C/ObjC, `tests/`, existing goldens and oracles (new first-party goldens
go only under this order's fixture directory), `docs/**` (the coordinator
records screenshots), `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other work
orders, git HEAD/index. No unsafe, new external deps, network, installs, GUI
launch by the worker, commit, stage, reset, deletion. Files under 800 lines.

## Acceptance criteria (all headless, no display)
1. Model: for `rust/fixtures/v1/anim/loop.svg` and `playback/loop2.svg` the
   model has at least one track, key times inside `[0, loop]`, sorted per track;
   for compose-v1.svg (static) zero tracks and the static empty-state string
   from ANIMATE-FIX; with no document the no-document string.
2. Layout: for rect widths 260 to 3840 and heights 120 to 600, transport, ruler
   and track rows never overlap, stay inside the rect, name column is at most
   40% of width, and the track area height is at least 0 (all or nothing, no
   negative geometry).
3. Mapping: `x_to_time(time_to_x(t)) == t` within 1/48 s for 1000 seeded times;
   playhead x clamps to the ruler; ruler tick labels never overlap (label width
   from the type scale numeric size) for widths 260 to 3840.
4. Scrub: pointer down/drag on the ruler sets the driver time through the
   existing API, clamped to loop, and release leaves it where it ended;
   Left/Right move exactly one frame (1/24 s) and wrap per loop mode; Space
   toggles play state; tests assert driver state, not pixels.
5. Hit-testing: transport buttons are at least 28 px; clicking a diamond selects
   its key (selection stored, no document edit); clicking empty track space
   does not change selection.
6. Paint: for loop.svg at t=0.8 s the rendered timeline (1x and 2x, dark and
   light) contains the playhead column in accent colour at the mapped x, one
   diamond per model key at the mapped positions (sampled pixels), and ruler
   ticks at the major spacing; no text path draws outside its cell.
7. No regression: the strip behaviour (hidden for static documents) and
   `animation/tests_ui.rs` pass; a headless Draw <-> Animate switch sequence
   keeps the stage composed (reuses the ANIMATE-FIX test).
8. Golden: timeline footer for loop.svg (playing and paused, selected key, long
   track list scrolled) at 1x and 2x, dark and light, matches first-party
   goldens (policy below, candidates until reviewed).
9. Existing tests pass; no new external package; no unsafe. `result.md` states
   what track data was available and what is intentionally disabled.

If an AC cannot be met, stop with the exact blocker; no test weakening.

## Golden-image policy (applies to every golden in this order)
- Goldens are first-party: rendered by the Rust renderer (`WidgetCanvas` /
  `vexel-raster` software path), never by C oracles. Test code compares RGBA.
- Compare tolerance, documented in the test file header and `result.md`: software
  rasterised shapes and icon masks match exactly (tolerance 0). Anything that
  draws CoreText glyphs may differ by OS version: per-channel delta at most 4 on
  at most 0.5% of pixels, no pixel above 24; otherwise assert on geometry
  (rects, positions) rather than pixels.
- Blessing is a separate manual step: a `VEXEL_BLESS_GOLDENS=1` run writes
  candidate goldens plus review PNGs (via `vexel-png`, already a `vexel-app`
  dependency) to `artifacts/<ID>/review/`. Candidates are NOT accepted until a
  human has viewed the PNGs (screenshots must be viewed) and the coordinator
  has recorded the live-window screenshot in `docs/archive/live/` (see Live
  check). A worker may not mark goldens accepted. Normal test runs never write.
- Fixtures: `rust/fixtures/ui/<area>/` (new files only), 1x and 2x, dark and
  light where the order says so.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-ui -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-ui --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-ui --package vexel-app -- --check`
- `python3 scripts/wo_validate.py WO-RUST-UI-TIMELINE-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Live check (coordinator, a person at the Mac; not the worker)
1. Open `rust/fixtures/v1/anim/loop.svg`, click Animate, screenshot: ruler, tracks, diamonds, playhead and transport visible; document visible on the stage.
2. Press play, screenshot twice to see the playhead move; scrub by dragging the ruler; step with arrow keys.
3. Open compose-v1.svg (static) in Animate and screenshot the empty state.
Record each viewed screenshot in `docs/archive/live/` as
`YYYY-MM-DD-ui-timeline-01-<what>.png` with a line in
`docs/archive/live/README.md` (what it shows, what was verified, what remains
unverified). Launch from a Finder/Terminal session in the foreground, not from
a harness; find the window id with Quartz `CGWindowListCopyWindowInfo`, then
`screencapture -x -l <id> out.png`, and view the PNG before accepting goldens.

## Evidence
`artifacts/WO-RUST-UI-TIMELINE-01/`: `result.md` (<=400 words), `verification.json`,
`ac-map.json`, `changed-path-audit.json`, `review/` (candidate golden PNGs).

## Worker and review
Worker: gpt-6-luna High, Standard speed, one writer, existing allowance
only; stop on quota errors. Candidate only until a fresh read-only reviewer
(separate session, Luna or Sol Medium, diff + order only) reruns the frozen
commands and checks each AC and the dependency on ANIMATE-FIX is merged. The coordinator integrates the exact
patch, reruns main-tree checks, does the live check, accepts goldens, then
updates STATE.md, LEDGER.md and ledger.jsonl.
