# WO-RUST-UI-TIMELINE-02 result (candidate)

All ACs met headless except that two accepted-golden tests now fail by design (AC8/AC9): `ui_chrome_goldens::chrome_frames_match_first_party_goldens` (Animate frames: footer + new hint text) and `ui_timeline_golden::footer_frames_match_first_party_goldens` (new ruler/transport/diamonds/thumb). The coordinator re-blesses after the live check; the worker did not touch them. Candidates: `review/*.png` (12, 1280x800, light/dark, 1x/2x: animate-loop-1s, animate-12-tracks, draw-loop-strip).

Tests: vexel-app 344 passed, 2 failed (the goldens above); vexel-ui 46 passed. vexel-ui `cargo fmt --check` already fails at HEAD (pre-existing); vexel-app is clean.

Tooltip path: `layout/paint.rs` owns `tip_size`/`paint_tip` (extracted from `layout/tooltip.rs`, which delegates; hunk under 30 lines). `timeline/paint.rs` paints the disabled-control tip below the control, inside the footer, once `TimelineModel::tip_ready` holds (hover >= `motion::TOOLTIP_DELAY_NS` on the driver clock). It is not routed through `motion`/`main.rs` (protected).

Tokens: no `border_strong` token exists; the scroll thumb uses `control_edge`. Dark diamonds use `text_secondary` (3:1 pixel test passes); light unchanged. Labels `text_secondary` on `panel` pass 4.5:1 in both themes.

`chrome.timeline`: KEPT. Reader: `tests/animate_workspace.rs` (protected). Animate hint updated.

Updated pins (existing tests): ui_timeline_input (24 to 30 fps, wrap 23 to 29 frames, disabled controls not hit); ui_timeline_paint (dark group diamonds text_secondary; 1 s ruler has 3 majors); ui_polish_behaviour (Animate hint); animation/tests_ui (step while playing pauses and steps; step back at 0 wraps while looping; strip floats 8 px in and 8 px above the zoom cluster).

Notes: loops that are not a whole number of frames end-clamp to the loop end (not k/30). `main.rs` scrolls the footer for any point right/below the footer origin (including the inspector); `Timeline::wheel` (added, tested) has correct containment for WIRING/coordinator to adopt. Strip `label()` unchanged; quantisation applied at seek and draw.
