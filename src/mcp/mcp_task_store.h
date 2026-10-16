#ifndef VEXEL_MCP_TASK_STORE_H
#define VEXEL_MCP_TASK_STORE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define MCP_TASK_STORE_ACTIVE_LIMIT 2U
#define MCP_TASK_STORE_RETAINED_LIMIT 8U
#define MCP_TASK_STORE_RESULT_LIMIT (2U * 1024U * 1024U)
#define MCP_TASK_STORE_TTL_MS UINT64_C(900000)
#define MCP_TASK_STORE_MESSAGE_CAP 128U
#define MCP_TASK_STORE_ID_CAP 33U

typedef struct mcp_task_store mcp_task_store;

typedef enum mcp_task_status {
    MCP_TASK_WORKING = 0,
    MCP_TASK_COMPLETED,
    MCP_TASK_CANCELLED,
    MCP_TASK_FAILED
} mcp_task_status;

typedef struct mcp_task_snapshot {
    char id[MCP_TASK_STORE_ID_CAP];
    char message[MCP_TASK_STORE_MESSAGE_CAP];
    mcp_task_status status;
    uint64_t owner;
    uint64_t created_ms;
    uint64_t updated_ms;
    uint64_t expires_ms;
    uint16_t progress_permille;
    bool result_is_error;
    size_t result_length;
} mcp_task_snapshot;

bool mcp_task_store_create(mcp_task_store **out_store);
void mcp_task_store_destroy(mcp_task_store *store);
bool mcp_task_store_start(mcp_task_store *store, uint64_t owner,
    uint64_t now_ms, char out_id[MCP_TASK_STORE_ID_CAP]);
bool mcp_task_store_update(mcp_task_store *store, uint64_t owner,
    const char *id, uint64_t now_ms, uint16_t progress_permille,
    const char *message);
bool mcp_task_store_request_cancel(mcp_task_store *store, uint64_t owner,
    const char *id, uint64_t now_ms);
bool mcp_task_store_cancel_requested(mcp_task_store *store, uint64_t owner,
    const char *id);
bool mcp_task_store_finish(mcp_task_store *store, uint64_t owner,
    const char *id, uint64_t now_ms, mcp_task_status status,
    const char *message, const uint8_t *result_json, size_t result_length,
    bool result_is_error);
bool mcp_task_store_get(mcp_task_store *store, uint64_t owner,
    const char *id, uint64_t now_ms, mcp_task_snapshot *out_snapshot,
    uint8_t *result_buffer, size_t result_capacity);
const char *mcp_task_status_name(mcp_task_status status);

#endif
