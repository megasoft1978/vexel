#include "mcp_owner_queue.h"

#include <stdlib.h>
#include <string.h>

void mcp_owner_queue_init(mcp_owner_queue *queue)
{
    if (queue != NULL) *queue = (mcp_owner_queue){0};
}

mcp_owner_queue_result mcp_owner_queue_enqueue(mcp_owner_queue *queue,
    size_t owner_index, uint64_t owner_generation, const uint8_t *bytes,
    size_t length, uint64_t now_ms, uint64_t timeout_ms)
{
    if (queue == NULL || bytes == NULL || length == 0U ||
        owner_generation == 0U) return MCP_OWNER_QUEUE_INVALID;
    if (queue->count >= MCP_OWNER_QUEUE_CAPACITY ||
        queue->bytes > MCP_OWNER_QUEUE_BYTES_LIMIT ||
        length > MCP_OWNER_QUEUE_BYTES_LIMIT - queue->bytes)
        return MCP_OWNER_QUEUE_FULL;
    uint8_t *copy = malloc(length);
    if (copy == NULL) return MCP_OWNER_QUEUE_NO_MEMORY;
    memcpy(copy, bytes, length);
    size_t tail = (queue->head + queue->count) % MCP_OWNER_QUEUE_CAPACITY;
    uint64_t deadline = timeout_ms > UINT64_MAX - now_ms ? UINT64_MAX :
        now_ms + timeout_ms;
    queue->entries[tail] = (mcp_owner_queue_entry){
        .owner_index = owner_index,
        .owner_generation = owner_generation,
        .deadline_ms = deadline,
        .bytes = copy,
        .length = length
    };
    queue->count++;
    queue->bytes += length;
    return MCP_OWNER_QUEUE_ENQUEUED;
}

mcp_owner_queue_take_result mcp_owner_queue_take(mcp_owner_queue *queue,
    bool owner_busy, uint64_t now_ms, mcp_owner_queue_item *out_item)
{
    if (out_item != NULL) *out_item = (mcp_owner_queue_item){0};
    if (queue == NULL || out_item == NULL || queue->count == 0U)
        return MCP_OWNER_QUEUE_EMPTY;
    mcp_owner_queue_entry *entry = &queue->entries[queue->head];
    if (owner_busy && now_ms < entry->deadline_ms)
        return MCP_OWNER_QUEUE_WAITING;
    mcp_owner_queue_take_result result = owner_busy ? MCP_OWNER_QUEUE_EXPIRED :
        MCP_OWNER_QUEUE_READY;
    *out_item = (mcp_owner_queue_item){entry->owner_index,
        entry->owner_generation, entry->bytes, entry->length};
    if (queue->bytes >= entry->length) queue->bytes -= entry->length;
    else queue->bytes = 0U;
    *entry = (mcp_owner_queue_entry){0};
    queue->head = (queue->head + 1U) % MCP_OWNER_QUEUE_CAPACITY;
    queue->count--;
    return result;
}

void mcp_owner_queue_item_destroy(mcp_owner_queue_item *item)
{
    if (item == NULL) return;
    free(item->bytes);
    *item = (mcp_owner_queue_item){0};
}
