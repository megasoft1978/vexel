# W5 filters, masks, patterns, and markers

## Implemented

- Local alpha and luminance `<mask>` definitions support `maskUnits`,
  `maskContentUnits`, region bounds, and nested masks.
- Pattern fills and strokes support user-space and object-bounding-box units,
  `patternContentUnits`, `patternTransform`, `viewBox`, and repeat tiling through
  cached RGBA tiles.
- Marker start, mid, and end placement works on paths, lines, polylines, and
  polygons. The decoder handles `marker` shorthand, `markerUnits`, `orient`
  values `auto`, `auto-start-reverse`, and numeric angles, `refX`/`refY`, and
  marker viewBox mapping.
- The bounded filter subset includes `feGaussianBlur`, `feOffset`, `feFlood`,
  `feBlend` (normal, multiply, screen, darken, lighten), `feComposite`,
  `feMerge`, `feColorMatrix` (matrix, saturate, hueRotate,
  luminanceToAlpha), and `feDropShadow`. `SourceGraphic`, `SourceAlpha`, named
  `result` chaining, filter and primitive units, filter regions, and
  `color-interpolation-filters` in `linearRGB` and `sRGB` are supported.
- CSS supports `mix-blend-mode`, `isolation`, `vector-effect`,
  `shape-rendering`, `paint-order`, marker properties, `mask-type`, and filter
  color interpolation. Group blends support normal, multiply, screen, darken,
  and lighten. `<switch>` selects the first child matching the default `en`
  language or an injected language tag; `<a>` passes through to its children.
  Local `<symbol>` viewBox traversal accepts W5 paint properties.

## Limits and unsupported cases

- Masks, patterns, and markers allow at most 1,024 instances per decode and
  eight nested definitions. Pattern tiles are limited to 4,096 pixels per
  axis, 1,048,576 pixels per tile, and 16 MiB of aggregate retained tile data.
- A filter chain has at most 16 primitives and eight merge inputs. Work is
  capped at 50 million pixel-primitive operations; filter scratch and isolated
  group scratch each have a 64 MiB cap.
- Turbulence, displacement maps, convolution, filter images, lighting, and
  morphology return stable named unsupported diagnostics. No external paint,
  mask, marker, or filter references are loaded.
- Marker overflow clipping and keyword marker reference positions are not
  implemented. `non-scaling-stroke` currently supports uniform affine and
  viewport scales. `stroke-linejoin: arcs` and `miter-clip` use stable round
  and miter fallbacks. Crisp edges use the deterministic center sample.
- `<switch>` handles `systemLanguage`; `requiredFeatures` and
  `requiredExtensions` return unsupported. The corpus keeps its language
  alternatives visually equivalent because librsvg selection depends on its
  host language; an exact unit check verifies `en`, `it`, and fallback behavior
  through the injectable decoder entry point.

## Verification

- `./dev build` and `cmake --preset mac-release && cmake --build --preset
  mac-release` passed. `build/mac-debug/bin/shape_test` passed.
- `python3 -m unittest tests.test_shape_render
  tests.svg.shape.test_w5_render tests.realworld.test_corpus` passed all 39
  tests. The authored real-world corpus contains 87 files: 86 supported files
  rendered successfully and one morphology fixture returned
  `unsupported-feMorphology`. The differential report has zero failures.
- `./dev shape-fuzz --cases 1000000 --duration 600 --seed 20261010 --output
  .cache/w5-filters/shape-fuzz-report.json` passed 1,000,000 cases under
  AddressSanitizer and UndefinedBehaviorSanitizer in 47.204 seconds. It recorded
  zero crashes, sanitizer reports, or invariant failures. All 22 feature
  classes, including each W5 class, reached both parse and render coverage.
- `ctest --test-dir build/mac-debug --output-on-failure`, `./dev asan`, and
  `./dev ubsan` each passed 29 of 32 tests; one test was skipped and two failed
  because the environment has no Metal device/queue. `native-viewer-smoke`
  could not present a frame, and `index-replay` could not create an offscreen
  Metal renderer. The shape, CSS, and W5 reference tests passed in all three
  configurations.
- After building Release, `python3 -m unittest discover -s tests -t .` ran 269
  tests: 258 passed, eight failed, two errored, and one skipped. Seven failures
  are benchmark sandbox process/capture assertions. The remaining failure and
  both errors are native driver checks blocked by LaunchServices or unavailable
  Metal presentation/readback. `./dev test-fast` was run before the Release
  build and also reported 269 tests with the same benchmark failures; its
  native driver checks additionally lacked the Release executable.
- `python3 scripts/ledger.py validate ledger.jsonl` passed with 406 entries.

No performance claim is made beyond these bounded test fixtures and fuzz
campaign. The implementation is selected SVG support, not full browser parity.
