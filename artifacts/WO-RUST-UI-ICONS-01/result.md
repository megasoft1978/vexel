# WO-RUST-UI-ICONS-01 result (candidate)
Choice (a): paths authored in 16-unit space; rasteriser (`icons_draw.rs`) scales by s; mask is 16*s px. `IconKind` ids 1..53 stable, 13 kinds appended (COUNT=67), `icon_definitions()` shape unchanged. Parser limit, `MAX_PATH`/`MAX_SEGMENTS` unchanged; legacy 24-unit masks untouched.
Deviation: device stroke is nominal 1.5 units rounded UP to an even pixel count (2/4/6 px at 1x/2x/3x, 2.0 units effective), same for all segments; axis-aligned segments snap to whole pixels. A 1.5 px (odd) stroke cannot satisfy AC2 (2x/1x covered ratio 3..5) and AC4 (<=45% partial) together. Coordinator may prefer another tradeoff.
Fringe: pixels with <=10/64 samples dropped.
Pins updated in `tests.rs` (outside listed paths): icon count 53->66, asset_hash 1x/2x/3x re-pinned (old C-oracle values in comment).
Tolerance-0 goldens: `rust/fixtures/ui/icons/icon-sheet-{dark,light}-{1,2}x.rgba` (CANDIDATES, not accepted).
Weak icons: eyedropper, gradient (reads as columns), timeline (resembles align-left), group, ruler, pause (boxes fill in at 1x), actual_size.
fmt --check fails on pre-existing unformatted files (draw.rs, tokens.rs, tests.rs, ...); new files are clean.
Not done: app does not consume icons (CHROME order); no after-screenshot of tool rail; fractional ui_scale crops masks (ceil scale, pre-existing).
