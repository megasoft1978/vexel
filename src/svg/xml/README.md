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

Accepted syntax includes XML 1.0 documents encoded as UTF-8 with an optional
leading BOM, XML declaration,
PI, comments, element and empty-element tags, quoted attributes, character
data, and CDATA. Predefined and numeric references are retained lexically;
custom entity references are rejected because DTDs are unsupported. Every
DOCTYPE/ENTITY declaration is rejected at its opening byte. No namespace URI
resolution, DTD, external entity, serializer, or tree is provided. XML 1.0
character and Name character ranges are validated; encoding declarations must
say UTF-8. Line endings and entity spellings are never normalized.
