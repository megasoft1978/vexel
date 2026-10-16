# vexel-platform-apple

This crate owns the macOS native window and event boundary. `RgbaFrame` and
`Window::present_rgba` form a pixel presentation seam for owned top-left,
tightly packed RGBA8 data. Input channels are straight (unassociated) sRGB;
alpha is independent coverage. Rows run from visual top to bottom without
padding. The presenter converts each pixel to premultiplied RGBA8 before
declaring `PremultipliedLast` to CoreGraphics. Frame dimensions are limited to
4096 by 4096 (64 MiB), both the exact byte length and allocated `Vec` capacity
are bounded, and backing scale must be one of the window boundary's supported
integral scales: 1, 2, or 3 pixels per point.

Presentation converts the owned pixels in place and copies them into immutable
Core Foundation data. It prepares the CGImage and a fully configured CALayer
off-hierarchy, using the current view-layer bounds, requested contents scale,
resize gravity, and prepared image contents. It then performs one guarded
hierarchy operation: `addSublayer` for the first frame or
`replaceSublayer_with` for a later frame. The `WindowRecord` retains the current
presenter layer only after that operation returns successfully. Candidate
preparation failures do not touch the current presenter. A failed publish
callback leaves the pure transaction's prior state unchanged; this unit test
does not establish AppKit's runtime exception or hierarchy behavior. Directly
supplied layer contents are not invalidated with `setNeedsDisplay`.

Live display behavior remains unverified for channel order, top-left
orientation, sRGB and alpha compositing, image retention after the call,
replacement and release behavior, resize and backing-scale replacement,
teardown, and native exception behavior. Package tests do not create a live
window or establish these properties. This is only the frame presentation
seam; it does not integrate a document model, editor workflow, SVG renderer,
or complete V1 application flow.

`draw_text_rgba` is a bounded safe Rust API for CoreText system or SF Mono text.
It checks dimensions, stride, clip bounds, UTF-8 length, font size, backing scale,
and total pixel storage before creating private Core Foundation/Core Graphics
handles. It draws regular or semibold text into caller-owned RGBA pixels. Native
font rendering is not exercised by package tests; a live macOS window and native
font appearance remain runtime verification items.
