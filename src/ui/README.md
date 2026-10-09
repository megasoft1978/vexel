# UI

The UI module owns a fixed-capacity retained widget tree, deterministic layout,
design tokens, input/focus state, semantic snapshots, native text adapters, and
virtualized inspector behavior. `ui_core` is platform-independent C17 and paints
node backgrounds through the public scalar SVG shape decoder and rasterizer.
Platform input and text services stay behind `src/platform/` interfaces.
