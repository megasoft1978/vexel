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
