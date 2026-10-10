# Source-preserving edit tests

`test_document_edit.c` is a golden-free native contract test for the headless
edit engine. It checks byte-exact unchanged regions and exercises the
inspector-facing source spans, decoded values, element path, ID lookup, and
offset mapping. The seeded 200,000-action history test validates every current
document, confirms untouched prefix/suffix bytes, and round-trips undo and redo.

Build and run with:

```sh
cmake --build build/mac-debug --target document_edit_test
./build/mac-debug/bin/document_edit_test
```
