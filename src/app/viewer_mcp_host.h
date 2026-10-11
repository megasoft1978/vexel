#ifndef VEXEL_APP_VIEWER_MCP_HOST_H
#define VEXEL_APP_VIEWER_MCP_HOST_H

#include "mcp/mcp_broker.h"
#include "viewer_sessions.h"

#include <stdbool.h>
#include <stddef.h>

typedef struct viewer_app viewer_app;
struct mcp_broker;

void viewer_mcp_ui_host_init(viewer_app *app, mcp_ui_host *out_host);
void viewer_mcp_set_broker(viewer_session_collection *sessions,
                           struct mcp_broker *broker);
void viewer_mcp_gui_mutation(viewer_app *app,
                             mcp_broker_mutation_kind kind);
size_t viewer_mcp_live_document_count(void *context);
bool viewer_mcp_live_document_at(void *context, size_t index,
                                 mcp_broker_live_document *out_document);
bool viewer_mcp_live_control_enabled(void *context);
bool viewer_mcp_current_selection(void *context, void *identity,
                                   document_handle *out_selection);
void viewer_mcp_selection_changed(void *context, void *identity,
                                  document_handle selection);
void viewer_mcp_commit_mutation(void *context, void *identity,
                                mcp_broker_mutation_kind kind,
                                document_handle selection,
                                bool selection_valid);
bool viewer_mcp_set_document_path(void *context, void *identity,
                                  const char *path);
bool viewer_mcp_owner_is_busy(void *context);
bool viewer_mcp_sessions_are_busy(void *context);
void viewer_mcp_connection_changed(viewer_session_collection *sessions,
                                   bool connected);
bool viewer_mcp_connection_needs_sync(const viewer_session_collection *sessions,
                                      bool connected);

#endif
