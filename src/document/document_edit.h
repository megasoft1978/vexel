#ifndef VEXEL_DOCUMENT_EDIT_H
#define VEXEL_DOCUMENT_EDIT_H

#include "document_model.h"
#include "source_save.h"

typedef struct document_edit document_edit;

typedef struct document_edit_limits {
    size_t history_commands;
    size_t history_bytes;
} document_edit_limits;

typedef enum document_edit_error {
    DOCUMENT_EDIT_OK = 0,
    DOCUMENT_EDIT_INVALID_ARGUMENT,
    DOCUMENT_EDIT_STALE_HANDLE,
    DOCUMENT_EDIT_NOT_FOUND,
    DOCUMENT_EDIT_DUPLICATE_ATTRIBUTE,
    DOCUMENT_EDIT_INVALID_NAME,
    DOCUMENT_EDIT_MALFORMED_XML,
    DOCUMENT_EDIT_ROOT_DELETE,
    DOCUMENT_EDIT_INVALID_OPERATION,
    DOCUMENT_EDIT_LIMIT,
    DOCUMENT_EDIT_HISTORY_LIMIT,
    DOCUMENT_EDIT_OUT_OF_MEMORY,
    DOCUMENT_EDIT_SAVE_FAILED
} document_edit_error;

typedef struct document_edit_result {
    document_edit_error error;
    svg_xml_diagnostic parse_diagnostic;
    document_edit *edit;
} document_edit_result;

typedef struct document_edit_attribute {
    document_source_span name;
    document_source_span value;
    uint8_t quote;
} document_edit_attribute;

typedef enum document_edit_template_kind {
    DOCUMENT_EDIT_TEMPLATE_RECT = 0,
    DOCUMENT_EDIT_TEMPLATE_CIRCLE,
    DOCUMENT_EDIT_TEMPLATE_ELLIPSE,
    DOCUMENT_EDIT_TEMPLATE_LINE,
    DOCUMENT_EDIT_TEMPLATE_PATH,
    DOCUMENT_EDIT_TEMPLATE_GROUP,
    DOCUMENT_EDIT_TEMPLATE_TEXT
} document_edit_template_kind;

#define DOCUMENT_EDIT_XML_FRAGMENT_LIMIT (64U * 1024U)
#define DOCUMENT_EDIT_GROUP_NODE_LIMIT 128U

typedef struct document_edit_template_attribute {
    base_bytes name;
    base_bytes value;
} document_edit_template_attribute;

typedef struct document_edit_save_result {
    document_edit_error error;
    source_save_result source_result;
} document_edit_save_result;

/* Source bytes are borrowed and must outlive the edit. The default history
 * keeps at most 1,000 commands and 64 MiB of inverse/inserted command bytes. */
document_edit_limits document_edit_default_limits(void);
document_edit_result document_edit_create(base_bytes source, document_edit_limits limits);
void document_edit_destroy(document_edit *edit);

/* The model, slices, and source spans are borrowed until the next successful
 * edit, undo, redo, or destroy. Saving the unchanged snapshot preserves them.
 * Handles from an earlier snapshot are rejected after any source state change. */
const document_model *document_edit_model(document_edit *edit);
bool document_edit_root(document_edit *edit, document_handle *out);
bool document_edit_find_id(document_edit *edit, base_bytes id, document_handle *out);
bool document_edit_element_at_offset(document_edit *edit, size_t offset,
                                     document_handle *out);
bool document_edit_tag_name(document_edit *edit, document_handle node,
                            base_bytes *out_name);
bool document_edit_source_span(document_edit *edit, document_handle node,
                               document_source_span *out_span);
size_t document_edit_attribute_count(document_edit *edit, document_handle node);
bool document_edit_attribute_at(document_edit *edit, document_handle node, size_t index,
                                document_edit_attribute *out_attribute);
bool document_edit_span_bytes(document_edit *edit, document_source_span span,
                              base_bytes *out_bytes);
bool document_edit_decode_span(document_edit *edit, document_source_span span,
                               uint8_t *buffer, size_t capacity, size_t *out_length);
bool document_edit_text_content(document_edit *edit, document_handle node,
                                uint8_t *buffer, size_t capacity, size_t *out_length);
bool document_edit_element_path(document_edit *edit, document_handle node,
                                uint8_t *buffer, size_t capacity, size_t *out_length);

size_t document_edit_byte_length(const document_edit *edit);
bool document_edit_materialize(const document_edit *edit, uint8_t *buffer,
                               size_t capacity, size_t *out_length);
bool document_edit_is_dirty(const document_edit *edit);
size_t document_edit_history_count(const document_edit *edit);
size_t document_edit_history_cursor(const document_edit *edit);

document_edit_error document_edit_set_attribute(document_edit *edit,
    document_handle node, base_bytes name, base_bytes value);
document_edit_error document_edit_add_attribute(document_edit *edit,
    document_handle node, base_bytes name, base_bytes value);
document_edit_error document_edit_remove_attribute(document_edit *edit,
    document_handle node, base_bytes name);
document_edit_error document_edit_set_text_content(document_edit *edit,
    document_handle node, base_bytes text);
document_edit_error document_edit_delete_element(document_edit *edit,
    document_handle node);
document_edit_error document_edit_duplicate_element(document_edit *edit,
    document_handle node, document_handle *out_duplicate);
document_edit_error document_edit_reorder_sibling(document_edit *edit,
    document_handle node, document_handle target, bool before);
document_edit_error document_edit_wrap_in_group(document_edit *edit,
    document_handle node, document_handle *out_group);
document_edit_error document_edit_ungroup(document_edit *edit,
    document_handle group);
/* Wrap or unwrap a bounded, consecutive sibling sequence in one source edit.
 * The APIs preserve every source byte outside inserted/removed <g> wrappers. */
document_edit_error document_edit_group_siblings(document_edit *edit,
    const document_handle *nodes, size_t node_count,
    document_handle *out_group);
document_edit_error document_edit_ungroup_siblings(document_edit *edit,
    const document_handle *groups, size_t group_count);
document_edit_error document_edit_insert_template(document_edit *edit,
    document_handle parent, document_edit_template_kind kind,
    const document_edit_template_attribute *attributes, size_t attribute_count,
    document_handle *out_inserted);
/* Insert one bounded XML fragment containing exactly one top-level element.
 * The fragment is borrowed for the call and must be well-formed in the
 * destination parent's context. The returned handle identifies that element. */
document_edit_error document_edit_insert_xml_fragment(document_edit *edit,
    document_handle parent, base_bytes fragment, document_handle *out_inserted);
/* Insert one text element with escaped attributes and text in a single undo step. */
document_edit_error document_edit_insert_text_element(document_edit *edit,
    document_handle parent, const document_edit_template_attribute *attributes,
    size_t attribute_count, base_bytes text, document_handle *out_inserted);
/* Replace a bounded contiguous sequence of child bytes under parent with one
 * validated XML fragment. Bytes outside region remain unchanged, and the
 * replacement is recorded as one undoable command. */
document_edit_error document_edit_replace_xml_region(document_edit *edit,
    document_handle parent, document_source_span region, base_bytes replacement);
/* Replace one small raw source span, validating the complete resulting XML.
 * Used for byte-local edits such as changing retained path number tokens. */
document_edit_error document_edit_replace_source_span(document_edit *edit,
    document_source_span span, base_bytes replacement);

document_edit_error document_edit_undo(document_edit *edit);
document_edit_error document_edit_redo(document_edit *edit);
document_edit_save_result document_edit_save_as(document_edit *edit,
    base_bytes path, const platform_atomic_file_hooks *hooks);

const char *document_edit_error_name(document_edit_error error);

#endif
