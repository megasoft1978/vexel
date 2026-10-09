#ifndef VEXEL_DOCUMENT_MODEL_H
#define VEXEL_DOCUMENT_MODEL_H

#include "base.h"
#include "xml_tokenizer.h"

typedef struct document_model document_model;

typedef struct document_handle {
    uint64_t document_id;
    uint32_t slot;
    uint32_t generation;
} document_handle;

typedef struct document_source_span {
    size_t offset;
    size_t length;
} document_source_span;

typedef enum document_model_error {
    DOCUMENT_MODEL_OK = 0,
    DOCUMENT_MODEL_INVALID_ARGUMENT,
    DOCUMENT_MODEL_PARSE,
    DOCUMENT_MODEL_LIMIT,
    DOCUMENT_MODEL_RESOURCE_EXHAUSTED,
    DOCUMENT_MODEL_OUT_OF_MEMORY,
    DOCUMENT_MODEL_ID_EXHAUSTED
} document_model_error;

typedef struct document_model_result {
    document_model_error error;
    svg_xml_diagnostic parse_diagnostic;
    document_model *model;
} document_model_result;

/* The model is single-owner mutable state; callers must quiesce readers before
 * removal or destroy. Source is borrowed and must outlive the model. Parse never
 * changes source bytes. Handles are valid only during their model lifetime. */
document_model_result document_model_parse(base_bytes source);
void document_model_destroy(document_model *model);
size_t document_model_node_count(const document_model *model);
size_t document_model_live_node_count(const document_model *model);
bool document_model_root(const document_model *model, document_handle *out);
bool document_model_first_child(const document_model *model, document_handle parent,
                                document_handle *out);
bool document_model_next_sibling(const document_model *model, document_handle node,
                                 document_handle *out);
bool document_model_parent(const document_model *model, document_handle node,
                           document_handle *out);
bool document_model_tag_name(const document_model *model, document_handle node,
                             base_bytes *out_name);
size_t document_model_attribute_count(const document_model *model, document_handle node);
bool document_model_attribute_at(const document_model *model, document_handle node,
                                 size_t index, base_bytes *out_name,
                                 base_bytes *out_value);
bool document_model_depth(const document_model *model, document_handle node,
                          size_t *out_depth);
bool document_model_node_at(const document_model *model, size_t slot,
                            document_handle *out);
bool document_model_source_span(const document_model *model, document_handle node,
                                document_source_span *out_span);
bool document_model_remove_subtree(document_model *model, document_handle node);
const char *document_model_error_name(document_model_error error);

#endif
