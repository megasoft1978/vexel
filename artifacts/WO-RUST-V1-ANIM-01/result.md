# WO-RUST-V1-ANIM-01 result

## Decisions
- Candidate audit: protocol and v1-document candidates were already in main (protocol identical after rustfmt; document a superset), so only the candidate-app preview files were taken verbatim (`source-diff.json`). The candidates' recorded base predates `rust/`, so main at the seed commit was the comparison.
- New `vexel-gif` (no dependencies, `forbid(unsafe_code)`). New app modules live in `src/animation/` (`frame`, `playback`, `export`, `job`, `present`); `main.rs` gained one `mod` line, `preview.rs` an `Operation::Export` variant and parse arm.
- Frames come from the real chain: decode, `evaluate_frame`, `vexel-render` (1x), composited over white. The preview window raster now uses it (cached decode); unsupported features are typed errors, never approximated.
- Export is a background job in the window loop and is cancelled on close. The file is fully encoded in memory, written to a temp sibling, then hard-linked (no overwrite) or renamed, so failures leave nothing partial. The MCP preview profile gained `export_animation` (gif only, fps 1-30, duration up to 10 s).
- Frame count is `ceil(duration * fps)` (1e-9 tolerance), sampled at `i / fps`, as the C exporter does.

## GIF scope
GIF89a, one global palette: exact when at most 256 slots, otherwise a fixed 6x7x6 palette without dithering (max channel error 26). Alpha 0 is transparent; other alpha is opaque (the app composites over white first). Delay is `round(100/fps)` centiseconds, minimum 1. Limits: 120 frames, 60 fps, 4096 px, 32M pixels, 128 MiB.

## ImageIO comparison
The C path (ImageIO) gives better palettes and exact fractional delays but needs platform FFI, is not deterministic across OS versions and is hard to test headlessly. The first-party encoder is deterministic and covered by a test-only decoder, at lower colour quality.

## Not executed
- Live playback feel in the native window (timing, smoothness, per-frame render cost) was not run; no GUI was launched.
- Third-party viewer compatibility was checked only with Pillow and macOS `sips`, not browsers, Preview or Quick Look.
- Playback in the window is still driven by `PreviewState`; `Playback` and `FrameQueue` are tested but not yet wired into it.
- Frame-sequence export, the `OutputTooLarge` limit test, and the 512 px export size cap of the C job were not done.
