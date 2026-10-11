# W12 polish2: editor chrome real-window review

The editor chrome was reviewed in the native macOS window in both system
appearances. The screenshots below are direct `screencapture -l 21036` captures
of the live window (2784 × 1888 pixels; 1280 × 832 points at 2× backing scale).

![Editor chrome in light appearance](../ui/chrome-light.png)

![Editor chrome in dark appearance](../ui/chrome-dark.png)

## Changes

- Removed the icon-less toolbar sliver and gave the Layers and Inspector panel
  toggles clear sidebar-collapse icons and descriptive tooltips.
- Kept layer type and name readable by middle-ellipsizing long names and
  truncating the secondary `#id` first. Added distinct stop, circle, document,
  gradient, group, path, text, and image icons. Visibility and lock controls
  have distinct on/off glyphs and action tooltips; hidden rows are dimmed.
- Removed the duplicate Line tool. The tool rail now has grouped sections,
  distinct Pen and Node icons, readable labels, and shortcut tooltips.
- Added the Pixel Grid command to View and the `⌘G` shortcut. It defaults on
  only above 800% zoom, and its overlay uses subtle opacity.
- Padded and vertically centered the status bar; the selection summary is at
  the right and secondary status text uses the secondary label color.
- Corrected the native tooltip paint order so hover tooltips remain visible
  above the canvas. Toolbar hover, pressed, disabled, and keyboard-focus
  treatments remain visible and legible.

## Real-window checks

- Opened the editor with `VEXEL_TEST_LAYOUT_DUMP` set and confirmed the live
  window layout dump updated as the pointer, selection, and tool state changed.
- Captured light and dark appearances from the same live window. Checked the
  Layers and Inspector tooltips, a toolbar shortcut tooltip, and the Pan button
  while pressed. Confirmed the Undo/Redo disabled treatment and a keyboard Tab
  focus ring.
- Selected a layer, hid it, and locked/unlocked it. The hidden row dimmed, the
  eye and padlock states were distinguishable, and their tooltips described
  the available action.
- Zoomed to 835%, where the subtle pixel grid appeared. Toggled it off with
  `⌘G` and back on; the View menu showed Pixel Grid checked.
- Reviewed the increased-contrast and reduced-motion settings in the native
  window, then restored the original Auto appearance and turned both settings
  back off.

## Verification

- `./dev t --all`: 57/57 CTests and 69/69 Python tests passed. Slow Python
  modules are excluded by this command's documented behavior.
- `./dev editor-smoke`: passed live canvas mapping, layer selection,
  visibility/lock, inspector edits, draw/undo, and exact save checks.
- `./dev viewer-smoke`: passed test-open, reload, zoom, fit, actual size, and
  quit checks.
- `python3 scripts/ledger.py validate ledger.jsonl`: 406 entries valid.
- `git diff --check`: passed. The four selection-render goldens were refreshed
  for the status-bar layout change and pass in the full suite.

This lane has no frozen W12 work order under `work/`, so no lifecycle event was
added to `ledger.jsonl`.

