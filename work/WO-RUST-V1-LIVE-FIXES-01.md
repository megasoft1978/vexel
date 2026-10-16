---
id: WO-RUST-V1-LIVE-FIXES-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 1d077a5539d44b64586b5416ccf7dab99f9f58f9
spec_sha256: 4ba09d6954814be04b53952dd3bd3ca10088abc601580f4df33ef35ef873591f
parallel_safe: false
network: false
---
# Fix defects found in the first live run of vexel-app

## Goal
The first on-screen run (`vexel rust/fixtures/v1/compose/compose-v1.svg`, 1280x852
window) rendered the canvas but showed five defects. Fix them with tests that
need no display, and add a repeatable live-check procedure. No new features.

## Observed defects (evidence from the live run)
1. Top bar and bottom status still read "No document open" while a document is
   open. Text is hard-coded in `render.rs` (top bar, status bar) and
   `model.rs`/`main.rs` (`status: "No document open"`, reset on resize and
   scale change); nothing derives it from the open `AppDocument`.
2. A small label "compose-v1.svg 974x674, 7 shapes" (`compose/present.rs`
   `status_text`/view status) is drawn at the left edge over the tool rail at
   mid-height, clipped.
3. Circle and ellipse are visibly faceted (~12-16 segments). Cause:
   `vexel-svg-decode/src/geometry.rs` (~line 402) uses
   `transformed_flatness_tolerance` = 0.25/scale (`vexel-svg-path`
   `flatten.rs`), which is the C-reference rule; the current byte-exact C
   parity goldens encode these facets.
4. Tool rail Pen and Line buttons show the label twice: the glyph slot in
   `render.rs` uses the strings "Pen"/"Line" and the caption repeats them.
5. Inspector Fill/Stroke show "Unavailable" and Transform inputs are empty; the
   inspector is static (`draw_inspector`) and has no selection input.

## Assumptions
- Item 1/5 data comes from the open document and the session selection
  (`session/` controller, `compose` shapes and hit testing). If the selected
  shape's fill/stroke/transform are not reachable without changing a
  protected crate, stop and report the exact missing accessor.
- Defect 3 is a deliberate, recorded quality change; C parity for curved
  shapes is intentionally superseded, parity for non-curved shapes is kept.
- Circle and ellipse share one decode branch; paths, arcs and rounded rects
  are NOT changed.

## Scope
1. Document-driven chrome: a pure function (e.g. `chrome_text(&AppState) ->
   ChromeText`) yields title/top-bar text (file name, dirty marker), status
   text (path, size WxH, shape count, dirty, error if any) and inspector
   content. `Model`/`render` consume it; the literal "No document open" is
   produced only when no document is open; resize/scale handlers must not
   overwrite it with the placeholder.
2. Reposition the document status label into the status bar (or stage-local
   area) so it never overlaps the rail, and clip it to its container.
3. Circle/ellipse flattening: tolerance such that the max radial error of the
   polygon against the analytic circle/ellipse is <= 0.1 device px at 1x
   (scale-aware as today); keep the 12-step floor and the 4096 edge cap
   (over-cap still returns the existing Limit error). Document the formula.
4. Rail: single glyph per tool (symbol only, or label only), no duplicate text;
   Pen and Line get symbolic glyphs consistent with Select/Rect/Ellipse; layout
   unchanged.
5. Inspector wiring: with a selection show the shape's fill, stroke, stroke
   width and transform from the document; with no selection show an explicit
   "No selection" (not "Unavailable"); empty inputs show placeholder text.
   Read-only display is enough; no editing added.

## Non-goals
New tools or edits, undo/redo, save, performance work, theme/layout redesign,
other curve types, changing any non-curved golden, C code, workflow changes,
the worker launching the GUI.

## Files allowed to touch
- `rust/crates/vexel-app/src/**`, `rust/crates/vexel-app/tests/**`
- `rust/crates/vexel-ui/src/**`, `tests/**` only for glyph/label/text-fit
  needs proven by a failing test
- `rust/crates/vexel-svg-decode/src/geometry.rs` and
  `rust/crates/vexel-svg-decode/tests/**` (circle/ellipse expectations only)
- `rust/crates/vexel-platform-apple/src/native_window/app.rs` (activation only)
- Goldens regenerated ONLY for fixtures whose source contains `circle` or
  `ellipse` (found by grep, e.g. `rust/fixtures/v1/compose/**` and
  `rust/crates/vexel-render/tests/golden/**` entries), each listed in the audit
- `artifacts/WO-RUST-V1-LIVE-FIXES-01/**`, `scripts/live_check.sh` (new)

## Protected paths
Workspace/lock manifests, `vexel-svg-path`, `vexel-scene`, `vexel-render`
sources, all other crates, C/ObjC, `tests/`, every golden/oracle not named
above (non-curved goldens must stay byte-identical, proven by sha256 before
and after), `docs/**`, `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other work
orders, git HEAD/index. No new external deps, network, installs, commit,
stage, reset, deletion. Unsafe only where it already exists in
`native_window/app.rs`; add none. Files under 800 lines.

## Acceptance criteria
1. Chrome: after opening an SVG, top bar, status bar text and window title
   contain the file name, document size and shape count; dirty state shows a
   marker; failed open shows the typed error and keeps the previous document's
   text; resize and scale events do not reset it. Pure-function tests.
2. The document status label rect lies inside the status bar/stage and is
   disjoint from `layout.rail` for widths 1000..=1920 (test over layouts).
3. Flatten bound: test computes max radial error of the emitted polygon for
   circles of r=1,5,40,300 and ellipses (rx/ry 3:1) at scales 1 and 2 and
   asserts <= 0.1 device px; vertex count for r=40 at 1x is >= 48; over-cap
   radius still returns the Limit error.
4. Golden scope: sha256 listing before/after in `artifacts/.../goldens.json`;
   every changed golden's fixture contains circle or ellipse; all other
   goldens, including `shapes-rounded`, path and line fixtures, are
   byte-identical; changed ones carry a justification ("C parity facets
   superseded by 0.1 px bound") and a before/after max pixel diff.
5. Rail: no tool button renders the same string twice (test enumerates the
   glyph and caption per tool); Pen and Line glyphs are non-text symbols or
   the caption is omitted.
6. Inspector: select-tool pick on a shape of compose-v1.svg yields inspector
   data equal to that shape's fill, stroke, stroke width and transform from
   the document; deselect yields "No selection"; tested headlessly via the
   session controller, not "Unavailable".
7. Activation: `native_window/app.rs` shows the window, makes it key and
   activates the app when launched from a non-GUI parent (terminal/harness).
   Use only documented AppKit calls (`activate`/`activateIgnoringOtherApps`,
   `makeKeyAndOrderFront`); if activation cannot be made reliable, report the
   exact limit. Add a display-free unit check where possible and state what is
   unexecuted. Context: automated click/drag injection could not be verified
   in the first run because the app did not become frontmost.
8. Live-check script `scripts/live_check.sh` plus procedure in `result.md`:
   launch `target/.../vexel <fixture>`; find its window id with CGWindowList
   (`python3 -c "import Quartz..."` or `osascript`, owner name `vexel`);
   `screencapture -x -l <windowid> out.png`; report window frontmost/key (via
   `osascript` System Events) and capture path. Exits non-zero if no window is
   found. The worker does not run it; the coordinator repeats it and
   compares: top bar text, status label placement, circle smoothness, rail
   labels, inspector after a click on a shape.
9. Existing `vexel-app`, `vexel-svg-decode`, `vexel-render` tests pass (counts
   may only increase); no `unsafe` added; no new package in `cargo tree -p
   vexel-app`.
10. `result.md` (<=400 words) lists each defect's fix, the flattening formula,
    goldens changed, and what is unexecuted (live window, activation, click
    injection).

If an AC cannot be met, stop with the exact blocker; no test weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-svg-decode --locked --offline`
- `cargo +stable test -p vexel-render --locked --offline`
- `cargo +stable test -p vexel-platform-apple --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app --package vexel-svg-decode -- --check`
- `python3 scripts/wo_validate.py WO-RUST-V1-LIVE-FIXES-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Evidence
`artifacts/WO-RUST-V1-LIVE-FIXES-01/`: `result.md`, `verification.json`,
`ac-map.json`, `changed-path-audit.json`, `goldens.json`.

## Worker and review
Worker: gpt-6-luna High, Standard speed, one writer, existing allowance only;
stop on quota errors. Candidate only until a fresh read-only reviewer
(separate session, diff + order only) reruns the frozen commands and checks
each AC. The coordinator integrates the exact patch, reruns main-tree checks,
runs the AC 8 live check in an isolated session (window must be frontmost),
then updates STATE.md, LEDGER.md and ledger.jsonl.
