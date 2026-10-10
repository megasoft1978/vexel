# Platform media lane

## Built

- Added CoreText/CoreGraphics UTF-8 shaping with ordered font-family fallback,
  generic family mapping, numeric weight, italic style, letter spacing,
  kerning, ligatures, bidi layout, glyph advances, and point-scaled outlines.
- Exported path commands as move, line, cubic, and close. Quadratic path
  elements are converted to equivalent cubic curves. Text-anchor helpers use
  the measured CoreText line width.
- Added ImageIO decoding for PNG, JPEG, single-frame GIF, and WebP when
  available. Results are owned premultiplied RGBA8; static images are bounded
  by encoded bytes, dimensions, and pixel count. Multi-frame inputs are
  rejected with a stable `animated` status.
- Added a strict `data:image/png;base64,` decoder and checked bilinear scaling.
- Added generated PNG/JPEG/GIF fixtures, C structural/boundary tests, a seeded
  200,000-case Base64 campaign, Python CLI checks, and the test-only
  `media_probe` outline-JSON/PPM tool.

## Limits and ownership

Text inputs are limited to 64 KiB, 16,384 glyphs, and 262,144 outline commands;
family lists have at most 32 entries of at most 256 bytes each. Image input is
limited to 32 MiB, 64 million pixels, and 16,384 pixels per dimension. The C
headers describe borrowed inputs, owned outputs, stable error names, and the
required destroy functions. The implementation uses only macOS system
frameworks.

## Unverified

- WebP decoding depends on the OS ImageIO codec and is not guaranteed on macOS
  13; no WebP fixture is included.
- Emoji outlines are deliberately returned as unsupported. Color glyph layers,
  vertical writing, multi-line text, and shaping options beyond the declared
  family/weight/style/letter-spacing request are outside this interface.
- GIF animation is rejected, including otherwise decodable first frames, to
  keep the returned image contract static.
- The implementation is not yet connected to SVG `<text>` or `<image>`; a later
  integration lane can consume these plain-data interfaces.
