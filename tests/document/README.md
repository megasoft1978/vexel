# Source save tests

The C contract test uses an authored XML preservation fixture and real temporary
files on the host filesystem. It verifies exact bytes, mode retention, short
writes, parse-before-write, injected write and rename failures, and cleanup.
`tests.test_source_save` exercises the native API executable and writes
byte/hash evidence for its independent Python-authored preservation fixtures.

`test_document_model.c` checks source-backed element spans, tree traversal,
document-scoped handles, subtree tombstones, stale/foreign rejection, malformed
XML, and byte preservation. `tests.test_document_model` records the compact
machine-readable handle matrix in the work-order evidence directory.
