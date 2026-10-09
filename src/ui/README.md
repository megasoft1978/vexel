# UI state

UI state values are owner-thread-only snapshots. The index progress adapter
converts generation-tagged byte progress into a small loading indicator model;
`first_preview_ready` means the index found a shape candidate. It does not mean
a raster tile exists. Call `ui_index_progress_publish_preview` only after the
worker returns an owned tile for the current generation; cancellation, failure,
or a newer generation clears the visible-preview state. Rendering a native
progress control remains the window layer's responsibility. `viewer_controls.h`
adds caller-owned navigation, progress, semantic, and RGBA overlay values.

Navigation state and pan use backing pixels; document coordinates are world
units; `screen = world * scale + pan`. Use `resize_scaled` for logical viewport
dimensions and the scaled drag/wheel/pinch helpers for logical pointer or
cursor positions at the AppKit boundary. They multiply both by the current
backing scale before updating one transform, so fit and anchored zoom share a
coordinate space.
Wheel input uses
`exp(-delta_y * 0.001)` and pinch input uses `exp(magnification)`. Both clamp
scale to 0.05–64 and solve pan from the world point under the cursor. Resize
keeps the world point under the old viewport center at the new center. Fit
centers the document at the smaller viewport/document ratio. Public update
functions reject non-finite values and check computed results before mutation.

Navigation and overlay functions use no allocator and retain no caller buffers.
Semantic snapshots write into an explicit caller-owned array. Progress cancel
sets `cancel_requested` and does not change the active phase; a later worker
update marks the generation canceled, complete, or failed. A newer generation
clears its cancel request and stale generations are rejected.
