#ifndef VEXEL_APP_MCP_BROKER_HOOK_H
#define VEXEL_APP_MCP_BROKER_HOOK_H

typedef struct viewer_session_collection viewer_session_collection;

/* The viewer calls poll on its owner thread; requests are serialized there. */
void mcp_broker_hook_poll(viewer_session_collection *sessions);
/* A non-NULL call is a per-window teardown hint; NULL shuts down the runtime hook. */
void mcp_broker_hook_shutdown(void *runtime_or_window);

#endif
