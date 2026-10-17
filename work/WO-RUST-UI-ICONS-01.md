---
id: WO-RUST-UI-ICONS-01
revision: 1
status: READY
milestone: UI-MODERN-2026-10
owner: project-owner
executor: gpt-6-luna
base_sha: 76dc7bef0762fac199d292cb96d2e51ba33dbcc3
spec_sha256: f66840bfa5106ff303d8b05864e1f3975230730b2211e8334b38f6922abe64bc
parallel_safe: True
network: false
---
# First-party vector icon set on a 16 px grid in vexel-ui

## Goal
Replace glyph hacks (Unicode glyphs and the 24-unit path set whose tool-rail
renders appear tiny and off-centre) with one first-party stroke icon set drawn
from path data on a 16 px grid, 1.5 px stroke, round caps and joins, and render
it crisply at 1x and 2x. Evidence: critique item 1 in `docs/ui/design-direction-2026-10.md`; the tool rail in
`docs/archive/live/2026-10-16-compose-v1-after-live-fixes.png`.

## Dependencies
None (parallel with TOKENS). Read first: `docs/ui/design-direction-2026-10.md` (research finding 8, section
5), `rust/crates/vexel-ui/src/icons.rs` (`IconKind`, `IconCache`, mask
rasteriser, `MAX_PATH`, 24-unit `AXIS`), `icon_paths.rs` (`DEFINITIONS`,
`icon_validate_all`), the gallery images
`docs/ui/uikit-icons-gallery-{dark,light}.png`.

## Assumptions
- Authoring space is 16x16 units. Stroke is 1.5 units; at 1x one unit is one
  pixel (1.5 px stroke), at 2x three device pixels. Keyline: geometry stays 1
  unit inside the canvas (coordinates 1..15); element gap at least 1 unit;
  corner radius 1 unit for elements 6+ units wide.
- The existing rasteriser is 24-axis with scales 1..3. The worker chooses
  either (a) store new paths in 16-space and scale by 1.5 inside the
  rasteriser, or (b) author in 24-space at stroke 2.25; whichever is chosen,
  the public `IconKind` enum values and `icon_definitions()` stay source
  compatible so `vexel-app` keeps compiling. State the choice in `result.md`.
- Icons are stroke-only glyphs; filled dots (e.g. keyframe diamond variants,
  node handles) are small closed paths, listed as exceptions.
- Mask cache is caller-owned and bounded (existing); keep that.

## Scope
1. Redraw the existing 53 icons to the 16 grid rules. Mandatory first-class
   quality for: select, pen, rectangle, ellipse, line, undo, redo, save,
   export, eye, eye-off, lock, chevron-right/down, plus, minus, fit, zoom
   in/out, layers, sidebar left/right, more. Add new kinds (append only, after
   `SIDEBAR_RIGHT`): EXPORT, PLAY, PAUSE, STEP_BACK, STEP_FORWARD, SKIP_START,
   SKIP_END, LOOP, KEYFRAME (diamond), KEYFRAME_FILLED, EASING, CURVE, TIMELINE.
   No decorative icons. Total at most 70.
2. Tool-rail drawing helper `icon_centered(canvas, rect, kind, state, tokens)`:
   16 px glyph centred in a 32 px target (equal margins, integer-aligned at 1x,
   half-pixel-safe at 2x), colour by `ControlState` from tokens.
3. Pixel snapping rule documented in the module header: horizontal and vertical
   strokes land on pixel centres at 1x (odd widths snap, 1.5 px renders as a
   1 px core plus 0.5 coverage) so they are crisp, not blurry.
4. Review sheet: all icons at 16 px, 1x and 2x, dark and light, with states.

## Non-goals
Colours (TOKENS owns them: use current `Tokens`), layout, tool behaviour,
animation, icon fonts, SVG import of third-party icon sets, AppKit toolbar
images.

## Files allowed to touch
- `rust/crates/vexel-ui/src/{icons,icon_paths}.rs`, new
  `rust/crates/vexel-ui/src/{icons_draw,tests_icons}.rs`
- `rust/crates/vexel-app/tests/ui_icons_*.rs` (new)
- `rust/fixtures/ui/icons/**` (new), `artifacts/WO-RUST-UI-ICONS-01/**`
- `rust/crates/vexel-ui/src/lib.rs` (`mod`/`pub use` registration lines only)

## Protected paths
Workspace/lock manifests and every `Cargo.toml` (coordinator only), all other
crates, C/ObjC, `tests/`, existing goldens and oracles (new first-party goldens
go only under this order's fixture directory), `docs/**` (the coordinator
records screenshots), `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other work
orders, git HEAD/index. No unsafe, new external deps, network, installs, GUI
launch by the worker, commit, stage, reset, deletion. Files under 800 lines.

## Acceptance criteria (all headless, no display)
1. Every `IconKind` except `None` has a definition; `icon_validate_all()` passes;
   count at most 70; names unique, lowercase snake case.
2. Non-empty: each icon renders a mask with at least 8 covered pixels and
   covered area between 4% and 60% of the 16x16 box at 1x, and a mask whose
   covered-pixel count at 2x is within 4x +/- 25% of the 1x count.
3. Grid rules by parse: all coordinates lie within the keyline (1 to 15 units),
   bounds are not empty, and the ink bounds centre is within 0.75 unit of the
   canvas centre (exempt list: chevrons, play, documented in the test).
4. Crispness at 1x: for each icon, the share of mask pixels with coverage
   strictly between 16 and 239 (of 255) is at most 45% for icons built of axis
   aligned strokes (rectangle, line-h/v, plus, minus, align family), and the
   outer edge of those icons has at least one pixel row/column at coverage
   at least 200. At 2x the same sets hold with 35%.
5. Consistency: an automatic check that every stroke width in the renderer
   equals 1.5 units (constant, not per-icon) and caps/joins are round (assert on
   the rasteriser's parameters, not pixels).
6. Centred helper: for icon in {select, pen, rectangle, ellipse, line} in a
   32x32 target at 1x and 2x, the ink bounds centre is within 1 device pixel of
   the target centre; no mask pixel falls outside the target.
7. Determinism and bounds: identical masks across two runs; cache never grows
   beyond its fixed array; `MAX_PATH`/segment limits are not raised without a
   stated reason in `result.md`.
8. Golden: icon sheet at 1x and 2x, dark and light matches first-party goldens
   with tolerance 0 (software masks), candidates until reviewed.
9. Existing `vexel-ui` and `vexel-app` tests pass (counts may only increase),
   except gallery pins updated and listed; no new external package; no unsafe.
   `result.md` lists any icon the worker judged weak and why.

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
- `python3 scripts/wo_validate.py WO-RUST-UI-ICONS-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Live check (coordinator, a person at the Mac; not the worker)
1. Open the review sheet PNGs at 2x and 1x; view them (Read tool or Preview) before accepting any golden.
2. Run the built `vexel` once CHROME consumes the icons (or the gallery fixture if available) and screenshot the tool rail at Retina and at a non-Retina display or scaled mode.
3. Verify glyphs are centred, equal optical weight, crisp, not blurry, and no icon is unreadable.
Record each viewed screenshot in `docs/archive/live/` as
`YYYY-MM-DD-ui-icons-01-<what>.png` with a line in
`docs/archive/live/README.md` (what it shows, what was verified, what remains
unverified). Launch from a Finder/Terminal session in the foreground, not from
a harness; find the window id with Quartz `CGWindowListCopyWindowInfo`, then
`screencapture -x -l <id> out.png`, and view the PNG before accepting goldens.

## Evidence
`artifacts/WO-RUST-UI-ICONS-01/`: `result.md` (<=400 words), `verification.json`,
`ac-map.json`, `changed-path-audit.json`, `review/` (candidate golden PNGs).

## Worker and review
Worker: gpt-6-luna Medium, Standard speed, one writer, existing allowance
only; stop on quota errors. Candidate only until a fresh read-only reviewer
(separate session, Luna or Sol Medium, diff + order only) reruns the frozen
commands and checks each AC. The coordinator integrates the exact
patch, reruns main-tree checks, does the live check, accepts goldens, then
updates STATE.md, LEDGER.md and ledger.jsonl.
