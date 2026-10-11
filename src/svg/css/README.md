# Bounded SVG paint CSS

`css.c` implements the source-preserving CSS subset used by `svg_shape_decode`.
It borrows immutable SVG bytes and XML tokens. Selector, property, declaration,
and value spans continue to point into the original source; only bounded
rule/declaration metadata is owned by the stylesheet.

The cascade recognizes 38 properties: fill, fill-rule, stroke, stroke-width,
stroke-linecap, stroke-linejoin, stroke-miterlimit, stroke-dasharray,
stroke-dashoffset, clip-path, stop-color, stop-opacity, fill-opacity,
stroke-opacity, opacity, visibility, display, color, font-family, font-size,
font-weight, font-style, text-anchor, letter-spacing, word-spacing, filter,
mask, mix-blend-mode, isolation, vector-effect, shape-rendering, paint-order,
marker and its start/mid/end longhands, mask-type, and
color-interpolation-filters. Text properties inherit through groups and
nested text spans. Presentation
attributes, supported stylesheet declarations, and inline style participate in
the cascade by importance, specificity, and source order. Fill/stroke values
and color inherit; `clip-path` is evaluated at its declaring element.

Selectors support comma lists and up to four descendant terms. Each term may
have an element, one ID, and up to sixteen classes. Class values split on CSS
whitespace without allocating strings. Child/sibling combinators, pseudo and
attribute selectors, namespace selectors, at-rules, imports, and external
resources are not evaluated. Unsupported matched declarations produce a stable
diagnostic instead of silently changing the cascade.

The stylesheet is capped at 1 MiB, 4,096 rules, 16,384 declarations, and
4,194,304 selector-match visits. Color parsing lives in `color.c` and accepts
148 CSS named colors, `rgb()`/`rgba()`/`hsl()`/`hsla()`, 3/4/6/8 digit hex,
`transparent`, and context-resolved `currentColor`. `color.c` does not fetch or
own source text; the caller supplies the span and receives RGBA bytes.

CSS cascade and SVG styling behavior follow CSS Cascade Level 3 and SVG 2
Styling (consulted 2026-10-09). The implementation is original project code;
no renderer or editor implementation was consulted or copied.
