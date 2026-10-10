# Platform media primitives

This module provides macOS-backed, plain-C interfaces for text outlines and
embedded image data. `platform_text.m` uses CoreText/CoreGraphics; it returns
caller-owned glyph and path arrays with no CoreText types in the public API.
Coordinates are in points, with glyph outlines expressed in a y-up baseline
coordinate system. CoreText performs bidi shaping, fallback, kerning, and
ligature selection. Generic families map to Times, Helvetica Neue, and Menlo.

`platform_image.m` uses ImageIO to decode one static PNG, JPEG, GIF frame, or
ImageIO-supported WebP from a borrowed byte slice. It returns owned,
top-row-first, premultiplied RGBA8. Decoding performs no file or network access.
The URI helper accepts the exact `data:image/png;base64,` prefix and strict
canonical Base64. The bilinear scaler accepts and returns premultiplied RGBA8.

Public limits are 64 KiB of text, 16,384 glyphs, 262,144 path commands, 32 MiB
of encoded image input, 64 million pixels, and 16,384 pixels per dimension.
Multi-frame images are rejected with `animated`; static GIF input decodes its
single frame. Emoji/color-only glyphs return `unsupported`. WebP is available
only when the installed macOS ImageIO supports the format.

Results must start empty (`PLATFORM_TEXT_RESULT_INIT`, `PLATFORM_IMAGE_INIT`, or
`PLATFORM_IMAGE_BYTES_INIT`) and are released with the matching destroy call.
The caller retains all input slices. See `tests/platform_media/` and
`media_probe` for structural checks and test-only differential output.
