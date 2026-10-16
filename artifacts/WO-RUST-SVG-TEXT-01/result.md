# WO-RUST-SVG-TEXT-01 result (candidate, uncommitted)

Layering: `vexel-platform-apple::text_layout` (CoreText, macOS, unsafe confined to
`src/text_layout.rs`, 22 unsafe blocks / 22 SAFETY comments, whole call and the
`CGPathApply` callback guarded) returns glyph-local outlines like C
`platform_text.m` + `app_text_layout`. `vexel-svg-engine` defines
`TextLayoutProvider` (default none => today's typed `Unsupported`) and
`src/text.rs` decodes `<text>/<tspan>` (solid fill only; stroke, paint servers,
clip, `textPath`, CDATA fail closed), reusing `vexel-svg-media` helpers. The
engine never depends on the platform crate. `vexel-render/src/text.rs` is only the
Unsupported(Text) policy: text outlines are ordinary flattened filled shapes.

Mirrored C constants: 64 KiB text, 16384 platform glyphs, 262144 path segments,
32 families, 256 family bytes, 96/72 scale, 64 MiB / 1048576 document text/glyph
limits. C quirks kept for parity: dx[0] folded into the start and re-added per
glyph; x[i] override discards dx[i]; word spacing never moves ASCII glyphs (C byte
cursor bug) though the cursor still advances.

Font caveat: goldens depend on installed fonts and macOS 27.0.1 (26A434); the
parity tests print an explicit SKIP if a recorded font file is missing.

Not executed: app wiring and live use (WO-RUST-SVG-WIRE-01). Note the decoder
still requires a non-None `DecodeOptions.media` to reach the engine.
