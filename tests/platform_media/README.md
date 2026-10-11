# Platform media tests

`platform_media_test.c` checks CoreText shaping structure, glyph and outline
limits, empty/combining/RTL text, fallback, emoji status, text-anchor offsets,
ImageIO PNG/JPEG/GIF decoding, dimension and byte caps, corruption handling,
premultiplied pixels, bilinear scaling, and a seeded 200,000-case Base64
campaign. CTest generates temporary fixtures before running this binary.

`generate_fixtures.py` creates PNG and GIF bytes directly with Python's standard
library and converts a generated opaque PNG to JPEG with macOS `/usr/bin/sips`.
It also writes compact oversized-dimension and oversized-pixel PNGs. No fixture
is downloaded. `tests/test_platform_media.py` exercises the test-only
`media_probe` JSON and PPM output.
