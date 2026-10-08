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
