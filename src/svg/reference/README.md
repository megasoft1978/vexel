# Local SVG references

The reference module borrows immutable source bytes and decoder tokens. It
builds a fixed-capacity open-addressed ID index with 131,072 `uint32_t` slots
(512 KiB), independent of source length and ID count, and rejects more than
65,536 IDs or duplicate IDs. It recognizes only `href="#id"` and correctly
root-bound `xlink:href="#id"`; missing IDs, external URLs, malformed
references, cycles, and unsupported targets return stable `unsupported`
diagnostics. It never loads external resources.

The index maps IDs to token offsets in the original source. `traverse.c`
resolves live uses by recursively walking those tokens at the use position. No
subtree copy, rewritten source buffer, generated wrapper, second tokenizer
array, or diagnostic source map is used. Diagnostics therefore retain their
original source byte offsets. Reference traversal bounds and inheritance are
documented in `../shape/traverse-README.md`.

`<defs>` and uninstantiated `<symbol>` subtrees do not paint. Referenced groups,
basic shapes, and symbols support use-site translation, symbol viewBox mapping,
inherited paint, and nested references subject to depth, instance, and work
caps. A referenced source span is limited to 64 KiB, with a 1 MiB aggregate
span-visit budget per decode. These counters bound repeated traversal; they do
not trigger any source copy. The original source remains borrowed and unchanged. SVG 2 Document
Structure was consulted 2026-10-09. The implementation is original project
code.

Attribute validation includes stroke-capable paths and basic shapes, rounded
rectangle `rx`/`ry`, CSS presentation values, and nested `<svg>` viewport
attributes. Text validation includes `xml:space` and the supported font, text
anchor, and spacing properties; image hrefs are accepted only by the media-aware
shape decoder when they use the supported embedded Base64 PNG/JPEG form.
Unsupported attributes still fail with the original token's source offset.
Nested viewBox mappings and viewport clipping are performed by shape traversal;
this module only indexes and validates local references.

Root `xmlns:*` declarations and unknown prefixed export metadata (for example
Inkscape labels or Illustrator layer tags) are ignored for rendering while the
borrowed source remains unchanged. A prefixed spelling of a recognized SVG
paint or geometry attribute remains unsupported, so foreign metadata cannot
silently override SVG semantics. `xlink:href` continues to be accepted only
for validated local references.
