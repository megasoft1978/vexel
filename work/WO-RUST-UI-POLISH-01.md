---
id: WO-RUST-UI-POLISH-01
revision: 1
status: READY
milestone: UI-MODERN-2026-10
owner: project-owner
executor: gpt-6-luna
base_sha: a73d59df3ca15bc882d4fa5af562ed055eee8a8f
spec_sha256: c180006e96e7ee46e5b7e9ccab533a19b03137504dc7855215186c37baea908f
parallel_safe: False
network: false
---
# UI polish after chrome review: fit, well, empty states, hover, status

## Goal
Close the gaps the independent review of the merged chrome found (live
screenshot `docs/archive/live/2026-10-17-ui-upgrade-merged-draw-light.png`):
(1) a 160x120 document is lost at 100% in a huge well and the status bar says
"Fit" while the zoom cluster says 100%; (2) the light well (#f7fafd) is nearly
the panel colour; (3) the empty inspector is dash-filled fields under heavy
header bands; (4) only 7 controls react to hover, tooltips are never painted,
and status readouts sit on different baselines; (5) the status hint is static
and Redo never enables. `vexel-app` only.

## Dependencies
CHROME, CANVAS, MOTION, ICONS, TOKENS merged (HEAD). The TIMELINE order runs
concurrently and owns the Animate footer/strip and `src/animation/**`: do not
touch them. Read first: `docs/ui/design-direction-2026-10.md` 3, 4.1, 4.6, 5;
`vexel-app/src/{layout,canvas,motion}/`, `render.rs`, `inspector.rs`, `chrome.rs`.

## Assumptions
- Fit cap: Fit never exceeds 400% (`FIT_MAX_ZOOM`), still never below the
  current minimum clamp. Plain 100% button and wheel zoom are unchanged.
  Above 100% the artboard is drawn by re-rendering the document at the view
  scale when the canvas paint path already does so; otherwise nearest-neighbour
  scaling of the cached raster. The worker reports which and the coordinator
  judges crispness in the live check. No new rendering path is added.
- Well colour uses existing surface levels: light mode well = level 0
  (`t.window`) darkened by an existing ramp step if one exists, else the
  `border`-mixed value computed in `canvas/paint.rs` with `lerp_color` (no new
  token). If contrast cannot be reached without a token, STOP and report the
  need: `vexel-ui/src/tokens.rs` is a protected path (coordinator edits it).
- `V1Document` exposes `undo()`/`redo()` only (no availability query), and
  `vexel-v1-document` is protected. The app keeps an in-memory history cursor:
  `redo_depth` increments on a successful undo, decrements on a successful
  redo, and resets to 0 on any new edit or document load. Undo availability
  keeps today's rule. Report that a core accessor is the proper follow-up.
- Tooltip delay 500 ms from the last hover change; dismissed on press, leave,
  or popover open. Reduced motion: no fade, still delayed.

## Scope
1. Fit: `View::fit` caps at `FIT_MAX_ZOOM` = 4.0; refit on stage/doc change
   unchanged. Status zoom readout reads the canvas state: `Fit` is dropped and
   the readout always shows the same percentage string as the cluster label
   (single formatter shared by both; Fit mode shows e.g. `400%`).
2. Well: distinct light well with a 1 px edge on the stage inner border
   (`t.border`), dark mode unchanged unless the same edge reads correctly.
3. Inspector empty state: with no selection show one "Select a shape" card
   (icon, title, one hint line) in place of Fill/Stroke/Transform sections;
   section headers lose the heavy band (transparent at rest, `panel_hover` on
   hover, 1 px divider above). With a selection nothing changes.
4. Hover: `MotionFrame` and `FOCUS_ORDER` grow to cover toolbar buttons,
   section headers (up to 8), splitters; the registry id map in `motion/mod.rs`
   is the single source and a unit test asserts every `Target` variant maps to
   a registered id. `MotionAdapter::pointer` hit-tests with
   `hit_test_with(.., popover_open)`. Disabled buttons never hover. Tooltips:
   paint `layout::tooltip(target)` as a rounded level-3 label (name plus
   shortcut) near the target, flipped to stay inside the window, after the
   delay. Status readouts (zoom, size, save, MCP word) share one baseline
   computed once from `l.status`.
5. Status hint by context: no document "Open an SVG to begin (Cmd+O)"; Draw by
   tool (Select "Click a shape to select it", Pen/Rectangle/Ellipse/Line a
   drawing hint); Animate keeps its text. Redo enabled iff `redo_depth > 0`,
   Undo per existing rule; both feed the toolbar button enabled state and the
   keyboard Cmd+Shift+Z path (no behaviour change when disabled).

## Non-goals
Animate footer/strip/timeline, `animation/**`, new tokens unless blocked as
above, new tools, persistence, document edits, other crates.

## Files allowed to touch
- `rust/crates/vexel-app/src/{layout,canvas,motion}/**` except
  `src/animation/**`, `render.rs`, `inspector.rs`, `chrome.rs`, `model.rs`
- `rust/crates/vexel-app/tests/ui_polish_*.rs` (new), `rust/fixtures/ui/polish/**`
  (new), `artifacts/WO-RUST-UI-POLISH-01/**`
- `main.rs` call-site wiring (at most 40 changed lines), single-line `lib.rs`
  `mod` additions.
Existing goldens may be updated only by candidates in `artifacts/.../review/`
(see policy); do not overwrite accepted fixtures.

## Protected paths
Every `Cargo.toml`, workspace/lock files, all other crates (including
`vexel-ui` and `vexel-v1-document`), `src/animation/**`, C/ObjC, `tests/`,
accepted goldens and oracles, `docs/**`, `STATE.md`, `LEDGER*.md`,
`ledger.jsonl`, other work orders, git HEAD/index. No unsafe, new deps,
network, installs, GUI launch by the worker, commit, stage, reset, deletion.
Files under 800 lines.

## Acceptance criteria (headless)
1. Fit: for docs 160x120, 40x40, 1000x800, 4000x3000 and stages from 260x140
   to 3000x2000, `View::fit` zoom equals min(fit_x, fit_y, 4.0), the artboard
   is centred and inside the stage with the margin; 160x120 in a 1400x900 stage
   gives exactly 400%. 5,000 seeded cases never panic or yield NaN.
2. Readout: for Fit, 100%, wheel-zoomed and tool-zoomed states the status zoom
   string equals the cluster label string (single shared formatter test).
3. Well: pixel assertion on a frame (light and dark) that the stage fill differs
   from the panel fill by at least 6 sRGB-luma levels in light mode and has a
   1 px `border` edge row/column; contrast of text over the well unchanged.
4. Inspector: empty selection lays out exactly one card and no Fill/Stroke/
   Transform rows; selecting a shape restores prior row geometry (existing pins
   unchanged); header hit rects are unchanged; no header band fill at rest
   (pixel assertion).
5. Motion coverage: every `Target` (tool, segment, button, section header,
   splitter) has a unique registry id; hovering each yields a changed
   `ControlVisual` hover fill (disabled buttons excepted); with a popover open
   a point under the popover hits the popover, not the chrome beneath; focus
   order and existing motion tests still pass.
6. Tooltips: with a fake clock, no tooltip before 500 ms, present after, gone
   on press/leave; text equals `layout::tooltip`; the rect stays inside windows
   of 720x480 and 1280x800 for every target; frame pixels differ from the
   no-tooltip frame only inside the tooltip rect.
7. Status: zoom, size, save and MCP text share one baseline (assert equal text
   rects' vertical centre within 1 px); hint strings match the table for no
   document, each tool and Animate.
8. History: a scripted sequence edit, edit, undo, undo, redo, edit yields redo
   enabled after each undo, still enabled after the first redo, disabled after
   the final edit, and Redo dispatch while disabled is a no-op; load resets.
9. Golden candidates: full window 1280x800 light and dark, Draw, compose-v1.svg
   (160x120) and empty state, plus inspector empty/selected and one tooltip
   frame, 1x and 2x; re-blessed with `VEXEL_BLESS_GOLDENS=1` into
   `artifacts/WO-RUST-UI-POLISH-01/review/` and recorded in `result.md`
   (which existing goldens changed and why). Tolerance per the CHROME
   golden policy (CoreText text 4/0.5%/24, rasters exact). Never accepted by
   the worker.
10. Existing tests pass, pins updated and listed; no new deps; no unsafe.

If an AC cannot be met, stop with the exact blocker; no test weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-ui -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-ui --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-ui --package vexel-app -- --check`
- `python3 scripts/wo_validate.py WO-RUST-UI-POLISH-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Live check (coordinator, a person at the Mac)
Launch compose-v1.svg in light and dark, Draw. Screenshot at the default size
(find the window with Quartz `CGWindowListCopyWindowInfo`, then
`screencapture -x -l <id>`; view each PNG). Verify: 160x120 fills the well at
Fit and is crisp; status zoom equals the cluster; light well is visibly
distinct; empty inspector shows one card; hover on every toolbar/rail/section
header/splitter; a tooltip appears after about 0.5 s; status baselines align;
hint changes with tool; edit then Undo enables Redo, Redo restores. Record
screenshots as `docs/archive/live/YYYY-MM-DD-ui-polish-01-<what>.png` with
README lines (shown, verified, unverified), then accept goldens.

## Evidence
`artifacts/WO-RUST-UI-POLISH-01/`: `result.md` (<=400 words, including the
fit-render method used, whether a token was needed, and the core-accessor
follow-up), `verification.json`, `ac-map.json`, `changed-path-audit.json`,
`review/`.

## Worker and review
Worker: gpt-6-luna High, one writer, existing allowance only; stop on quota
errors. Candidate until a fresh read-only reviewer reruns the frozen commands
and checks each AC. The coordinator integrates the patch, merges after the
TIMELINE order, does the live check, accepts goldens, then updates STATE.md,
LEDGER.md and ledger.jsonl.
