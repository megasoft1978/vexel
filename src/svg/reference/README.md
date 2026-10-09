# Local SVG reference support

This module borrows immutable source bytes and tokenizer tokens. It indexes at
most 65,536 non-empty IDs in a bounded open-addressed table, rejects duplicate
IDs, and recognizes only `href="#id"` or a correctly namespace-bound
`xlink:href="#id"`. Missing IDs, external URLs, malformed references and
duplicate IDs return the stable `unsupported` diagnostic at a source offset;
ID table exhaustion returns `limit`. It never loads external resources.

`<defs>` and uninstantiated `<symbol>` subtrees do not paint. A bounded
expansion pass supports shape/group/symbol targets, x/y translation, symbol
viewBox mapping, inherited use paint, and nested references. It caps expanded
bytes at 16 MiB, ID count at 65,536, reference depth at 16, and instances at
65,536. Missing IDs, duplicate IDs, cycles, external URLs, unsupported targets,
and unbound xlink prefixes fail closed. The expansion buffer is temporary and
the returned document retains the caller's original borrowed source bytes.

Copied source spans map parse diagnostics back to their original byte offsets.
Diagnostics in the small generated wrapper syntax fall back to the first live
`use` element's source offset. SVG 2 Document Structure was consulted
2026-10-09. The implementation is original project code.
