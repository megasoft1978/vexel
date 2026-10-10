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
    DOCUMENT_EDIT_TEMPLATE_GROUP
} document_edit_template_kind;

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
 * edit, undo, redo, save-as, or destroy. Handles from an earlier snapshot are
 * rejected after any state change. */
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
document_edit_error document_edit_insert_template(document_edit *edit,
    document_handle parent, document_edit_template_kind kind,
    const document_edit_template_attribute *attributes, size_t attribute_count,
    document_handle *out_inserted);

document_edit_error document_edit_undo(document_edit *edit);
document_edit_error document_edit_redo(document_edit *edit);
document_edit_save_result document_edit_save_as(document_edit *edit,
    base_bytes path, const platform_atomic_file_hooks *hooks);

const char *document_edit_error_name(document_edit_error error);

#endif
