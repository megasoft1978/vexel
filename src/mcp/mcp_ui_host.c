#include "mcp_ui_host.h"

#include <stdlib.h>

static mcp_ui_host_status headless_get_view(void *context,
                                             mcp_ui_view_state *out_state)
{
    (void)context;
    if (out_state != NULL) *out_state = (mcp_ui_view_state){0};
    return MCP_UI_HOST_UNAVAILABLE;
}

static mcp_ui_host_status headless_set_view(void *context,
    const mcp_ui_view_request *request)
{
    (void)context;
    (void)request;
    return MCP_UI_HOST_UNAVAILABLE;
}

static mcp_ui_host_status headless_get_tool(void *context, mcp_ui_tool *out_tool)
{
    (void)context;
    if (out_tool != NULL) *out_tool = MCP_UI_TOOL_SELECT;
    return MCP_UI_HOST_UNAVAILABLE;
}

static mcp_ui_host_status headless_set_tool(void *context, mcp_ui_tool tool)
{
    (void)context;
    (void)tool;
    return MCP_UI_HOST_UNAVAILABLE;
}

static mcp_ui_host_status headless_get_ui_state(void *context,
                                                mcp_ui_state *out_state)
{
    (void)context;
    if (out_state != NULL) *out_state = (mcp_ui_state){0};
    return MCP_UI_HOST_UNAVAILABLE;
}

static mcp_ui_host_status headless_capture(void *context, mcp_ui_png *out_png)
{
    (void)context;
    if (out_png != NULL) *out_png = (mcp_ui_png){0};
    return MCP_UI_HOST_UNAVAILABLE;
}

static mcp_ui_host_status headless_command(void *context, const char *command_id)
{
    (void)context;
    (void)command_id;
    return MCP_UI_HOST_UNAVAILABLE;
}

static mcp_ui_host_status headless_timeline_get(void *context,
    mcp_ui_timeline_state *out_state)
{
    (void)context;
    if (out_state != NULL) *out_state = (mcp_ui_timeline_state){0};
    return MCP_UI_HOST_UNAVAILABLE;
}

static mcp_ui_host_status headless_timeline_set(void *context,
    const mcp_ui_timeline_update *update)
{
    (void)context;
    (void)update;
    return MCP_UI_HOST_UNAVAILABLE;
}

static const mcp_ui_host_vtable headless_vtable = {
    headless_get_view,
    headless_set_view,
    headless_get_tool,
    headless_set_tool,
    headless_get_ui_state,
    headless_capture,
    headless_command,
    headless_timeline_get,
    headless_timeline_set
};

void mcp_ui_host_init_headless(mcp_ui_host *host)
{
    if (host != NULL) *host = (mcp_ui_host){&headless_vtable, NULL};
}

void mcp_ui_png_destroy(mcp_ui_png *png)
{
    if (png == NULL) return;
    free(png->bytes);
    *png = (mcp_ui_png){0};
}

mcp_ui_host_status mcp_ui_host_get_view(const mcp_ui_host *host,
                                         mcp_ui_view_state *out_state)
{
    if (host == NULL || host->vtable == NULL || host->vtable->get_view == NULL ||
        out_state == NULL) return MCP_UI_HOST_INVALID_ARGUMENT;
    return host->vtable->get_view(host->context, out_state);
}

mcp_ui_host_status mcp_ui_host_set_view(const mcp_ui_host *host,
    const mcp_ui_view_request *request)
{
    if (host == NULL || host->vtable == NULL || host->vtable->set_view == NULL ||
        request == NULL) return MCP_UI_HOST_INVALID_ARGUMENT;
    return host->vtable->set_view(host->context, request);
}

mcp_ui_host_status mcp_ui_host_get_tool(const mcp_ui_host *host,
                                         mcp_ui_tool *out_tool)
{
    if (host == NULL || host->vtable == NULL || host->vtable->get_tool == NULL ||
        out_tool == NULL) return MCP_UI_HOST_INVALID_ARGUMENT;
    return host->vtable->get_tool(host->context, out_tool);
}

mcp_ui_host_status mcp_ui_host_set_tool(const mcp_ui_host *host, mcp_ui_tool tool)
{
    if (host == NULL || host->vtable == NULL || host->vtable->set_tool == NULL)
        return MCP_UI_HOST_INVALID_ARGUMENT;
    return host->vtable->set_tool(host->context, tool);
}

mcp_ui_host_status mcp_ui_host_get_ui_state(const mcp_ui_host *host,
                                             mcp_ui_state *out_state)
{
    if (host == NULL || host->vtable == NULL || host->vtable->get_ui_state == NULL ||
        out_state == NULL) return MCP_UI_HOST_INVALID_ARGUMENT;
    return host->vtable->get_ui_state(host->context, out_state);
}

mcp_ui_host_status mcp_ui_host_capture_window_png(const mcp_ui_host *host,
                                                    mcp_ui_png *out_png)
{
    if (host == NULL || host->vtable == NULL ||
        host->vtable->capture_window_png == NULL || out_png == NULL)
        return MCP_UI_HOST_INVALID_ARGUMENT;
    return host->vtable->capture_window_png(host->context, out_png);
}

mcp_ui_host_status mcp_ui_host_run_command(const mcp_ui_host *host,
                                            const char *command_id)
{
    if (host == NULL || host->vtable == NULL || host->vtable->run_command == NULL ||
        command_id == NULL) return MCP_UI_HOST_INVALID_ARGUMENT;
    return host->vtable->run_command(host->context, command_id);
}

mcp_ui_host_status mcp_ui_host_timeline_get(const mcp_ui_host *host,
    mcp_ui_timeline_state *out_state)
{
    if (host == NULL || host->vtable == NULL ||
        host->vtable->timeline_get == NULL || out_state == NULL)
        return MCP_UI_HOST_INVALID_ARGUMENT;
    return host->vtable->timeline_get(host->context, out_state);
}

mcp_ui_host_status mcp_ui_host_timeline_set(const mcp_ui_host *host,
    const mcp_ui_timeline_update *update)
{
    if (host == NULL || host->vtable == NULL ||
        host->vtable->timeline_set == NULL || update == NULL)
        return MCP_UI_HOST_INVALID_ARGUMENT;
    return host->vtable->timeline_set(host->context, update);
}
