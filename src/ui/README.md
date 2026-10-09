# UI state

UI state values are owner-thread-only snapshots. The index progress adapter
converts generation-tagged byte progress into a small loading indicator model;
`first_preview_ready` means the index found a shape candidate. It does not mean
a raster tile exists. Call `ui_index_progress_publish_preview` only after the
worker returns an owned tile for the current generation; cancellation, failure,
or a newer generation clears the visible-preview state. Rendering a native
progress control remains the window layer's responsibility.
