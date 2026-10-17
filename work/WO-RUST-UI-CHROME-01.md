---
id: WO-RUST-UI-CHROME-01
revision: 1
status: READY
milestone: UI-MODERN-2026-10
owner: project-owner
executor: gpt-6-luna
base_sha: 76dc7bef0762fac199d292cb96d2e51ba33dbcc3
spec_sha256: ea8a733f3bd373fbbd4f1034d48adcb88112c6c753479938b52e4cddf717183a
parallel_safe: False
network: false
---
# Modern chrome layout: toolbar, tool rail, inspector sections, status bar

## Goal
Implement the layout and chrome of `docs/ui/design-direction-2026-10.md` section 5 in `vexel-app`: 44 px top
toolbar with a segmented Draw | Animate control and grouped icon buttons, a
44 px icon-only tool rail with tooltips, a resizable inspector (default 296 px)
with collapsible grouped sections, the MCP band folded into a 24 px status bar,
and a resizable footer slot for the timeline. Fixed panels, no floating.
Evidence: critique items 1 to 6.

## Dependencies
Requires WO-RUST-UI-TOKENS-01 and WO-RUST-UI-ICONS-01 merged. Optional
integration (not required): MOTION (adopt `ControlVisual` when present),
CANVAS and TIMELINE (call their `paint` entry points when present, otherwise the
current stage/timeline paths). Read first: `docs/ui/design-direction-2026-10.md` 3 and 5;
`docs/adr/0017-ui-direction-blender-aligned.md`; `vexel-app/src/{model,render,chrome,inspector}.rs`
(`Layout::new` constants 48/28/70/54/208-252), `docs/ui/README.md`.

## Assumptions
- Draw and Animate stay the only workspaces in this app (ADR-0017's Nodes and
  Inspect remain future); the segmented control is built for N segments.
- `Layout::new` keeps returning `Option<Layout>` with minimum window sizes
  re-derived: width at least 720 (inspector auto-collapses below 900 as today),
  height at least 480; report the new minima.
- The 70 px MCP band becomes a status-bar indicator (dot + word `MCP`, never
  colour alone) opening a popover that holds the current activity text and
  Connect button. Behaviour of Connect is unchanged.
- Section collapsed/expanded and panel sizes are in-memory UI state this order;
  persistence is a follow-up (report which settings key would hold it).
- Materials (vibrancy) are NOT in this order (owner decision Q2).

## Scope
1. New pure `vexel-app/src/layout/` module: `Layout` v2 with rects for toolbar,
   rail, stage, inspector (+ splitter rect), footer (+ splitter), status; resize
   clamps (inspector 240-420, footer 120 to 50% of content); `hit_test`.
   `model.rs::Layout` delegates to it (public fields preserved or migrated with
   all call sites updated in allowed files).
2. Toolbar: wordmark removed in favour of document name (title 15 pt) with an
   unsaved dot badge; centre segmented control with sliding thumb geometry
   (drawn at the thumb's current position, animation by MOTION when present);
   right groups [Undo Redo] and [Save Export] in shared rounded containers,
   enabled/disabled distinct, icon + tooltip text; panel toggles.
3. Tool rail: 32 px icon buttons, 16 px icons centred (ICONS helper), selected
   state (accent.subtle + accent glyph), group divider, tooltip data
   (name + shortcut). No labels.
4. Inspector: section model (Selection, Fill, Stroke, Transform, Animation),
   header rows with chevron, 13 pt semibold, collapse state, empty state per
   section, colour field (swatch + hex), numeric field with unit suffix and
   tabular numerals; keyboard: Tab order, Space/Return toggles a focused header.
5. Status bar 24 px: hint left, report centre, readouts right (zoom, size, save
   state) and the MCP indicator; the old activity band and its layout are removed.
6. Surface assignment per the elevation table (window/canvas well level 0,
   panels level 1, toolbar level 2), borders only where a level step is absent.

## Non-goals
Canvas content (CANVAS), timeline content (TIMELINE), transitions (MOTION), new
tools, document editing, material/vibrancy, AppKit settings window, workspace
persistence, Nodes/Inspect workspaces.

## Files allowed to touch
- `rust/crates/vexel-app/src/{model,render,chrome,inspector,file_ui}.rs`,
  `src/layout/**` (new), `src/preview_tests.rs` only to update layout pins
- `rust/crates/vexel-app/tests/ui_chrome_*.rs` (new), `rust/fixtures/ui/chrome/**`
  (new), `artifacts/WO-RUST-UI-CHROME-01/**`
- lib.rs registration; `main.rs` call-site wiring (see below)

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
1. Geometry: for sizes from the new minimum up to 4096x4096 and both modes,
   `Layout::new` rects are inside the window, panels do not overlap, toolbar is
   44 px, rail 44 px, status 24 px, stage is at least 260x140, inspector within
   240..420 (or absent below 900 wide), footer only in Animate and within bounds;
   sizes below the minimum return `None`. 5,000 seeded sizes never panic.
2. Resize: dragging a splitter by any delta clamps to range and never makes the
   stage smaller than 260 px wide; layout round-trips (set size then read).
3. Hit-testing: each toolbar button, rail tool, segment, section header and
   splitter hit-tests to its own target; tool targets are at least 32 px, other
   targets at least 28 px; no two targets overlap.
4. Segmented: selecting Animate sets the mode and thumb target rect equals the
   Animate segment rect; Draw/Animate keyboard (Left/Right when focused) works.
5. Inspector: collapsing a section removes its rows from layout and increases
   the next section's y by the collapsed height; expanded state restores
   exact prior geometry; focus order follows reading order; empty selection
   shows the empty state in the Selection section only.
6. Status/MCP: the 70 px activity band is gone (assert total chrome height);
   the MCP popover content equals the old activity text and Connect still
   dispatches the same command; MCP state is conveyed by word plus icon, never
   colour alone (assert text present for each state).
7. Tokens: every chrome fill is a `Tokens` field (no literal colour in
   `render.rs`; assert by a source scan test); computed contrast of toolbar,
   rail, inspector text against their surfaces passes the TOKENS gates.
8. Golden: full window at 1280x800 and 960x640, Draw and Animate, dark and
   light, 1x and 2x, with compose-v1.svg state and an empty (no document)
   state, matches first-party goldens (policy below; chrome uses CoreText
   text, so text-tolerance applies), candidates until reviewed.
9. Existing tests pass (layout pins updated and listed); no new external
   package; no unsafe. `result.md` lists new minimum window size and which
   optional integrations (MOTION, CANVAS, TIMELINE) were present.

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
- `python3 scripts/wo_validate.py WO-RUST-UI-CHROME-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Live check (coordinator, a person at the Mac; not the worker)
1. Launch with compose-v1.svg in dark and light and in Draw then Animate. Screenshot each at the default window size and at the minimum size.
2. Verify against the wireframes in the design doc: toolbar groups, segmented control, icon rail centred and legible, inspector sections collapse/expand, one status bar, no MCP band.
3. Drag the inspector splitter and the footer splitter; resize the window down to the minimum.
4. Compare to the 2026-10-16 and 2026-10-17 screenshots and state improvements and any regression.
Record each viewed screenshot in `docs/archive/live/` as
`YYYY-MM-DD-ui-chrome-01-<what>.png` with a line in
`docs/archive/live/README.md` (what it shows, what was verified, what remains
unverified). Launch from a Finder/Terminal session in the foreground, not from
a harness; find the window id with Quartz `CGWindowListCopyWindowInfo`, then
`screencapture -x -l <id> out.png`, and view the PNG before accepting goldens.

## Evidence
`artifacts/WO-RUST-UI-CHROME-01/`: `result.md` (<=400 words), `verification.json`,
`ac-map.json`, `changed-path-audit.json`, `review/` (candidate golden PNGs).

## Worker and review
Worker: gpt-6-luna High, Standard speed, one writer, existing allowance
only; stop on quota errors. Candidate only until a fresh read-only reviewer
(separate session, Luna or Sol Medium, diff + order only) reruns the frozen
commands and checks each AC. The coordinator integrates the exact
patch, reruns main-tree checks, does the live check, accepts goldens, then
updates STATE.md, LEDGER.md and ledger.jsonl.
