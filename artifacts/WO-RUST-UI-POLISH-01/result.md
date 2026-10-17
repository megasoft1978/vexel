# WO-RUST-UI-POLISH-01 result (candidate; uncommitted: the sandbox hook refuses `git commit`)

Fit render method: the canvas already recomposes at view scale (`target_compose_size`), so 400% Fit re-renders the document (no nearest-neighbour scaling, no new path). No new token was needed: light well = `surface[2]` (`canvas::well_fill`), edge = `t.border` on all four stage sides in both themes.

Core accessor follow-up: `V1Document` already exposes `history_count()` / `history_cursor()`; redo availability is `count - cursor`. The shell keeps the specified `redo_depth` counter (`history.rs`); a test asserts it agrees with the core cursor. Switching to the accessor is the follow-up.

## Deviations / decisions
- `FOCUS_ORDER` keeps its 7 tab stops: the accepted MOTION test indexes `FOCUS_ORDER[6]` after a wrap. Hover coverage uses new `REGISTERED` (24 ids: modes, tools, 7 buttons, 8 headers, 2 splitters), the single id map; a test checks every Target maps into it uniquely.
- Empty selection omits the Fill/Stroke/Transform headers and bodies; Selection holds the 112 px card, Animation follows. With a selection geometry equals `Layout::new` (pins unchanged).
- `MotionAdapter::pointer` signature kept; `pointer_in(.., &Model)` adds popover-aware hit testing and disabled controls. Tooltip API: `tooltip()` / `poll_tooltip()` (not in `redraw_after`, which existing tests pin to idle). Tooltips are painted by `overlay::overlay_tooltip` after the stage blit so rail tooltips can cover the canvas. No fade (instant) in all motion modes.
- Latent bug fixed in `inspector.rs`: the unit rect of a number field ran past the window edge for two-digit values ("12 px") and CoreText rejected the whole frame.
- Toolbar Undo/Redo clicks are still not wired to actions (out of scope).

## Changed expectations (existing tests)
- `ui_canvas_view.rs`: fit cap 100% -> 400% (name, two asserts).
- `ui_canvas_paint.rs`: well = `well_fill`; stage corner is now the border edge; probes moved 1 px inside.
- `ui_canvas_pointer.rs`: zoom factors relative to the fitted zoom.
- `animate_workspace.rs`: stage pixel comparison excludes the 1 px edge.
- Existing goldens NOT overwritten. `ui_chrome_goldens` (32 frames) and `ui_canvas_golden` (4 frames) fail until accepted: every frame changes (well, edge, status text, inspector, zoom label). Candidates: `review/existing-chrome/`, `review/existing-canvas/`. New candidates (16 frames, 1x/2x): `review/*.rle|png`; accepted ones go to `fixtures/ui/polish/`.
- `vexel-ui` is not fmt-clean at baseline (11 files, protected); the frozen fmt check fails there only. `vexel-app` is clean.

Live check (debug app, compose-v1.svg, light, 1280x820): `docs/archive/live/2026-10-17-ui-polish-01-{before,after}-draw-light.png`. The "before" build already had the 400% cap. After: 160x120 fills the well at 400%, status says 400% like the cluster, distinct well with edge, single card, aligned readouts. Hover, tooltip and dark not verifiable live (no click injection); covered headless.
