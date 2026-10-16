# macOS window adapter

`platform_window.m` implements the C platform contract with AppKit. The
Objective-C file is built with ARC; the public C interface contains no Cocoa
types. Native window creation, event pumping, resize, and destruction are
main-thread-only. Open uses a non-blocking sheet with a completion handler, so
the viewer's event polling continues while the file picker is visible. The
request clears its owner before window destruction dismisses an outstanding
sheet; a late completion then cannot dereference the freed window handle.
`platform_window_destroy` clears delegate/view back-references, closes the
native window, frees queued events, and releases its strong window reference
before freeing the opaque handle.

`platform_dialog.h` exposes main-thread-only Save, unsaved-changes, and error
dialogs without Cocoa types in the C contract. Save suggests an `.svg` name and
returns a bounded UTF-8 path. The testing backend injects Save, Save / Don't
Save / Cancel, and error-alert responses without displaying native UI.
The PNG save panel returns a bounded path, 1×/2×/4× scale, and a document or
selection choice for the editor export flow.

`platform_clipboard.m` reads and writes bounded UTF-8 SVG fragments on the
general pasteboard and publishes the `public.svg-image` type. It also writes
PNG bytes to the pasteboard. `platform_png.m` encodes caller-owned RGBA8 pixels
through ImageIO into an owned PNG blob; input is limited to 4096 pixels per
axis, 16 million pixels, and 64 MiB of encoded output. It retains no caller
buffer.

The smoke-only injection declarations are private to `app/test_driver/` and
invoke the target view's event handlers directly. This keeps synthetic pointer
tests independent of LaunchServices activation while checking the same event
translation as AppKit delivery. They do not substitute for checking physical
keyboard, pointer, display scale changes, or window-manager behavior by hand.

`platform_metal.m` owns the Metal device, command queue, pipeline, texture, and
CAMetalLayer. The C boundary accepts a borrowed RGBA pointer plus its exact
length and retains no caller memory. It renders a fullscreen triangle into the
current drawable, waits up to five seconds for explicit command completion, and
compares every byte of the uploaded shared texture. Diagnostics name only the
first differing byte. The native Metal smoke presents once, clears the layer,
resizes the AppKit window, and validates a second frame.

The interactive viewer uses `platform_metal_present`, a main-thread
presentation path that lazily creates and reuses its device, command queue,
shader pipeline, and dimension-matched shared texture. It submits without
readback verification. `platform_metal_reset` releases those cached resources
and detaches the layer before the caller destroys the window.

Repeated native replay uses the separate headless
`platform_metal_copy_and_hash` entry point. It blits the bounded shared input
texture to a second Metal texture, waits for completion with the same five-
second bound, then compares and hashes the GPU readback. This path avoids
AppKit window teardown and does not measure presentation timing.

`platform_menus.m` installs the application, File, View, and Window menus. Its
File menu supports Open, Reload, and a persistent list of up to twelve recent
SVG paths. Open panels are restricted to `.svg`; the content view accepts SVG
file URLs by drag and drop, and the application delegate forwards Finder
`openFiles`/`openURLs` callbacks through the same bounded event contract. The
viewer polls size and modification time every 250 ms for auto-reload. The event
queue coalesces pointer movement and window size/scale notifications and caps
all queued events at 256.

`tests/viewer/native_smoke.sh` runs `native_smoke.py`, which exercises reload,
zoom, checkerboard, Open, file selection, actual size, and quit through System
Events. Automation and viewer shutdown each have bounded timeouts; on failure
the harness records process state and the viewer log, then terminates both
process groups. The smoke needs an interactive macOS session, Accessibility
permission for `osascript`, and a working Metal device; a headless or Metal-less
environment cannot verify the native presentation path.

`platform_task.c` implements the opaque C worker queue with pthreads. It accepts
capacity 1–64 and 1–4 workers, and uses caller-provided allocation functions for
the queue and fixed slot array. The queue never owns task context: callers keep
it alive until queued cancellation or callback return. Callbacks poll the
atomic cancellation token; queued cancellation prevents execution, while
running cancellation is cooperative. Advancing generation cancels older work
and prevents its results from being published. Shutdown requests cancellation
and joins every worker before destroy frees storage. The owner must quiesce
callers before destroy; debug builds assert if an admitted call remains active.
A callback that ignores cancellation can delay shutdown and join.

`platform_task_submit_owned` adds an optional terminal cleanup callback without
changing borrowed `platform_task_submit`. Ownership transfers only when submit
returns success. Cleanup runs exactly once after the callback returns or after
a queued task is discarded, and never while the queue mutex is held. Failed
submissions do not call cleanup. Queue slots are released under the mutex
before cleanup runs, so cleanup can safely release snapshots or update module
accounting without extending the queue critical section.

`platform_recovery_store.m` resolves an existing native-project path through
`realpath`, hashes its canonical filesystem representation with CommonCrypto
SHA-256, and returns a `.vxrecover` path below the current user's
`~/Library/Application Support/Vexel/Recovery`. Tests can provide an isolated
Application Support root. The `Vexel` and `Recovery` directories are created
with mode 0700 and opened without following symlinks; existing components must
belong to the current user and have no group/other permissions. This module
chooses a location only; background snapshots and restart recovery remain app
integration work.
