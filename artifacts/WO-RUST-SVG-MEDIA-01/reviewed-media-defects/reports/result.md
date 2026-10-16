# WO-RUST-SVG-MEDIA-01 candidate

Implemented the full owned MEDIA surface: strict embedded PNG/JPEG URI and base64 parsing with frozen limits/offsets; meet/slice/none placement; explicit provider decode; premultiplied bilinear sampling; text entity/whitespace, family/style/anchor/spacing and position parsing; glyph outline placement/flattening; and bounded layout caching. All 24 catalog symbols and algorithm adaptations are mapped in [parity-map.json](parity-map.json).

Public `render_text_request` and `decode_image_request` use the accepted typed scene requests. Each validates its independent host `SourceMap` before provider work and preserves that map on shapes appended through the real `SceneBuilder` and `DraftToken`. A real lexical/animation/ledger fixture proves both output mappings and that an invalid map causes no provider call or append. Provider fixtures are test-only.

The original 12 assigned cases and their coverage are recorded individually in [coverage.json](coverage.json). Focused pure and real-fixture tests pass. The mixed CLI/C suites, native providers, renderer fidelity, CoreText/librsvg comparison and application integration remain later proof; no completion is claimed for them. Original test inputs and oracles remain unchanged.

Exact command results and scoped audit are in [verification.json](verification.json). This is a candidate for independent review, not work-order acceptance or product completion.
