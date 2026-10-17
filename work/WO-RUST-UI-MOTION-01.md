---
id: WO-RUST-UI-MOTION-01
revision: 1
status: READY
milestone: UI-MODERN-2026-10
owner: project-owner
executor: gpt-6-luna
base_sha: 76dc7bef0762fac199d292cb96d2e51ba33dbcc3
spec_sha256: cfaa6c866fc30f193ce636e77ad2b084ca828695d34ddedcb5daced985c2ed19
parallel_safe: False
network: false
---
# Hover, press, focus states and 120-180 ms micro-transitions

## Goal
Add hover, pressed, focus and disabled visual states and eased micro-transitions
(`docs/ui/design-direction-2026-10.md` sections 4.5 and 4.6) to the immediate-mode UI loop, without ever
delaying input or document redraw, and prove the frame budget. Evidence:
critique item 11.

## Dependencies
Requires WO-RUST-UI-TOKENS-01 merged (motion constants, state colours). Does not
require CHROME: it works on any rect-registered control and exposes a small
API CHROME adopts later. Read first: `docs/ui/design-direction-2026-10.md` 4.5/4.6; `vexel-ui/src/interaction.rs`,
`widget_input.rs` (`PointerState`), `vexel-app/src/main.rs` (event loop, dirty
flag, redraw scheduling), `vexel-app/src/model.rs` (`hit_test`, `Target`).

## Assumptions
- Transitions are pure functions of (start time, now, duration, easing): no
  timers in the model. The loop asks `Motion::next_deadline()` and schedules one
  redraw at the soonest animating control; with nothing animating the app is
  idle (no continuous redraw, no CPU use).
- Time source is injected (`Clock` trait / `u64` ns parameter) so tests are
  deterministic.
- Reduced motion: when `accessibilityDisplayShouldReduceMotion` is true (read by
  the existing platform layer, passed in as a bool) all durations are 0. If the
  platform read is not already exposed, accept a bool and report the gap.
- Easing: cubic-bezier solver (Newton plus bisection fallback) with the three
  curves of 4.5; never linear.

## Scope
1. `vexel-ui/src/motion.rs`: `Easing`, `Transition<T>` for f32/colour channels,
   `Motion` registry keyed by control id (bounded, fixed array of 64), state
   machine rest/hover/pressed/focus/disabled with the durations: hover in
   120, out 90, press in 80, release 120, focus ring 120, toggle/segment thumb
   180. Pointer-driven drags are never animated.
2. `vexel-app/src/motion/`: adapter feeding pointer move/down/up and keyboard
   focus from the loop into `Motion`, producing per-control interpolated fill
   and glyph colours read by the renderer through a `ControlVisual` struct.
3. Redraw scheduling: `main.rs` call-site wiring (at most 30 lines) requesting
   redraw only while a transition is active.
4. Segmented-control thumb slide helper (position interpolation) for CHROME.

## Non-goals
Layout, new widgets, icons, tokens, panel resize animation, document/canvas
animation, spring physics, vsync/CADisplayLink changes.

## Files allowed to touch
- `rust/crates/vexel-ui/src/{motion,tests_motion}.rs` (new),
  `rust/crates/vexel-ui/src/interaction.rs`
- `rust/crates/vexel-app/src/motion/**` (new), `rust/crates/vexel-app/tests/ui_motion_*.rs` (new)
- `rust/fixtures/ui/motion/**` (new), `artifacts/WO-RUST-UI-MOTION-01/**`
- lib.rs registration lines; `main.rs` call-site wiring only (see below)

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
1. Easing: for each curve, `f(0)=0`, `f(1)=1` within 1e-4, monotonic on 1000
   samples (the ease-out, standard and ease-in curves), and the solver agrees
   with a reference table of 11 documented points within 1e-3.
2. Durations: a hover transition started at t0 is at 0 at t0, at 1.0 at
   t0+120 ms, and 50-80% at t0+60 ms (ease-out front-loaded); a re-entry
   mid-transition continues from the current value (no jump larger than 0.02).
3. Interrupt safety: 10,000 random pointer enter/leave/down/up sequences
   (seeded) never panic, never leave a control in a non-terminal state after
   all inputs stop and 300 ms pass, and never exceed the 64-control bound
   (excess controls snap to their target state).
4. Reduced motion: with the flag set every transition completes in the same
   frame (duration 0) and `next_deadline()` is `None`.
5. Idle: with no active transition `next_deadline()` is `None` and the loop
   adapter requests no redraw (assert via the adapter's returned
   `Option<Duration>`).
6. Input is never delayed: pointer-down on a control sets its pressed target in
   the same call; the document-affecting action (hit-test result) does not
   depend on transition progress (test: action fires on the first event even
   when the visual is at 0).
7. Frame budget: composing one frame's `ControlVisual`s for 64 animating
   controls costs at most 50 microseconds median over 10,000 iterations in the
   release test profile (documented, test skipped with a printed reason in
   debug), and performs zero heap allocation per frame (verified by a
   counting allocator in the test or by a fixed-array-only API assertion).
   The existing `perf_gate.rs` thresholds are not raised.
8. Visual states: a swatch sheet of rest/hover/pressed/focus/disabled/selected
   for button, icon button, segmented control and text field (colours from
   `Tokens`) matches first-party goldens (policy below, candidates).
9. Existing tests pass; no new external package; no unsafe. `result.md` states
   how the platform reduced-motion flag reaches the code and the redraw cost
   model.

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
- `cargo +stable test -p vexel-app --release --test perf_gate --locked --offline`
- `python3 scripts/wo_validate.py WO-RUST-UI-MOTION-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Live check (coordinator, a person at the Mac; not the worker)
1. Run the built `vexel`. Hover the Draw/Animate tabs and tool rail, press and release, tab through focus: transitions visible, no lag versus pointer.
2. Idle check: with the pointer still for 10 s, Activity Monitor shows near-zero CPU for the process.
3. Toggle System Settings > Accessibility > Display > Reduce motion and verify transitions become instant. Screenshot hover and focus states.
Record each viewed screenshot in `docs/archive/live/` as
`YYYY-MM-DD-ui-motion-01-<what>.png` with a line in
`docs/archive/live/README.md` (what it shows, what was verified, what remains
unverified). Launch from a Finder/Terminal session in the foreground, not from
a harness; find the window id with Quartz `CGWindowListCopyWindowInfo`, then
`screencapture -x -l <id> out.png`, and view the PNG before accepting goldens.

## Evidence
`artifacts/WO-RUST-UI-MOTION-01/`: `result.md` (<=400 words), `verification.json`,
`ac-map.json`, `changed-path-audit.json`, `review/` (candidate golden PNGs).

## Worker and review
Worker: gpt-6-luna Medium, Standard speed, one writer, existing allowance
only; stop on quota errors. Candidate only until a fresh read-only reviewer
(separate session, Luna or Sol Medium, diff + order only) reruns the frozen
commands and checks each AC. The coordinator integrates the exact
patch, reruns main-tree checks, does the live check, accepts goldens, then
updates STATE.md, LEDGER.md and ledger.jsonl.
