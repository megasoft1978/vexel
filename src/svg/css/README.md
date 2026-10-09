# Bounded SVG paint CSS

`css.c` implements the paint-only CSS subset used by `svg_shape_decode`. It
borrows the original SVG byte slice and XML token vector. Selectors, property
names, declarations, and values remain byte spans into that input; the module
never rewrites or owns source text. The stylesheet object owns only bounded
rule/declaration metadata and must be destroyed after decoding.

The module recognizes fill, fill-rule, stroke, stroke-width, stroke-linecap,
stroke-linejoin, stroke-miterlimit, stroke-dasharray, and stroke-dashoffset.
Selectors are one optional element, one optional ID, and up to sixteen class
terms, with comma lists. It does not evaluate combinators, pseudo selectors,
attribute selectors, namespaces, at-rule bodies, imports, or resources. Rules,
declarations, stylesheet bytes, and total selector-match work have fixed caps
in `css.h`; rule storage grows only as needed, and allocation products are
checked before allocation. Matching class values are split on CSS
whitespace without allocating class strings; names/keywords and `!important`
are ASCII case-insensitive as CSS requires.

The cascade applies inherited computed paint, presentation attributes, matched
rules, and inline declarations by importance, specificity, and source order.
The shape decoder validates the winning paint value and reports its original
value byte offset. Non-paint declarations and unsupported selectors remain in
the unchanged SVG source and have no rendering effect.

This is original project code. CSS cascade and SVG styling behavior follow
CSS Cascade Level 3 and SVG 2 Styling (consulted 2026-10-09); no renderer or
editor implementation was consulted or copied.
