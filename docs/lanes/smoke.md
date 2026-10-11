# Native smoke coverage

`./dev editor-smoke` launches the native editor on a real macOS window and
drives it with Quartz mouse and keyboard events. The test opens a fixed 64 × 64
SVG, moves and resizes its rectangle, draws and undoes a rectangle and polygon,
and checks that saving changes only the selected element's transform. It then
selects the layer from the Layers panel, hides and restores its preview, locks
it and confirms a canvas drag is blocked, and edits X and Fill in the inspector.
The final saved bytes must retain the original source layout and contain only
the requested X and fill changes plus the composed transform.

Canvas coordinates come from the running editor, not window-layout guesses.
Set `VEXEL_TEST_LAYOUT_DUMP` to a file path when starting `svgworkbench` to have
it atomically write the content-screen origin, backing scale, canvas rectangle,
document dimensions, view transform, and current interactive node bounds. The
smoke helper reads this JSON for every document, layer-row, and inspector hit.
The script moves the test window to `{20, 40}` so its real screen origin stays
stable while the helper waits for the layout dump to catch up. Its tool
shortcuts send physical V, R, and Y keys (key codes 9, 15, and 16), independent
of the configured keyboard layout. Text-field contents use Quartz Unicode
events, preserving punctuation such as the leading `#` in a color value.

The editor smoke captures the selection, layer selection, hidden preview,
locked layer, blocked drag, inspector X edit, inspector fill edit, and final
saved state with `screencapture -l <CGWindowID>`. To retain those screenshots
and the live layout dump for review, run:

```sh
VEXEL_SMOKE_CAPTURE_DIR=.cache/w11-smoke/captures ./dev editor-smoke
```

Layer visibility is a preview-only editor override. It must not dirty or alter
the editable SVG source; `editor-followups` checks both materialized source
bytes and the rendered preview. The smoke also saves while visibility is
toggled and verifies that this UI state remains usable after saving. It waits
for the render job's “Preview updated” status before retaining the hidden-layer
screenshot, so a failed preview decode cannot pass just because the row icon
changed.

Saving used to invalidate the cached document model while the editor retained
layer and selection handles into that model. The first smoke run after the
chrome redesign showed that row labels and inspector fields disappeared after
Cmd-S, despite the saved SVG being correct. A focused `editor-followups`
regression failed on the missing `<rect>` and `#first` outline label after
save. The save path now retains the parsed index because it saves the exact
current snapshot without changing document bytes; model handles therefore
remain valid. This keeps selection and layer overrides available after save.

The hidden-preview smoke also caught a source-length mismatch: visibility
overrides can make the render-only SVG longer than the editable source. The
render job now receives the materialized preview length, and the editor smoke
requires a successful preview update before checking the saved source.

`./dev viewer-smoke` remains the native viewer-window smoke. Run it alongside
the editor smoke when changing shared window, presentation, or canvas behavior.
