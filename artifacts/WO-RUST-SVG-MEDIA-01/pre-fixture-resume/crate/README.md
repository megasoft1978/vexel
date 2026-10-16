# vexel-svg-media

This crate owns the source-level SVG text and embedded PNG/JPEG work assigned
by `WO-RUST-SVG-MEDIA-01`. It provides strict data URI/base64 parsing, image
geometry, explicit `MediaProvider` decode calls, premultiplied bilinear image
sampling, text entity/whitespace/style parsing, glyph outline flattening, and
an owned 32-entry text layout cache.

`image_element` and `render_text_element` require caller-supplied provider and
work control values. `append_image_plan` and `render_text_element` append
inside a scene draft and finalize against the actual `DraftToken`; the caller
must roll back that draft after any error. Source assignment is explicit on
append so `<use>` traversal can map output to its host element. The current
shared `TextDecodeRequest` and `ImageDecodeRequest` do not yet carry that host
span, so composition cannot satisfy the frozen use-site mapping assertions
until the coordinator repairs that shared request contract.

No default font or pixel provider is supplied. Native CoreText/ImageIO
providers, real decoder/raster composition, the full mixed C oracle suites,
and UI/render integration remain later gates. The
focused Rust parity tests cover pure parsing, geometry, cache behavior, and
outline conversion only; they do not claim those integration results.
