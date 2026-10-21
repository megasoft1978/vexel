# ADR-0034: AVFoundation bindings for compressed video export

- Status: proposed (coordinator to confirm number and lock/provenance entry)
- Date: 2026-10-21
- Related: ADR-0027, ADR-0032

## Decision

Add three exact-pinned crates to `vexel-platform-apple`, all from the same
objc2 generated-bindings family already in use (`=0.3.2`, same repository
`madsmtm/objc2`): `objc2-av-foundation`, `objc2-core-media`,
`objc2-core-video`. Licence: `Zlib OR Apache-2.0 OR MIT` (compatible with the
MPL-2.0 project). Default features stay off; only the AVAssetWriter/Reader,
CMTime/CMSampleBuffer and CVPixelBuffer features are enabled. They add
`objc2-core-audio` and `objc2-core-audio-types` to `Cargo.lock` as optional
transitive entries of `objc2-av-foundation` (not compiled).

## Use

`vexel-platform-apple::VideoWriter` writes H.264/HEVC MP4/MOV through
`AVAssetWriter`, BT.709 tagged, exact `CMTime(ticks, timescale)` times, frames
composited on an opaque background. `AVAssetReader` is used only by tests.
The provenance lock JSON under `docs/dependencies/` still needs the three
crates' checksums (coordinator-owned).
