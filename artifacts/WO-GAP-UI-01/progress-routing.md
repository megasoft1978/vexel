# Progress routing through `viewer_editor_set_operation_progress`

The function (`src/app/viewer_editor_status.c`) sets the status centre region message and shows the
Cancel affordance (`EDITOR_BUTTON_STATUS_CANCEL`); deactivating it clears only its own progress text, so
a result or error reported in the meantime is kept. Every production call site:

| operation | file:line | call | change |
|---|---|---|---|
| open/index start | `src/app/viewer.c:264` (`viewer_request_open`) | `(editor, true, "Loading SVG")` | new; only when an editor already exists (the first open has no editor to show it in) |
| open/index and render end | `src/app/viewer.c:486` (`service_job`) | `(editor, false, NULL)` | new; runs for the current-generation load, new-document and edit-render job before their result/status is set |
| render start | `src/app/viewer.c:681` (`start_editor_render`) | `(editor, true, "Rendering preview")` | new; after the render job is queued successfully |
| export start | `src/app/viewer_export.c:169` (`export_png_run`) | `(editor, true, "Exporting PNG" / "Copying PNG")` | new; after the save dialog and validation, before rasterising |
| export end | `src/app/viewer_export.c:226` (`viewer_export_png`) | `(editor, false, NULL)` | new; wrapper clears it on every exit path, after the result status ("PNG exported", error text) was set |

Other changes made for this routing: `viewer_persist_workspace` callback is now installed after the initial
workspace restore in `src/app/viewer.c` (restoring a remembered or test-pinned workspace no longer writes to settings).

Limits, stated plainly:
- PNG export is synchronous on the main thread, so its centre message is only drawn if a frame is composed
  while it runs; the call sites exist and set the state, but the message is not guaranteed to be painted.
- The Cancel button is drawn while an operation is active, but no command handler is bound to
  `EDITOR_BUTTON_STATUS_CANCEL` (pre-existing); cancelling is not wired by this order.
- Messages not routed (they remain results via `viewer_editor_set_status`): "Preview updated", save and
  clipboard results, and error texts; those are transient reports, not progress.
