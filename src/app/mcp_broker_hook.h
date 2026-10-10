#ifndef VEXEL_APP_MCP_BROKER_HOOK_H
#define VEXEL_APP_MCP_BROKER_HOOK_H

typedef struct viewer_app viewer_app;

/* The viewer calls poll on its owner thread; MCP requests never touch editor state off-thread. */
void mcp_broker_hook_poll(viewer_app *app);
void mcp_broker_hook_shutdown(viewer_app *app);

#endif
