#ifndef VEXEL_APP_MCP_BROKER_IO_H
#define VEXEL_APP_MCP_BROKER_IO_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define MCP_BROKER_IO_CLIENTS 8U
#define MCP_BROKER_IO_POLL_BYTE_BUDGET 65536U
#define MCP_BROKER_IO_LINE_BUDGET 8U
#define MCP_BROKER_IO_SYSCALL_BUDGET 128U
#define MCP_BROKER_IO_CLIENT_BUDGET 8192U
#define MCP_BROKER_IO_CLIENT_QUEUE_BYTES (4U * 1024U * 1024U)
#define MCP_BROKER_IO_CLIENT_QUEUE_MESSAGES 64U
#define MCP_BROKER_IO_PROGRESS_BYTES (64U * 1024U)

/* One event-loop thread owns each pool. Do not destroy a pool from a callback.
 * attach consumes descriptor only on success; caller closes it on failure.
 * Line buffers are callback-borrowed. Explicit close invalidates them at once.
 * Enqueue copies payload and key. Callback context outlives pool use. */
typedef struct mcp_broker_io mcp_broker_io;
typedef uint64_t mcp_broker_io_owner;

typedef enum mcp_broker_io_reason {
    MCP_BROKER_IO_EOF,
    MCP_BROKER_IO_SOCKET_ERROR,
    MCP_BROKER_IO_OVERSIZED,
    MCP_BROKER_IO_PROTOCOL_ERROR,
    MCP_BROKER_IO_QUEUE_FULL,
    MCP_BROKER_IO_SHUTDOWN,
    MCP_BROKER_IO_REQUESTED,
    MCP_BROKER_IO_INVALID_FRAME
} mcp_broker_io_reason;

typedef enum mcp_broker_io_enqueue_result {
    MCP_BROKER_IO_QUEUED,
    MCP_BROKER_IO_SKIPPED,
    MCP_BROKER_IO_CLOSED
} mcp_broker_io_enqueue_result;

typedef enum mcp_broker_io_message_kind {
    MCP_BROKER_IO_TERMINAL,
    MCP_BROKER_IO_PROGRESS
} mcp_broker_io_message_kind;

typedef bool (*mcp_broker_io_line_fn)(void *context, mcp_broker_io_owner owner,
                                      const uint8_t *line, size_t length);
typedef void (*mcp_broker_io_disconnect_fn)(void *context,
    mcp_broker_io_owner owner, mcp_broker_io_reason reason);

typedef struct mcp_broker_io_callbacks {
    mcp_broker_io_line_fn line;
    mcp_broker_io_disconnect_fn disconnect;
    void *context;
} mcp_broker_io_callbacks;

typedef struct mcp_broker_io_counters {
    uint32_t syscalls;
    uint32_t lines;
    uint32_t clients_visited;
    size_t read_bytes;
    size_t written_bytes;
    size_t per_client_bytes[MCP_BROKER_IO_CLIENTS];
} mcp_broker_io_counters;

bool mcp_broker_io_create(const mcp_broker_io_callbacks *callbacks,
                          mcp_broker_io **out_pool);
void mcp_broker_io_destroy(mcp_broker_io *pool);
bool mcp_broker_io_attach(mcp_broker_io *pool, int descriptor,
                          mcp_broker_io_owner *out_owner);
bool mcp_broker_io_is_connected(const mcp_broker_io *pool,
                                mcp_broker_io_owner owner);
bool mcp_broker_io_close(mcp_broker_io *pool, mcp_broker_io_owner owner,
                         mcp_broker_io_reason reason);
mcp_broker_io_enqueue_result mcp_broker_io_enqueue(mcp_broker_io *pool,
    mcp_broker_io_owner owner, const uint8_t *json, size_t length,
    mcp_broker_io_message_kind kind, const uint8_t *task_key,
    size_t task_key_length);
mcp_broker_io_counters mcp_broker_io_poll(mcp_broker_io *pool);

#endif
