# SVG shape decoding

The shape module turns borrowed SVG bytes and XML tokens into owned flattened
edges, per-shape fill and stroke state, gradients, clips, and opacity groups.
`traverse.c` owns bounded token traversal and inherited contexts;
`traverse_use.c` resolves live `<use>` instances. `transform.c` parses affine
transforms; `shape_path.c` decodes paths and `shape_geometry.c` decodes basic geometry; `shape_custom.c`
shares style, paint, stroke, and clip setup with text/image geometry. `text.c`
lays out text runs, `text_cache.c` bounds and owns shaped runs, and `image.c`
decodes embedded data through the injected provider. `viewport.c` resolves root,
nested `<svg>`, and symbol viewports; `document.c` owns public cleanup and
`selection.c` provides editor mapping, bounds, and hit tests.

`svg_shape_decode` also attaches a bounded, source-backed SMIL model on
`svg_shape_document.animation`. Animation elements are omitted from static
traversal while their target/keyframe source spans and stable diagnostics stay
available for timeline enumeration and explicit-time evaluation.

Supported geometry includes all SVG path commands (`M/L/H/V/C/S/Q/T/A/Z`,
absolute and relative), `<line>`, `<polyline>`, `<polygon>`, `<circle>`,
`<ellipse>`, and `<rect>`. Paths retain subpath and closed-stroke metadata so
strokes support caps, joins, miter limits, dash arrays, and dash offsets.
Rounded rectangles implement the `rx`/`ry` copy rule, radius clamping, and
elliptical corners. Curves are flattened with a transformed 0.25-unit tolerance
and a recursion cap of 16. Paths are limited to 65,536 commands and documents
to 262,144 edges.

Transforms include matrix, translate, scale, rotate (including a center point),
`skewX`, and `skewY`. The paint CSS cascade covers fill/stroke rules, color and
opacity properties, visibility, display, clips, and gradient stops. The shared
color parser accepts 148 named colors, `rgb()`/`rgba()`/`hsl()`/`hsla()`, 3/4/6/8
digit hex, `transparent`, and context-resolved `currentColor`. Selectors allow
up to four descendant terms and sixteen class terms per term. Child/sibling,
pseudo, attribute, namespace, at-rule, and resource selectors remain unsupported.

Root and nested `<svg>` viewports support viewBox transforms and
`preserveAspectRatio` alignment, `meet`, `slice`, and `none`. Nested viewports
clip their descendants. Shape lengths accept percentages and `px`, `pt`, `mm`,
`cm`, `in`, and `em`; `em` currently resolves to 16 user units and does not
compute inherited `font-size` outside text layout. Fill/stroke opacity and
isolated element/group opacity are supported. Opacity group rendering uses
bounded tile scratch and returns `limit` if nested layers exceed 64 MiB.

An additive media-aware entry point accepts injected text and image providers;
the core decoder does not link to platform frameworks. `<text>` and `<tspan>`
inherit font family, size, weight, style, anchor, and spacing from the CSS
cascade. They accept x/y/dx/dy coordinate lists, decode the five XML named
entities and numeric entities, collapse whitespace by default, and honor
`xml:space="preserve"`. CoreText glyph outlines are converted into the regular
shape edge pipeline. Text is capped at 64 KiB, 16,384 glyphs, and 262,144
provider path commands per document; repeated identical runs use a bounded
decode-local cache. `<textPath>` returns a stable unsupported diagnostic.

`<image>` accepts strict Base64 `data:image/png` and `data:image/jpeg` URIs,
local x/y/width/height, transforms, opacity, clip paths, and
`preserveAspectRatio` meet/slice/none. The injected ImageIO provider decodes
premultiplied RGBA with a 32 MiB encoded-input cap and a 64-million-pixel
aggregate budget per document. The scalar rasterizer samples bilinearly.
External image references and SVG data URIs remain unsupported; no filesystem
or network resource is loaded.

Each rendered shape carries its borrowed-source element byte range and
transform. Additive queries expose user-space and device-space bounds, resolve
a source byte offset to the most specific rendered element, and hit-test in
reverse paint order with fill, stroke tolerance, and clip membership. Every
decoded path also retains command-group and numeric-literal byte spans for
node-editing correspondence. The source bytes remain borrowed and unchanged.

`<defs>`, `<title>`, `<desc>`, `<metadata>`, and Inkscape's
`sodipodi:namedview` are non-rendering; metadata subtrees are structurally
tokenized and then skipped. Unknown prefixed export attributes are ignored
without changing their borrowed source bytes. `<symbol>` is
non-rendering unless reached by a live `<use>`. Local `href="#id"` and
root-bound `xlink:href="#id"` references
traverse the original token array at the use position. The ID table supports up
to 65,536 unique IDs; reference depth is capped at 16, instances at 65,536,
each referenced span at 64 KiB, and aggregate span visits at 1 MiB. Diagnostics
keep offsets into the original borrowed source.

Clip paths support local references, `userSpaceOnUse` or `objectBoundingBox`,
and unions of up to 64 basic shape/path children. Group/use clips use their
subtree AABB. Nested clip declarations inside clip geometry and external
references fail closed. Raster work, clip geometry, stylesheet data, and
references retain their fixed budgets; unsupported XML/CSS semantics return
stable diagnostics rather than dropping source or fetching resources.

The deterministic scalar 4×4 rasterizer is compared with librsvg using small
fixtures. Text uses structural, ink-bound, and ink-area tolerances because
CoreText and Pango may resolve different sans-serif faces and rasterize glyph
edges differently. This is selected-feature differential evidence, not full
SVG parity.
SVG semantics follow W3C SVG 2 Shapes, Paths, CSS, Paint Servers, Document
Structure, and Clipping specs (consulted 2026-10-09). No SVG editor source is
used.

## W5 paint servers and effects

The decoder also resolves local `<mask>`, `<pattern>`, `<marker>`, and
`<filter>` references. Masks support alpha and luminance sampling, both mask
coordinate systems, and nested local masks. Patterns support fill and stroke
paint, user-space and object-bounding-box units, content units, transforms,
viewBox mapping, and repeat tiling from a decode-local RGBA tile cache. Marker
start, mid, and end positions work on paths, lines, polylines, and polygons;
`marker` shorthand, stroke-width or user-space sizing, `auto`,
`auto-start-reverse`, numeric orientation, reference points, and marker viewBox
mapping are included.

Filters support bounded `feGaussianBlur`, `feOffset`, `feFlood`, `feBlend`,
`feComposite`, `feMerge`, `feColorMatrix`, and `feDropShadow` chains. Filter
and primitive units, regions, named results, `SourceGraphic`, `SourceAlpha`,
and `color-interpolation-filters` (`linearRGB` or `sRGB`) are handled. Stable
named unsupported diagnostics cover turbulence, displacement, convolution,
image, lighting, and morphology primitives. CSS blending supports normal,
multiply, screen, darken, and lighten with isolated groups. `<switch>` selects
the first supported `systemLanguage` child; the default language is `en`, and
`svg_shape_decode_with_system_language` injects another bounded tag. `<a>` is a
pass-through container. Existing `<symbol>` viewBox traversal also accepts
the W5 paint properties.

The CSS cascade recognizes 38 properties, including W5 effects, markers,
masking, blending, paint order, shape rendering, and vector effects. Crisp-edge
rendering uses the deterministic center sample. Non-scaling strokes currently
support uniform affine and viewport scales. `stroke-linejoin: arcs` and
`miter-clip` use stable round and miter fallbacks.

Definitions and scratch stay bounded: masks, patterns, and markers cap
instances at 1,024 per decode; pattern tiles cap each axis at 4,096 pixels, each tile at 1,048,576
pixels, and aggregate retained tiles at 16 MiB. Mask, pattern, and marker
recursion is capped at eight. Filter chains cap at 16 primitives, merges at
eight inputs, 50 million pixel-primitive work units, and 64 MiB of filter
scratch. Isolated group scratch remains capped at 64 MiB. Cycles and exhausted
limits fail with stable diagnostics. External filter, mask, pattern, and marker
references are not loaded.
