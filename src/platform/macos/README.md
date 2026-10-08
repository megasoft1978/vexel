# macOS window adapter

`platform_window.m` implements the C platform contract with AppKit. The
Objective-C file is built with ARC; the public C interface contains no Cocoa
types. Native window creation, event pumping, resize, and destruction are
main-thread-only. `platform_window_destroy` clears delegate/view back-references,
closes the native window, frees queued events, and releases its strong window
reference before freeing the opaque handle.

The smoke-only injection declarations are private to `app/test_driver/` and
exercise AppKit event dispatch without Accessibility permission or wall-clock
sleep. They do not substitute for checking physical keyboard, pointer, display
scale changes, or window-manager behavior by hand.

`platform_metal.m` owns the Metal device, command queue, pipeline, texture, and
CAMetalLayer. The C boundary accepts a borrowed RGBA pointer plus its exact
length and retains no caller memory. It renders a fullscreen triangle into the
current drawable, waits up to five seconds for explicit command completion, and
compares every byte of the uploaded shared texture. Diagnostics name only the
first differing byte. The native Metal smoke presents once, clears the layer,
resizes the AppKit window, and validates a second frame.

Repeated native replay uses the separate headless
`platform_metal_copy_and_hash` entry point. It blits the bounded shared input
texture to a second Metal texture, waits for completion with the same five-
second bound, then compares and hashes the GPU readback. This path avoids
AppKit window teardown and does not measure presentation timing.
