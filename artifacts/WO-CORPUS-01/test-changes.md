# WO-CORPUS-01 test assertion changes

Existing assertions that pinned the old fail-closed behaviour were changed, not deleted.

## tests/svg/xml/test_xml_tokenizer.c

- Old: `expect_error("<!DOCTYPE a><a/>",SVG_XML_ERROR_UNSUPPORTED_DTD);`
- New: `check_doctype()`. It asserts that five DOCTYPE forms (bare, public/system identifiers, internal subset with an entity, comments/PIs/quoted `>` and `]` inside the subset, after an XML declaration) tokenize with no error, emit one `SVG_XML_TOKEN_DOCTYPE` token whose source span is exactly the declaration, and count one element. It also asserts that a declared entity referenced in text or in an attribute value still fails with `SVG_XML_ERROR_MALFORMED_SYNTAX`; that a DOCTYPE after the root, inside the root, duplicated, without trailing whitespace, with an unterminated subset or unterminated declaration fails with `SVG_XML_ERROR_MALFORMED_SYNTAX`; and that a top-level `<!ENTITY ...>` still fails with `SVG_XML_ERROR_UNSUPPORTED_DTD`.

## tests/svg/index/test_index.c

- `check_diagnostic_equivalence` compares the incremental index diagnostic with the one-shot tokenizer on malformed inputs.
- Old list entry: `"<!DOCTYPE svg><svg/>"` (malformed only while DOCTYPE was rejected).
- New list entry: `"<!ENTITY e \"x\"><svg/>"` (still `SVG_XML_ERROR_UNSUPPORTED_DTD` at the same opening byte in both paths).
- Added `check_doctype_indexed()`: a document with a DOCTYPE and internal subset indexes to completion with 2 elements.

## Added (no pre-existing assertion changed)

- `tests/svg/index/test_token_stream.c`: `check_doctype_stream()` checks the resumable stream equals the one-shot tokens at byte budgets 3..9 for a DOCTYPE with subset.
- `tests/svg/shape/test_ignored.c` (CTest `shape-ignored-constructs`): per construct decode succeeds, render equals the document without the construct, summary count and first offset; entity reference after a DOCTYPE still fails as XML; a known unsupported rendering attribute (`text-decoration`) still fails closed.
- `tests/svg/shape/test_render_frame_ignored.py` (CTest `render-frame-ignored-note`): the `render-frame` note lines.

Nothing in `tests/oracle/**`, `tests/golden/**` or any golden fixture was changed.
