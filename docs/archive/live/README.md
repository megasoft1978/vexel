# Live window evidence

Real-window screenshots of the Rust app (`screencapture -l <CGWindowID>` of the `vexel` window, macOS 27, Retina).

- `2026-10-16-compose-v1-after-live-fixes.png` — compose-v1.svg after WO-RUST-V1-LIVE-FIXES-01: title, top bar, status bar, Pen/Line glyphs, inspector "No selection", rounder curves.
- `2026-10-17-animate-workspace-blank-stage.png` — after clicking the Animate tab: stage blank and Timeline says "No document to animate" although the title shows `compose-v1.svg` is open (defect).
- `2026-10-17-animate-workspace-after-redraw.png` — a later frame: the stage renders the document in the Animate workspace, but the Timeline still says "No document to animate".

Input injection (`cliclick`, Accessibility trusted) reached the window for one click only; the app was never the frontmost process when launched from a harness terminal, and the bundled `Vexel.app` exits at startup with "macOS application startup failed" when launched from the harness. Native pointer drawing, shortcuts, panels and playback remain to be verified by a person at the Mac.

- `2026-10-17-ui-icons-01-app-rail-unchanged-before-chrome.png`: debug `vexel` Draw workspace; tool rail still uses the old glyph hacks because `vexel-app` does not consume the new icons until WO-RUST-UI-CHROME-01. Verified: baseline only. Unverified: tool rail with new icons at Retina/non-Retina.
- `2026-10-17-ui-icons-01-sheet-dark-2x.png`, `2026-10-17-ui-icons-01-sheet-light-1x.png`: icon contact sheets (66 icons plus state rows) from `vexel-app/tests/ui_icons_sheet.rs`, enlarged to 4 device px per pixel. Verified by eye: crisp, centred, readable. Unverified: live window use.
