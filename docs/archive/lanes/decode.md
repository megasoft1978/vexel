# Decode and rasterizer lane

## Survey gap table

The starting point is commit `ed929e04aa419aa8ffc6848d12f34c55b704d22a`.
The decoder already handled SVG path commands, basic shapes, custom stroke
geometry, common transforms, evenodd fills, basic CSS, local references,
linear and centered radial gradients, and bounded clipping. The gaps identified
for this lane were path strokes, rounded rectangles, full CSS colors, additional
style/transform behavior, nested viewports and lengths, extended gradients, and
descendant selectors.

| Area | Gap at lane start | Lane action |
|---|---|---|
| Path and basic shapes | `<path>` stroke rejected; `rect` ignored `rx`/`ry` | Added path strokes and elliptical rounded rectangles; retained subpaths/open/closed metadata for caps, joins and dashes |
| Color | Five names, `#RGB`, `#RRGGBB`; separate smaller stop parser | Added one parser for 148 CSS names, RGB/HSL functions, alpha hex, `transparent`, and contextual `currentColor` |
| Style and transforms | No skew or opacity; type/id/class selectors only | Added skew, fill/stroke/element/group opacity, visibility/display, and bounded descendant selectors |
| Viewports and lengths | Root viewBox only; unitless/px root dimensions | Added nested `<svg>`, preserve-aspect-ratio alignment/meet/slice/none, percentages and physical/em units |
| Gradients | Pad spread, literal stops, centered radial default | Added reflect/repeat, gradient transforms, inherited stops via href, and focal coordinates |

SVG `<title>`, `<desc>`, and `<metadata>` blocks are now tokenized and skipped as
non-rendering content, so accessible labels and metadata do not prevent nearby
geometry from rendering. Unknown rendering/XML/CSS features continue to fail
with stable diagnostics instead of producing guessed pixels.

## Verification and limitations

The focused differential suite passed all 21 tests. Its `rsvg-convert`
comparison covered 62 fixtures with zero threshold failures, and repeatability
checks passed. Added fixtures cover path
strokes/arcs, elliptical rounded rectangles, named/functional colors, CSS
composition, isolated group opacity, nested/root viewports, lengths, and
extended gradients. Existing differential thresholds were left unchanged. The
seeded internal renderer comparison covers 200 geometry documents and 160
paint/clip documents. `shape_test` and `css_test` pass.

The checked-in 24-file pinned SVG corpus was rendered with
`build/mac-debug/bin/shape_render_cli`; outputs were written under
`/private/tmp` and removed afterward. **18 of 24 files rendered successfully.**
The six unsupported files are `image-embedded-data`,
`namespaces-custom-attributes`, `stress-text-collection`, `text-basic`,
`text-positioning`, and `text-spans` (images, text, or prefixed foreign
attributes remain outside this lane).

`./dev bench-corpus --check` was also attempted. It could not validate the
separate 58-file M4 index-preview report because this worktree has no retained
`.cache/M4/head-to-head-raw.json`. The checked-in historical report records 2
of 58 files as previewable before this lane; that count is not a rerun and is
not comparable to the 18/24 direct shape-render corpus result above.

Supported lengths are percentages, `px`, `pt`, `mm`, `cm`, `in`, and `em`;
`em` currently resolves to 16 user units and does not use inherited
`font-size`. CSS supports up to four descendant selector terms; child/sibling,
pseudo, attribute, namespace, and resource selectors remain unsupported. Text,
external resources, and broader SVG features outside the implemented subset
remain unsupported. Opacity groups use bounded tile scratch and fail with a
`limit` diagnostic above 64 MiB.

`./dev build` and the macOS Release preset build pass. Debug CTest passes 18 of
20 tests; `native-viewer-smoke` and `index-replay` fail because the environment
has no usable Metal/window device. Both `./dev asan` and `./dev ubsan` report
the same two environment failures, while all shape, CSS, and parser tests pass
without sanitizer reports. The full Python suite, including `./dev test-fast`,
ran 213 tests: 203 passed, 8 failed, and 2 errored. The remaining failures are
the two Metal driver errors, the 100-cycle Metal smoke failure, and seven
benchmark-harness failures where sandboxed subprocesses exit 71.

The final shape fuzzer command was
`./dev shape-fuzz --duration 600 --cases 1000000 --seed 20261010 --output /private/tmp/w2-decode-shape-fuzz.json`.
It passed 1,000,000 cases in 34.966 seconds with zero crashes, invariant
failures, or ASan/UBSan reports. All nine new feature families had nonzero
case and render counts; the report and empty sanitizer stderr are in
`/private/tmp`.
