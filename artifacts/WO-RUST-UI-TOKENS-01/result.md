# WO-RUST-UI-TOKENS-01 result (worker, uncommitted; git is blocked in this harness)

Tests: vexel-ui 24 pass (debug and release, identical frozen hashes), vexel-app 273 pass. cargo tree: no new package; no unsafe.
fmt: new files clean; the baseline `cargo fmt --check` already fails on pre-existing files (draw.rs, tests.rs, lib.rs, ...).

## Doc deviations (doc table stale, tokens changed, gates unchanged)
- Light ramp: doc 95/98.5/100/93/90/87 is not monotonic with 3 L steps (AC1). Light is now 98.5/95/91.5/88/84.5/81 (falls), chroma 2.
- Light accent doc L48 #1074C6 is 4.27:1 on level 1 (AC2 needs 4.5): now L45 #006DBE. Light success/warning/error L45 (doc 50/52/46) for 4.5:1 on L95.
- Dark text.high doc L93 (13.54:1) fails the pre-existing 14:1 test pin: now L95.
- Light text.low L43 (doc 46) for 4.5:1 on level 2.
- Accented focus ring == accent (pre-existing test relation); non-accented dark ring is accent L+10.

## tests.rs pin updates
dark.window/panel/accent, light.window/panel_raised/canvas_well/accent, dark.panel_raised/canvas_well, dark_high.focus_ring, selection pixel [38,60,88] -> [40,54,71], toolbar 40->44, inspector 320->296.

## Goldens
Candidates only, in rust/fixtures/ui/tokens; review PNGs in artifacts/.../review. Not accepted.


## Token changes (old -> new) and pin updates

- Dark: window/panel/panel_raised/canvas_well/accent and the six surface levels are now LCH-derived (`#171A1D`, `#1E2125`, `#25282C`, `#2E3135`, `#36393D`, `#3F4247`); `text.high` L93 -> L95; `focus_ring` -> `#ADD9FF`.
- Light: ramp L98.5 down to L81 in 3.5 steps; accent L48 -> L45 (`#006DBE`); success/warning/error L45; `text.low` L46 -> L43.
- Pins in `tests.rs`: dark and light colour pins, the selection pixel [38,60,88] -> [40,54,71] (accent_subtle blend on surface 1), toolbar 40 -> 44, inspector 320 -> 296. The 14:1 and 5.5:1 contrast asserts are unchanged.
- Side effect: `canvas_well` now equals `window` in both themes (the well was previously darker).
- Coordinator: removed a stray `tests.rs-e` sed backup; the live screenshot is archived by the coordinator.
