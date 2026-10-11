#ifndef VEXEL_DOCUMENT_EDIT_INTERNAL_H
#define VEXEL_DOCUMENT_EDIT_INTERNAL_H

#include "document_edit.h"

typedef struct document_edit_blob {
    uint8_t *bytes;
    size_t length;
    size_t references;
} document_edit_blob;

typedef struct document_edit_piece {
    size_t offset;
    size_t length;
    document_edit_blob *blob;
} document_edit_piece;

typedef struct document_edit_command {
    size_t offset;
    document_edit_blob *before;
    document_edit_blob *after;
    uint32_t target_slot;
    bool coalescible_attribute;
    uint64_t before_state;
    uint64_t after_state;
    char label[DOCUMENT_EDIT_HISTORY_LABEL_CAP];
} document_edit_command;

typedef enum document_edit_history_action {
    DOCUMENT_EDIT_HISTORY_ACTION_EDIT_ATTRIBUTE,
    DOCUMENT_EDIT_HISTORY_ACTION_ADD_ATTRIBUTE,
    DOCUMENT_EDIT_HISTORY_ACTION_REMOVE_ATTRIBUTE,
    DOCUMENT_EDIT_HISTORY_ACTION_EDIT_TEXT,
    DOCUMENT_EDIT_HISTORY_ACTION_DELETE,
    DOCUMENT_EDIT_HISTORY_ACTION_DUPLICATE,
    DOCUMENT_EDIT_HISTORY_ACTION_MOVE,
    DOCUMENT_EDIT_HISTORY_ACTION_GROUP,
    DOCUMENT_EDIT_HISTORY_ACTION_UNGROUP,
    DOCUMENT_EDIT_HISTORY_ACTION_ADD_RECTANGLE,
    DOCUMENT_EDIT_HISTORY_ACTION_ADD_CIRCLE,
    DOCUMENT_EDIT_HISTORY_ACTION_ADD_ELLIPSE,
    DOCUMENT_EDIT_HISTORY_ACTION_ADD_LINE,
    DOCUMENT_EDIT_HISTORY_ACTION_ADD_PATH,
    DOCUMENT_EDIT_HISTORY_ACTION_ADD_GROUP,
    DOCUMENT_EDIT_HISTORY_ACTION_ADD_TEXT,
    DOCUMENT_EDIT_HISTORY_ACTION_ADD_SHAPE,
    DOCUMENT_EDIT_HISTORY_ACTION_EDIT_OBJECTS,
    DOCUMENT_EDIT_HISTORY_ACTION_EDIT_PATH,
    DOCUMENT_EDIT_HISTORY_ACTION_RESIZE,
    DOCUMENT_EDIT_HISTORY_ACTION_ROTATE
} document_edit_history_action;

typedef struct document_edit_node_info {
    document_source_span span;
    svg_xml_span name;
    svg_xml_span open;
    svg_xml_span close;
    size_t first_attribute;
    size_t attribute_count;
    bool empty;
} document_edit_node_info;

typedef struct document_edit_attribute_info {
    svg_xml_span name;
    svg_xml_span value;
    svg_xml_span source;
    uint8_t quote;
} document_edit_attribute_info;

typedef struct document_edit_text_info {
    uint32_t owner;
    svg_xml_span value;
    bool cdata;
} document_edit_text_info;

struct document_edit {
    base_bytes original;
    document_edit_limits limits;
    document_edit_error last_error;
    svg_xml_diagnostic last_diagnostic;
    document_edit_piece *pieces;
    size_t piece_count;
    size_t piece_capacity;
    size_t byte_length;
    size_t blob_bytes;
    document_edit_command *commands;
    size_t command_count;
    size_t command_cursor;
    size_t command_capacity;
    uint64_t state_id;
    uint64_t saved_state_id;
    uint64_t next_state_id;
    uint8_t *snapshot;
    size_t snapshot_length;
    document_model *model;
    svg_xml_token *tokens;
    size_t token_count;
    document_edit_node_info *nodes;
    size_t node_count;
    document_edit_attribute_info *attributes;
    size_t attribute_count;
    document_edit_text_info *texts;
    size_t text_count;
};

bool document_edit_read_range(const document_edit *edit, size_t offset,
                              size_t length, uint8_t *output);
bool document_edit_replace_raw(document_edit *edit, size_t offset, size_t remove_length,
                               document_edit_blob *insert);
void document_edit_piece_table_destroy(document_edit *edit);
document_edit_blob *document_edit_blob_create(document_edit *edit,
    const uint8_t *bytes, size_t length, document_edit_error *out_error);
void document_edit_blob_retain(document_edit_blob *blob);
void document_edit_blob_release(document_edit *edit, document_edit_blob *blob);
void document_edit_invalidate_index(document_edit *edit);
bool document_edit_ensure_index(document_edit *edit);
bool document_edit_handle_info(document_edit *edit, document_handle handle,
                               document_edit_node_info **out_info);
document_edit_error document_edit_commit_replace(document_edit *edit,
    size_t offset, size_t remove_length, const uint8_t *insert, size_t insert_length,
    uint32_t target_slot, bool coalescible_attribute,
    document_edit_history_action action);
base_bytes document_edit_history_action_label(document_edit_history_action action);
document_edit_history_action document_edit_history_template_action(
    document_edit_template_kind kind);
void document_edit_history_relabel_latest(document_edit *edit,
    document_edit_history_action action);
document_edit_error document_edit_replace_xml_region_labeled(document_edit *edit,
    document_handle parent_handle, document_source_span region,
    base_bytes replacement, document_edit_history_action action);
bool document_edit_validate_replace(document_edit *edit,
    size_t region_offset, size_t region_length, size_t patch_offset,
    size_t remove_length, const uint8_t *insert, size_t insert_length,
    bool fragment, bool start_tag);
bool document_edit_decode_bytes(base_bytes raw, uint8_t *buffer,
                                size_t capacity, size_t *out_length);
bool document_edit_encode_attribute(base_bytes input, uint8_t quote,
                                    uint8_t **out_bytes, size_t *out_length);
bool document_edit_encode_text(base_bytes input, uint8_t **out_bytes,
                               size_t *out_length);
bool document_edit_name_valid(base_bytes name);
bool document_edit_span_equals(document_edit *edit, svg_xml_span span,
                               base_bytes bytes);

#endif
