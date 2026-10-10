#ifndef VEXEL_MCP_BROKER_H
#define VEXEL_MCP_BROKER_H

#include "mcp_protocol.h"
#include "document_edit.h"

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
} mcp_broker_host;

bool mcp_broker_create(const mcp_broker_host *host, mcp_broker **out);
void mcp_broker_destroy(mcp_broker *broker);
void mcp_broker_bind_current(mcp_broker *broker);
mcp_protocol_backend mcp_broker_protocol_backend(mcp_broker *broker);

#endif
