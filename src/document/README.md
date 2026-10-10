# Lossless no-op source save

`source_save_noop` accepts borrowed source and destination byte slices plus
caller-owned token storage. It applies the XML tokenizer's 16 MiB and 256-depth
defaults before it invokes filesystem code. A successful parse writes the exact
source slice; there is no token serializer. Tokenizer errors never inspect or
mutate the destination.

The macOS adapter accepts non-empty, NUL-free paths no longer than 1023 bytes,
requires an existing regular destination, and creates an exclusive temporary
file in its parent directory. It handles interrupted and partial writes,
applies the original POSIX mode bits, fsyncs and closes the temporary file,
then atomically renames it over the destination. Rename is the commit point.
Failures before rename leave the old target untouched and remove the temporary
file. Directory fsync happens after commit; failure then returns
`SOURCE_SAVE_DURABILITY` with `renamed=true`, and does not imply the old target
is still present. File metadata beyond POSIX mode bits is out of scope.

The optional hooks replace only write and rename operations for deterministic
tests. Null hooks use native macOS calls. The caller keeps all slices and token
storage alive for the duration of the call and owns their storage throughout.

`document_model_parse` builds a bounded tree over immutable borrowed XML source.
Element names, attributes, and full element ranges remain source slices. The
model owns its node and attribute indexes, but never owns or changes document
bytes. Node handles use a process-unique document ID plus slot and generation;
deleting a subtree tombstones its nodes and invalidates every affected handle.
The parser uses tokenizer defaults and a 64 MiB token-storage ceiling; malformed
input and exhausted limits produce an error result without a partial model.
The tokenizer reports both token-storage-cap exhaustion and token-vector
allocation failure as `SVG_XML_ERROR_OUTPUT_EXHAUSTED`; the model preserves that
ambiguity as `DOCUMENT_MODEL_RESOURCE_EXHAUSTED`. Its own node/attribute table
allocation failures report `DOCUMENT_MODEL_OUT_OF_MEMORY`.

`document_view.h` gives UI code an allocation-free read-only facade for node
counts, child iteration, exact tag and attribute slices, and slot-range paging.
Ranges return stable rows without constructing a second tree. The mutable model
is single-owner; readers must be quiescent during subtree removal or destroy.

`document_edit.h` adds a headless source-preserving edit engine. It stores the
borrowed original source as the first piece and records inserted bytes in a
bounded piece table. Attribute changes replace only the value span, retain the
original quote, and escape XML metacharacters. Structural commands retain the
exact element bytes; text content is escaped and replaces the element's content.
Template insertion is limited to `rect`, `circle`, `ellipse`, `line`, `path`,
and `g`, with a 64 KiB serialized-template limit.

The command log defaults to 1,000 commands and 64 MiB of before/after command
bytes. Consecutive sets of the same attribute coalesce. A successful edit,
undo, redo, or save invalidates the current inspector snapshot; query again for
fresh handles. Handles from earlier snapshots fail with the stable
`stale_handle` error. The piece table does not copy the document when it applies
a splice. Inspector snapshots and affected-region validation use temporary
contiguous bytes because the current XML tokenizer accepts a contiguous slice.

Each operation validates its changed start tag, element, parent subtree, or
document before committing. The tokenizer continues to preserve UTF-8 BOMs,
line endings, whitespace, comments, CDATA, entity spellings, unknown elements,
and namespace prefixes outside the edited spans. Query helpers expose source
spans, decoded text/attribute values, element paths, ID lookup, and
source-offset-to-element mapping. `document_edit_save_as` uses the existing
atomic `source_save` adapter and therefore replaces an existing regular file;
creating a brand-new destination is outside that adapter's current contract.
