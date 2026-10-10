# Source-backed SVG traversal

`traverse.c` owns the decoder's single tokenization pass, fixed-capacity local
ID index, CSS and transform contexts, and depth-first rendering order. It
borrows the original source and token offsets. A live `<use>` recursively walks
the referenced tokens at the use location; gradient and clip resolution use
spans from the same original token array. The `<use>` path creates no rewritten
SVG, copied referenced subtree, generated wrapper, second token array, or
source map. The clip module separately decodes a bounded child span through a
small generated wrapper; its scratch limits are documented in
`../clip/README.md`.

The source limit is 16 MiB. The baseline decoder token array remains sized by
the tokenizer's established source-length formula. The ID table has a fixed
131,072 `uint32_t` slots (512 KiB) when any IDs exist and supports at most
65,536 unique IDs. Each referenced source span is at most 64 KiB and repeated
reference spans total at most 1 MiB per decode; these bounds cap `<use>`
traversal work and do not authorize use-subtree copying. The aggregate byte
budget makes all 65,536 instances reachable only when each target span is at
most 16 bytes. Other rendering bounds are 64 XML/instance tree levels,
reference depth 16, 65,536 use instances, 4,194,304 active element visits, and
4,096 transform operations per decode.

`<use>` composes the use transform before its `x`/`y` translation and, for a
symbol, before the symbol viewBox mapping. Paint and clip sampling receive the
referencing element's current inverse affine. Singular transforms with a
gradient or clip fail with a stable `unsupported` diagnostic. `clip-path` is
applied where declared: group and use declarations use the whole instantiated
subtree's local AABB, resolve one clip record, and intersect it with clips on
descendant shapes. It is not inherited into each child style.

A `clip-path` on a `<symbol>` is rejected at its source attribute or computed
CSS value. The local librsvg reference renderer ignores that declaration on a
symbol, so the decoder does not claim the clipped rendering is supported.

Malformed or external references and cycles return `unsupported`; exhausted
depth, instance, visit, ID, or transform bounds return `limit`. Diagnostics
point directly into the original source. Use-site paint is passed as the
virtual parent paint for a referenced target, and traversal emits its geometry
before continuing with following siblings so z-order follows the use position.

W5 traversal resolves local mask, pattern, marker, and filter definitions at
the use site while retaining their original token offsets. `<a>` passes paint
and transform context to its descendants. `<switch>` walks only its first
child whose bounded `systemLanguage` list matches the configured language;
headless decode defaults to `en`, and callers may inject a language tag.
Recursive mask, pattern, and marker references have explicit depth and instance
caps. Unsupported filter primitives and `switch` capability requirements
return stable diagnostics instead of being silently skipped.
