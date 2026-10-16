# WO-RUST-SVG-MEDIA-01 candidate

Implemented the MEDIA crate's owned image and text primitives: strict PNG/JPEG data URI and base64 parsing, image limits and meet/slice placement, explicit provider decode, premultiplied bilinear image sampling, text entity/whitespace/style/position parsing, glyph outline flattening, and the bounded owned layout cache. Image/text append helpers use `SceneBuilder`, source assignment at append time, and `DraftToken` finalization. Cache and provider results use Rust ownership and `Drop`; no fake default provider or new dependency was added.

The symbol inventory and adaptations are in [parity-map.json](parity-map.json). Focused Rust checks and all twelve assigned original cases are recorded in [coverage.json](coverage.json). The frozen scope and original source/suite digests match.

**Composition blocker:** accepted `TextDecodeRequest` and `ImageDecodeRequest` omit the active use-site source span. The original `<use>` text oracle requires that span to map output to each host element. Owned append functions require it explicitly; the coordinator must add it to the shared requests before composition can pass those assertions. Native providers, the full mixed C suites, raster differentials, and draft rollback integration remain unverified later gates.

Verification: locked offline crate check passed; locked offline tests passed (8/8); work-order ready validation passed. Exact results and unverified gates are in [verification.json](verification.json). This is a candidate for independent root review, not work-order acceptance or product completion.
