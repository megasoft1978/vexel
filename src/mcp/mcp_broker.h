#ifndef VEXEL_MCP_BROKER_H
#define VEXEL_MCP_BROKER_H

#include "mcp_protocol.h"
#include "document_edit.h"
#include "project/project_bundle.h"
#include "project/project_timeline_session.h"

#define MCP_BROKER_MAX_DOCUMENTS 8U
#define MCP_BROKER_PATH_CAP 1024U

typedef struct mcp_broker mcp_broker;

typedef struct mcp_broker_host {
    void *context;
    document_edit *(*current_edit)(void *context);
    const char *(*current_path)(void *context);
    bool (*current_selection)(void *context, document_handle *out);
    bool (*replace_current_edit)(void *context, document_edit *expected,
                                 document_edit *replacement,
                                 document_handle selection);
    bool (*destroy_current_edit)(void *context, document_edit *edit);
    bool (*set_current_path)(void *context, const char *path);
    void (*agent_edited)(void *context, const char *path);
    void (*selection_changed)(void *context, document_handle selection);
    project_timeline_session *(*current_timeline_session)(void *context);
    const project_bundle_view *(*current_project_bundle)(void *context);
    void (*native_timeline_changed)(void *context);
    project_bundle_result (*add_native_drawing_asset)(void *context,
        base_bytes name, base_bytes svg);
    bool (*save_current_project)(void *context, const char *path,
                                 size_t path_length, bool save_as);
    void (*agent_action_completed)(void *context, const char *tool_name,
                                   bool succeeded);
} mcp_broker_host;

bool mcp_broker_create(const mcp_broker_host *host, mcp_broker **out);
void mcp_broker_destroy(mcp_broker *broker);
void mcp_broker_bind_current(mcp_broker *broker);
mcp_protocol_backend mcp_broker_protocol_backend(mcp_broker *broker);

#endif
