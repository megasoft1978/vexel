#ifndef VEXEL_MCP_BROKER_H
#define VEXEL_MCP_BROKER_H

#include "mcp_protocol.h"
#include "mcp_ui_host.h"
#include "document_edit.h"

#define MCP_BROKER_MAX_DOCUMENTS 24U
#define MCP_BROKER_MAX_LIVE_DOCUMENTS 8U
#define MCP_BROKER_MAX_HEADLESS_DOCUMENTS 16U
#define MCP_BROKER_PATH_CAP 1024U

typedef struct mcp_broker mcp_broker;

typedef enum mcp_broker_mutation_kind {
    MCP_BROKER_MUTATION_EDIT,
    MCP_BROKER_MUTATION_TRANSACTION,
    MCP_BROKER_MUTATION_UNDO,
    MCP_BROKER_MUTATION_REDO,
    MCP_BROKER_MUTATION_SAVE,
    MCP_BROKER_MUTATION_ANIMATION
} mcp_broker_mutation_kind;

typedef struct mcp_broker_live_document {
    void *identity;
    document_edit *edit;
    const char *path;
    uint32_t window_number;
    document_handle selection;
    bool selection_valid;
    bool active;
    mcp_ui_host ui_host;
} mcp_broker_live_document;

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
    size_t (*live_document_count)(void *context);
    bool (*live_document_at)(void *context, size_t index,
                             mcp_broker_live_document *out_document);
    bool (*live_control_enabled)(void *context);
    bool (*current_selection_for_document)(void *context, void *identity,
                                            document_handle *out_selection);
    void (*selection_changed_for_document)(void *context, void *identity,
                                            document_handle selection);
    void (*commit_mutation)(void *context, void *identity,
                            mcp_broker_mutation_kind kind,
                            document_handle selection, bool selection_valid);
    bool (*set_document_path)(void *context, void *identity, const char *path);
    bool (*owner_is_busy)(void *context);
} mcp_broker_host;

bool mcp_broker_create(const mcp_broker_host *host, mcp_broker **out);
void mcp_broker_destroy(mcp_broker *broker);
void mcp_broker_bind_current(mcp_broker *broker);
void mcp_broker_bind_live_documents(mcp_broker *broker);
mcp_protocol_backend mcp_broker_protocol_backend(mcp_broker *broker);

#endif
