---
id: WO-RUST-UI-CANVAS-01
revision: 1
status: READY
milestone: UI-MODERN-2026-10
owner: project-owner
executor: gpt-6-luna
base_sha: 76dc7bef0762fac199d292cb96d2e51ba33dbcc3
spec_sha256: 2914f1a15f27fc92138c6d9f478539f2a48b0b7365d6f045133fab40cf4d2c01
parallel_safe: False
network: false
---
# Artboard presentation, zoom/pan/fit and selection overlays

## Goal
Present the document as an artboard in a canvas well (soft shadow, checkerboard
under transparency), add zoom/pan/fit controls with a zoom label, and draw hover
and selection outlines with handles over the stage, per `docs/ui/design-direction-2026-10.md` section 5.
Evidence: critique item 7; compose-v1.svg fills the stage as a bare white
rectangle in `docs/archive/live/2026-10-16-compose-v1-after-live-fixes.png`.

## Dependencies
Requires WO-RUST-UI-TOKENS-01 merged (colours, radii, checkerboard values).
Independent of CHROME; it paints inside `layout.stage` and is wired through the
existing `DocumentView::blit_document` site. Read first: `docs/ui/design-direction-2026-10.md` 5;
`vexel-app/src/compose/present.rs` (`DocumentView`, `blit_document`),
`compose/pointer.rs`, `tools/controller.rs`, `tools/map.rs`, `main.rs` (pointer
and scroll events).

## Assumptions
- The document keeps composing at the size the current code chooses; zoom/pan
  is a view transform applied at present time. If composing must change for
  crisp zoom (re-render at zoom), do it through the existing stage-size-aware
  compose path and report the cost; do not touch `vexel-render`.
- Hit-testing, selection and edit semantics are unchanged; only their screen
  mapping changes through one `view_to_doc`/`doc_to_view` pair with tests.
- Zoom range 10% to 3200%, steps 1.25x; Cmd+0 fit, Cmd+1 100%; trackpad pinch
  and Cmd+scroll zoom about the pointer; two-finger scroll and Space-drag pan.
  If the app has no scroll/pinch events wired, wire only those the loop already
  receives and list the rest as unexecuted.
- Checkerboard: 8 px squares, tokens `checker_a/checker_b`, drawn only where the
  document is transparent (document background `none`), clipped to the artboard.

## Scope
1. `vexel-app/src/canvas/`: `View {zoom, pan}`, fit/100%/zoom-at-point math,
   artboard rect, shadow (0 2 12 at 35% black, 2 layered soft rects, no blur
   dependency), hairline, checkerboard, zoom controls cluster (minus, label,
   plus, Fit) as a rect list with hit targets 28 px and tooltips text.
2. Overlays: hover outline (1 px accent at 60%), selection outline (1.5 px
   accent), 8 resize handles (8x8 white fill, accent edge) plus rotation
   affordance omitted unless it already exists; drawn in view space so lines
   stay 1 px at any zoom.
3. Wiring: replace the bare stage fill with the canvas well and artboard at the
   stage blit site; route zoom/pan events; keep MCP/preview paths unchanged.

## Non-goals
Editing semantics, new tools, snapping, rulers, guides, rotation tool, document
render changes, timeline, chrome layout.

## Files allowed to touch
- `rust/crates/vexel-app/src/canvas/**` (new), `src/compose/{present,pointer}.rs`,
  `src/tools/{controller,map}.rs`, `src/tools/tests_*.rs` only for mapping tests
- `rust/crates/vexel-app/tests/ui_canvas_*.rs` (new), `rust/fixtures/ui/canvas/**`
  (new), `artifacts/WO-RUST-UI-CANVAS-01/**`
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
1. View math: `fit` centres the document with at least 24 px margin on a set of
   stage sizes (260x140 up to 3840x2160) and aspect ratios, never zooms above
   100% unless the document is smaller (then at most 100%); `zoom_at(point)` keeps
   the document point under the pointer fixed within 0.5 px; zoom clamps to
   10%..3200%; `view_to_doc(doc_to_view(p)) == p` within 1e-6 on 1000 seeded
   points.
2. Existing tool tests still pass when the identity view is used; with a 200%
   view and a 50 px pan, a click at the mapped screen point selects the same
   element as at identity (using compose-v1.svg element hit tests).
3. Artboard: rendering the stage at 1x and 2x for compose-v1.svg paints the
   well (level 0), a shadow ring darker than the well within 14 px outside the
   artboard and absent inside, a 1 px hairline, and the document pixels equal
   to `compose_document` output at that size inside the artboard (hash).
4. Checkerboard: a transparent test document shows alternating `checker_a/b`
   8 px squares anchored to the artboard origin and clipped to it; an opaque
   document shows none. Squares are 8 device px at 1x and 16 at 2x.
5. Overlays: hover and selection outlines are 1 and 1.5 logical px wide at 25%,
   100% and 800% zoom (measured on the rendered pixels), handles are 8 logical
   px, do not move when zooming, and are hidden when the selection is empty.
6. Zoom controls: the label reads `NNN%` with no decimals; hit targets are at
   least 28 px; the cluster does not overlap the artboard's top-left 100 px nor
   leave the stage at stage widths down to 260 px.
7. Zoom performance: presenting a cached compose with only a view change does
   not re-run `compose_document` unless zoom exceeds the cached resolution by
   more than 2x (counter assertion), and a pan of 1000 events stays under the
   existing `perf_gate` frame budget.
8. Golden: stage with artboard, shadow, checkerboard, hover, selection and
   handles at 1x and 2x, dark and light, matches first-party goldens (policy
   below; shadow tolerance as documented, candidates until reviewed).
9. Existing tests pass; no new external package; no unsafe. `result.md` lists
   the zoom/pan events actually wired and those not.

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
- `python3 scripts/wo_validate.py WO-RUST-UI-CANVAS-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Live check (coordinator, a person at the Mac; not the worker)
1. Open compose-v1.svg, screenshot at fit. Verify an artboard floats in a darker well with a soft shadow and the zoom cluster sits bottom-left.
2. Click an element: selection outline and handles visible and crisp; hover another: lighter outline. Zoom in/out with controls and Cmd-scroll; confirm outlines stay 1 px and the label updates.
3. Open a transparent-background SVG fixture and screenshot the checkerboard.
Record each viewed screenshot in `docs/archive/live/` as
`YYYY-MM-DD-ui-canvas-01-<what>.png` with a line in
`docs/archive/live/README.md` (what it shows, what was verified, what remains
unverified). Launch from a Finder/Terminal session in the foreground, not from
a harness; find the window id with Quartz `CGWindowListCopyWindowInfo`, then
`screencapture -x -l <id> out.png`, and view the PNG before accepting goldens.

## Evidence
`artifacts/WO-RUST-UI-CANVAS-01/`: `result.md` (<=400 words), `verification.json`,
`ac-map.json`, `changed-path-audit.json`, `review/` (candidate golden PNGs).

## Worker and review
Worker: gpt-6-luna High, Standard speed, one writer, existing allowance
only; stop on quota errors. Candidate only until a fresh read-only reviewer
(separate session, Luna or Sol Medium, diff + order only) reruns the frozen
commands and checks each AC. The coordinator integrates the exact
patch, reruns main-tree checks, does the live check, accepts goldens, then
updates STATE.md, LEDGER.md and ledger.jsonl.
