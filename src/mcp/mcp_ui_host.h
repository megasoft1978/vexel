#ifndef VEXEL_MCP_UI_HOST_H
#define VEXEL_MCP_UI_HOST_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define MCP_UI_CAPTURE_MAX_AXIS 4096U
#define MCP_UI_CAPTURE_MAX_PIXELS (16U * 1000U * 1000U)

typedef enum mcp_ui_host_status {
    MCP_UI_HOST_OK,
    MCP_UI_HOST_UNAVAILABLE,
    MCP_UI_HOST_INVALID_ARGUMENT,
    MCP_UI_HOST_DISABLED,
    MCP_UI_HOST_BUSY,
    MCP_UI_HOST_LIMIT,
    MCP_UI_HOST_FAILED
} mcp_ui_host_status;

typedef enum mcp_ui_view_action {
    MCP_UI_VIEW_SET,
    MCP_UI_VIEW_FIT,
    MCP_UI_VIEW_FIT_WIDTH,
    MCP_UI_VIEW_ACTUAL_SIZE
} mcp_ui_view_action;

typedef struct mcp_ui_view_state {
    double zoom;
    double pan_x;
    double pan_y;
    double viewport_width;
    double viewport_height;
    uint32_t document_width;
    uint32_t document_height;
} mcp_ui_view_state;

typedef struct mcp_ui_view_request {
    mcp_ui_view_action action;
    double zoom;
    double pan_x;
    double pan_y;
} mcp_ui_view_request;

typedef enum mcp_ui_tool {
    MCP_UI_TOOL_SELECT,
    MCP_UI_TOOL_PAN,
    MCP_UI_TOOL_ZOOM,
    MCP_UI_TOOL_RECTANGLE,
    MCP_UI_TOOL_ELLIPSE,
    MCP_UI_TOOL_LINE,
    MCP_UI_TOOL_POLYGON,
    MCP_UI_TOOL_POLYLINE,
    MCP_UI_TOOL_PEN,
    MCP_UI_TOOL_NODE,
    MCP_UI_TOOL_TEXT
} mcp_ui_tool;

typedef struct mcp_ui_state {
    bool live_control_enabled;
    bool agent_connected;
    bool dirty;
    bool can_undo;
    bool can_redo;
    bool has_selection;
    bool structure_visible;
    bool inspector_visible;
    size_t selected_count;
    char selected_id[128];
    char status[192];
} mcp_ui_state;

typedef struct mcp_ui_png {
    uint8_t *bytes;
    size_t length;
    uint32_t width;
    uint32_t height;
} mcp_ui_png;

typedef struct mcp_ui_timeline_state {
    double time_seconds;
    double duration_seconds;
    size_t track_count;
    size_t selected_track;
    bool playing;
    bool loop;
} mcp_ui_timeline_state;

typedef struct mcp_ui_timeline_update {
    bool set_time;
    double time_seconds;
    bool set_playing;
    bool playing;
    bool set_loop;
    bool loop;
    bool set_selected_track;
    size_t selected_track;
} mcp_ui_timeline_update;

typedef struct mcp_ui_host_vtable {
    mcp_ui_host_status (*get_view)(void *context, mcp_ui_view_state *out_state);
    mcp_ui_host_status (*set_view)(void *context,
                                   const mcp_ui_view_request *request);
    mcp_ui_host_status (*get_tool)(void *context, mcp_ui_tool *out_tool);
    mcp_ui_host_status (*set_tool)(void *context, mcp_ui_tool tool);
    mcp_ui_host_status (*get_ui_state)(void *context, mcp_ui_state *out_state);
    mcp_ui_host_status (*capture_window_png)(void *context, mcp_ui_png *out_png);
    mcp_ui_host_status (*run_command)(void *context, const char *command_id);
    mcp_ui_host_status (*timeline_get)(void *context,
                                      mcp_ui_timeline_state *out_state);
    mcp_ui_host_status (*timeline_set)(void *context,
                                      const mcp_ui_timeline_update *update);
} mcp_ui_host_vtable;

typedef struct mcp_ui_host {
    const mcp_ui_host_vtable *vtable;
    void *context;
} mcp_ui_host;

void mcp_ui_host_init_headless(mcp_ui_host *host);
void mcp_ui_png_destroy(mcp_ui_png *png);
mcp_ui_host_status mcp_ui_host_get_view(const mcp_ui_host *host,
                                        mcp_ui_view_state *out_state);
mcp_ui_host_status mcp_ui_host_set_view(const mcp_ui_host *host,
                                        const mcp_ui_view_request *request);
mcp_ui_host_status mcp_ui_host_get_tool(const mcp_ui_host *host,
                                        mcp_ui_tool *out_tool);
mcp_ui_host_status mcp_ui_host_set_tool(const mcp_ui_host *host,
                                        mcp_ui_tool tool);
mcp_ui_host_status mcp_ui_host_get_ui_state(const mcp_ui_host *host,
                                            mcp_ui_state *out_state);
mcp_ui_host_status mcp_ui_host_capture_window_png(const mcp_ui_host *host,
                                                   mcp_ui_png *out_png);
mcp_ui_host_status mcp_ui_host_run_command(const mcp_ui_host *host,
                                           const char *command_id);
mcp_ui_host_status mcp_ui_host_timeline_get(const mcp_ui_host *host,
                                           mcp_ui_timeline_state *out_state);
mcp_ui_host_status mcp_ui_host_timeline_set(const mcp_ui_host *host,
                                           const mcp_ui_timeline_update *update);

#endif
