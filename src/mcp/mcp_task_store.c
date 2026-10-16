#define _DARWIN_C_SOURCE
#include "mcp_task_store.h"

#include <pthread.h>
#include <stdlib.h>
#include <string.h>

#define MCP_TASK_STORE_CAP (MCP_TASK_STORE_ACTIVE_LIMIT + MCP_TASK_STORE_RETAINED_LIMIT)

typedef struct mcp_task_entry {
    bool used;
    bool cancel_requested;
    char id[MCP_TASK_STORE_ID_CAP];
    char message[MCP_TASK_STORE_MESSAGE_CAP];
    uint64_t owner;
    uint64_t created_ms;
    uint64_t updated_ms;
    uint64_t expires_ms;
    uint16_t progress_permille;
    mcp_task_status status;
    bool result_is_error;
    uint8_t *result_json;
    size_t result_length;
} mcp_task_entry;

struct mcp_task_store {
    pthread_mutex_t mutex;
    mcp_task_entry entries[MCP_TASK_STORE_CAP];
};

static void entry_clear(mcp_task_entry *entry)
{
    free(entry->result_json);
    *entry = (mcp_task_entry){0};
}

static void message_set(char destination[MCP_TASK_STORE_MESSAGE_CAP],
                        const char *message)
{
    if (message == NULL) message = "";
    size_t length = strnlen(message, MCP_TASK_STORE_MESSAGE_CAP);
    if (length >= MCP_TASK_STORE_MESSAGE_CAP) length = MCP_TASK_STORE_MESSAGE_CAP - 1U;
    memcpy(destination, message, length);
    destination[length] = '\0';
}

static void store_expire(mcp_task_store *store, uint64_t now_ms)
{
    for (size_t index = 0U; index < MCP_TASK_STORE_CAP; ++index) {
        mcp_task_entry *entry = &store->entries[index];
        if (entry->used && entry->status != MCP_TASK_WORKING &&
            now_ms >= entry->expires_ms) entry_clear(entry);
        else if (entry->used && entry->status == MCP_TASK_WORKING &&
            now_ms >= entry->expires_ms) {
            entry->cancel_requested = true;
            message_set(entry->message, "Task reached its time limit; stopping");
            entry->updated_ms = now_ms;
        }
    }
}

static mcp_task_entry *entry_find(mcp_task_store *store, uint64_t owner,
                                  const char *id)
{
    if (id == NULL) return NULL;
    size_t length = strnlen(id, MCP_TASK_STORE_ID_CAP);
    if (length != MCP_TASK_STORE_ID_CAP - 1U) return NULL;
    for (size_t index = 0U; index < MCP_TASK_STORE_CAP; ++index) {
        mcp_task_entry *entry = &store->entries[index];
        if (entry->used && entry->owner == owner &&
            memcmp(entry->id, id, MCP_TASK_STORE_ID_CAP) == 0) return entry;
    }
    return NULL;
}

static void task_id_generate(char output[MCP_TASK_STORE_ID_CAP])
{
    uint8_t random_bytes[16U];
    static const char hex[] = "0123456789abcdef";
    arc4random_buf(random_bytes, sizeof(random_bytes));
    for (size_t index = 0U; index < sizeof(random_bytes); ++index) {
        output[index * 2U] = hex[random_bytes[index] >> 4U];
        output[index * 2U + 1U] = hex[random_bytes[index] & 0x0fU];
    }
    output[32U] = '\0';
}

bool mcp_task_store_create(mcp_task_store **out_store)
{
    if (out_store == NULL) return false;
    *out_store = NULL;
    mcp_task_store *store = calloc(1U, sizeof(*store));
    if (store == NULL) return false;
    if (pthread_mutex_init(&store->mutex, NULL) != 0) {
        free(store);
        return false;
    }
    *out_store = store;
    return true;
}

void mcp_task_store_destroy(mcp_task_store *store)
{
    if (store == NULL) return;
    for (size_t index = 0U; index < MCP_TASK_STORE_CAP; ++index)
        entry_clear(&store->entries[index]);
    (void)pthread_mutex_destroy(&store->mutex);
    free(store);
}

bool mcp_task_store_start(mcp_task_store *store, uint64_t owner,
    uint64_t now_ms, char out_id[MCP_TASK_STORE_ID_CAP])
{
    if (store == NULL || owner == 0U || out_id == NULL) return false;
    out_id[0] = '\0';
    (void)pthread_mutex_lock(&store->mutex);
    store_expire(store, now_ms);
    size_t active = 0U, retained = 0U, slot = MCP_TASK_STORE_CAP;
    size_t oldest_terminal = MCP_TASK_STORE_CAP;
    for (size_t index = 0U; index < MCP_TASK_STORE_CAP; ++index) {
        if (!store->entries[index].used && slot == MCP_TASK_STORE_CAP) slot = index;
        if (store->entries[index].used &&
            store->entries[index].status == MCP_TASK_WORKING) active++;
        else if (store->entries[index].used) {
            retained++;
            if (oldest_terminal == MCP_TASK_STORE_CAP ||
                store->entries[index].updated_ms <
                    store->entries[oldest_terminal].updated_ms)
                oldest_terminal = index;
        }
    }
    if (retained >= MCP_TASK_STORE_RETAINED_LIMIT &&
        oldest_terminal < MCP_TASK_STORE_CAP) {
        entry_clear(&store->entries[oldest_terminal]);
        slot = oldest_terminal;
    }
    bool okay = active < MCP_TASK_STORE_ACTIVE_LIMIT && slot < MCP_TASK_STORE_CAP;
    if (okay) {
        mcp_task_entry *entry = &store->entries[slot];
        entry->used = true;
        entry->owner = owner;
        entry->status = MCP_TASK_WORKING;
        entry->created_ms = now_ms;
        entry->updated_ms = now_ms;
        entry->expires_ms = now_ms > UINT64_MAX - MCP_TASK_STORE_TTL_MS ?
            UINT64_MAX : now_ms + MCP_TASK_STORE_TTL_MS;
        entry->progress_permille = 0U;
        message_set(entry->message, "Preparing export");
        task_id_generate(entry->id);
        memcpy(out_id, entry->id, sizeof(entry->id));
    }
    (void)pthread_mutex_unlock(&store->mutex);
    return okay;
}

bool mcp_task_store_update(mcp_task_store *store, uint64_t owner,
    const char *id, uint64_t now_ms, uint16_t progress_permille,
    const char *message)
{
    if (store == NULL || progress_permille > 1000U || message == NULL ||
        strnlen(message, MCP_TASK_STORE_MESSAGE_CAP) >= MCP_TASK_STORE_MESSAGE_CAP)
        return false;
    (void)pthread_mutex_lock(&store->mutex);
    store_expire(store, now_ms);
    mcp_task_entry *entry = entry_find(store, owner, id);
    bool okay = entry != NULL && entry->status == MCP_TASK_WORKING &&
        progress_permille >= entry->progress_permille;
    if (okay) {
        entry->progress_permille = progress_permille;
        entry->updated_ms = now_ms;
        message_set(entry->message, message);
    }
    (void)pthread_mutex_unlock(&store->mutex);
    return okay;
}

bool mcp_task_store_request_cancel(mcp_task_store *store, uint64_t owner,
    const char *id, uint64_t now_ms)
{
    if (store == NULL) return false;
    (void)pthread_mutex_lock(&store->mutex);
    store_expire(store, now_ms);
    mcp_task_entry *entry = entry_find(store, owner, id);
    bool okay = entry != NULL && entry->status == MCP_TASK_WORKING;
    if (okay) {
        entry->cancel_requested = true;
        entry->updated_ms = now_ms;
        message_set(entry->message, "Cancellation requested");
    }
    (void)pthread_mutex_unlock(&store->mutex);
    return okay;
}

bool mcp_task_store_cancel_requested(mcp_task_store *store, uint64_t owner,
                                     const char *id)
{
    if (store == NULL) return true;
    (void)pthread_mutex_lock(&store->mutex);
    mcp_task_entry *entry = entry_find(store, owner, id);
    bool canceled = entry == NULL || entry->cancel_requested ||
        entry->status != MCP_TASK_WORKING;
    (void)pthread_mutex_unlock(&store->mutex);
    return canceled;
}

bool mcp_task_store_finish(mcp_task_store *store, uint64_t owner,
    const char *id, uint64_t now_ms, mcp_task_status status,
    const char *message, const uint8_t *result_json, size_t result_length,
    bool result_is_error)
{
    if (store == NULL || (status != MCP_TASK_COMPLETED &&
            status != MCP_TASK_CANCELLED && status != MCP_TASK_FAILED) ||
        message == NULL || strnlen(message, MCP_TASK_STORE_MESSAGE_CAP) >=
            MCP_TASK_STORE_MESSAGE_CAP || result_length > MCP_TASK_STORE_RESULT_LIMIT ||
        (result_length > 0U && result_json == NULL)) return false;
    uint8_t *copy = NULL;
    if (result_length > 0U) {
        copy = malloc(result_length);
        if (copy == NULL) return false;
        memcpy(copy, result_json, result_length);
    }
    (void)pthread_mutex_lock(&store->mutex);
    mcp_task_entry *entry = entry_find(store, owner, id);
    bool okay = entry != NULL && entry->status == MCP_TASK_WORKING;
    if (okay) {
        entry->status = status;
        entry->updated_ms = now_ms;
        entry->expires_ms = now_ms > UINT64_MAX - MCP_TASK_STORE_TTL_MS ?
            UINT64_MAX : now_ms + MCP_TASK_STORE_TTL_MS;
        if (status == MCP_TASK_COMPLETED) entry->progress_permille = 1000U;
        entry->result_json = copy;
        entry->result_length = result_length;
        entry->result_is_error = result_is_error;
        message_set(entry->message, message);
        copy = NULL;
    }
    (void)pthread_mutex_unlock(&store->mutex);
    free(copy);
    return okay;
}

bool mcp_task_store_get(mcp_task_store *store, uint64_t owner,
    const char *id, uint64_t now_ms, mcp_task_snapshot *out_snapshot,
    uint8_t *result_buffer, size_t result_capacity)
{
    if (store == NULL || out_snapshot == NULL) return false;
    *out_snapshot = (mcp_task_snapshot){0};
    (void)pthread_mutex_lock(&store->mutex);
    store_expire(store, now_ms);
    mcp_task_entry *entry = entry_find(store, owner, id);
    bool okay = entry != NULL && entry->result_length <= result_capacity &&
        (entry->result_length == 0U || result_buffer != NULL);
    if (okay) {
        memcpy(out_snapshot->id, entry->id, sizeof(entry->id));
        memcpy(out_snapshot->message, entry->message, sizeof(entry->message));
        out_snapshot->status = entry->status;
        out_snapshot->owner = entry->owner;
        out_snapshot->created_ms = entry->created_ms;
        out_snapshot->updated_ms = entry->updated_ms;
        out_snapshot->expires_ms = entry->expires_ms;
        out_snapshot->progress_permille = entry->progress_permille;
        out_snapshot->result_is_error = entry->result_is_error;
        out_snapshot->result_length = entry->result_length;
        if (entry->result_length > 0U)
            memcpy(result_buffer, entry->result_json, entry->result_length);
    }
    (void)pthread_mutex_unlock(&store->mutex);
    return okay;
}

const char *mcp_task_status_name(mcp_task_status status)
{
    switch (status) {
        case MCP_TASK_WORKING: return "working";
        case MCP_TASK_COMPLETED: return "completed";
        case MCP_TASK_CANCELLED: return "cancelled";
        case MCP_TASK_FAILED: return "failed";
    }
    return "failed";
}
