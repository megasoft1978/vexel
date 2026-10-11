# Editcore lane

The edit layer in `src/document/document_edit.*` uses the existing source-backed
document model and XML tokenizer. A piece table keeps the original source
borrowed and applies splices without copying the whole document into its edit
representation. History defaults to 1,000 commands and 64 MiB of stored
before/after bytes; same-attribute consecutive sets coalesce. Stale handles are
rejected after each successful state change.

Implemented commands cover set/add/remove attribute, escaped text-content
replacement, subtree deletion and duplication, sibling reorder, wrapping and
ungrouping with `<g>`, and bounded `rect`, `circle`, `ellipse`, `line`, `path`,
and `g` templates. Inspector queries expose attribute name/value spans and
decoded values, tag, parent/child traversal through `document_model`, text,
element paths, ID lookup, element source spans, and source-offset mapping.
Every edit validates its changed tag, element, parent subtree, or document
before changing the piece table. Save calls the existing atomic source-save
path and tracks the saved history state.

Verification is in `tests/edit/test_document_edit.c`: byte-preserving fixtures,
query and structural-operation cases, save/dirty behavior, and a deterministic
200,000-action edit/undo/redo property pass. The XML tokenizer is still
contiguous-input-only, so inspector snapshots and validation regions use
temporary contiguous buffers; edit storage itself remains span based. The
existing atomic-save adapter replaces an existing regular file, so a new
filesystem destination must already exist. This lane does not change the
platform adapter to add create-new atomic-save semantics.
