# Lane w3-decode2 — real-world SVG robustness

## Implementation

The dashed cubic reproduction (`demo3.svg`) failed preflight because stroke work expanded across repeated dash pieces for each sample. Stroke geometry is now materialized once and queried through a conservative, bounded local-space grid. Solid strokes use the same cache. If a shape exceeds the grid bounds, rendering falls back to a bounded linear scan and charges that work during preflight. The test-only scalar renderer remains available for byte comparisons.

The grid uses cells at least 1 user unit wide, at most 128 cells per axis, and global limits of 65,536 cells, 1,000,000 references, and 16 MiB. Stroke pieces and joins are each capped at 65,536; dash construction is capped at 1,000,000 steps. The renderer retains its checked 200,000,000-operation work limit. Boundary marking now applies miter reach at joins instead of over-expanding every edge.

The original `demo3.svg` renders successfully at 1280×800 with the Release workbench command. Namespace-prefixed export metadata attributes and `sodipodi:namedview` metadata are also handled without weakening checks for recognized SVG attributes.

## Authored corpus and librsvg comparison

`tests/realworld/` contains 60 deterministic fixtures: Feather-style icons, Material-style icons, Inkscape and Illustrator exports, charts, logos, a 2,000-polygon map mosaic, and special cases. `tests/realworld/test_corpus.py` invokes `shape_render_cli` and `rsvg-convert` with an 8-second timeout per process and verifies source hashes and expected support.

Result: **60/60 pass**, with 0 failures. The maximum per-pixel channel error was 152/160; the maximum RGB error where both renders are nontransparent was 118/128. The `max pixel` metric includes alpha and jointly-covered RGB; `RGB` reports the RGB-only maximum over jointly-covered pixels. `Different pixels` counts pixels where either measured value differs. All fixtures completed within the process timeout.

| Fixture | Group | Vexel ms | librsvg ms | Max pixel | Max joint RGB | Different pixels |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `fixtures/charts/bar-chart-01.svg` | charts | 98.15 | 15.01 | 0 | 0 | 0 |
| `fixtures/charts/bar-chart-02.svg` | charts | 97.52 | 14.54 | 0 | 0 | 0 |
| `fixtures/charts/bar-chart-03.svg` | charts | 98.74 | 14.95 | 0 | 0 | 0 |
| `fixtures/charts/bar-chart-04.svg` | charts | 98.56 | 14.06 | 0 | 0 | 0 |
| `fixtures/charts/bar-chart-05.svg` | charts | 98.90 | 14.85 | 0 | 0 | 0 |
| `fixtures/feather/activity.svg` | feather | 6.56 | 13.49 | 14 | 8 | 238 |
| `fixtures/feather/archive.svg` | feather | 6.16 | 13.13 | 9 | 1 | 400 |
| `fixtures/feather/bell.svg` | feather | 5.19 | 13.01 | 65 | 34 | 254 |
| `fixtures/feather/calendar.svg` | feather | 7.67 | 13.00 | 46 | 3 | 413 |
| `fixtures/feather/camera.svg` | feather | 7.63 | 12.98 | 58 | 20 | 442 |
| `fixtures/feather/check-circle.svg` | feather | 8.61 | 12.88 | 77 | 13 | 426 |
| `fixtures/feather/clock.svg` | feather | 8.04 | 13.24 | 74 | 8 | 397 |
| `fixtures/feather/cloud-upload.svg` | feather | 7.31 | 13.13 | 85 | 85 | 398 |
| `fixtures/feather/download.svg` | feather | 5.16 | 12.78 | 9 | 1 | 230 |
| `fixtures/feather/eye.svg` | feather | 7.65 | 14.35 | 71 | 20 | 420 |
| `fixtures/feather/file-text.svg` | feather | 6.79 | 12.75 | 9 | 1 | 452 |
| `fixtures/feather/folder.svg` | feather | 5.47 | 12.77 | 11 | 8 | 322 |
| `fixtures/feather/heart.svg` | feather | 6.52 | 13.21 | 85 | 85 | 342 |
| `fixtures/feather/lock.svg` | feather | 6.84 | 13.08 | 58 | 3 | 385 |
| `fixtures/feather/mail.svg` | feather | 8.75 | 12.78 | 30 | 8 | 443 |
| `fixtures/feather/map-pin.svg` | feather | 7.49 | 13.14 | 85 | 85 | 380 |
| `fixtures/feather/search.svg` | feather | 6.03 | 13.32 | 74 | 63 | 292 |
| `fixtures/feather/settings.svg` | feather | 7.16 | 12.90 | 68 | 6 | 352 |
| `fixtures/feather/shopping-cart.svg` | feather | 6.44 | 13.27 | 27 | 8 | 335 |
| `fixtures/feather/user.svg` | feather | 6.55 | 13.36 | 77 | 63 | 316 |
| `fixtures/illustrator/illustrator-export-01.svg` | illustrator | 24.90 | 13.72 | 23 | 23 | 492 |
| `fixtures/illustrator/illustrator-export-02.svg` | illustrator | 24.97 | 13.39 | 31 | 31 | 490 |
| `fixtures/illustrator/illustrator-export-03.svg` | illustrator | 24.80 | 13.67 | 26 | 26 | 497 |
| `fixtures/illustrator/illustrator-export-04.svg` | illustrator | 24.61 | 13.60 | 35 | 35 | 473 |
| `fixtures/illustrator/illustrator-export-05.svg` | illustrator | 25.13 | 14.07 | 27 | 27 | 514 |
| `fixtures/illustrator/illustrator-export-06.svg` | illustrator | 25.03 | 13.44 | 28 | 28 | 490 |
| `fixtures/inkscape/inkscape-page-01.svg` | inkscape | 16.01 | 13.20 | 71 | 46 | 288 |
| `fixtures/inkscape/inkscape-page-02.svg` | inkscape | 15.42 | 13.37 | 94 | 94 | 279 |
| `fixtures/inkscape/inkscape-page-03.svg` | inkscape | 15.70 | 12.97 | 118 | 118 | 289 |
| `fixtures/inkscape/inkscape-page-04.svg` | inkscape | 15.13 | 12.84 | 59 | 50 | 297 |
| `fixtures/inkscape/inkscape-page-05.svg` | inkscape | 15.95 | 13.01 | 118 | 118 | 296 |
| `fixtures/inkscape/inkscape-page-06.svg` | inkscape | 15.87 | 13.21 | 95 | 60 | 293 |
| `fixtures/inkscape/inkscape-page-07.svg` | inkscape | 16.83 | 13.53 | 89 | 89 | 301 |
| `fixtures/inkscape/inkscape-page-08.svg` | inkscape | 16.79 | 13.02 | 74 | 63 | 295 |
| `fixtures/logos/curve-mark-01.svg` | logos | 23.83 | 13.50 | 141 | 118 | 327 |
| `fixtures/logos/curve-mark-02.svg` | logos | 23.06 | 13.08 | 142 | 78 | 327 |
| `fixtures/logos/curve-mark-03.svg` | logos | 23.32 | 12.75 | 152 | 62 | 338 |
| `fixtures/logos/curve-mark-04.svg` | logos | 22.81 | 13.27 | 105 | 40 | 347 |
| `fixtures/maps/map-mosaic-2000.svg` | maps | 187.87 | 46.47 | 48 | 31 | 60206 |
| `fixtures/material/account.svg` | material | 4.49 | 14.01 | 56 | 5 | 162 |
| `fixtures/material/bolt.svg` | material | 4.27 | 12.95 | 24 | 11 | 147 |
| `fixtures/material/bookmark.svg` | material | 3.63 | 12.91 | 14 | 14 | 48 |
| `fixtures/material/chat.svg` | material | 3.60 | 12.89 | 14 | 6 | 24 |
| `fixtures/material/dashboard.svg` | material | 4.02 | 12.57 | 0 | 0 | 0 |
| `fixtures/material/edit.svg` | material | 4.79 | 12.96 | 40 | 6 | 129 |
| `fixtures/material/favorite.svg` | material | 4.31 | 12.87 | 40 | 14 | 193 |
| `fixtures/material/home.svg` | material | 4.70 | 13.60 | 16 | 12 | 125 |
| `fixtures/material/layers.svg` | material | 6.37 | 13.57 | 40 | 11 | 456 |
| `fixtures/material/notifications.svg` | material | 3.84 | 12.51 | 51 | 7 | 128 |
| `fixtures/material/place.svg` | material | 3.82 | 12.19 | 54 | 37 | 240 |
| `fixtures/material/verified.svg` | material | 4.60 | 12.85 | 40 | 11 | 272 |
| `fixtures/special/arc-and-evenodd.svg` | special | 4.82 | 12.78 | 67 | 18 | 409 |
| `fixtures/special/dashed-cubic.svg` | special | 20.03 | 13.67 | 56 | 16 | 425 |
| `fixtures/special/dotted-circuit.svg` | special | 11.60 | 12.85 | 9 | 1 | 422 |
| `fixtures/special/nested-clip-gradient.svg` | special | 17.79 | 13.55 | 58 | 21 | 502 |

## Render-time profiles

The Release workbench `--render-frame` path was run three times per case and frame size; the table reports medians. Every invocation returned success. The target frame sizes are 1280×800 and 3840×2160.

| Fixture | Frame | Median ms |
| --- | ---: | ---: |
| chart-72-paths | 1280x800 | 30.97 |
| chart-72-paths | 3840x2160 | 62.25 |
| curved-logo | 1280x800 | 13.96 |
| curved-logo | 3840x2160 | 42.05 |
| map-2000-polygons | 1280x800 | 56.17 |
| map-2000-polygons | 3840x2160 | 80.10 |

Direct intrinsic-raster stress runs used generated copies with root dimensions set to the target size while retaining the source viewBox. The stats CLI returned the following; a `limit` is the renderer’s work cap rejecting the request before painting.

| Fixture | Size | Result | Time ms | Work operations |
| --- | ---: | --- | ---: | ---: |
| chart-72-paths | 1280×800 | pass | 254.97 | 53,017,886 |
| chart-72-paths | 3840×2160 | limit:0 | 398.37 | — |
| curved-logo | 1280×800 | pass | 414.63 | 86,534,667 |
| curved-logo | 3840×2160 | limit:0 | 178.80 | — |
| map-2000-polygons | 1280×800 | pass | 85.66 | 23,589,180 |
| map-2000-polygons | 3840×2160 | pass | 389.43 | 108,343,451 |

The direct 4K chart and curved-logo requests reached the 200M work limit and returned `limit:0` in under 0.4 seconds. The 4K map mosaic completed in 0.39 seconds at 108,343,451 counted operations. The regular viewer-frame path completed all three fixtures at both sizes in at most 0.081 seconds median.

## External corpus preview check

The pinned 58-file real-world corpus was checked read-only, one trial per file. **2/58 previewed**, 56 returned unsupported, 0 hit limits, and 0 crashed. The previewable files were `files/real/world-map-blank.svg` and `files/real/world-map-countries.svg`. No files in that external corpus were modified.

## Sanitizers and verification

- `./dev build` and `cmake --build --preset mac-release`: completed successfully.
- `python3 -m unittest tests.test_shape_render tests.realworld.test_corpus`: 24 tests passed. The real-world corpus test was rerun after correcting its different-pixel counter: `python3 -m unittest tests.realworld.test_corpus` passed (1 test).
- `build/mac-debug/bin/shape_test`: passed. Seeded optimized-versus-scalar renderer comparisons were byte-identical for 200 randomized documents and 160 paint/clip documents.
- `./dev shape-fuzz --duration 600 --seed 20261010 --cases 1000000 --output build/shape-fuzz-w3-decode2.json`: 1,000,000 cases under ASan+UBSan; 0 crashes, sanitizer reports, or invariants; one expected cap-exhaustion probe. Process elapsed 37.191 seconds. Peak RSS was not collected.
- `./dev asan` and `./dev ubsan` built successfully. Their CTest runs passed 24/26; `native-viewer-smoke` and `index-replay` could not run because the environment has no usable Metal/offscreen device and LaunchServices support.
- `./dev verify` was blocked by those same two native CTest failures. `./dev test-fast` and `python3 -m unittest discover -s tests -t .` each reported 241/251 passing, 8 failures, and 2 errors. The errors were native Metal/LaunchServices tests; failures included benchmark-harness process/capture/timeout assertions and PlatformDriver cycle coverage. These full-suite harness failures remain unresolved.
- `python3 scripts/ledger.py validate ledger.jsonl`: valid, 406 entries.

## Files and remaining limits

Changed raster stroke caching, bounded grid lookup, work preflight, and conservative bounds; expanded SVG export metadata handling; added focused regressions and the authored corpus generator, manifest, fixtures, and differential test. Module details and cache limits are in `src/raster/README.md`; decoder behavior is described in `src/svg/reference/README.md` and `src/svg/shape/README.md`.

Direct intrinsic 4K rendering of the chart and curved logo still exceeds the deliberate work cap. The 58-file external corpus has only two previewable examples. Full suite completion is unverified because of native Metal/LaunchServices failures and the listed harness assertions. No performance claim is made for inputs outside the measured fixtures.
