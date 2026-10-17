# WO-RUST-UI-MOTION-01 result

Status: candidate. Goldens are NOT accepted (human review pending).

## Coordinator amendment (AC2), verbatim
"a 120 ms hover-in transition is between 50% and 90% complete at t0+60 ms (the section 4.5 ease_out cubic-bezier(0.2,0,0,1) gives 87.8% there)". The 4.5 curves are unchanged; other AC2 sub-checks as written.
Also decided: the registry is `MotionRegistry` (`vexel_ui::Motion` is the token struct).

## What changed
- `vexel-ui/src/motion.rs`: `Easing`, `bezier` (Newton + bisection), `Transition`, `MotionRegistry` (fixed array of 64), `ControlVisual`/`ControlColors`, `ThumbSlide`, `Clock`.
- `vexel-app/src/motion/`: `MotionAdapter` (pointer, Tab focus, `redraw_after`), `MotionFrame`, paint helpers.
- `render.rs`: `render_with(.., &MotionFrame)`; `render()` wraps it with the rest frame. `main.rs`: adapter wiring. Diff: main +17/-1, render +20/-21.
- Tests: `tests_motion.rs` (9 incl. 1 pre-existing tokens test in the filter), `ui_motion_adapter.rs` (4), `ui_motion_budget.rs` (1), `ui_motion_swatch.rs` (2). Goldens: `rust/fixtures/ui/motion/states-{dark,light}@{1,2}x.png`.

## Changed expectations
Existing tests unchanged. Rail buttons and mode tabs now render from `ControlVisual`; at rest the pixels are the same as before (rest fill panel_raised, glyph disabled_text / text_primary).

## Reduced motion
Platform layer has no `accessibilityDisplayShouldReduceMotion` accessor. `MotionAdapter::new(bool)` / `MotionRegistry::new(bool)` accept it; `main.rs` passes constant `false`. GAP: wire the platform read later. With the flag every duration is 0 and `next_deadline()` is `None`.

## Redraw cost model
Events only set targets (O(1), no heap). While any transition is active `redraw_after()` is `Some(<=16 ms)` and the loop marks dirty; idle is `None`. The existing loop still polls events every 16 ms (pre-existing); measured idle debug-app CPU 0.4-0.7%. Frame compose for 64 animating controls: 2.4 us median release, 0 allocations (counting allocator).

## Golden tolerance
Software shapes, icon masks and scalar glyphs only: tolerance 0, PNG byte equality. No CoreText.

## Gaps
No keyboard-focus model exists in the shell: Tab only cycles the adapter's focus ring across tabs and rail (no activation). Pointer leaving the window sends no event, so hover persists until the next move. `vexel-ui` is in fmt-debt and `tests_open.rs`/new workspace members trip the repo fmt gate at baseline; new files are rustfmt-clean. Live check (reduce-motion toggle, hover screenshots) left to coordinator.
