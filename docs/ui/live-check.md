# Live check (needs an unlocked Mac session)

Automated clicking cannot reach the app while the screen is locked
(`CGSSessionScreenIsLocked`), so these rows are verified by tests and worker
screenshots only. Run them once at the Mac and record the result in
`docs/archive/live/` (screenshots) and `STATE.md`.

Build and launch:

```sh
cargo build --release -p vexel-app
./build/rust/aarch64-apple-darwin/release/vexel rust/fixtures/v1/anim/loop.svg
```

Capture a window: find its id with Python `Quartz.CGWindowListCopyWindowInfo` (owner
`vexel`), then `screencapture -x -l <id> out.png`.

| # | Check | Expected |
|---|-------|----------|
| 1 | Switch System Settings > Appearance to Dark, relaunch | Dark tokens: six elevation levels, readable text, accent adapts; artboard well distinct |
| 2 | Click Animate, then Draw | Stage stays visible; timeline footer shows transport, ruler, tracks, diamonds, playhead |
| 3 | Hover toolbar buttons, tool rail, section headers | Hover fill eases in (~120 ms); tooltip after ~500 ms |
| 4 | Click Export | Popover anchored under the button with GIF / SVG / PNG rows; Esc closes; each row opens the native Save panel |
| 5 | Cmd+S, Cmd+Shift+S, Cmd+O | One Save panel per Cmd+S (no double fire); Open lists `.svg` and `.vexel` |
| 6 | Draw a rectangle, Cmd+Z, Cmd+Shift+Z, then the Undo/Redo buttons | Same results as the keys; Redo enables only after an undo |
| 7 | Space plays/pauses, Home/Left/Right step, drag the ruler | Playback loops; steps land on the 1/30 s grid in both workspaces |
| 8 | Scroll over the inspector and over the footer | Only the footer scrolls its rows |
| 9 | Start a GIF export and quit mid-way | No `.vexel-export-*` directory is left beside the target |
| 10 | Real Codex client over `--mcp-preview` with `--grant-dir` | Tools list, draw, save, export; activity shown in the status bar |

## Results (2026-10-17, unlocked session, automated clicks via cliclick)

| # | Result | Evidence |
|---|--------|----------|
| 2 | PASS: Animate shows transport, ruler, tracks, diamonds, playhead; stage stays visible | scratchpad row2-animate2.png |
| 4 | PASS (Esc): popover anchored under Export with GIF/SVG/PNG rows; a real Escape (System Events key code 53) closed it and focus returned to Export. Outside click also closes it. Earlier 'Esc failed' was the cliclick kp:esc tool not delivering the key, not the app. Native Save panels not opened. | scratchpad repro-open.png, repro-esc2.png |
| 1, 3, 5-10 | NOT RUN | |

No open defect for check 4. Model test ui_wiring_export.rs covers Esc.

## Round 2 results (2026-10-17, unlocked, System Events key codes and cliclick)

| # | Result | Evidence |
|---|--------|----------|
| 7 | PASS: after releasing Export focus (two Escapes, click on inspector), Space started playback (readout 0:00.53, pause icon), second Space paused (readout identical across two captures). Home=0:00.00, Right x2=0:00.07, Left x1=0:00.03 (1/30 s grid). | scratchpad row7-*.png |
| 3 | INCONCLUSIVE: synthetic cliclick moves over Undo produced no hover fill and no tooltip after 0.8 s, nor after a retry with a 1 px wiggle. Needs a real mouse at the Mac. | scratchpad row3-*.png |
| 5 | NOT RUN: Cmd+S on a tracked fixture (`loop.svg`) could overwrite the repo file if it saves directly instead of opening a panel. Needs a supervised run. | |
| 6 | NOT RUN: needs an in-memory edit first (rectangle tool drag); not done this round. | |
| 8 | NOT RUN: no reliable scroll-event tool confirmed. | |
| 1, 9, 10 | OWNER: appearance change, export-to-file and quit, external Codex client. | |

## Round 3 (2026-10-17)

- Row 5: Cmd+O via System Events produced no Open panel in a screenshot. Not verified. Cmd+S left untested: it could write the tracked fixture. Repo verified unchanged (`git status` clean, fixture sha matches).
- Row 3: still inconclusive (synthetic moves give no hover feedback).
- Rows 6 and 8: not run. Row 6 needs an in-memory edit on the open document. Row 8 needs a scroll-event check that was not executed.

## Headless evidence (model level, no screen touched; 2026-10-17)

Run: `cargo test -p vexel-app --test <file>` for ui_polish_behaviour (11 pass), ui_timeline2_footer (11), ui_wiring_keys (4), ui_wiring_toolbar (5), ui_wiring_export (6), ui_timeline_input (6). All pass.

- Row 3 (tooltip): `tooltip_appears_after_500_ms_and_goes_on_press_or_leave`, `reduced_motion_still_delays_the_tooltip`, `tooltip_rects_stay_inside_the_window_for_every_target`. Timing and placement verified in the model. Visual hover on the real window still needs the Mac.
- Row 6 (undo/redo): `redo_is_enabled_only_after_an_undo_and_a_load_resets_it`. Redo gating verified in the model. Key-driven undo on the real window still needs the Mac.
- Row 8 (scroll): `wheel_over_the_whole_footer_scrolls_and_clamps_and_never_reaches_the_stage`, `twelve_rows_in_six_show_a_proportional_thumb_that_tracks_scroll`. Footer wheel behaviour verified in the model. Real wheel events still need the Mac.
- Row 5 (file panels): export and key routing verified in ui_wiring_export and ui_wiring_keys. Native Save/Open panel counts cannot be tested headless: still needs the Mac.

## Round 4: real window, scratch copy only (2026-10-18)

Ran on a scratchpad copy of loop.svg; the repo fixture was never opened for editing or saving.

- Row 3 (tooltip): PASS. Hovering Undo shows the hover fill and "Undo ⌘Z". Rail Rectangle: no tooltip at 0.25 s, "Rectangle R" tooltip at 1.0 s.
- Row 6 (undo/redo): PASS. Cmd+Z removes the rectangle and enables Redo; Cmd+Shift+Z restores it and disables Redo. Undo and Redo buttons produce the same artboard states.
- Row 8 (scroll): INCONCLUSIVE. Real wheel events over the inspector and the footer changed nothing, because the footer has four rows and no overflow.
- Row 5 (file panels): INCONCLUSIVE. Cmd+S left the document Unsaved with no visible Save panel; the window count rose by one, source unknown. Cmd+Shift+S and Cmd+O were not run after the earlier stuck Open panel.
- Row 7: PASS (round 2).

## Round 5: real window, scratch copies only (2026-10-18)

- Row 8 (scroll): PASS. On a 14-track scratch file, a real wheel over the footer scrolled the rows (first visible row changed to r9) and showed a scroll thumb. Inspector control not run.
- Row 5 (Cmd+S): PASS for "one Save panel per Cmd+S". On an edited scratch copy, one Save panel appeared (window count +1), no file was written, and Escape cancelled it. Cmd+Shift+S and Cmd+O not run.
- Observation: the system appearance now reads Light; the app follows it. Row 1 (dark mode) still needs an owner check in both appearances.
