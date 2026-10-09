# Platform boundary

`platform_window.h` is the C-facing contract for native window lifecycle and
normalized input. It exposes an opaque window handle and plain value events;
AppKit types and Objective-C ownership stay in the macOS adapter.

All calls that touch a window must run on the main thread. The caller owns each
handle and must destroy it after a close request. Event polling is non-blocking.
Resize and scale events report backing pixel dimensions and scale. Pointer
coordinates are logical points from the content view's top-left. Key text is
bounded UTF-8 and never ends in a partial code point.

This initial platform module supports Apple Silicon macOS 13+. No unsupported
platform implementation or stub is provided. See `macos/README.md` for the
AppKit adapter and smoke command.
