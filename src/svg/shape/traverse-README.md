# Source-backed SVG traversal

`traverse.c` owns the decoder's single tokenization pass, fixed-capacity local
ID index, CSS and transform contexts, and depth-first rendering order. It
borrows the original source and token offsets. A live `<use>` recursively walks
the referenced tokens at the use location; gradient and clip resolution keep
using the same original token array. It creates no rewritten SVG, copied
referenced subtree, generated wrapper, second token array, or source map.

The source limit is 16 MiB. The baseline decoder token array remains sized by
the tokenizer's established source-length formula. The ID table has a fixed
131,072 `uint32_t` slots (512 KiB) when any IDs exist and supports at most
65,536 unique IDs. Each referenced source span is at most 64 KiB and repeated
reference spans total at most 1 MiB per decode; these bounds cap traversal work
and do not authorize copying. Other rendering bounds are 64 XML/instance tree
levels, reference depth 16, 65,536 use instances, 4,194,304 active element
visits, and 4,096 transform operations per decode.

Malformed or external references and cycles return `unsupported`; exhausted
depth, instance, visit, ID, or transform bounds return `limit`. Diagnostics
point directly into the original source. Use-site paint is passed as the
virtual parent paint for a referenced target, and traversal emits its geometry
before continuing with following siblings so z-order follows the use position.
