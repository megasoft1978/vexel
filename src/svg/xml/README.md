# Bounded XML tokenizer

`svg_xml_tokenize` reads a borrowed `base_bytes` view and writes caller-owned
`svg_xml_token` entries. It never allocates, reads files, resolves entities,
normalizes line endings, or mutates input. Every span is a half-open byte range
into the exact original UTF-8 input; attribute value and comment/PI/text/CDATA
content spans exclude their delimiters. QName `prefix` is empty when absent;
`local_name` is always populated. Namespaces are not resolved. Token data is
valid only while the caller's input remains alive and unchanged.

The scanner is iterative, linear in input length plus the bounded attribute
and name comparisons, and uses a fixed 256-entry element-name stack. Duplicate
attribute detection compares prior attribute tokens for the current element,
so the worst case is quadratic in attributes on that element. No recursion or
heap allocation occurs. On any failure `token_count` and success counts are
zero and `diagnostic` contains a stable category and an offset in `[0,input.len]`.
Output exhaustion is an ordinary diagnostic.

Defaults are 16 MiB input, 256 nested elements, 1,000,000 nodes,
4,096 attributes per element, 1,000,000 attributes total, and 4 MiB per
character-data or CDATA run. Names are additionally capped at 4,096 UTF-8
bytes as a fixed implementation bound. The caller can lower each configurable
cap but cannot raise it; depth cannot exceed the fixed 256-entry stack. A node is an
element, comment, PI, in-element text run, or CDATA run. XML declaration is
not a node. Whitespace outside the document element is tokenized but does not
count as a node.

`svg_xml_tokenize_large` uses the same grammar, token layout, and non-input
limits while permitting a source up to 1 GiB. It still writes into caller-owned
token storage and reports output exhaustion normally. The bounded index module
uses it on a validation worker; this does not change the 16 MiB cap or
allocation-free behavior of `svg_xml_tokenize`.

`svg_xml_token_stream` is the resumable full-token scanner for sources up to
1 GiB. It borrows immutable source bytes, checks XML characters incrementally,
and emits one-shot-equivalent token records across bounded calls. Its owned
token vector grows only within the caller's byte budget; `take` transfers the
completed vector, while `destroy` frees any vector not transferred. Cancellation
is checked during character scanning and parsing. A failed stream exposes its
stable diagnostic but cannot transfer a partial token vector. The index uses
this API on its worker so `token_at` data is produced by resumable scanning,
independently of the owner's progressive element reader.
Long text, comment, CDATA, and processing-instruction runs retain their scan
cursor across steps; the C regression test compares these token spans against
the one-shot tokenizer while checking the byte budget on every step.
`svg_xml_token_stream_bytes_read` reports the grammar-parser frontier (after
character validation) so other threads can safely determine whether an early
source span has been validated. If validation fails, the error offset bounds
which earlier spans remain valid.

`svg_xml_reader` is a resumable source scanner for large documents. It borrows
the immutable source, retains lexical state across calls, and advances no more
than the requested budget (capped at 64 KiB). It emits complete element tag
spans as they become available; comments, CDATA, and processing instructions
are skipped while preserving their boundary state. The reader remains separate
from the full-token stream so the owner can index element spans without waiting
for grammar validation. Consumers must not treat early tag events as proof
that the whole input is valid. Name bytes above ASCII are passed through for
the token stream to check against XML 1.0 Name rules.

Accepted syntax includes XML 1.0 documents encoded as UTF-8 with an optional
leading BOM, XML declaration,
PI, comments, element and empty-element tags, quoted attributes, character
data, and CDATA. Predefined and numeric references are retained lexically;
custom entity references are rejected because DTDs are unsupported. A single
`<!DOCTYPE>` before the root element, with or without an internal subset, is
skipped unprocessed and emitted as one `SVG_XML_TOKEN_DOCTYPE` token whose
source span covers the declaration (`xml_doctype.h` holds the shared skipper);
no entity is declared or expanded, so a reference to a non-predefined entity
still fails as malformed syntax. A DOCTYPE after the root, a second DOCTYPE and
a bare `<!ENTITY` are still rejected. No namespace URI
resolution, DTD, external entity, serializer, or tree is provided. XML 1.0
character and Name character ranges are validated; encoding declarations must
say UTF-8. Line endings and entity spellings are never normalized.
