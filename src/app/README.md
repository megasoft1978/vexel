# App viewer modules

`viewer.c` owns the native viewer state on the AppKit thread. It owns the window,
the one-worker task queue, the reusable composed frame buffer, navigation state,
and the loaded document. The worker reads and renders an immutable source and
publishes it through the generation-checked task queue. The source stays alive
as long as the decoded document borrows it. Closing during load cancels the
task, advances its generation, keeps pumping AppKit events until task cleanup,
then joins the worker before releasing document storage.

`viewer_load.c` bounds input to `SVG_INDEX_MAX_SOURCE_BYTES` (1 GiB) and uses the
public shape decode and raster render interfaces. The current raster tile is
limited to 4096 pixels per axis. `frame_compose.c` is a pure, allocation-free
RGBA composition path after the caller allocates the output buffer. It supports
viewports through 8192 pixels per axis, draws a 16-pixel checkerboard under
alpha, uses bilinear sampling through 4x and nearest sampling above 4x, and
keeps a small visible portion of a panned document so it can be recovered.

The native window is 1280 by 800 points at open. Navigation and composition use
backing pixels. The frame buffer is recomposed only after a frame-affecting
change and is uploaded through the reusable asynchronous Metal presenter. The
Metal device, queue, pipeline, and dimension-matched texture are owned by the
platform Metal adapter and released by `platform_metal_reset`.

`render_frame_cli.c` loads synchronously only for headless use, composes through
the same function as the native window, then writes a binary P6 PPM. Width and
height are bounded to 1..8192, zoom to the navigation range 0.05..64, pan offsets
to +/-10,000,000 backing pixels, and command path arguments to 4096 bytes.
Unsupported and malformed SVGs produce a distinct error image, write the shape
error name and byte offset to stderr when available, and return failure.

This is a viewer, not an editor. It has no editing tools, SVG text rendering, or
accessibility tree yet. Rendering remains limited to the SVG subset accepted by
the current decoder and rasterizer; unsupported features are reported instead
of being silently omitted.
