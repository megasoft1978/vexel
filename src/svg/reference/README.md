# Local SVG references

This module borrows immutable source bytes and tokenizer tokens. It indexes at
most 65,536 IDs in a bounded open-addressed table and recognizes only
`href="#id"` or a correctly namespace-bound `xlink:href="#id"`. Missing IDs,
external URLs, malformed references, duplicate IDs, and unsupported targets
return stable `unsupported` diagnostics at source offsets; bounds return
`limit`. It never loads external resources.

`<defs>` and uninstantiated `<symbol>` subtrees do not paint. A bounded
expansion pass supports shape/group/symbol targets, x/y translation, symbol
viewBox mapping, inherited use paint, and nested references. Each referenced
subtree copy is limited to 64 KiB; the use expansion sub-budget is 512 KiB per
job. The combined use and clip sub-budgets are at most 1 MiB. Reference depth,
instances, source-map entries, and IDs are each capped at 65,536. Generated
wrapper syntax uses fixed buffers below 4 KiB.

Copied source spans map parse diagnostics back to their original byte offsets.
Diagnostics in generated wrapper syntax fall back to the first live `use`
element's source offset. SVG 2 Document Structure was consulted 2026-10-09.
The implementation is original project code.
