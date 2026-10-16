# vexel-svg-decode

This crate is an incomplete candidate for WO-RUST-SVG-DECODE-01. It provides a
source-borrowing `Decoder` with bounded token steps, scene draft commit/rollback,
CSS inheritance, path decoding through `vexel-svg-path`, several primitive
geometry adapters, viewport/transform helpers, switch selection, and ignored
feature accounting.

It is not a complete SVG decoder. Paint server resolution, most style behavior,
use/reference expansion, clip/mask/pattern/filter planning and pattern-ticket
resumption, custom geometry, text and image decoding, nested viewports, and many
original assigned assertions remain unimplemented. Unsupported input paths fail
closed where recognized. `Decoder::resume_pattern` rejects requests because the
engine does not yet issue pattern requests. Do not use this candidate as a full
renderer entry point.

The later composition layer must supply real `DecodeOperations` and native
`MediaProvider` implementations. There are no default providers or synthetic
successful responses in this crate. No dependencies were added.

See `artifacts/WO-RUST-SVG-DECODE-01/result.md`, `parity-map.json`,
`coverage.json`, and `verification.json` for the symbol inventory and exact
verification evidence.
