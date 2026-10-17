---
id: WO-RUST-V1-ANIMATE-FIX-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 87569d9ae8c071561794a62cdf415d041498e58b
spec_sha256: 2dc8fe8e3fc4d3cf2842433a2846d259b7d3c361285f8aa856daf31e0dd72f11
parallel_safe: false
network: false
---
# Fix the Animate workspace stage and Timeline panel state in vexel-app

## Goal
Opening a document and switching Draw <-> Animate must keep the document on the
stage, and the Timeline panel must describe the open document truthfully.
Evidence: `docs/archive/live/README.md` and
`docs/archive/live/2026-10-17-animate-workspace-{blank-stage,after-redraw}.png`
(compose-v1.svg open, Animate tab: stage blank, Timeline "No document to
animate"; a later frame shows the stage but the Timeline text is unchanged).

## Dependencies
Read first (vexel-app, `rust/crates/vexel-app/src`): `main.rs` (event loop,
render trigger lines ~177-215, blit/strip at ~255), `model.rs` (`Layout::new`,
`select_mode`), `render.rs` (`draw_timeline`), `chrome.rs` (`AppState`,
`chrome_text`), `compose/present.rs` (`DocumentView::show/is_current/accept/
blit_document`), `animation/{driver,strip}.rs`.

## Root-cause hypotheses (confirm by test or print first; report which hold)
H1 (read from code, confirm): `render.rs::draw_timeline` paints the literal
"No document to animate" unconditionally. `render::render` receives only
`Model`, whose `chrome` carries no timeline state, so the text can never
reflect the document. Explains the unchanged text in both screenshots.
H2 (read from code, confirm): `Layout::new` shrinks `stage.height` by the
timeline height (up to 106 px) in Animate mode. The stage size therefore
differs from Draw, `DocumentView::is_current(revision, stage)` turns false,
`show()` sets `composed = None`, and the stage is blank until the async
compose worker answers. `blit_document` also drops any composed frame whose
size differs from the stage. Explains the blank frame right after the click
and the document appearing on a later redraw.
H3 (to disprove or confirm): after the mode switch no redraw is guaranteed when
the worker result arrives (`poll()` sets dirty, but check the loop order and
that `model.dirty` plus `doc.poll()` cannot miss a result, and that the
driver `set_size` invalidate does not repeatedly drop the animated frame).
H4 (to disprove): `driver.strip_visible()` (loop > 0) is not fed for static
or not-yet-synced documents; confirm the strip and the panel text use the same
"has a loop" predicate after the fix.
If none of H1-H3 explain the blank stage in a headless reproduction, stop and
report the exact observation instead of guessing.

## Assumptions
- Static document = no SMIL loop (`Driver::loop_seconds() == 0`); animated =
  `loop_seconds() > 0`. The strip stays hidden for static documents (existing
  tested behaviour, do not change it).
- Timeline panel text states: no document open -> "No document to animate";
  document open, static -> "No animation in this document"; document open,
  animated -> loop summary using `strip::label`-style time text (exact wording
  decided by the worker, one line, asserted in tests).
- Fix preference: while a new compose for the same revision is pending, keep
  showing the previous composed frame (stale but present) or compose at the
  Animate stage size immediately; either must satisfy the ACs. Do not remove
  the stage-size-aware compose.

## Scope
1. Feed the Timeline panel from document state: add a timeline text field to
   `ChromeText`/`AppState` (pure function of open/static/animated state) and
   draw it in `draw_timeline`; no hardcoded document claim in `render.rs`.
2. Remove the blank-stage gap on Draw <-> Animate switches (H2/H3 outcome).
3. Verify and, if broken, fix: SMIL document in Animate mode shows the frame
   and the strip (play/pause, scrub, time label); static document shows the
   frame and no strip.

## Non-goals
Playback engine changes, new Timeline features (tracks, keyframes), layout
redesign, Draw-mode behaviour changes, export, MCP changes, live driving by the
worker.

## Files allowed to touch
- `rust/crates/vexel-app/src/**`, `rust/crates/vexel-app/tests/**`
- `artifacts/WO-RUST-V1-ANIMATE-FIX-01/**`

## Protected paths
Every other crate, workspace/lock manifests, `Cargo.toml` of any crate,
fixtures and goldens (read only; use existing `rust/fixtures/v1/compose/
compose-v1.svg`, `rust/fixtures/v1/anim/loop.svg`,
`rust/fixtures/v1/playback/loop2.svg`; static docs may be inline strings in
tests), C/ObjC, `docs/**`, `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other work
orders, git HEAD/index. No unsafe, new deps, network, installs, GUI launch,
commit, stage, reset, deletion. Files under 800 lines.

## Acceptance criteria (all headless, no display)
1. Timeline text: a pure function maps (no document | static | animated) to the
   three distinct strings above; tests cover all three, and a render of the
   Animate layout for a static document contains no "No document to animate"
   text path (assert on the chrome text, not pixels).
2. Layout/model state: with compose-v1.svg open and `Mode::Animate`, the
   chrome timeline text is the static string and `layout.stage` is non-empty;
   for loop.svg it is the animated string and `strip::layout(stage)` is Some and
   `driver.strip_visible()` is true; for the static document the strip is not
   visible.
3. Stage hash: for a static document, the stage pixels produced for
   `Layout::new(w,h,Mode::Animate)` (after the compose worker finishes, via
   `DocumentView::poll`) equal `compose_document` output at that stage size, and
   the pixels equal the Draw-mode stage content for the common area when
   composed at the same size (state the comparison used; hash with the repo's
   existing hash helper if any).
4. No blank gap: after `select_mode` Draw->Animate with a composed document,
   a `blit_document` of the very next frame (worker result not yet delivered)
   still paints document pixels on the stage (or, if the fix composes
   synchronously, the first frame is already correct). Test fails before the fix.
5. Repeated switching: Draw->Animate->Draw->Animate (>= 4 switches, stage
   sizes alternate) with polling between, ends with the document composed at
   the current stage size and `status` not stuck at "Composing"; no frame in the
   sequence is a blank stage once the document has composed once.
6. Animated document: loop.svg in Animate mode renders a non-blank stage and
   `present_animated` is accepted for the current revision/stage after a
   switch (driver invalidation on stage-size change does not leave the stage
   static); the time label matches `strip::label`.
7. Existing `vexel-app` tests still pass (counts may only increase); new tests
   run without a display; `cargo tree -p vexel-app` shows no new package.
8. `result.md` (<=300 words): which hypotheses were confirmed, the fix chosen
   for the blank gap, exact Timeline strings, and what remains unexecuted
   (live window).

If an AC cannot be met, stop with the exact blocker; no test weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app -- --check`
- `python3 scripts/wo_validate.py WO-RUST-V1-ANIMATE-FIX-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Live check (coordinator, person at the Mac; not the worker)
1. Launch the built `vexel` binary with `rust/fixtures/v1/compose/compose-v1.svg`
   as the document and bring the window frontmost (not from a harness terminal).
2. Click Animate. Screenshot with `screencapture -l <CGWindowID>`: stage shows
   the 7 shapes at once, Timeline reads "No animation in this document", no
   strip.
3. Click Draw, then Animate, four times; screenshot after each Animate; stage
   never blank.
4. Repeat with `rust/fixtures/v1/anim/loop.svg`: strip (play/pause, scrub,
   m:ss.cs label) visible; play, pause, scrub once.
5. Archive screenshots under `docs/archive/live/` (coordinator only) and note
   results in the order's integration entry.

## Evidence
`artifacts/WO-RUST-V1-ANIMATE-FIX-01/`: `result.md`, `verification.json`,
`ac-map.json`, `changed-path-audit.json`.

## Worker and review
Worker: gpt-6-luna High, Standard speed, one writer, existing allowance only; stop
on quota errors. Candidate only until a fresh read-only reviewer (separate
session) reruns the frozen commands and checks each AC. The coordinator
integrates the exact patch, reruns main-tree checks, performs the live check,
then updates STATE.md, LEDGER.md and ledger.jsonl.
