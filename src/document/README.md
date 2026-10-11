# Lossless no-op source save

`source_save_noop` accepts borrowed source and destination byte slices plus
caller-owned token storage. It applies the XML tokenizer's 16 MiB and 256-depth
defaults before it invokes filesystem code. A successful parse writes the exact
source slice; there is no token serializer. Tokenizer errors never inspect or
mutate the destination.

The macOS adapter accepts non-empty, NUL-free paths no longer than 1023 bytes.
For an existing regular destination it writes an exclusive temporary sibling,
preserves the original POSIX mode bits, and atomically renames over the target.
For a new destination it writes and fsyncs a temporary sibling, then publishes
it with an atomic no-replace link; new files use mode `0600`. A concurrently
created destination is left untouched. Failures before publication preserve the
old target or leave a new target absent and remove the temporary file. Parent
directory fsync follows commit; a post-commit error reports durability failure.
Metadata beyond POSIX mode bits is out of scope.

Replacement hooks cover write and rename operations for deterministic tests;
new-file hooks cover writes and no-replace publication. Null hooks use native
macOS calls. The caller keeps all slices and token storage alive for the
duration of the call and owns their storage throughout.

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
Template insertion supports `rect`, `circle`, `ellipse`, `line`, `path`, `g`,
and empty `text` elements, with a 64 KiB serialized-template limit. Text content
can be inserted into self-closing elements and cleared again through the same
undoable operation. `document_edit_insert_xml_fragment` accepts one top-level
element in a well-formed fragment up to 64 KiB; it validates the containing
parent before committing and returns a fresh handle for the inserted element.

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
source-offset-to-element mapping. `document_edit_save_as` saves to either an
existing regular file or a new destination using the atomic macOS adapters.

The edit engine also provides `document_edit_replace_xml_region` for replacing
a bounded child region with one validated fragment in one command, and
`document_edit_replace_source_span` for byte-local edits that validate the
complete resulting XML. The latter keeps path node edits limited to their
retained numeric tokens. `document_edit_insert_text_element` escapes attribute
values and text, validates the resulting element in its parent context, and
inserts it as one undoable command.
