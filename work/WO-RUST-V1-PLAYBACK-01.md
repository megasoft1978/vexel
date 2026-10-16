---
id: WO-RUST-V1-PLAYBACK-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: 4d399ccac18fbdd7d5afd34189acebc0e5a1ae93
spec_sha256: 606087a5d3a449c01ac6532d6d79dad5c042fc260c06adcfce84bcf30ae8f61c
parallel_safe: false
network: false
---
# Drive animation playback in the live window (timer, keyboard, MCP, timeline strip)

## Goal
The `vexel-app` animation modules (`animation::playback::{Playback, FrameQueue,
ManualClock, MonotonicClock}`, `animation::frame::{decode, AnimatedScene::render}`,
`animation::present`, GIF export job) are tested headlessly but the live window
shows only a static first frame, and `session::PlaybackHost` has no production
implementer. Make the window play: a timer-driven loop advances the clock,
evaluates the animation at clock time, and presents each frame through the
compose render path; play/pause/seek/loop work from the keyboard, from MCP
`animation_control` and from a minimal timeline strip. No new animation
features, authoring, export changes or renderer changes.

## Dependencies (blocking)
Starts only after `WO-RUST-V1-INTEGRATE-01` is ACCEPTED and on main (it
introduces `session/` with `AppDocument`, `revision()`, `edit`, `PlaybackHost`,
`McpHost` playback bookkeeping, and `compose::DocumentView::show`). If
`session/` is absent at start, stop and report; do not stub it. Already on main:
ANIM-01 `animation/*`. Read first: `work/WO-RUST-V1-INTEGRATE-01.md` and its
`artifacts/*/result.md`, `work/WO-RUST-V1-ANIM-01.md`, `animation/{playback,
frame,present}.rs`, `session/{mod,hooks,mcp}.rs`, `compose/{mod,present,
worker}.rs`, `main.rs` loop, `vexel-ui` `Slider`/`PointerState`.

## Assumptions
- `McpHost` keeps its own `playing/time/anchor` fields after INTEGRATE-01; this
  order replaces them with the single `Playback<MonotonicClock>` so keyboard,
  MCP and strip share one clock. If main already unified them, report.
- `AnimatedScene::render(time, &AtomicBool)` is the cancellable evaluate+raster
  step; its output must reach the screen through the same blit as static
  compose frames (`compose::blit_document`), not a second presenter. If
  `compose::Frame` cannot take an animated tile without changing `vexel-render`
  or compose pixel semantics, stop and report the gap.
- Loop length is `AnimatedScene::loop_seconds()` (<= `MAX_LOOP_SECONDS`); a
  document with no loop reports "no animation" and play is a no-op.
- Playback state is not document state: no `revision` bump, no dirty, not saved.

## Scope
1. Window-free `animation::driver` (new): owns `Playback<C: LoopClock>`, a decoded
   `AnimatedScene` cache keyed by `revision`, a `FrameQueue` (capacity 2) and a
   cancel `AtomicBool`. `tick(revision, bytes) -> TickOutcome` renders at most
   one frame per call at `playback.time()`, skips if `time` quantised to the
   frame step equals the last presented step, pushes to the bounded queue and
   returns the newest frame (older dropped). Frame step = 1/30 s (constant).
2. Cancel-on-edit: when `revision` differs from the cached scene, the in-flight
   render is cancelled, queued frames are dropped, the scene is re-decoded from
   the new bytes and playback time is clamped to the new loop (paused at 0 if
   the loop vanished). Frames from an older revision are never presented.
3. Production `PlaybackHost` implementer in the shell (the driver) so
   `AppDocument::request_frame` has a real consumer; fake host tests remain.
4. Shell loop: while playing, the existing event loop wakes on a ~16 ms timer
   (or the existing poll budget), calls `tick`, and marks the stage dirty only
   when a new frame was produced. Idle loop cost is unchanged.
5. Keyboard: Space toggles play/pause (when not typing in a text field and not
   claimed by a tool), Home seeks 0, Left/Right step one frame while paused,
   `L` toggles loop vs play-once (play-once pauses at the end frame; loop wraps).
   Bindings must not collide with `tool_key`/Cmd+Z/Cmd+S; list the final map
   in `result.md`.
6. MCP `animation_control` (play/pause/seek) routes to the same driver; replies
   keep their current JSON shape and error strings; `McpHost` loses its private
   clock fields.
7. Minimal timeline strip drawn by the shell using existing `vexel-ui` pieces
   (`Slider` for the scrub bar, existing button/icon/text drawing, no new widget
   crate code): play/pause button, scrub bar bound to `time/loop_seconds`, time
   label `m:ss.cs / m:ss.cs`. Hidden when the document has no loop. Dragging
   the scrub bar pauses and seeks; release keeps paused. If `Slider` lacks a
   usable pointer-drag path, implement strip hit-testing as a pure function in
   the shell and report the gap. No redesign, no keyframe editing UI.
8. Editing (tool, MCP, undo/redo) while playing does not pause; the next tick
   renders the edited document at the current time (Scope 2).

## Non-goals
Keyframe/timeline editing, audio, GIF/export changes, variable fps, onion skin,
stage click-to-seek, multi-document, UI redesign, performance targets,
live-window verification by the worker.

## Files allowed to touch
- `rust/crates/vexel-app/src/**`, `rust/crates/vexel-app/tests/**`, `README.md`
  (vexel-app section), `rust/crates/vexel-app/Cargo.toml` (existing path deps only)
- `rust/fixtures/v1/playback/**` (new fixtures only)
- `artifacts/WO-RUST-V1-PLAYBACK-01/**`

## Protected paths
Workspace/lock manifests (coordinator only), `vexel-render`, `vexel-svg-engine`,
`vexel-svg-animation*`, `vexel-gif`, `vexel-ui` (use as is), `vexel-project*`,
all other crates, C/ObjC, `tests/`, goldens, oracles, `docs/**`, `STATE.md`,
`LEDGER*.md`, `ledger.jsonl`, other work orders, git HEAD/index. No unsafe, new
external deps, network, installs, GUI launch, commit, stage, reset, deletion.
Files under 800 lines.

## Acceptance criteria (all headless: `ManualClock`, synthetic events, offscreen frames)
1. Deterministic frames: for a fixture loop (>= 2 s, one rect with a keyframed
   attribute) drive the driver with `ManualClock` through t=0, 0.5, 1.0, end,
   wrap; each `tick` frame hash equals the hash of
   `AnimatedScene::render(t)` blitted through the compose path for the same
   size, and hashes at distinct times differ.
2. Play/pause/seek: play advances time with the clock; pause holds it; seek
   clamps to `[0, loop]`, keeps play state; non-finite seek leaves state
   unchanged; play on a static document returns "no animation" and stays
   paused; loop wraps and play-once stops at the end frame.
3. Frame pacing: advancing the clock by less than one frame step yields no new
   frame; a 10 s stall yields one frame at the current time, not a backlog; the
   queue never exceeds capacity (assert `len()` after bursts).
4. Cancel-on-edit: an edit (`AppDocument::edit` adding a shape) between ticks
   bumps `revision`; the next frame hash equals the edited document at the same
   clock time; no frame of the old revision is returned afterwards; a pre-set
   cancel flag makes `render` return cancelled and presents nothing.
5. Loop vanishes: edit/undo removing the animation pauses at 0 and sets
   `strip_visible() == false`; undo back restores it.
6. Keyboard map: synthetic key events for Space/Home/Left/Right/L produce the
   expected `Playback` state; keys are ignored when a text field or tool owns
   focus; Cmd+Z/Cmd+S/tool keys unchanged (existing tests pass).
7. MCP: synthetic `animation_control` play/seek/pause go through the driver;
   replies equal the INTEGRATE-01 replies for the same inputs; `McpHost` has no
   private clock.
8. Timeline strip: pure layout/hit-test tests for play button, scrub bar
   (pointer x -> seconds, clamped) and label format `0:01.50 / 0:02.00`;
   strip hidden for static documents; drag pauses and seeks.
9. `PlaybackHost`: the production implementer receives (bytes, revision,
   seconds) via `AppDocument::request_frame`; playback never dirties
   (`is_dirty()`, `revision()`, `source()` unchanged after a play/seek session).
10. Existing `vexel-app` tests (including animation, GIF export, compose stale
    results, NATIVE-PANELS, SAVE, INTEGRATE) pass unmodified; counts only
    increase. New tests need no display.
11. `cargo tree -p vexel-app` shows no new external package; no `unsafe`.
12. `result.md` (<=400 words): key map, `McpHost` clock replacement, queue/cancel
    design, deltas vs INTEGRATE-01, unexecuted live checks.

If an AC cannot be met, stop with the exact blocker; no test weakening.

## Manual live-check list (coordinator, isolated session, not worker-verifiable)
1. Open an animated `.svg`/`.vexel`: strip appears with play button, scrub bar,
   label; static document shows no strip.
2. Space plays smoothly and loops; CPU idles when paused. Scrub drag pauses
   and updates live; Left/Right step frames.
3. Draw a shape while playing: it appears on the next frames, no flicker or
   stale old frame; Cmd+Z removes it.
4. MCP `animation_control` play/pause/seek moves the window; title stays clean.
5. GIF export while playing stays responsive; Save while playing is unaffected.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app -- --check`
- `python3 scripts/wo_validate.py WO-RUST-V1-PLAYBACK-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`; `git diff --check`

## Evidence
`artifacts/WO-RUST-V1-PLAYBACK-01/`: `result.md`, `verification.json`,
`ac-map.json`, `changed-path-audit.json`.

## Worker and review
Worker: gpt-6-luna High, Standard speed, one writer, existing allowance only; stop
on quota errors. Candidate only until a fresh read-only reviewer (diff + order
only) reruns the frozen commands and checks each AC. The coordinator integrates
the exact patch, reruns main-tree checks, does the live checks above, then
updates STATE.md, LEDGER.md and ledger.jsonl.
