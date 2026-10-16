#define _DARWIN_C_SOURCE
#define _POSIX_C_SOURCE 200809L

#include "mcp/mcp_json.h"
#include "mcp/mcp_socket.h"

#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <sys/un.h>
#include <time.h>
#include <unistd.h>

#define MCP_IO_TIMEOUT_SECONDS 30
#define MCP_IO_CHUNK 8192U
#define MCP_PENDING_MAX 256U
#define MCP_PENDING_BYTES MCP_JSON_MAX_BYTES

typedef enum rpc_id_type { RPC_ID_STRING, RPC_ID_NUMBER } rpc_id_type;

typedef struct rpc_identity {
    uint8_t *bytes;
    size_t length;
    size_t allocation;
    rpc_id_type type;
} rpc_identity;

typedef struct pending_request {
    rpc_identity id;
    bool active;
    bool forwarded;
} pending_request;

typedef struct pending_set {
    pending_request slots[MCP_PENDING_MAX];
    size_t used_bytes;
    size_t count;
    size_t outstanding;
} pending_set;

static int connect_broker(void)
{
    char path[sizeof(((struct sockaddr_un *)0)->sun_path)];
    if (!mcp_socket_path(path, sizeof(path))) {
        (void)fputs("vexel-mcp: VEXEL_MCP_SOCKET or TMPDIR does not name a valid local socket path\n", stderr);
        return -1;
    }
    struct stat status;
    if (lstat(path, &status) != 0 || !S_ISSOCK(status.st_mode) || status.st_uid != getuid() ||
        (status.st_mode & (S_IRWXG | S_IRWXO)) != 0) {
        (void)fputs("vexel-mcp: Vexel is not running with its private MCP broker enabled\n", stderr);
        return -1;
    }
    struct sockaddr_un address = {0};
    size_t path_length = strlen(path);
    address.sun_family = AF_UNIX;
    address.sun_len = (uint8_t)(offsetof(struct sockaddr_un, sun_path) + path_length + 1U);
    memcpy(address.sun_path, path, path_length + 1U);
    int descriptor = socket(AF_UNIX, SOCK_STREAM, 0);
    if (descriptor < 0 || connect(descriptor, (struct sockaddr *)&address, sizeof(address)) != 0) {
        if (descriptor >= 0) (void)close(descriptor);
        (void)fputs("vexel-mcp: could not connect to the running Vexel app\n", stderr);
        return -1;
    }
    struct timeval timeout = {MCP_IO_TIMEOUT_SECONDS, 0};
    (void)setsockopt(descriptor, SOL_SOCKET, SO_RCVTIMEO, &timeout, sizeof(timeout));
    (void)setsockopt(descriptor, SOL_SOCKET, SO_SNDTIMEO, &timeout, sizeof(timeout));
    return descriptor;
}

/* Each direction owns one bounded line plus one fixed read chunk. A complete
 * line stays queued until its destination accepts it; chunk tails stay owned. */
typedef struct relay_stream {
    uint8_t *line;
    size_t length;
    size_t written;
    uint8_t scratch[MCP_IO_CHUNK];
    size_t cursor;
    size_t count;
    bool ready;
    bool eof;
    bool oversized;
    bool request_line;
    bool response_line;
    bool cancel_line;
    bool identity_valid;
    rpc_identity identity;
    size_t slot;
    size_t target_slot;
    int64_t deadline;
} relay_stream;

static const uint8_t parse_error[] =
    "{\"jsonrpc\":\"2.0\",\"id\":null,\"error\":{\"code\":-32700,"
    "\"message\":\"message exceeds the 2 MiB limit\"}}\n";

static int64_t now_ms(void)
{
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now) != 0) return -1;
    return (int64_t)now.tv_sec * 1000 + now.tv_nsec / 1000000;
}

static int64_t io_deadline(int64_t now)
{
    return now + MCP_IO_TIMEOUT_SECONDS * 1000;
}

static void identity_destroy(rpc_identity *identity)
{
    free(identity->bytes);
    *identity = (rpc_identity){0};
}

static bool copy_rpc_identity(const mcp_json_doc *document, size_t token,
                              rpc_identity *identity)
{
    if (token >= document->token_count) return false;
    const mcp_json_token *value = &document->tokens[token];
    if (value->type != MCP_JSON_STRING && value->type != MCP_JSON_NUMBER) return false;
    size_t capacity = value->end - value->start;
    uint8_t *bytes = malloc(capacity + 1U);
    if (bytes == NULL) return false;
    size_t length = 0U;
    bool okay;
    if (value->type == MCP_JSON_STRING) {
        okay = mcp_json_string_copy(document, token, (char *)bytes, capacity, &length);
    } else {
        okay = mcp_json_token_copy(document, token, (char *)bytes, capacity, &length);
    }
    if (!okay) { free(bytes); return false; }
    *identity = (rpc_identity){bytes, length, capacity + 1U,
        value->type == MCP_JSON_STRING ? RPC_ID_STRING : RPC_ID_NUMBER};
    return true;
}

static bool identity_equal(const rpc_identity *left, const rpc_identity *right)
{
    return left->type == right->type && left->length == right->length &&
        memcmp(left->bytes, right->bytes, left->length) == 0;
}

static size_t pending_find(const pending_set *pending, const rpc_identity *identity)
{
    for (size_t index = 0U; index < MCP_PENDING_MAX; index++) {
        if (pending->slots[index].active &&
            identity_equal(&pending->slots[index].id, identity)) return index;
    }
    return MCP_PENDING_MAX;
}

static bool identity_matches_token(const rpc_identity *identity,
                                   const mcp_json_doc *document, size_t token)
{
    if (token >= document->token_count) return false;
    const mcp_json_token *value = &document->tokens[token];
    rpc_id_type type = value->type == MCP_JSON_STRING ? RPC_ID_STRING : RPC_ID_NUMBER;
    if (type != identity->type) return false;
    if (type == RPC_ID_NUMBER) {
        size_t length = value->end - value->start;
        return length == identity->length &&
            memcmp(document->source + value->start, identity->bytes, length) == 0;
    }
    size_t cursor = value->start + 1U;
    size_t end = value->end - 1U;
    size_t compared = 0U;
    while (cursor < end) {
        uint8_t decoded[4];
        size_t decoded_length = 1U;
        uint8_t byte = document->source[cursor++];
        if (byte != '\\') decoded[0] = byte;
        else {
            if (cursor >= end) return false;
            uint8_t escape = document->source[cursor++];
            uint8_t simple = escape == '"' ? '"' : escape == '\\' ? '\\' :
                escape == '/' ? '/' : escape == 'b' ? '\b' : escape == 'f' ? '\f' :
                escape == 'n' ? '\n' : escape == 'r' ? '\r' : escape == 't' ? '\t' : 0U;
            if (simple != 0U) decoded[0] = simple;
            else {
                if (escape != 'u' || end - cursor < 4U) return false;
                uint32_t point = 0U;
                for (size_t index = 0U; index < 4U; index++) {
                    uint8_t hex = document->source[cursor++];
                    uint32_t digit = hex >= '0' && hex <= '9' ? hex - '0' :
                        hex >= 'a' && hex <= 'f' ? hex - 'a' + 10U :
                        hex >= 'A' && hex <= 'F' ? hex - 'A' + 10U : 16U;
                    if (digit > 15U) return false;
                    point = (point << 4U) | digit;
                }
                if (point >= 0xd800U && point <= 0xdbffU) {
                    if (end - cursor < 6U || document->source[cursor] != '\\' ||
                        document->source[cursor + 1U] != 'u') return false;
                    cursor += 2U;
                    uint32_t low = 0U;
                    for (size_t index = 0U; index < 4U; index++) {
                        uint8_t hex = document->source[cursor++];
                        uint32_t digit = hex >= '0' && hex <= '9' ? hex - '0' :
                            hex >= 'a' && hex <= 'f' ? hex - 'a' + 10U :
                            hex >= 'A' && hex <= 'F' ? hex - 'A' + 10U : 16U;
                        if (digit > 15U) return false;
                        low = (low << 4U) | digit;
                    }
                    if (low < 0xdc00U || low > 0xdfffU) return false;
                    point = 0x10000U + ((point - 0xd800U) << 10U) + low - 0xdc00U;
                }
                if (point <= 0x7fU) decoded[0] = (uint8_t)point;
                else if (point <= 0x7ffU) {
                    decoded[0] = (uint8_t)(0xc0U | (point >> 6U));
                    decoded[1] = (uint8_t)(0x80U | (point & 0x3fU)); decoded_length = 2U;
                } else if (point <= 0xffffU) {
                    decoded[0] = (uint8_t)(0xe0U | (point >> 12U));
                    decoded[1] = (uint8_t)(0x80U | ((point >> 6U) & 0x3fU));
                    decoded[2] = (uint8_t)(0x80U | (point & 0x3fU)); decoded_length = 3U;
                } else if (point <= 0x10ffffU) {
                    decoded[0] = (uint8_t)(0xf0U | (point >> 18U));
                    decoded[1] = (uint8_t)(0x80U | ((point >> 12U) & 0x3fU));
                    decoded[2] = (uint8_t)(0x80U | ((point >> 6U) & 0x3fU));
                    decoded[3] = (uint8_t)(0x80U | (point & 0x3fU)); decoded_length = 4U;
                } else return false;
            }
        }
        if (decoded_length > identity->length - compared ||
            memcmp(identity->bytes + compared, decoded, decoded_length) != 0) return false;
        compared += decoded_length;
    }
    return compared == identity->length;
}

static size_t pending_find_token(const pending_set *pending,
                                 const mcp_json_doc *document, size_t token,
                                 bool forwarded_only)
{
    for (size_t index = 0U; index < MCP_PENDING_MAX; index++) {
        const pending_request *request = &pending->slots[index];
        if (request->active && (!forwarded_only || request->forwarded) &&
            identity_matches_token(&request->id, document, token)) return index;
    }
    return MCP_PENDING_MAX;
}

static bool pending_reserve(pending_set *pending, rpc_identity *identity, size_t *slot)
{
    size_t cost = identity->allocation;
    if (pending->count == MCP_PENDING_MAX || cost > MCP_PENDING_BYTES - pending->used_bytes ||
        pending_find(pending, identity) != MCP_PENDING_MAX) return false;
    size_t index = 0U;
    while (index < MCP_PENDING_MAX && pending->slots[index].active) index++;
    if (index == MCP_PENDING_MAX) return false;
    pending->slots[index] = (pending_request){*identity, true, false};
    *identity = (rpc_identity){0};
    pending->used_bytes += cost;
    pending->count++;
    *slot = index;
    return true;
}

static void pending_remove(pending_set *pending, size_t slot)
{
    if (slot >= MCP_PENDING_MAX || !pending->slots[slot].active) return;
    pending->used_bytes -= pending->slots[slot].id.allocation;
    if (pending->slots[slot].forwarded) pending->outstanding--;
    identity_destroy(&pending->slots[slot].id);
    pending->slots[slot].active = false;
    pending->slots[slot].forwarded = false;
    pending->count--;
}

static void pending_destroy(pending_set *pending)
{
    for (size_t index = 0U; index < MCP_PENDING_MAX; index++)
        pending_remove(pending, index);
}

static bool parse_rpc_line(const uint8_t *line, size_t length, bool client,
                           relay_stream *stream, const pending_set *pending)
{
    mcp_json_doc document = {0};
    if (!mcp_json_parse(line, length, &document)) return true;
    size_t method = document.root < document.token_count &&
        document.tokens[document.root].type == MCP_JSON_OBJECT ?
        mcp_json_object_get(&document, document.root, "method") : MCP_JSON_NO_TOKEN;
    size_t id = document.root < document.token_count &&
        document.tokens[document.root].type == MCP_JSON_OBJECT ?
        mcp_json_object_get(&document, document.root, "id") : MCP_JSON_NO_TOKEN;
    bool okay = true;
    size_t version = document.root < document.token_count &&
        document.tokens[document.root].type == MCP_JSON_OBJECT ?
        mcp_json_object_get(&document, document.root, "jsonrpc") : MCP_JSON_NO_TOKEN;
    bool valid_envelope = mcp_json_string_equals(&document, version, "2.0");
    bool valid_method = method < document.token_count &&
        document.tokens[method].type == MCP_JSON_STRING;
    if (valid_envelope && valid_method && id != MCP_JSON_NO_TOKEN) {
        bool valid_id = id < document.token_count &&
            (document.tokens[id].type == MCP_JSON_STRING ||
             document.tokens[id].type == MCP_JSON_NUMBER);
        if (client && valid_id) {
            size_t allocation = document.tokens[id].end - document.tokens[id].start + 1U;
            if (allocation > MCP_PENDING_BYTES - pending->used_bytes ||
                pending->count == MCP_PENDING_MAX ||
                pending_find_token(pending, &document, id, false) != MCP_PENDING_MAX) {
                mcp_json_doc_destroy(&document);
                return false;
            }
            stream->request_line = true;
            if (!copy_rpc_identity(&document, id, &stream->identity)) okay = false;
            else stream->identity_valid = true;
        }
    } else if (valid_envelope && method == MCP_JSON_NO_TOKEN && id != MCP_JSON_NO_TOKEN) {
        bool valid_id = id < document.token_count &&
            (document.tokens[id].type == MCP_JSON_STRING ||
             document.tokens[id].type == MCP_JSON_NUMBER);
        if (!client && valid_id) {
            stream->response_line = true;
            stream->target_slot = pending_find_token(pending, &document, id, true);
        }
    } else if (client && valid_envelope && valid_method &&
               mcp_json_string_equals(&document, method, "notifications/cancelled")) {
        size_t params = mcp_json_object_get(&document, document.root, "params");
        size_t request_id = params < document.token_count ?
            mcp_json_object_get(&document, params, "requestId") : MCP_JSON_NO_TOKEN;
        if (request_id < document.token_count &&
            (document.tokens[request_id].type == MCP_JSON_STRING ||
             document.tokens[request_id].type == MCP_JSON_NUMBER)) {
            stream->cancel_line = true;
            stream->target_slot = pending_find_token(pending, &document, request_id, true);
        }
    }
    mcp_json_doc_destroy(&document);
    return okay;
}

static bool set_nonblocking(int descriptor, int *old_flags)
{
    *old_flags = fcntl(descriptor, F_GETFL);
    return *old_flags >= 0 && fcntl(descriptor, F_SETFL, *old_flags | O_NONBLOCK) == 0;
}

static bool complete_line(relay_stream *stream, bool client, bool newline, int64_t now,
                          const pending_set *pending)
{
    if (client && newline && stream->length > 0U &&
        stream->line[stream->length - 1U] == '\r') stream->length--;
    stream->request_line = false;
    stream->response_line = false;
    stream->cancel_line = false;
    stream->identity_valid = false;
    stream->slot = SIZE_MAX;
    stream->target_slot = SIZE_MAX;
    if (!parse_rpc_line(stream->line, stream->length, client, stream, pending)) return false;
    stream->line[stream->length++] = '\n';
    stream->ready = true;
    stream->deadline = io_deadline(now);
    return true;
}

/* Returns false only for an overlong/truncated broker line. Stdio overflow
 * discards through its delimiter and schedules exactly one local parse error. */
static bool consume_chunk(relay_stream *stream, bool client, bool *error_pending,
                          int64_t now, const pending_set *pending)
{
    while (!stream->ready && stream->cursor < stream->count) {
        uint8_t byte = stream->scratch[stream->cursor++];
        if (byte == '\n') {
            if (stream->oversized) {
                stream->oversized = false;
                stream->length = 0U;
                *error_pending = true;
                return true;
            }
            if (client && stream->length == 0U) continue;
            if (!complete_line(stream, client, true, now, pending)) return false;
        } else if (!stream->oversized) {
            if (stream->length == MCP_JSON_MAX_BYTES) {
                if (!client) return false;
                stream->oversized = true;
                stream->length = 0U;
            } else stream->line[stream->length++] = byte;
        }
    }
    if (!stream->ready && stream->eof && stream->cursor == stream->count) {
        if (stream->oversized) {
            stream->oversized = false;
            *error_pending = true;
        } else if (stream->length > 0U) {
            if (!client) return false;
            if (!complete_line(stream, true, false, now, pending)) return false;
        }
    }
    return true;
}

static bool read_chunk(int descriptor, relay_stream *stream, int64_t now,
                       bool *received_bytes)
{
    *received_bytes = false;
    ssize_t amount = read(descriptor, stream->scratch, sizeof(stream->scratch));
    if (amount < 0) return errno == EINTR || errno == EAGAIN || errno == EWOULDBLOCK;
    stream->cursor = 0U;
    stream->count = (size_t)amount;
    stream->eof = amount == 0;
    *received_bytes = amount > 0;
    if (descriptor != STDIN_FILENO) stream->deadline = io_deadline(now);
    return true;
}

static bool write_pending(int descriptor, const uint8_t *bytes, size_t length,
                          size_t *written, int64_t *deadline, int64_t now)
{
    ssize_t amount = write(descriptor, bytes + *written, length - *written);
    if (amount < 0) return errno == EINTR || errno == EAGAIN || errno == EWOULDBLOCK;
    if (amount == 0) return false;
    *written += (size_t)amount;
    *deadline = io_deadline(now);
    return true;
}

static bool flush_line(int descriptor, relay_stream *stream, int64_t now)
{
    if (!write_pending(descriptor, stream->line, stream->length, &stream->written,
                       &stream->deadline, now)) return false;
    if (stream->written == stream->length) {
        stream->length = 0U;
        stream->written = 0U;
        stream->ready = false;
    }
    return true;
}

static void clear_line_metadata(relay_stream *stream)
{
    stream->request_line = false;
    stream->response_line = false;
    stream->cancel_line = false;
    stream->identity_valid = false;
    stream->slot = SIZE_MAX;
    stream->target_slot = SIZE_MAX;
    identity_destroy(&stream->identity);
}

static bool can_read(const relay_stream *stream)
{
    return !stream->ready && !stream->eof && stream->cursor == stream->count;
}

static bool poll_timeout(int64_t deadline, int64_t now, int *timeout)
{
    if (deadline <= now) return false;
    int remaining = (int)(deadline - now);
    if (*timeout < 0 || remaining < *timeout) *timeout = remaining;
    return true;
}

static bool relay_loop(int broker, relay_stream *client, relay_stream *output,
                       pending_set *pending)
{
    bool error_pending = false;
    size_t error_written = 0U;
    int64_t error_deadline = 0;
    int64_t drain_deadline = 0;
    int64_t pending_deadline = 0;
    bool half_closed = false;
    for (;;) {
        int64_t now = now_ms();
        if (now < 0) return false;
        if (!consume_chunk(output, false, &error_pending, now, pending)) return false;
        if (output->ready && output->response_line) {
            if (output->target_slot < MCP_PENDING_MAX &&
                pending->slots[output->target_slot].active &&
                pending->slots[output->target_slot].forwarded) {
                pending_remove(pending, output->target_slot);
                if (pending->outstanding == 0U) pending_deadline = 0;
            }
            output->response_line = false;
        }
        bool had_error = error_pending;
        if (!error_pending && !consume_chunk(client, true, &error_pending, now, pending)) return false;
        if (!had_error && error_pending) error_deadline = io_deadline(now);
        if (client->ready && client->request_line && client->slot == SIZE_MAX) {
            if (!client->identity_valid ||
                !pending_reserve(pending, &client->identity, &client->slot)) return false;
            client->identity_valid = false;
        }
        if (client->eof && client->cursor == client->count && !client->ready &&
            !error_pending && !half_closed) {
            if (shutdown(broker, SHUT_WR) != 0) return false;
            half_closed = true;
            drain_deadline = io_deadline(now);
        }
        if (output->eof && !output->ready && output->cursor == output->count) {
            return !client->ready && !error_pending;
        }
        /* A local error may never interrupt a partially written broker line. */
        bool write_error = error_pending && (!output->ready || output->written == 0U);
        struct pollfd descriptors[3] = {
            {STDIN_FILENO, can_read(client) && !error_pending ? POLLIN : 0, 0},
            {broker, (short)((can_read(output) ? POLLIN : 0) |
                            (client->ready ? POLLOUT : 0)), 0},
            {STDOUT_FILENO, output->ready || error_pending ? POLLOUT : 0, 0}
        };
        /* Ignore inactive descriptors, including persistent HUP on stdin. */
        for (size_t i = 0U; i < 3U; i++) {
            if (descriptors[i].events == 0) descriptors[i].fd = -1;
        }
        int timeout = -1;
        if ((client->ready && !poll_timeout(client->deadline, now, &timeout)) ||
            ((output->ready || output->length > 0U) &&
             !poll_timeout(output->deadline, now, &timeout)) ||
            (error_pending && !poll_timeout(error_deadline, now, &timeout)) ||
            (half_closed && !poll_timeout(drain_deadline, now, &timeout)) ||
            (pending->outstanding > 0U &&
             !poll_timeout(pending_deadline, now, &timeout))) return false;
        int amount = poll(descriptors, 3U, timeout);
        if (amount < 0 && errno == EINTR) continue;
        if (amount <= 0) return false;
        for (size_t i = 0U; i < 3U; i++) {
            if ((descriptors[i].revents & POLLNVAL) != 0) return false;
        }
        now = now_ms();
        if (now < 0) return false;
        if ((descriptors[2].revents & POLLOUT) != 0) {
            if (write_error) {
                if (!write_pending(STDOUT_FILENO, parse_error, sizeof(parse_error) - 1U,
                                   &error_written, &error_deadline, now)) return false;
                if (error_written == sizeof(parse_error) - 1U) {
                    error_pending = false;
                    error_written = 0U;
                }
            } else {
                if (!flush_line(STDOUT_FILENO, output, now)) return false;
                if (!output->ready) clear_line_metadata(output);
            }
        } else if ((descriptors[2].revents & (POLLERR | POLLHUP)) != 0) return false;
        if (client->ready && (descriptors[1].revents & (POLLERR | POLLHUP)) != 0 &&
            (descriptors[1].revents & POLLOUT) == 0) return false;
        if ((descriptors[1].revents & POLLOUT) != 0) {
            bool was_request = client->ready && client->request_line;
            bool was_cancel = client->ready && client->cancel_line;
            size_t slot = client->slot;
            if (!flush_line(broker, client, now)) return false;
            if (was_request && !client->ready) {
                if (slot >= MCP_PENDING_MAX || !pending->slots[slot].active) return false;
                pending->slots[slot].forwarded = true;
                if (pending->outstanding++ == 0U) pending_deadline = io_deadline(now);
            }
            if (was_cancel && !client->ready) {
                if (client->target_slot < MCP_PENDING_MAX &&
                    pending->slots[client->target_slot].active &&
                    pending->slots[client->target_slot].forwarded) {
                    pending_remove(pending, client->target_slot);
                    if (pending->outstanding == 0U) pending_deadline = 0;
                }
            }
            if (!client->ready) clear_line_metadata(client);
        }
        if (can_read(output) &&
            (descriptors[1].revents & (POLLIN | POLLHUP | POLLERR)) != 0) {
            bool received_bytes = false;
            if (!read_chunk(broker, output, now, &received_bytes)) return false;
            if (received_bytes && pending->outstanding > 0U)
                pending_deadline = io_deadline(now);
        }
        if (can_read(client) && !error_pending &&
            (descriptors[0].revents & (POLLIN | POLLHUP | POLLERR)) != 0) {
            bool received_bytes = false;
            if (!read_chunk(STDIN_FILENO, client, now, &received_bytes)) return false;
        }
    }
}

static bool relay(int broker, relay_stream *client, relay_stream *output)
{
    pending_set pending = {0};
    pending.used_bytes = sizeof(pending);
    bool okay = relay_loop(broker, client, output, &pending);
    pending_destroy(&pending);
    identity_destroy(&client->identity);
    identity_destroy(&output->identity);
    return okay;
}

int main(void)
{
    int broker = connect_broker();
    if (broker < 0) return EXIT_FAILURE;
    relay_stream client = {0};
    relay_stream output = {0};
    client.line = malloc(MCP_JSON_MAX_BYTES + 1U);
    output.line = malloc(MCP_JSON_MAX_BYTES + 1U);
    int stdin_flags = -1;
    int stdout_flags = -1;
    int broker_flags = -1;
    struct sigaction action = {0};
    action.sa_handler = SIG_IGN;
    (void)sigemptyset(&action.sa_mask);
    bool okay = client.line != NULL && output.line != NULL &&
        sigaction(SIGPIPE, &action, NULL) == 0 &&
        set_nonblocking(STDIN_FILENO, &stdin_flags) &&
        set_nonblocking(STDOUT_FILENO, &stdout_flags) &&
        set_nonblocking(broker, &broker_flags) && relay(broker, &client, &output);
    if (!okay) (void)fputs("vexel-mcp: bounded stdio/socket relay failed\n", stderr);
    if (stdin_flags >= 0) (void)fcntl(STDIN_FILENO, F_SETFL, stdin_flags);
    if (stdout_flags >= 0) (void)fcntl(STDOUT_FILENO, F_SETFL, stdout_flags);
    free(client.line);
    free(output.line);
    (void)close(broker);
    return okay ? EXIT_SUCCESS : EXIT_FAILURE;
}
