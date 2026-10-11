#ifndef VEXEL_MCP_OWNER_QUEUE_H
#define VEXEL_MCP_OWNER_QUEUE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define MCP_OWNER_QUEUE_CAPACITY 16U
#define MCP_OWNER_QUEUE_BYTES_LIMIT (4U * 1024U * 1024U)

typedef struct mcp_owner_queue_entry {
    size_t owner_index;
    uint64_t owner_generation;
    uint64_t deadline_ms;
    uint8_t *bytes;
    size_t length;
} mcp_owner_queue_entry;

typedef struct mcp_owner_queue {
    mcp_owner_queue_entry entries[MCP_OWNER_QUEUE_CAPACITY];
    size_t head;
    size_t count;
    size_t bytes;
} mcp_owner_queue;

typedef struct mcp_owner_queue_item {
    size_t owner_index;
    uint64_t owner_generation;
    uint8_t *bytes;
    size_t length;
} mcp_owner_queue_item;

typedef enum mcp_owner_queue_result {
    MCP_OWNER_QUEUE_ENQUEUED,
    MCP_OWNER_QUEUE_FULL,
    MCP_OWNER_QUEUE_INVALID,
    MCP_OWNER_QUEUE_NO_MEMORY
} mcp_owner_queue_result;

typedef enum mcp_owner_queue_take_result {
    MCP_OWNER_QUEUE_EMPTY,
    MCP_OWNER_QUEUE_WAITING,
    MCP_OWNER_QUEUE_READY,
    MCP_OWNER_QUEUE_EXPIRED
} mcp_owner_queue_take_result;

void mcp_owner_queue_init(mcp_owner_queue *queue);
mcp_owner_queue_result mcp_owner_queue_enqueue(mcp_owner_queue *queue,
    size_t owner_index, uint64_t owner_generation, const uint8_t *bytes,
    size_t length, uint64_t now_ms, uint64_t timeout_ms);
mcp_owner_queue_take_result mcp_owner_queue_take(mcp_owner_queue *queue,
    bool owner_busy, uint64_t now_ms, mcp_owner_queue_item *out_item);
void mcp_owner_queue_item_destroy(mcp_owner_queue_item *item);

#endif
