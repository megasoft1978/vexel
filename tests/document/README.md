# Source save tests

The C contract test uses authored XML preservation bytes and real temporary
files on the host filesystem. It verifies exact bytes, mode retention, short
writes, parse-before-write, injected write and rename failures, atomic creation
of a new destination, no-replace collision handling, and temporary-file cleanup.
`tests.test_source_save` exercises the native API executable and writes
byte/hash evidence for its independent Python-authored preservation fixtures.

`test_document_model.c` checks source-backed element spans, tree traversal,
document-scoped handles, subtree tombstones, stale/foreign rejection, malformed
XML, and byte preservation. `tests.test_document_model` records the compact
machine-readable handle matrix in the work-order evidence directory.

`../edit/test_document_edit.c` covers source spans and decoded inspector
queries; exact edits across BOM/CRLF/tab/namespace/comment/entity/CDATA input;
attribute escaping and quoting; text replacement; delete/duplicate/reorder/
wrap/ungroup/template insertion; stale handles; save and dirty state; and
undo/redo round trips. Its deterministic randomized pass executes 200,000
edit/undo/redo actions, tokenizes every resulting state, checks untouched byte
ranges, then undoes to the exact source and redoes to the captured final bytes.
