# W10 Polish: real-window review

Polished the modern macOS editor chrome after reviewing it in a live 1280 × 832
point window. The screenshots below were captured from the running release
window with Quartz window ID 18096; each PNG is 2784 × 1888 pixels.

![Light appearance editor window](../ui/chrome-light.png)

![Dark appearance editor window](../ui/chrome-dark.png)

## Changes

- Raised the status label content area so its baseline and descenders stay inside
  the bottom bar. The regression check validates the label frame against the
  window edge.
- Combined zoom, document dimensions, pointer coordinates, and sharp-render
  progress in one padded canvas HUD pill.
- Kept selection fill exclusive to the checked layer row. Hover and row-action
  focus use a lighter treatment. Visibility and lock icons remain available on
  every row; hidden rows dim, names use title case, IDs sit secondary, and
  search uses a magnifier.
- Removed duplicate icon-button outlines and selected underlines. Toolbar and
  tool-rail icons use 22-point glyphs in 32-point targets, with subtle group
  dividers and shortcut tooltips. Tool-rail tooltips open over the canvas so
  they do not cover layer rows.
- Grouped Inspector fields into Geometry and Appearance, collapsed technical
  attributes under Other attributes, added fill/stroke swatches and color
  fields, and added the centered no-selection state.
- Redrew the pen and node-edit paths and kept the node-edit tool distinct from
  the Pen selection state.

## Real-window checks

- Selected `Rectangle #card`; only that row retained the blue selection fill.
- Clicked the Title row eye. The row dimmed and the document became dirty;
  Command-Z restored visibility. Toggled its lock on and off; the document
  remained saved.
- Hovered the Polygon tool and confirmed the `Polygon Y` shortcut tooltip stays
  clear of the layer tree.
- Inspected the Rectangle fields, including the Fill and Stroke color values,
  swatches, and the full “Stroke width” label.
- Checked the bottom status text and the combined HUD in both system
  appearances. Light mode was the original system setting and was restored
  after the dark capture.

The dark screenshot was checked against
[`uikit-contrast-report.json`](../ui/uikit-contrast-report.json). Normal dark
secondary text measures 6.247:1 on selection fill and 6.762:1 on hover; both
exceed the 4.5:1 text minimum. Control edges measure 3.033:1 on hover, above the
3:1 control minimum.

## Verification

- `./dev t --all`: 57/57 CTest cases passed; 69/69 Python tests passed.
- `cmake --preset mac-release` and `cmake --build --preset mac-release` passed.
- Ran `build/mac-release/bin/svgworkbench` with a temporary showcase SVG and
  captured `chrome-light.png` and `chrome-dark.png` from the actual window.
- Updated the editor chrome, style, and UI-kit screenshot baselines to match the
  reviewed visuals; the full test run passed after those updates.
