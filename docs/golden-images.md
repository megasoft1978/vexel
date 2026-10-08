# Golden image comparison

The `scripts/golden_diff.py` utility measures an actual RGBA PNG against one
fixture pinned by `tests/golden/manifest.lock`. It verifies the frozen baseline
hash and the fixture's SVG source hash before producing output. It never writes
to a baseline.

## PNG support

The decoder supports 8-bit RGBA PNGs without interlacing and PNG row filters
None, Sub, Up, Average, and Paeth. It validates chunk CRCs, ordering, image
data size, and compressed stream completion. The only ancillary chunk accepted
is the six-byte `bKGD` chunk present in the locked images. Other color types,
bit depths, interlacing, chunks, truncated input, and images above the 64 MiB
encoded or 16,777,216 pixel limits are errors. The program uses only Python's
standard library.

## Metrics and tolerance

Exact metrics always compare all four channels. The report contains exact equal
and differing pixels, differing channels, maximum channel delta, total
absolute channel delta, and mean absolute channel delta over all RGBA channel
slots. These values never change when tolerance is enabled.

`--edge-radius` is an explicit Chebyshev radius from 0 through 2 (default 1).
`--edge-channel-tolerance` is a maximum channel delta from 0 through 16
(default 1). A differing pixel is tolerated only when it is within the declared
channel delta and within the declared radius of an 8-neighbor color or alpha
boundary in the expected image. Radius 0 disables tolerance. Tolerated and
non-tolerated pixels are reported separately. Exit 0 means all differences
were tolerated; exit 1 means at least one failed; exit 2 means an input,
metadata, format, or output error.

## Report and heatmap

The version 1 JSON report records fixture ID; expected and actual PNG SHA-256;
pinned SVG SHA-256; build SHA; scale, frame, font, backend; dimensions;
tolerance settings; exact metrics; tolerance counts; and pass/fail result.
Metadata is mandatory. SHA values use 64 lowercase hexadecimal characters;
scale is 0.01–64; frame is 0–1,000,000; font and backend are bounded printable
strings. The RGBA heatmap marks unchanged pixels transparent, tolerated pixels
yellow, and failing pixels red. Both output files must be new and remain under
repo-local `.cache/`; path symlinks and output overwrites are rejected.

Example:

```sh
python3 scripts/golden_diff.py \
  --fixture shapes-basic \
  --actual .cache/render/shapes-basic.png \
  --report .cache/golden/shapes-basic.json \
  --heatmap .cache/golden/shapes-basic-diff.png \
  --svg-sha256 6f546464f23d6cd95be30fe2c98130b84b0f75796e4578b330b0695dcf6de5c7 \
  --build-sha 0000000000000000000000000000000000000000000000000000000000000000 \
  --scale 1 --frame 0 --font 'System Sans' --backend scalar
```

Golden baseline changes require a separate READY work order and a recorded
reason. This tool does not provide a baseline update operation.
