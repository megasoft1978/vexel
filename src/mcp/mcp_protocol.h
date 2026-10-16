#ifndef VEXEL_MCP_PROTOCOL_H
#define VEXEL_MCP_PROTOCOL_H

#include "mcp_json.h"
#include "mcp_protocol_async.h"

typedef struct mcp_protocol_session {
    char protocol_version[16];
    uint64_t window_started_ms;
    uint32_t requests_in_window;
    bool initialized;
    bool modern;
} mcp_protocol_session;

typedef struct mcp_tool_response {
    mcp_json_buffer structured_json;
    mcp_json_buffer content_json;
    bool is_error;
} mcp_tool_response;

typedef bool (*mcp_tool_call_fn)(void *context, const mcp_json_doc *request,
                                 size_t arguments, const char *tool_name,
                                 mcp_tool_response *out);
typedef bool (*mcp_resource_read_fn)(void *context, const mcp_json_doc *request,
                                     const char *uri, mcp_json_buffer *out_contents);

typedef struct mcp_protocol_backend {
    mcp_tool_call_fn call_tool;
    mcp_resource_read_fn read_resource;
    void *context;
} mcp_protocol_backend;

void mcp_protocol_session_reset(mcp_protocol_session *session);
bool mcp_protocol_handle(mcp_protocol_session *session, const uint8_t *line,
                         size_t length, const mcp_protocol_backend *backend,
                         mcp_json_buffer *response);
bool mcp_protocol_handle_async(mcp_protocol_session *session, const uint8_t *line,
                         size_t length, const mcp_protocol_backend *backend,
                         const mcp_protocol_async_backend *async_backend,
                         mcp_json_buffer *response);
bool mcp_protocol_request_expects_response(const uint8_t *line, size_t length);
bool mcp_tools_list_json(mcp_json_buffer *out);
bool mcp_tool_validate(const mcp_json_doc *doc, size_t arguments,
                       const char *tool_name, const char **out_error);

#endif
