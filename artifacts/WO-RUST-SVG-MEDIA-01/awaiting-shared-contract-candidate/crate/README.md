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
R2 atomic request cleanup is blocked: the accepted public builder API offers
no savepoint inside an active same-scene draft. Late failures can retain
image/text output, text state and cache entries. Rolling back the outer draft
also destroys its prefix and consumes its token. The exact shared API gap and
seven executable defect reproducers are recorded in
`artifacts/WO-RUST-SVG-MEDIA-01/atomicity-blocker.md`. This candidate does not
satisfy R2 atomicity and requires a coordinator-owned shared repair.
The same blocker report records the missing cumulative document text-command
budget: the accepted text state/request does not expose that counter, and cache
admission counts cannot substitute for repeated document layout uses.
Pure parsing helpers, `image_element`, `append_image_plan`, and the local
`TextCache` remain available for focused callers and tests.

No default font or pixel provider is supplied. Native CoreText/ImageIO
providers, real decoder/raster composition, the full mixed C oracle suites,
and UI/render integration remain later gates. The
focused Rust parity tests cover parsing, geometry, cache behavior, outline
conversion, source-map rejection before provider calls, and source-map
preservation on real SceneBuilder appends. They do not claim native provider,
mixed renderer, or final application results.

Additional R2 tests cover Unsupported style failures at original source offsets,
inherited font families and tspan state, font units/anchors/spacing, local and
accepted shared cache byte/command caps, and compound contour topology. The
atomicity reproducers passing means the existing defect was reproduced.
