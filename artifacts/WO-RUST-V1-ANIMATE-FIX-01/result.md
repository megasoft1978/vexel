# WO-RUST-V1-ANIMATE-FIX-01 result

Hypotheses: H1 confirmed (draw_timeline hardcoded the literal). H2 confirmed
(Animate stage height differs; show() dropped the composed frame; tests
no_blank_gap / repeated_switching / animated_document failed before the fix).
H3 not a defect (poll sets dirty before render; driver set_size invalidate
re-presents after compose; covered by animated_document test). H4: strip and
Timeline text both use loop_seconds > 0 via the driver; driver is now synced
as soon as the revision is known (animation/stage.rs).

Fix: DocumentView keeps `previous` composed frame while a compose for a new
revision/stage is pending; blit_document draws it nearest-neighbour scaled to
fit, centred (stale but present). Stage-size-aware compose kept. Shell loop
logic moved to animation::stage::sync (used by main.rs and tests).

Timeline strings: none open "No document to animate"; static "No animation in
this document"; animated "Animated, loop m:ss.cs" (strip::format_time).
ChromeText.timeline / AppState.loop_seconds feed render::draw_timeline.

Tests: rust/crates/vexel-app/tests/animate_workspace.rs (7). Lib 219 + all
integration tests pass (test-output.txt).

Live: compose-v1.svg Animate screenshot (animate-static.png) shows stage and
"No animation in this document", no strip. loop.svg Animate and repeated
click switching not executed in the live window (focus/key injection failed).
Pre-existing, untouched: strip also draws in Draw mode; its label is tiny on
Retina.
