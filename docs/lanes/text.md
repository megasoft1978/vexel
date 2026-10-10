# Wave 4 text, images, and selection data

## Implemented

- The shape decoder accepts injected text and image providers. The app adapter
  maps those calls to CoreText and ImageIO without adding platform dependencies
  to `vexel_shape`.
- `<text>` and `<tspan>` use inherited font-family, size, weight, style,
  text-anchor, letter-spacing, and word-spacing. x/y/dx/dy lists, XML entities,
  whitespace collapsing, and `xml:space="preserve"` are handled before
  CoreText outlines enter the existing path and raster pipelines.
- `<image>` accepts embedded Base64 PNG/JPEG data URIs, sizing,
  `preserveAspectRatio`, transforms, clip paths, and opacity. ImageIO provides
  static premultiplied RGBA; the scalar rasterizer samples it bilinearly.
- Decoded shapes expose source element spans, transforms, user/device bounds,
  source-offset resolution, and reverse-order fill/stroke hit-testing. Paths
  retain source spans for each command group and numeric literal.
- Media tests use fake providers for deterministic structural checks. The
  platform-backed differential CLI compares actual text and image output with
  `rsvg-convert`.

## Bounds and unsupported cases

Text is limited to 64 KiB, 16,384 glyphs, and 262,144 outline commands per
decode. The run cache is decode-local and bounded to 32 entries, 256 KiB of
text keys, and 262,144 cached commands. Images are limited to 32 MiB encoded
per source and 64 million decoded pixels per document. Input source bytes stay
borrowed and unchanged. External image references, SVG image data, and
`<textPath>` return stable unsupported diagnostics. Platform ImageIO support
and installed fonts determine available image formats and font faces.

## Differential tolerance

Image differential coverage uses a 1x1 PNG scaled under a clip, translation,
and half opacity; alpha and coverage must match exactly, with at most one value
of RGB-channel difference for premultiplication rounding. Text
differential coverage asserts nonempty outlines, a width difference of at most
8 pixels, a height difference of at most 4 pixels, a horizontal-center
difference of at most 4 pixels, and an ink-pixel count difference no greater
than half the reference count (with a 32-pixel minimum allowance). CoreText and
Pango can resolve different generic sans-serif fonts and antialias their edges
differently, so the text check compares placement and silhouette rather than
requiring identical glyph pixels. The shape unit test also asserts font/run
structure, nested span handling, and source correspondence.

## Verification

- `./dev build` passed.
- `ctest --test-dir build/mac-debug --output-on-failure` passed 26 of 29
  tests. The new `svg-text-image` target passed. The three failures are the
  stale viewer-frame test described below and two tests that require Metal.
- `./dev asan` and `./dev ubsan` built successfully and each passed 26 of 29
  tests, with the same three failures. The focused ASan `svg-text-image` test
  passed.
- `./dev shape-fuzz --cases 1000000 --duration 600 --seed 12648430
  --output .cache/w4-text/shape-fuzz-report.json` passed 1,000,000 cases under
  ASan and UBSan, with zero crashes, sanitizer reports, or invariant failures.
  Text and image syntax both reached render coverage.
- `python3 -m unittest tests.svg.test_text_image_render
  tests.realworld.test_corpus` passed all 3 tests. The real-world decoder report
  contains 60 cases: 60 passed, 0 failed.
- `./dev bench-corpus --check` passed for 58 files and 116 fresh trials. The
  benchmark corpus produced 2 previewable outcomes and 56 unsupported outcomes,
  with 0 limit or crash outcomes.
- `python3 -m unittest discover -s tests -t .` ran 253 tests and reported 9
  failures and 2 errors. Seven failures are benchmark sandbox subprocesses,
  one is a native driver smoke test blocked by the environment, and one is the
  stale viewer-frame expectation. The two errors require unavailable Metal
  readback/replay support.

## Remaining limits and environment failures

The viewer-frame test still treats a bare `<text>` element as unsupported;
text rendering now correctly accepts it. It should be updated in its owning
test lane without weakening its other assertions. `native-viewer-smoke` cannot
present a frame because no Metal device or command queue is available here;
`index-replay` cannot create its offscreen Metal renderer.

The selected fixtures do not establish complete SVG text shaping parity,
vertical writing, textPath layout, color-font emoji outlines, animated images,
or support for external image resources. Platform ImageIO support and installed
fonts determine available image formats and font faces.
