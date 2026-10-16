# WO-RUST-V1-INTEGRATE-01 result

One document session: `session::AppDocument` (wraps `ProjectSession`) owns the only `V1Document`. `revision()` bumps only when `source()` bytes change (or the document is replaced). The shell renders when `revision` or stage size changes via `DocumentView::show(bytes, revision, stage)`; `DocumentView` keeps no source copy.

## Hooks
- `AppDocument::edit(|doc| ..)` and `DocumentEditor` / `AppDocument::apply` (`session/hooks.rs`). DRAW-TOOLS attaches in `main.rs::route_pointer` and the key path: `files.document_mut().edit(|d| tools.pointer(input, d, &map))`.
- `impl CanvasMap for compose::PointMap` (completes the DRAW-TOOLS integration step). Tool rail clicks set the tool; Escape/Delete/Return reach `ToolController::key`.
- `PlaybackHost::request_frame(source, revision, seconds)` plus `AppDocument::request_frame`. ANIM attaches here; no production implementer yet (see gaps).
- `edit_shortcut`: Cmd+Z / Shift+Cmd+Z (unbound before), routed to `AppDocument::undo/redo`.

## Preview and shell unified
`--mcp-preview` now starts the shell (one process, one window) with a 512x512 blank document and the MCP stdio server (`preview_app::start_server`). Requests are applied by `session::McpHost::handle` through `AppDocument::edit`: shape add, animation add, undo/redo, info/list, animation list, play/pause/seek, export (session source handed to `ExportJob`). `preview_app::run` and `PreviewState` remain, unused by `main`.

## Deltas vs the order's assumptions
- No `DocumentView::hit`: the equivalent is `Composed::hit_test` / `DocumentView::pointer`.
- `DocumentView::open` now takes `&mut AppDocument`; `DocumentView::resize` removed (replaced by `show` with a new stage). Compose view tests were adapted; compose-layer failure cases (bad UTF-8, malformed XML, unsupported filter/mask/text) now feed raw bytes through `show`, with the same assertions.
- `.svg` open lives in `ProjectSession::open` (dispatch by `.svg` extension), so File > Open and `AppDocument::open_path` share it. `ProjectSession` gained `svg_origin` (display name only).
- `OPEN_EXTENSIONS` stays `["vexel"]`: two NATIVE-PANELS tests assert the exact panel filter and must stay unmodified. File > Open therefore does not offer `.svg` in the panel; CLI and `open_path` do.
- A malformed SVG now fails at open (typed `InvalidScene`) instead of at compose.
- Preview replies are sent right after the edit, not after the frame is presented.
- Tools wiring (pointer, rail, keys) was added to `main.rs`; the model had no tool state.

## Gaps
- Playback state (play/pause/seek) is tracked by `McpHost` on the monotonic clock and reported, but the shell stage shows compose's static first frame; the old preview window's animated raster is not carried over. `PlaybackHost` has no production implementer.
- Undoing back to an empty document shows compose's "nothing drawable" status.
- `git` was refused by the sandbox hook: nothing committed or staged, `git diff --check` not run.

## Live checks not executed
All six items of the order's manual list (CLI `.svg` launch, File > Open/Cancel/corrupt, draw plus Cmd+Z, Cmd+S on untitled, MCP add then save/reopen, animation playback and close prompt).
