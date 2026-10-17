# WO-RUST-UI-CHROME-01 result (candidate; goldens NOT accepted)

New layout module `vexel-app/src/layout/` (mod, state, paint, tests). `model.rs::Layout` is now a re-export;
`render.rs` delegates chrome painting; `inspector.rs` rewritten as collapsible sections.

New minimum window: 720 x 480 pt (was 440 x 300). Inspector absent below 900 pt wide.
Optional integrations present: MOTION no, CANVAS no, TIMELINE no (stage blit, strip and the old timeline placeholder are unchanged).

## Changed expectations (each recorded)
- model.rs `compact_bounds_hide_inspector_and_keep_stage_valid`: sizes 520x420/439/299 -> 720x480/719/479; stage >= 260 (was 400).
- model.rs `inspector_is_omitted_until_its_fixed_content_fits` replaced by `inspector_appears_from_900_points_wide`: inspector no longer hides for short windows (sections clip instead).
- model.rs `compact_painted_control_geometry...`: 440x300 -> 720x480; `activity`/`timeline` -> `status`/`footer`; the 900x520 no-inspector assertion dropped with the height rule.
- model.rs `standard_workbench...`: `timeline` field renamed `footer`.
- model.rs `line_tool_is_the_fifth_rail_slot...`: size list 440x300 -> 720x480.
- Layout fields: `activity` removed, `timeline` -> `footer`; `ChromeText` gained `state: AppState`; `Model` gained `tool`, `mcp`, `ui`; `Target` gained variants.
- `safe_render_geometry` clamps 440x300 -> 720x480 (main.rs).
- Legacy `chrome::status_label_rect/top_label_rect/rail_glyph` and their tests are kept untouched but no longer used by render.

## Decisions and limits
- Golden tolerance: at most 0.5% of pixels may differ, no channel delta above 24 (CoreText text). Stored as RLE (`rust/fixtures/ui/chrome/*.rle`), review PNGs in `review/`.
- Splitters (4 px) are exempt from the 28 px target minimum on their thin axis. MCP hit rect reaches 4 px above the status bar for 28 px.
- Undo/Save enabled = document open and dirty; Redo always disabled (no history cursor exposed to the chrome); Export = open. Connect stays inert (as the old disabled button).
- Footer spans the stage column only (rail stays full height); wireframe shows full width.
- Zoom readout is the literal "Fit" (no zoom state exists yet).
- Persistence key for collapsed sections and sizes: `ui.chrome` (follow-up).
- Tab order covers segmented control and section headers only; toolbar buttons are not focus stops. Tooltip data exists (`layout::tooltip`) but hover painting waits for MOTION.
- Popover overlaps the stage on windows below 900 wide and is then covered by the document blit.
- Live window check done in light theme only (system appearance was light); dark checked via headless review PNGs. Animate and minimum-size live captures, drag and click not exercised.
- `cargo fmt --check` for vexel-ui reports a pre-existing diff in `vexel-ui/src/draw.rs` (not touched); vexel-app is clean.
- No git commit: git commands were refused by the sandbox hook; changes are left uncommitted.
