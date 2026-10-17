# WO-RUST-UI-CANVAS-01 result

Canvas well with artboard (two soft shadow layers, 1 px hairline, 8 px checkerboard under transparency), `View` zoom/pan/fit math, `ViewMap` (single view/frame/document mapping used by picking, tools and overlays), hover/selection outlines with 8 handles drawn in view space, zoom cluster (rect list, painted bottom-left).

Compose: the artboard is composed at zoom x backing resolution (`DocumentView::compose_target`) and presented through the view. It is recomposed only when the wanted resolution is more than 2x or less than 1/4 of the cached one (cap 16 Mpx / 8192 px per axis); beyond the cap the cached frame is bilinear-upscaled.

Zoom/pan events wired: Cmd+scroll and trackpad pinch (zoom about pointer), two-finger scroll (pan), Space+drag (pan), Cmd+0 fit, Cmd+1 100%, Cmd+= / Cmd+- steps, cluster clicks, hover. Unwired: none of the order's list.

Goldens: tolerance 0 (software raster only, no CoreText); cluster text excluded from pixels (geometry asserted). Candidates until a human accepts them.

Compat: a `DocumentView` the shell never adopted (no `compose_target` call) keeps the legacy presentation (frame is the stage, over white), so the existing animate_workspace assertions are unchanged.

Gaps: AC6 corner clearance holds for stage heights >= 180 (current Layout minimum is 164); pan budget borrowed from the perf-baseline compose case (measured ~11 ms/frame in debug); git is blocked in the worker harness so nothing is committed.
