# Luna architecture review: V1 native frame presentation

Read-only review of the existing Apple window, input callbacks, UI canvas, raster tile, and pinned Apple bindings. No files or tests were changed by the review.

Recommendation: expose a small `Window::present_rgba(frame)` API from `vexel-platform-apple`. It should accept validated RGBA pixel data, keep AppKit/CoreGraphics use inside the platform crate, and preserve a complete previous frame until the replacement image is ready. The app crate should not retain native handles or call Objective-C. The API needs explicit dimensions, backing scale, byte count, row order, alpha convention, and memory bounds.

The existing `WindowEvent::Resize` and `WindowEvent::BackingScaleChanged` provide point and pixel dimensions. The app can rerender at the reported pixel size and submit a new frame. Retaining the latest image on the view/layer handles exposure redraws; direct layer contents replacement avoids adding a second renderer. The `Window` is already main-thread-owned and not `Send`/`Sync`.

Relevant existing seams: `vexel-platform-apple::Window`, `WindowEvent`, `vexel-ui::WidgetCanvas` / `widget_paint`, and `vexel-raster::Tile`. Apple bindings already enable CoreGraphics image/data-provider, CoreFoundation data, and CoreAnimation layer contents. An owned RGBA frame can be converted to an image inside the Apple crate without an external dependency; native access stays behind that safe API.

Risk: explicitly define RGBA channel order, alpha interpretation, row stride, top-left origin, backing scale, and the image lifetime held by Core Animation. Bound frame allocation and build the replacement completely before publishing it.

Scope distinction: this provides the missing app-shell display surface. It does not connect document state, SVG rendering, editor controls, animation, or MCP into an integrated app; those remain subsequent V1 implementation work.
