#define _DARWIN_C_SOURCE
#define _POSIX_C_SOURCE 200809L

#include "mcp_broker_hook.h"

#include "mcp/mcp_broker.h"
#include "mcp/mcp_json.h"
#include "mcp/mcp_owner_queue.h"
#include "mcp/mcp_protocol.h"
#include "mcp/mcp_socket.h"
#include "viewer_mcp_host.h"
#include "viewer_sessions.h"

#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <unistd.h>

#define MCP_HOOK_CLIENTS 8U
#define MCP_HOOK_BACKPRESSURE_MS 2000
#define MCP_HOOK_PENDING_TIMEOUT_MS UINT64_C(2000)

typedef struct mcp_hook_client {
    int descriptor;
    uint8_t *input;
    size_t length;
    uint64_t generation;
    mcp_protocol_session protocol;
} mcp_hook_client;

typedef struct mcp_hook_state {
    viewer_session_collection *sessions;
    int listener;
    bool owns_socket;
    bool start_attempted;
    bool reported_error;
    bool connected;
    uint64_t next_client_generation;
    char socket_path[sizeof(((struct sockaddr_un *)0)->sun_path)];
    mcp_broker *broker;
    mcp_hook_client clients[MCP_HOOK_CLIENTS];
    mcp_owner_queue pending;
} mcp_hook_state;

static mcp_hook_state hook = {.listener = -1};

static bool parent_directory_is_private(const char *socket_path)
{
    char parent[sizeof(((struct sockaddr_un *)0)->sun_path)];
    size_t length = strnlen(socket_path, sizeof(parent));
    if (length == 0U || length >= sizeof(parent)) return false;
    memcpy(parent, socket_path, length + 1U);
    char *slash = strrchr(parent, '/');
    if (slash == NULL || slash == parent) return false;
    *slash = '\0';
    struct stat status;
    return stat(parent, &status) == 0 && S_ISDIR(status.st_mode) && status.st_uid == getuid() &&
        (status.st_mode & (S_IWGRP | S_IWOTH)) == 0;
}

static int connect_existing_socket(const char *path)
{
    struct sockaddr_un address = {0};
    size_t length = strlen(path);
    address.sun_family = AF_UNIX;
    address.sun_len = (uint8_t)(offsetof(struct sockaddr_un, sun_path) + length + 1U);
    if (length >= sizeof(address.sun_path)) return -1;
    memcpy(address.sun_path, path, length + 1U);
    int descriptor = socket(AF_UNIX, SOCK_STREAM, 0);
    if (descriptor < 0) return -1;
    if (connect(descriptor, (struct sockaddr *)&address, sizeof(address)) != 0) {
        (void)close(descriptor);
        return -1;
    }
    (void)close(descriptor);
    return 0;
}

static bool start_listener(mcp_hook_state *state)
{
    if (!mcp_socket_path(state->socket_path, sizeof(state->socket_path)) ||
        !parent_directory_is_private(state->socket_path)) return false;
    struct stat existing;
    if (lstat(state->socket_path, &existing) == 0) {
        if (!S_ISSOCK(existing.st_mode) || existing.st_uid != getuid() ||
            (existing.st_mode & (S_IRWXG | S_IRWXO)) != 0) return false;
        if (connect_existing_socket(state->socket_path) == 0) return false;
        if (unlink(state->socket_path) != 0) return false;
    }
    int descriptor = socket(AF_UNIX, SOCK_STREAM, 0);
    if (descriptor < 0) return false;
    struct sockaddr_un address = {0};
    size_t length = strlen(state->socket_path);
    address.sun_family = AF_UNIX;
    address.sun_len = (uint8_t)(offsetof(struct sockaddr_un, sun_path) + length + 1U);
    memcpy(address.sun_path, state->socket_path, length + 1U);
    if (bind(descriptor, (struct sockaddr *)&address, sizeof(address)) != 0 ||
        chmod(state->socket_path, S_IRUSR | S_IWUSR) != 0 || listen(descriptor, (int)MCP_HOOK_CLIENTS) != 0) {
        (void)close(descriptor);
        (void)unlink(state->socket_path);
        return false;
    }
    int flags = fcntl(descriptor, F_GETFL, 0);
    if (flags < 0 || fcntl(descriptor, F_SETFL, flags | O_NONBLOCK) != 0) {
        (void)close(descriptor);
        (void)unlink(state->socket_path);
        return false;
    }
    state->listener = descriptor;
    state->owns_socket = true;
    return true;
}

static bool ensure_hook(viewer_session_collection *sessions)
{
    if (hook.sessions == sessions && hook.broker != NULL) return true;
    if (hook.sessions == sessions && hook.start_attempted) return false;
    if (hook.sessions != NULL) mcp_broker_hook_shutdown(NULL);
    hook = (mcp_hook_state){.listener = -1, .sessions = sessions,
        .start_attempted = true};
    for (size_t index = 0U; index < MCP_HOOK_CLIENTS; index++) hook.clients[index].descriptor = -1;
    mcp_broker_host host = {0};
    host.context = sessions;
    host.live_document_count = viewer_mcp_live_document_count;
    host.live_document_at = viewer_mcp_live_document_at;
    host.live_control_enabled = viewer_mcp_live_control_enabled;
    host.current_selection_for_document = viewer_mcp_current_selection;
    host.selection_changed_for_document = viewer_mcp_selection_changed;
    host.commit_mutation = viewer_mcp_commit_mutation;
    host.set_document_path = viewer_mcp_set_document_path;
    host.owner_is_busy = viewer_mcp_sessions_are_busy;
    if (!mcp_broker_create(&host, &hook.broker)) return false;
    if (!start_listener(&hook)) {
        mcp_broker_destroy(hook.broker);
        hook.broker = NULL;
        return false;
    }
    viewer_mcp_set_broker(sessions, hook.broker);
    for (size_t index = 0U; index < MCP_HOOK_CLIENTS; index++) {
        hook.clients[index].descriptor = -1;
        mcp_protocol_session_reset(&hook.clients[index].protocol);
    }
    return true;
}

static void close_client(mcp_hook_client *client)
{
    if (client == NULL) return;
    if (client->descriptor >= 0) (void)close(client->descriptor);
    free(client->input);
    *client = (mcp_hook_client){.descriptor = -1};
}

static bool send_all(int descriptor, const uint8_t *bytes, size_t length)
{
    size_t cursor = 0U;
    while (cursor < length) {
        ssize_t amount = send(descriptor, bytes + cursor, length - cursor, 0);
        if (amount < 0 && (errno == EAGAIN || errno == EWOULDBLOCK || errno == EINTR)) {
            struct pollfd descriptor_state = {descriptor, POLLOUT, 0};
            int status = poll(&descriptor_state, 1U, MCP_HOOK_BACKPRESSURE_MS);
            if (status > 0) continue;
            return false;
        }
        if (amount <= 0) return false;
        cursor += (size_t)amount;
    }
    return true;
}

static bool send_json_line(int descriptor, mcp_json_buffer *response)
{
    if (response->length == 0U) return true;
    return send_all(descriptor, (const uint8_t *)response->data, response->length) &&
        send_all(descriptor, (const uint8_t *)"\n", 1U);
}

static uint64_t hook_monotonic_ms(void)
{
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now) != 0) return 0U;
    return (uint64_t)now.tv_sec * UINT64_C(1000) +
        (uint64_t)now.tv_nsec / UINT64_C(1000000);
}

static bool send_busy_error(mcp_hook_client *client, const uint8_t *line,
                            size_t length, const char *message)
{
    if (client == NULL || client->descriptor < 0 ||
        !mcp_protocol_request_expects_response(line, length)) return true;
    if (message == NULL) return false;
    mcp_json_doc request = {0};
    bool parsed = mcp_json_parse(line, length, &request);
    size_t id = parsed ? mcp_json_object_get(&request, request.root, "id") :
        MCP_JSON_NO_TOKEN;
    mcp_json_buffer response = {0};
    bool okay = mcp_json_buffer_init(&response, 192U, MCP_JSON_MAX_BYTES) &&
        mcp_json_buffer_literal(&response, "{\"jsonrpc\":\"2.0\",\"id\":" ) &&
        (id == MCP_JSON_NO_TOKEN ? mcp_json_buffer_literal(&response, "null") :
            mcp_json_buffer_raw_token(&response, &request, id)) &&
        mcp_json_buffer_literal(&response, ",\"error\":{\"code\":-32001,\"message\":" ) &&
        mcp_json_buffer_quoted(&response, message, strlen(message)) &&
        mcp_json_buffer_literal(&response, "}}" ) &&
        send_json_line(client->descriptor, &response);
    mcp_json_buffer_destroy(&response);
    mcp_json_doc_destroy(&request);
    return okay;
}

static bool process_line(mcp_hook_client *client, const uint8_t *line, size_t length)
{
    mcp_json_buffer response = {0};
    if (!mcp_json_buffer_init(&response, 256U, MCP_JSON_MAX_BYTES)) return false;
    mcp_protocol_backend backend = mcp_broker_protocol_backend(hook.broker);
    bool okay = mcp_protocol_handle(&client->protocol, line, length, &backend, &response) &&
        send_json_line(client->descriptor, &response);
    mcp_json_buffer_destroy(&response);
    return okay;
}

static bool owner_is_busy(const mcp_hook_state *state)
{
    return state != NULL && viewer_mcp_live_control_enabled(NULL) &&
        viewer_mcp_sessions_are_busy(state->sessions);
}

static bool queue_pending(mcp_hook_state *state, size_t client_index,
                          const uint8_t *line, size_t length)
{
    mcp_hook_client *client = &state->clients[client_index];
    mcp_owner_queue_result queued = mcp_owner_queue_enqueue(&state->pending,
        client_index, client->generation, line, length, hook_monotonic_ms(),
        MCP_HOOK_PENDING_TIMEOUT_MS);
    if (queued == MCP_OWNER_QUEUE_ENQUEUED) return true;
    const char *reason = queued == MCP_OWNER_QUEUE_FULL ?
        "busy_retry: bounded owner-thread queue is full; retry after the editor interaction completes" :
        "busy_retry: owner-thread queue could not accept this request; retry shortly";
    return queued != MCP_OWNER_QUEUE_INVALID &&
        send_busy_error(client, line, length, reason);
}

static bool process_buffer(mcp_hook_state *state, size_t client_index)
{
    mcp_hook_client *client = &state->clients[client_index];
    size_t consumed = 0U;
    while (consumed < client->length) {
        uint8_t *newline = memchr(client->input + consumed, '\n', client->length - consumed);
        if (newline == NULL) break;
        size_t line_length = (size_t)(newline - (client->input + consumed));
        if (line_length > 0U && client->input[consumed + line_length - 1U] == '\r') line_length--;
        if (line_length > MCP_JSON_MAX_BYTES) return false;
        const uint8_t *line = client->input + consumed;
        if (owner_is_busy(state)) {
            mcp_json_doc parsed = {0};
            bool valid_json = mcp_json_parse(line, line_length, &parsed);
            mcp_json_doc_destroy(&parsed);
            if (valid_json) {
                if (!queue_pending(state, client_index, line, line_length)) return false;
            } else if (!process_line(client, line, line_length)) return false;
        } else if (!process_line(client, line, line_length)) return false;
        consumed = (size_t)(newline - client->input) + 1U;
    }
    if (consumed > 0U) {
        client->length -= consumed;
        memmove(client->input, client->input + consumed, client->length);
    }
    return client->length <= MCP_JSON_MAX_BYTES;
}

static void accept_clients(mcp_hook_state *state)
{
    for (size_t accepted = 0U; accepted < MCP_HOOK_CLIENTS; accepted++) {
        int descriptor = accept(state->listener, NULL, NULL);
        if (descriptor < 0) {
            if (errno != EAGAIN && errno != EWOULDBLOCK && errno != EINTR && !state->reported_error) {
                (void)fputs("vexel MCP broker: accept failed\n", stderr);
                state->reported_error = true;
            }
            return;
        }
        uid_t peer_uid;
        gid_t peer_gid;
        if (getpeereid(descriptor, &peer_uid, &peer_gid) != 0 || peer_uid != getuid()) {
            (void)close(descriptor);
            continue;
        }
        size_t index = 0U;
        while (index < MCP_HOOK_CLIENTS && state->clients[index].descriptor >= 0) index++;
        if (index == MCP_HOOK_CLIENTS) { (void)close(descriptor); continue; }
        uint8_t *buffer = malloc(MCP_JSON_MAX_BYTES + 1U);
        if (buffer == NULL) { (void)close(descriptor); continue; }
        uint64_t generation = ++state->next_client_generation;
        if (generation == 0U) generation = ++state->next_client_generation;
        state->clients[index] = (mcp_hook_client){.descriptor = descriptor,
            .input = buffer, .generation = generation};
        mcp_protocol_session_reset(&state->clients[index].protocol);
        int flags = fcntl(descriptor, F_GETFL, 0);
        if (flags < 0 || fcntl(descriptor, F_SETFL, flags | O_NONBLOCK) != 0) close_client(&state->clients[index]);
    }
}

static void service_client(mcp_hook_state *state, size_t index)
{
    mcp_hook_client *client = &state->clients[index];
    if (client->descriptor < 0 || client->input == NULL) return;
    for (;;) {
        ssize_t amount = recv(client->descriptor, client->input + client->length,
                              MCP_JSON_MAX_BYTES + 1U - client->length, 0);
        if (amount < 0 && errno == EINTR) continue;
        if (amount < 0 && (errno == EAGAIN || errno == EWOULDBLOCK)) break;
        if (amount <= 0) { close_client(client); return; }
        client->length += (size_t)amount;
        if (!process_buffer(state, index)) { close_client(client); return; }
    }
}

static void drain_pending(mcp_hook_state *state)
{
    for (;;) {
        bool busy = owner_is_busy(state);
        mcp_owner_queue_item item = {0};
        mcp_owner_queue_take_result result = mcp_owner_queue_take(&state->pending,
            busy, hook_monotonic_ms(), &item);
        if (result == MCP_OWNER_QUEUE_EMPTY || result == MCP_OWNER_QUEUE_WAITING)
            return;
        if (item.owner_index < MCP_HOOK_CLIENTS) {
            mcp_hook_client *client = &state->clients[item.owner_index];
            if (client->descriptor >= 0 &&
                client->generation == item.owner_generation) {
                if (result == MCP_OWNER_QUEUE_EXPIRED)
                    (void)send_busy_error(client, item.bytes, item.length,
                        "busy_retry: editor interaction did not finish within 2000 ms; retry after it completes");
                else if (!process_line(client, item.bytes, item.length))
                    close_client(client);
            }
        }
        mcp_owner_queue_item_destroy(&item);
    }
}

static bool has_connected_client(const mcp_hook_state *state)
{
    for (size_t index = 0U; index < MCP_HOOK_CLIENTS; index++)
        if (state->clients[index].descriptor >= 0) return true;
    return false;
}

void mcp_broker_hook_poll(viewer_session_collection *sessions)
{
    if (sessions == NULL || !ensure_hook(sessions)) return;
    mcp_broker_bind_live_documents(hook.broker);
    viewer_mcp_set_broker(sessions, hook.broker);
    accept_clients(&hook);
    for (size_t index = 0U; index < MCP_HOOK_CLIENTS; index++)
        service_client(&hook, index);
    drain_pending(&hook);
    bool connected = has_connected_client(&hook);
    if (connected != hook.connected ||
        viewer_mcp_connection_needs_sync(sessions, connected)) {
        hook.connected = connected;
        viewer_mcp_connection_changed(sessions, connected);
    }
}

void mcp_broker_hook_shutdown(void *runtime_or_window)
{
    if (runtime_or_window != NULL || hook.sessions == NULL) return;
    for (size_t index = 0U; index < MCP_HOOK_CLIENTS; index++) close_client(&hook.clients[index]);
    mcp_owner_queue_item item = {0};
    while (mcp_owner_queue_take(&hook.pending, false, UINT64_MAX, &item) ==
        MCP_OWNER_QUEUE_READY) mcp_owner_queue_item_destroy(&item);
    if (hook.listener >= 0) (void)close(hook.listener);
    if (hook.owns_socket) (void)unlink(hook.socket_path);
    viewer_mcp_connection_changed(hook.sessions, false);
    viewer_mcp_set_broker(hook.sessions, NULL);
    mcp_broker_destroy(hook.broker);
    hook = (mcp_hook_state){.listener = -1};
}
