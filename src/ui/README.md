# UI

The UI module provides a fixed-capacity retained tree, deterministic layout,
semantic snapshots, and C17 widget primitives. `ui_core` owns the light, dark,
and increased-contrast token profiles, type scale, and layout metrics. Its
legacy `ui_tree_init` palette and painter remain frozen for existing demo
goldens; new clients can use `ui_tree_init_accessible` and the expanded tokens.

`ui_widgets` provides retained constructors plus bounded state machines for
pointer controls, UTF-8 text editing, number fields, color fields, sliders, and
dropdowns. Editors keep an explicit semantic label, bounded value buffers, and
commit/cancel behavior. Number parsing rejects non-finite values; hex parsing
supports short and long RGB/RGBA forms. Number scrubbing uses the 4 pt base,
Option for 0.1×, and Shift for 10×. `ui_widget_paint` draws reusable controls
into caller-owned RGBA storage without viewport-sized scratch allocation; the
caller supplies its text callback and backing scale.
Empty states adapt their heading from a compact 16 pt panel treatment to the
22 pt roomy canvas treatment when given enough space.

The 45-icon path set is static project-authored geometry in a 24×24 grid. The
allowlisted parser accepts only absolute `M`, `L`, `C`, `Q`, `A`, and `Z`
commands with bounded coordinates. It rasterizes and caches masks at 1×, 2×,
and 3×, then applies theme and interaction-state tint. The legacy three-icon
mask painter remains available to the editor until chrome adopts the path set.

`ui_tree_add` copies full UTF-8 strings into a 1 MiB tree store and keeps a
63-byte preview for scalar clients. Semantic snapshots copy the full strings
into their own bounded store. The modern platform painter caps rendered labels
at 16,384 code points; full values remain available through the semantic API.
Platform text and accessibility integration remain owned by `src/platform/`
and the application layer.

The deterministic demo painter remains available for legacy goldens. The new
component and icon galleries live in `docs/ui/`; their 1×/2× light/dark PNGs
are checked by `tests/ui/verify_uikit_gallery.py`. Widget paint and both text
paths write directly into caller-owned pixels and avoid viewport-sized scratch
buffers.
