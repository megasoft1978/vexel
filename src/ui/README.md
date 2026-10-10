# UI

The UI module owns a fixed-capacity retained widget tree, deterministic layout,
semantic light/dark tokens, bounded UTF-8 labels and values, keyboard focus and
command routing, full-text semantic snapshots, and virtualized inspector rows.
`ui_widgets` provides constructors for buttons, checkboxes, text fields, sliders,
and rows. `ui_core` remains platform-independent C17. The modern macOS editor
uses its background/icon painter and a CoreText adapter in `src/platform/` for
SF/SF Mono glyph paths; legacy demo clients keep the deterministic scalar path.

`ui_tree_add` copies full UTF-8 strings into a 1 MiB tree store and keeps a
63-byte preview for scalar clients. Semantic snapshots copy the full strings
into their own bounded store. The modern platform painter clips long text to
the visible control and caps rendered strings at 16,384 code points; full
values remain available to VoiceOver up to the editor's 64 KiB per-field limit.
Icon masks use a small static 24×24 geometry set cached by theme and scale.

Semantic group and list labels remain available to accessibility clients but
are not painted as visible text. Their child text nodes supply the panel titles.
Hover and pressed node state is carried through the viewer's pointer bridge and
invalidates the next frame immediately.

The legacy `ui_tree_paint_into` path keeps its scalar ASCII glyph painter so
existing deterministic demo goldens remain stable. Production labels use
`platform_text_ui_draw` through CoreText and accept Unicode input. Both paths
write into caller-owned RGBA storage and avoid viewport-sized scratch buffers.
