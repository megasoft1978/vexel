---
id: WO-RUST-UI-TIMELINE-02
revision: 1
status: READY
milestone: UI-MODERN-2026-10
owner: project-owner
executor: gpt-6-luna
base_sha: 6c7544de24d19e75cca67c7f48dfe9ce5077e634
spec_sha256: 6f78be3d6d84aa9ebaf32f5f9056719b95d69c1c0c94236d2f56ce23733ee863
parallel_safe: False
network: false
---
# Timeline follow-ups: one frame grid, footer overflow, ruler, transport, Draw strip

## Goal
Fix what the review of TIMELINE-01 found (screenshots
`docs/archive/live/2026-10-17-ui-timeline-01-after-animate-loop.png`,
`...after-draw-loop.png`): Animate steps 1/24 s (`timeline/model.rs` FPS)
while the driver steps 1/30 s (`animation/driver.rs` FRAME_STEP); the footer
gives no cue that rows scroll; the 1 s loop ruler has three ticks and a label
under the playhead handle; unselected diamonds are dim on the dark footer; the
transport mixes unrelated controls; `status hint` says "drag the strip" in
Animate; and the Draw-mode strip is broken (tiny 0:00.00 text, a solid blue
block over the zoom bar at the stage bottom). `vexel-app` only.

## Dependencies
CHROME, MOTION, TIMELINE-01, POLISH-01 merged (HEAD). WIRING-01 runs
concurrently and owns `main.rs`, `model.rs`, `history.rs`, `file_ui.rs`,
`motion/**`: do not touch them. Read first: `timeline/*`,
`animation/{driver,keys,strip,playback}.rs`, `layout/{state,paint}.rs`
(footer, status hint), `chrome.rs` (`timeline` text field).

## Assumptions
- One constant: `animation::driver::FRAME_STEP` = 1/30 (render grid, exporter
  max fps 30). `timeline::model::FPS` is removed or re-exported from it.
  Frame index = `round(time / FRAME_STEP)`; stepping and seeking land on
  `k * FRAME_STEP`; display times are quantised to it.
- Step works playing or paused in both workspaces: stepping while playing
  pauses, steps, stays paused; wraps when looping, clamps otherwise (existing
  rule). Draw strip and Animate share one `Driver::step`.
- Tooltips: the POLISH-01 tooltip layer takes a target; for timeline buttons
  it is used through a timeline `Hit` to (rect, text) call inside
  `timeline/paint.rs`. If the layer cannot take non-`Target` input without
  touching `layout/tooltip.rs` beyond a 30-line hunk, STOP and report.
- `Model.chrome.timeline` is deleted only if a grep shows it is read nowhere
  but its own test; else keep and list the reader.

## Scope
1. Unify stepping as above in `timeline/`, `animation/{driver,keys}.rs`.
2. Footer overflow: when rows exceed the visible rows draw a slim scroll
   indicator (thumb in the name column edge, token `border_strong`); wheel and
   trackpad over the whole footer scroll rows (clamped, no stage zoom); footer
   resize splitter drag clamps to `FOOTER_MIN..` stage min (existing constants)
   and never leaves the stage below `STAGE_MIN_H` (hunk in `layout/state.rs`).
3. Ruler density: choose the tick step from pixels per second; loops under 3 s
   get 0.25 s ticks (labels at 0.5 s) and frame ticks when a frame is at least
   6 px; major labels use `text_secondary` meeting 4.5:1 on the ruler; the
   label under the playhead handle (+- handle half-width) is not drawn.
4. Unselected diamonds in dark mode use a token (`text_secondary`) with 3:1 on
   the lane; selected unchanged. No new token; if none works STOP (protected).
5. Transport grouping: left cluster step-first, step-back, play, step-next,
   step-last, loop; right cluster speed, then Ease and + Key as one disabled
   group (single rounded container). Disabled controls show a tooltip
   (`disabled_hint` text) via the tooltip layer after the polish delay.
6. Delete dead `chrome.timeline` per assumption. Animate hint reads
   "Space plays, drag the ruler to scrub" (one hunk in `layout/paint.rs`).
7. Draw strip stays Draw-only. Restyle with tokens: 28 px rounded level-2 bar,
   readout at body size (not tiny), thumb a 2 px accent rule, no solid blue
   block; it sits above the zoom cluster with an 8 px gap (or the cluster moves
   up), never overlapping any other control at 720x480 and 1280x800.

## Non-goals
Timeline editing, new tracks, keyframe drag changes, easing UI, playback
timing changes beyond the grid, other chrome, other crates, export.

## Files allowed to touch
- `rust/crates/vexel-app/src/timeline/**`, `src/animation/{driver,keys,strip}.rs`
  and their tests, `src/chrome.rs` (timeline field only),
  `src/layout/{state,paint}.rs` (footer clamp, hint, strip placement hunks)
- `rust/crates/vexel-app/tests/ui_timeline2_*.rs` (new),
  `rust/fixtures/ui/timeline2/**` (new), `artifacts/WO-RUST-UI-TIMELINE-02/**`

## Protected paths
Every `Cargo.toml`, lock files, other crates (including `vexel-ui`,
`vexel-v1-document`), `main.rs`, `model.rs`, `history.rs`, `file_ui.rs`,
`motion/**`, C/ObjC, `tests/` other than the new files, accepted goldens and
oracles, `docs/**`, `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other work
orders, git HEAD/index. No unsafe, new deps, network, installs, GUI launch by
the worker, commit, stage, reset, deletion. Files under 800 lines.

## Acceptance criteria (headless)
1. One constant: grep finds a single frame-step definition; for 5,000 seeded
   (time, direction, loop on/off, playing/paused) cases both workspaces
   produce identical times, every result equals `k/30` within 1e-9, wrap and
   clamp match the old rule; stepping while playing leaves it paused.
2. Footer: with 12 rows in 6 visible a scroll thumb is painted, its length
   proportional and position matching `scroll`; wheel deltas over lanes, name
   column and transport-free footer area scroll and clamp; none leaves the stage
   zoom changed; no thumb when all rows fit. Splitter drag to every extreme
   keeps footer >= FOOTER_MIN and stage >= STAGE_MIN_H (1,000 seeded drags).
3. Ruler: loops of 0.5, 1, 2, 10, 60 s at footer widths 400..1600 give strictly
   increasing ticks, no label overlap, 0.25 s ticks under 3 s, labels pass
   4.5:1, no label rect intersects the playhead handle rect at any time.
4. Diamonds: dark unselected diamond vs lane contrast >= 3:1 (pixel test), selected
   and light mode unchanged.
5. Transport: button rects are in the specified order, two clusters, no overlap
   at footer widths 400..1600; Ease/+ Key disabled and not hit-testable;
   hovering a disabled one for 500 ms (fake clock) shows its hint text inside
   the window.
6. `chrome.timeline` removed with all tests green (or reader documented);
   Animate hint string equals the new text; Draw hint unchanged.
7. Draw strip: at 720x480 and 1280x800 the strip rect, zoom cluster and status
   bar are pairwise disjoint; pixel test finds no fill of the accent colour
   larger than 4 px wide inside the strip except the thumb; readout font size
   equals body size; strip scrub, buttons and keyboard behaviour unchanged;
   absent in Animate.
8. Golden candidates: 1280x800 light and dark, Animate 1 s loop and 12-track
   file, Draw loop with strip, 1x and 2x, re-blessed with
   `VEXEL_BLESS_GOLDENS=1` into `artifacts/WO-RUST-UI-TIMELINE-02/review/` and
   recorded in `result.md` (existing goldens changed, why). Never accepted by
   the worker.
9. Existing tests pass, changed pins listed; no new deps; no unsafe.

If an AC cannot be met, stop with the exact blocker; no test weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-ui -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-ui --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-ui --package vexel-app -- --check`
- `python3 scripts/wo_validate.py WO-RUST-UI-TIMELINE-02 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Live check (coordinator, a person at the Mac)
Open loop.svg and a 12-track file, light and dark. Find the window with Quartz
`CGWindowListCopyWindowInfo`, `screencapture -x -l <id>`, view each PNG. Verify:
Left/Right step identically in Draw and Animate, playing or paused; the footer
shows a scroll cue and scrolls under the trackpad; footer splitter clamps; the
1 s ruler is dense and legible with no label under the playhead; diamonds read
in dark; transport clusters and disabled tooltips; hint text; Draw strip looks
clean above the zoom bar. Record as
`docs/archive/live/YYYY-MM-DD-ui-timeline-02-<what>.png` with README lines
(shown, verified, unverified), then accept goldens.

## Evidence
`artifacts/WO-RUST-UI-TIMELINE-02/`: `result.md` (<=400 words, incl. tooltip
path, token findings, `chrome.timeline` decision), `verification.json`,
`ac-map.json`, `changed-path-audit.json`, `review/`.

## Worker and review
Worker: gpt-6-luna High, one writer, existing allowance only; stop on quota
errors. Candidate until a fresh read-only reviewer reruns the frozen commands
and checks each AC. The coordinator integrates, merges with WIRING-01 (disjoint
paths), does the live check, accepts goldens, then updates STATE.md, LEDGER.md
and ledger.jsonl.
