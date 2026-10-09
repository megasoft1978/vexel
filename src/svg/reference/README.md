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
