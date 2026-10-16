#define _DARWIN_C_SOURCE
#define _POSIX_C_SOURCE 200809L

#include "mcp_broker_io.h"

#include "mcp/mcp_json.h"

#include <errno.h>
#include <fcntl.h>
#include <stdatomic.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

#define IO_INPUT_CAP (MCP_JSON_MAX_BYTES + 2U)
#define IO_PROGRESS_MESSAGE_RESERVE 8U
#define IO_PROGRESS_BYTE_RESERVE MCP_BROKER_IO_PROGRESS_BYTES

typedef struct io_message {
    struct io_message *next;
    uint8_t *bytes;
    size_t length;
    size_t offset;
    uint8_t *key;
    size_t key_length;
    mcp_broker_io_message_kind kind;
} io_message;

typedef struct io_client {
    int descriptor;
    mcp_broker_io_owner owner;
    uint8_t *input;
    size_t input_length;
    bool eof_pending;
    io_message *head;
    io_message *tail;
    size_t queued_bytes;
    size_t progress_bytes;
    size_t message_count;
    size_t progress_count;
} io_client;

struct mcp_broker_io {
    mcp_broker_io_callbacks callbacks;
    io_client clients[MCP_BROKER_IO_CLIENTS];
    size_t cursor;
};

static _Atomic uint64_t next_owner = 1U;

static bool allocate_owner(mcp_broker_io_owner *out_owner)
{
    uint64_t candidate = atomic_load_explicit(&next_owner, memory_order_relaxed);
    for (;;) {
        if (candidate == 0U || candidate == UINT64_MAX) return false;
        if (atomic_compare_exchange_weak_explicit(&next_owner, &candidate,
            candidate + 1U, memory_order_relaxed, memory_order_relaxed)) {
            *out_owner = candidate;
            return true;
        }
    }
}

static io_client *find_client(const mcp_broker_io *pool,
                              mcp_broker_io_owner owner)
{
    if (pool == NULL || owner == 0U) return NULL;
    for (size_t index = 0U; index < MCP_BROKER_IO_CLIENTS; ++index)
        if (pool->clients[index].descriptor >= 0 &&
            pool->clients[index].owner == owner)
            return (io_client *)&pool->clients[index];
    return NULL;
}

static void free_message(io_message *message)
{
    if (message == NULL) return;
    free(message->bytes);
    free(message->key);
    free(message);
}

static void close_client(mcp_broker_io *pool, io_client *client,
                         mcp_broker_io_reason reason)
{
    if (pool == NULL || client == NULL || client->descriptor < 0) return;
    mcp_broker_io_owner owner = client->owner;
    (void)close(client->descriptor);
    free(client->input);
    io_message *message = client->head;
    while (message != NULL) {
        io_message *next = message->next;
        free_message(message);
        message = next;
    }
    *client = (io_client){.descriptor = -1};
    if (pool->callbacks.disconnect != NULL)
        pool->callbacks.disconnect(pool->callbacks.context, owner, reason);
}

bool mcp_broker_io_create(const mcp_broker_io_callbacks *callbacks,
                          mcp_broker_io **out_pool)
{
    if (callbacks == NULL || out_pool == NULL || callbacks->line == NULL)
        return false;
    *out_pool = NULL;
    mcp_broker_io *pool = calloc(1U, sizeof(*pool));
    if (pool == NULL) return false;
    pool->callbacks = *callbacks;
    for (size_t index = 0U; index < MCP_BROKER_IO_CLIENTS; ++index)
        pool->clients[index].descriptor = -1;
    *out_pool = pool;
    return true;
}

void mcp_broker_io_destroy(mcp_broker_io *pool)
{
    if (pool == NULL) return;
    for (size_t index = 0U; index < MCP_BROKER_IO_CLIENTS; ++index)
        close_client(pool, &pool->clients[index], MCP_BROKER_IO_SHUTDOWN);
    free(pool);
}

bool mcp_broker_io_attach(mcp_broker_io *pool, int descriptor,
                          mcp_broker_io_owner *out_owner)
{
    if (pool == NULL || descriptor < 0 || out_owner == NULL) return false;
    size_t index = MCP_BROKER_IO_CLIENTS;
    for (size_t candidate = 0U; candidate < MCP_BROKER_IO_CLIENTS; ++candidate) {
        if (pool->clients[candidate].descriptor == descriptor) return false;
        if (index == MCP_BROKER_IO_CLIENTS &&
            pool->clients[candidate].descriptor < 0) index = candidate;
    }
    if (index == MCP_BROKER_IO_CLIENTS) return false;
    int flags = fcntl(descriptor, F_GETFL, 0);
    if (flags < 0 || fcntl(descriptor, F_SETFL, flags | O_NONBLOCK) != 0)
        return false;
#if defined(SO_NOSIGPIPE)
    int enabled = 1;
    if (setsockopt(descriptor, SOL_SOCKET, SO_NOSIGPIPE, &enabled,
                   sizeof(enabled)) != 0) return false;
#endif
    uint8_t *input = malloc(IO_INPUT_CAP);
    if (input == NULL) return false;
    mcp_broker_io_owner owner = 0U;
    if (!allocate_owner(&owner)) {
        free(input);
        return false;
    }
    pool->clients[index] = (io_client){
        .descriptor = descriptor, .owner = owner, .input = input};
    *out_owner = owner;
    return true;
}

bool mcp_broker_io_is_connected(const mcp_broker_io *pool,
                                mcp_broker_io_owner owner)
{
    return find_client(pool, owner) != NULL;
}

bool mcp_broker_io_close(mcp_broker_io *pool, mcp_broker_io_owner owner,
                         mcp_broker_io_reason reason)
{
    io_client *client = find_client(pool, owner);
    if (client == NULL || reason < MCP_BROKER_IO_EOF ||
        reason > MCP_BROKER_IO_INVALID_FRAME) return false;
    close_client(pool, client, reason);
    return true;
}

static bool has_line(const io_client *client, size_t *line_end,
                     size_t *consumed)
{
    const uint8_t *newline = memchr(client->input, '\n', client->input_length);
    if (newline == NULL) return false;
    *consumed = (size_t)(newline - client->input) + 1U;
    *line_end = *consumed - 1U;
    if (*line_end > 0U && client->input[*line_end - 1U] == '\r') --*line_end;
    return true;
}

static bool process_one_line(mcp_broker_io *pool, io_client *client,
                             mcp_broker_io_counters *counters)
{
    size_t line_end = 0U;
    size_t consumed = 0U;
    if (!has_line(client, &line_end, &consumed)) return true;
    if (line_end > MCP_JSON_MAX_BYTES) {
        close_client(pool, client, MCP_BROKER_IO_OVERSIZED);
        return false;
    }
    mcp_broker_io_owner owner = client->owner;
    bool okay = pool->callbacks.line(pool->callbacks.context, owner,
        client->input, line_end);
    if (client->descriptor < 0 || client->owner != owner ||
        find_client(pool, owner) != client) return false;
    if (!okay) {
        close_client(pool, client, MCP_BROKER_IO_PROTOCOL_ERROR);
        return false;
    }
    client->input_length -= consumed;
    memmove(client->input, client->input + consumed, client->input_length);
    ++counters->lines;
    return true;
}

static bool queue_frame(const uint8_t *json, size_t length,
                        uint8_t **out_bytes, size_t *out_length)
{
    if (json == NULL || out_bytes == NULL || out_length == NULL || length == 0U ||
        length > MCP_JSON_MAX_BYTES || length == SIZE_MAX) return false;
    uint8_t *bytes = malloc(length + 1U);
    if (bytes == NULL) return false;
    memcpy(bytes, json, length);
    bytes[length] = '\n';
    *out_bytes = bytes;
    *out_length = length + 1U;
    return true;
}

static bool progress_key_equal(const io_message *message, const uint8_t *key,
                               size_t key_length)
{
    return message->kind == MCP_BROKER_IO_PROGRESS && message->offset == 0U &&
        message->key_length == key_length && key_length != 0U &&
        memcmp(message->key, key, key_length) == 0;
}

static void remove_message(io_client *client, io_message *previous,
                           io_message *message)
{
    if (previous == NULL) client->head = message->next;
    else previous->next = message->next;
    if (client->tail == message) client->tail = previous;
    client->queued_bytes -= message->length;
    --client->message_count;
    if (message->kind == MCP_BROKER_IO_PROGRESS) {
        client->progress_bytes -= message->length;
        --client->progress_count;
    }
    free_message(message);
}

mcp_broker_io_enqueue_result mcp_broker_io_enqueue(mcp_broker_io *pool,
    mcp_broker_io_owner owner, const uint8_t *json, size_t length,
    mcp_broker_io_message_kind kind, const uint8_t *task_key,
    size_t task_key_length)
{
    io_client *client = find_client(pool, owner);
    if (client == NULL) return MCP_BROKER_IO_CLOSED;
    if (kind != MCP_BROKER_IO_TERMINAL && kind != MCP_BROKER_IO_PROGRESS)
        return MCP_BROKER_IO_SKIPPED;
    if (kind == MCP_BROKER_IO_PROGRESS &&
        (task_key == NULL || task_key_length == 0U || task_key_length > 128U))
        return MCP_BROKER_IO_SKIPPED;
    if (json == NULL || length == 0U || length > MCP_JSON_MAX_BYTES) {
        if (kind == MCP_BROKER_IO_TERMINAL) {
            close_client(pool, client, MCP_BROKER_IO_INVALID_FRAME);
            return MCP_BROKER_IO_CLOSED;
        }
        return MCP_BROKER_IO_SKIPPED;
    }
    uint8_t *bytes = NULL;
    size_t frame_length = 0U;
    if (!queue_frame(json, length, &bytes, &frame_length)) {
        if (kind == MCP_BROKER_IO_TERMINAL) {
            close_client(pool, client, MCP_BROKER_IO_QUEUE_FULL);
            return MCP_BROKER_IO_CLOSED;
        }
        return MCP_BROKER_IO_SKIPPED;
    }
    if (kind == MCP_BROKER_IO_PROGRESS) {
        io_message *previous = NULL;
        for (io_message *item = client->head; item != NULL; item = item->next) {
            if (progress_key_equal(item, task_key, task_key_length)) {
                size_t other_queued = client->queued_bytes - item->length;
                if (frame_length > IO_PROGRESS_BYTE_RESERVE -
                    (client->progress_bytes - item->length) ||
                    frame_length > MCP_BROKER_IO_CLIENT_QUEUE_BYTES - other_queued) {
                    free(bytes);
                    return MCP_BROKER_IO_SKIPPED;
                }
                io_message *replacement = calloc(1U, sizeof(*replacement));
                uint8_t *key = malloc(task_key_length);
                if (replacement == NULL || key == NULL) {
                    free(replacement); free(key); free(bytes);
                    return MCP_BROKER_IO_SKIPPED;
                }
                memcpy(key, task_key, task_key_length);
                *replacement = (io_message){item->next, bytes, frame_length, 0U,
                    key, task_key_length, kind};
                if (previous == NULL) client->head = replacement;
                else previous->next = replacement;
                if (client->tail == item) client->tail = replacement;
                client->queued_bytes = other_queued + frame_length;
                client->progress_bytes = client->progress_bytes -
                    item->length + frame_length;
                free_message(item);
                return MCP_BROKER_IO_QUEUED;
            }
            previous = item;
        }
        size_t progress_limit = MCP_BROKER_IO_CLIENT_QUEUE_BYTES -
            IO_PROGRESS_BYTE_RESERVE;
        if (frame_length > IO_PROGRESS_BYTE_RESERVE - client->progress_bytes ||
            client->queued_bytes > progress_limit ||
            frame_length > progress_limit - client->queued_bytes ||
            client->message_count >= MCP_BROKER_IO_CLIENT_QUEUE_MESSAGES -
                IO_PROGRESS_MESSAGE_RESERVE) {
            free(bytes);
            return MCP_BROKER_IO_SKIPPED;
        }
    }
    if (frame_length > MCP_BROKER_IO_CLIENT_QUEUE_BYTES - client->queued_bytes ||
        client->message_count >= MCP_BROKER_IO_CLIENT_QUEUE_MESSAGES) {
        free(bytes);
        if (kind == MCP_BROKER_IO_TERMINAL) {
            close_client(pool, client, MCP_BROKER_IO_QUEUE_FULL);
            return MCP_BROKER_IO_CLOSED;
        }
        return MCP_BROKER_IO_SKIPPED;
    }
    io_message *message = calloc(1U, sizeof(*message));
    if (message == NULL) {
        free(bytes);
        if (kind == MCP_BROKER_IO_TERMINAL) {
            close_client(pool, client, MCP_BROKER_IO_QUEUE_FULL);
            return MCP_BROKER_IO_CLOSED;
        }
        return MCP_BROKER_IO_SKIPPED;
    }
    if (kind == MCP_BROKER_IO_PROGRESS) {
        message->key = malloc(task_key_length);
        if (message->key == NULL) {
            free_message(message); free(bytes);
            return MCP_BROKER_IO_SKIPPED;
        }
        memcpy(message->key, task_key, task_key_length);
        message->key_length = task_key_length;
    }
    message->bytes = bytes;
    message->length = frame_length;
    message->kind = kind;
    if (client->tail == NULL) client->head = message;
    else client->tail->next = message;
    client->tail = message;
    client->queued_bytes += frame_length;
    ++client->message_count;
    if (kind == MCP_BROKER_IO_PROGRESS) {
        client->progress_bytes += frame_length;
        ++client->progress_count;
    }
    return MCP_BROKER_IO_QUEUED;
}

static bool write_one(mcp_broker_io *pool, io_client *client,
                      mcp_broker_io_counters *counters, size_t *budget)
{
    io_message *message = client->head;
    if (message == NULL || *budget == 0U || counters->syscalls >=
        MCP_BROKER_IO_SYSCALL_BUDGET) return true;
    size_t amount_wanted = message->length - message->offset;
    if (amount_wanted > *budget) amount_wanted = *budget;
    ++counters->syscalls;
#if defined(MSG_NOSIGNAL)
    ssize_t amount = send(client->descriptor, message->bytes + message->offset,
        amount_wanted, MSG_NOSIGNAL);
#else
    ssize_t amount = send(client->descriptor, message->bytes + message->offset,
        amount_wanted, 0);
#endif
    if (amount < 0 && errno == EINTR) return true;
    if (amount < 0 && (errno == EAGAIN || errno == EWOULDBLOCK)) return true;
    if (amount <= 0) {
        close_client(pool, client, MCP_BROKER_IO_SOCKET_ERROR);
        return false;
    }
    size_t written = (size_t)amount;
    message->offset += written;
    counters->written_bytes += written;
    *budget -= written;
    if (message->offset == message->length)
        remove_message(client, NULL, message);
    return true;
}

mcp_broker_io_counters mcp_broker_io_poll(mcp_broker_io *pool)
{
    mcp_broker_io_counters counters = {0};
    if (pool == NULL) return counters;
    size_t byte_budget = MCP_BROKER_IO_POLL_BYTE_BUDGET;
    for (size_t visited = 0U; visited < MCP_BROKER_IO_CLIENTS &&
         counters.syscalls < MCP_BROKER_IO_SYSCALL_BUDGET; ++visited) {
        size_t index = (pool->cursor + visited) % MCP_BROKER_IO_CLIENTS;
        io_client *client = &pool->clients[index];
        if (client->descriptor < 0) continue;
        ++counters.clients_visited;
        size_t client_budget = MCP_BROKER_IO_CLIENT_BUDGET;
        uint32_t lines_before_client = counters.lines;
        if (!process_one_line(pool, client, &counters)) continue;
        if (client->descriptor < 0) continue;
        if (client->eof_pending) {
            size_t line_end = 0U;
            size_t consumed = 0U;
            if (!has_line(client, &line_end, &consumed))
                close_client(pool, client, MCP_BROKER_IO_EOF);
            continue;
        }
        if (client->head != NULL && byte_budget > 0U) {
            size_t write_limit = client_budget < byte_budget ? client_budget : byte_budget;
            size_t before = counters.written_bytes;
            (void)write_one(pool, client, &counters, &write_limit);
            size_t written = counters.written_bytes - before;
            client_budget -= written;
            byte_budget -= written;
            counters.per_client_bytes[index] += written;
        }
        if (client->descriptor < 0 || byte_budget == 0U || client_budget == 0U ||
            counters.syscalls >= MCP_BROKER_IO_SYSCALL_BUDGET) continue;
        size_t allowance = client_budget;
        if (allowance > byte_budget) allowance = byte_budget;
        if (allowance > IO_INPUT_CAP - client->input_length)
            allowance = IO_INPUT_CAP - client->input_length;
        if (allowance == 0U) {
            close_client(pool, client, MCP_BROKER_IO_OVERSIZED);
            continue;
        }
        ++counters.syscalls;
        ssize_t amount = recv(client->descriptor,
            client->input + client->input_length, allowance, 0);
        if (amount < 0 && errno == EINTR) continue;
        if (amount < 0 && (errno == EAGAIN || errno == EWOULDBLOCK)) continue;
        if (amount < 0) {
            close_client(pool, client, MCP_BROKER_IO_SOCKET_ERROR);
            continue;
        }
        if (amount == 0) {
            client->eof_pending = true;
            size_t line_end = 0U;
            size_t consumed = 0U;
            if (counters.lines == lines_before_client &&
                !has_line(client, &line_end, &consumed))
                close_client(pool, client, MCP_BROKER_IO_EOF);
            continue;
        }
        client->input_length += (size_t)amount;
        counters.read_bytes += (size_t)amount;
        client_budget -= (size_t)amount;
        byte_budget -= (size_t)amount;
        counters.per_client_bytes[index] += (size_t)amount;
        if (counters.lines == lines_before_client &&
            !process_one_line(pool, client, &counters)) continue;
        if (client->descriptor >= 0 && client->input_length > MCP_JSON_MAX_BYTES + 1U)
            close_client(pool, client, MCP_BROKER_IO_OVERSIZED);
    }
    pool->cursor = (pool->cursor + 1U) % MCP_BROKER_IO_CLIENTS;
    return counters;
}
