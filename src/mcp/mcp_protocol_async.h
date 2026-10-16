#ifndef VEXEL_MCP_PROTOCOL_ASYNC_H
#define VEXEL_MCP_PROTOCOL_ASYNC_H

#include "mcp_json.h"

typedef enum mcp_protocol_async_disposition {
    MCP_PROTOCOL_ASYNC_UNHANDLED = 0,
    MCP_PROTOCOL_ASYNC_RESULT,
    MCP_PROTOCOL_ASYNC_ERROR,
    MCP_PROTOCOL_ASYNC_DEFERRED,
    MCP_PROTOCOL_ASYNC_FAILED
} mcp_protocol_async_disposition;

typedef enum mcp_protocol_async_payload_kind {
    MCP_PROTOCOL_ASYNC_PAYLOAD_ERROR = 0,
    MCP_PROTOCOL_ASYNC_PAYLOAD_TOOL_RESULT,
    MCP_PROTOCOL_ASYNC_PAYLOAD_TASK_GET,
    MCP_PROTOCOL_ASYNC_PAYLOAD_TASK_CANCEL
} mcp_protocol_async_payload_kind;

/*
 * The owner keeps context alive until no callback can run. All callbacks run on
 * the protocol owner thread. Request documents and their argument, tool-name,
 * task-ID, and request-ID-token slices are borrowed for the callback only and
 * must not be retained.
 *
 * RESULT writes one complete JSON-RPC result object (without its envelope) to
 * payload. A resultType:"task" must refer to a durable store entry created
 * before the callback returns. ERROR writes one JSON-RPC error object.
 * DEFERRED is valid only for legacy tools/call and emits no immediate response.
 * UNHANDLED selects the synchronous fallback; FAILED emits a bounded error.
 */
typedef struct mcp_protocol_async_backend {
    void *context;
    bool tasks_available;
    mcp_protocol_async_disposition (*call_tool)(void *context,
        const mcp_json_doc *request, size_t arguments, const char *tool_name,
        bool modern, bool request_supports_tasks, mcp_json_buffer *payload);
    mcp_protocol_async_disposition (*get_task)(void *context,
        const mcp_json_doc *request, const char *task_id,
        mcp_json_buffer *payload);
    mcp_protocol_async_disposition (*cancel_task)(void *context,
        const mcp_json_doc *request, const char *task_id,
        mcp_json_buffer *payload);
    void (*cancel_request)(void *context, const mcp_json_doc *request,
        size_t request_id_token);
} mcp_protocol_async_backend;

bool mcp_protocol_async_tasks_ready(const mcp_protocol_async_backend *backend);
bool mcp_protocol_async_validate_payload(const char *data, size_t length,
    mcp_protocol_async_payload_kind kind, bool *out_is_task);
bool mcp_protocol_async_task_id(const mcp_json_doc *doc, size_t params,
    char *out_id, size_t capacity);

/* Bind each token to its active legacy request and stop sending after terminal state. */
bool mcp_protocol_async_legacy_progress(const char *raw_token,
    size_t raw_token_length, uint32_t progress_permille,
    int64_t *prior_progress_permille, const char *message,
    mcp_json_buffer *out_notification);

#endif
