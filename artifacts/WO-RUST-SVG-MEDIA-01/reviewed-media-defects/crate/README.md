# vexel-svg-media

This crate owns the source-level SVG text and embedded PNG/JPEG work assigned
by `WO-RUST-SVG-MEDIA-01`. It provides strict data URI/base64 parsing, image
geometry, explicit `MediaProvider` decode calls, premultiplied bilinear image
sampling, text entity/whitespace/style parsing, glyph outline flattening, and
an owned 32-entry text layout cache.

`decode_image_request` and `render_text_request` consume the accepted typed
scene requests. They validate the request's independent `SourceMap` against
the builder, ledger, scene, and work control before calling a provider or
appending output. Appended image and text shapes retain that map, including a
`<use>` host annotation, while the request transform maps rendered geometry.
The caller owns the real `DraftToken` and rolls back the draft after an error.
Pure parsing helpers, `image_element`, `append_image_plan`, and the local
`TextCache` remain available for focused callers and tests.

No default font or pixel provider is supplied. Native CoreText/ImageIO
providers, real decoder/raster composition, the full mixed C oracle suites,
and UI/render integration remain later gates. The
focused Rust parity tests cover parsing, geometry, cache behavior, outline
conversion, source-map rejection before provider calls, and source-map
preservation on real SceneBuilder appends. They do not claim native provider,
mixed renderer, or final application results.
