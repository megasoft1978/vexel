# Source-preserving edit tests

`test_document_edit.c` is a golden-free native contract test for the headless
edit engine. It checks byte-exact unchanged regions and exercises the
inspector-facing source spans, decoded values, element path, ID lookup, and
offset mapping. It covers text templates, text insertion/removal on self-closing
elements, bounded single-root XML fragment insertion, malformed and multi-root
rejection, and save-as to a brand-new path. The seeded 200,000-action history
test validates every current document, confirms untouched prefix/suffix bytes,
and round-trips undo and redo. A second seeded 200,000-case campaign exercises
successful, malformed, and multi-root fragment inputs.
The auto-ID regression checks unique generated IDs for template and fragment
insertion, preservation/rejection of supplied IDs, and exact undo/redo bytes.

The editor also uses `document_edit_replace_xml_region` for atomic multi-sibling
edits, `document_edit_replace_source_span` for path number changes, and
`document_edit_insert_text_element` for escaped one-command text creation.

Build and run with:

```sh
cmake --build build/mac-debug --target document_edit_test
./build/mac-debug/bin/document_edit_test
```
