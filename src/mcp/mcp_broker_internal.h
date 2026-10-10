#ifndef VEXEL_MCP_BROKER_INTERNAL_H
#define VEXEL_MCP_BROKER_INTERNAL_H

#include "mcp_broker.h"
#include "mcp_json.h"

#include <stdbool.h>
#include <stdint.h>

typedef struct mcp_broker_document {
    bool used;
    bool app_owned;
    uint32_t serial;
    char id[32];
    char path[MCP_BROKER_PATH_CAP];
    document_edit *edit;
    uint8_t *owned_source;
    size_t owned_source_length;
    document_handle selection;
    bool selection_valid;
    char *last_render_base64;
    size_t last_render_length;
    uint32_t last_render_width;
    uint32_t last_render_height;
} mcp_broker_document;

struct mcp_broker {
    mcp_broker_host host;
    mcp_broker_document documents[MCP_BROKER_MAX_DOCUMENTS];
    mcp_broker_document *app_document;
    uint32_t next_serial;
};

bool mcp_access_path_allowed(const char *path);
bool mcp_access_resolve_existing(const char *path, char out[MCP_BROKER_PATH_CAP]);
bool mcp_broker_call_tool(void *context, const mcp_json_doc *request,
                          size_t arguments, const char *tool_name,
                          mcp_tool_response *out);
bool mcp_broker_read_resource(void *context, const mcp_json_doc *request,
                              const char *uri, mcp_json_buffer *out_contents);
mcp_broker_document *mcp_broker_find(mcp_broker *broker, const mcp_json_doc *doc,
                                     size_t args, const char *key);
mcp_broker_document *mcp_broker_current(mcp_broker *broker);
bool mcp_broker_set_result(mcp_tool_response *out, const char *json);
bool mcp_broker_set_error(mcp_tool_response *out, const char *code, const char *message);
bool mcp_broker_string_arg(const mcp_json_doc *doc, size_t args, const char *name,
                           char *out, size_t capacity, size_t *out_length);
bool mcp_broker_u64_arg(const mcp_json_doc *doc, size_t args, const char *name,
                        uint64_t *out, bool *present);
bool mcp_broker_bool_arg(const mcp_json_doc *doc, size_t args, const char *name,
                         bool *out);
bool mcp_broker_resolve_element(mcp_broker_document *document, const char *id,
                                size_t id_length, document_handle *out);
bool mcp_broker_element_id(mcp_broker_document *document, document_handle handle,
                           mcp_json_buffer *out);
bool mcp_broker_tree_json(mcp_broker_document *document, size_t cursor, size_t limit,
                          mcp_json_buffer *out);
void mcp_broker_after_edit(mcp_broker *broker, mcp_broker_document *document,
                           document_handle selection);
bool mcp_broker_render_png(mcp_broker *broker, mcp_broker_document *document,
                           const mcp_json_doc *request, size_t args,
                           mcp_tool_response *out);
bool mcp_broker_documents_call(mcp_broker *broker, const mcp_json_doc *request,
                               size_t args, const char *tool_name,
                               mcp_tool_response *out);
bool mcp_broker_edit_call(mcp_broker *broker, const mcp_json_doc *request,
                          size_t args, const char *tool_name,
                          mcp_tool_response *out);

#endif
