#ifndef VEXEL_MCP_BROKER_SCENE_INTERNAL_H
#define VEXEL_MCP_BROKER_SCENE_INTERNAL_H

#include "mcp_broker_internal.h"

typedef struct mcp_scene_bounds {
    double min_x;
    double min_y;
    double max_x;
    double max_y;
    bool valid;
    bool approximate;
} mcp_scene_bounds;

bool mcp_scene_string_arg(const mcp_json_doc *request, size_t args,
                          const char *name, char *out, size_t capacity,
                          size_t *out_length);
bool mcp_scene_number_arg(const mcp_json_doc *request, size_t object,
                          const char *name, double *out, bool *present);
bool mcp_scene_attribute_text(mcp_broker_document *document,
                              document_handle element, const char *name,
                              char *out, size_t capacity, size_t *out_length,
                              bool *present);
bool mcp_scene_element_bounds(mcp_broker_document *document,
                              document_handle element, mcp_scene_bounds *out);
bool mcp_scene_resolve_element_path(mcp_broker_document *document,
                                    const char *path, size_t path_length,
                                    document_handle *out);
bool mcp_scene_path_bounds(const char *path_data, size_t path_length,
                           mcp_scene_bounds *out);
bool mcp_broker_scene_query_call(mcp_broker *broker,
                                 const mcp_json_doc *request, size_t args,
                                 const char *tool_name,
                                 mcp_tool_response *out);
bool mcp_broker_scene_path_call(mcp_broker *broker,
                               const mcp_json_doc *request, size_t args,
                               const char *tool_name,
                               mcp_tool_response *out);
document_edit_error mcp_broker_scene_path_apply(
    mcp_broker_document *document, const mcp_json_doc *request, size_t args,
    const char *tool_name, document_handle *out_element);
document_edit_error mcp_broker_scene_style_apply(
    mcp_broker_document *document, const mcp_json_doc *request, size_t args,
    const char *tool_name, document_handle *out_element);
document_edit_error mcp_broker_scene_align_apply(
    mcp_broker_document *document, const mcp_json_doc *request, size_t args,
    const char *tool_name, document_handle *out_element);
document_edit_error mcp_broker_scene_duplicate(
    mcp_broker_document *document, document_handle duplicate,
    document_handle *out_duplicate);
document_edit_error mcp_broker_scene_apply_operation(
    mcp_broker_document *document, const mcp_json_doc *request, size_t args,
    const char *tool_name, document_handle *out_element, bool *out_created);

#endif
