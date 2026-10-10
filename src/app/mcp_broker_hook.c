#define _DARWIN_C_SOURCE
#define _POSIX_C_SOURCE 200809L

#include "mcp_broker_hook.h"

#include "viewer_editor_internal.h"
#include "viewer_internal.h"
#include "mcp/mcp_broker.h"
#include "mcp/mcp_json.h"
#include "mcp/mcp_protocol.h"
#include "mcp/mcp_socket.h"

#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <unistd.h>

#define MCP_HOOK_CLIENTS 8U
#define MCP_HOOK_BACKPRESSURE_MS 2000

typedef struct mcp_hook_client {
    int descriptor;
    uint8_t *input;
    size_t length;
    mcp_protocol_session protocol;
} mcp_hook_client;

typedef struct mcp_hook_state {
    viewer_app *app;
    int listener;
    bool owns_socket;
    bool start_attempted;
    bool reported_error;
    char socket_path[sizeof(((struct sockaddr_un *)0)->sun_path)];
    mcp_broker *broker;
    mcp_hook_client clients[MCP_HOOK_CLIENTS];
} mcp_hook_state;

static mcp_hook_state hook = {.listener = -1};

static document_edit *host_current_edit(void *context)
{
    viewer_app *app = context;
    return app == NULL || app->editor == NULL ? NULL : app->editor->edit;
}

static const char *host_current_path(void *context)
{
    viewer_app *app = context;
    return app == NULL || app->editor == NULL || app->load_status != VIEWER_LOAD_OK ? "" : app->path;
}

static bool host_current_selection(void *context, document_handle *out)
{
    viewer_app *app = context;
    if (app == NULL || app->editor == NULL || out == NULL) return false;
    *out = app->editor->selected;
    return true;
}

static bool host_replace_edit(void *context, document_edit *expected,
                              document_edit *replacement, document_handle selection)
{
    viewer_app *app = context;
    if (app == NULL || app->editor == NULL || app->editor->edit != expected || replacement == NULL ||
        !viewer_bump_generation(app)) return false;
    app->editor->edit = replacement;
    document_edit_destroy(expected);
    editor_select_handle(app->editor, selection);
    editor_refresh_outline(app->editor);
    viewer_editor_set_status(app->editor, "Agent edited document — transaction committed");
    app->editor_render_requested = true;
    app->frame_dirty = true;
    (void)viewer_set_window_title(app);
    return true;
}

static bool host_destroy_edit(void *context, document_edit *edit)
{
    viewer_app *app = context;
    if (app == NULL || app->editor == NULL || app->editor->edit != edit) return false;
    app->editor->edit = NULL;
    document_edit_destroy(edit);
    return true;
}

static bool host_set_path(void *context, const char *path)
{
    viewer_app *app = context;
    if (app == NULL || path == NULL || app->editor == NULL) return false;
    size_t length = strnlen(path, sizeof(app->path));
    if (length == 0U || length >= sizeof(app->path)) return false;
    memcpy(app->path, path, length + 1U);
    const char *base = path;
    for (const char *cursor = path; *cursor != '\0'; cursor++)
        if (*cursor == '/') base = cursor + 1;
    size_t base_length = strnlen(base, sizeof(app->display_name) - 1U);
    for (size_t index = 0U; index < base_length; index++) {
        unsigned char byte = (unsigned char)base[index];
        app->display_name[index] = byte >= 32U && byte < 127U ? (char)byte : '?';
    }
    app->display_name[base_length] = '\0';
    viewer_update_file_stamp(app);
    app->frame_dirty = true;
    return viewer_set_window_title(app);
}

static void host_agent_edited(void *context, const char *path)
{
    viewer_app *app = context;
    if (app == NULL || app->editor == NULL) return;
    const char *base = path == NULL ? "document" : path;
    if (path != NULL) {
        for (const char *cursor = path; *cursor != '\0'; cursor++)
            if (*cursor == '/') base = cursor + 1;
    }
    char status[192];
    int amount = snprintf(status, sizeof(status), "Agent edited — %.120s", base);
    if (amount > 0 && (size_t)amount < sizeof(status)) viewer_editor_set_status(app->editor, status);
    if (path != NULL && strcmp(path, app->path) == 0) {
        if (viewer_bump_generation(app)) app->editor_render_requested = true;
    }
    app->frame_dirty = true;
}

static void host_selection_changed(void *context, document_handle selection)
{
    viewer_app *app = context;
    if (app == NULL || app->editor == NULL) return;
    editor_select_handle(app->editor, selection);
    editor_refresh_outline(app->editor);
    app->frame_dirty = true;
}

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

static bool ensure_hook(viewer_app *app)
{
    if (hook.app == app && hook.broker != NULL) return true;
    if (hook.app == app && hook.start_attempted) return false;
    if (hook.app != NULL) mcp_broker_hook_shutdown(hook.app);
    hook = (mcp_hook_state){.listener = -1, .app = app, .start_attempted = true};
    for (size_t index = 0U; index < MCP_HOOK_CLIENTS; index++) hook.clients[index].descriptor = -1;
    mcp_broker_host host = {0};
    host.context = app;
    host.current_edit = host_current_edit;
    host.current_path = host_current_path;
    host.current_selection = host_current_selection;
    host.replace_current_edit = host_replace_edit;
    host.destroy_current_edit = host_destroy_edit;
    host.set_current_path = host_set_path;
    host.agent_edited = host_agent_edited;
    host.selection_changed = host_selection_changed;
    if (!mcp_broker_create(&host, &hook.broker)) return false;
    if (!start_listener(&hook)) {
        mcp_broker_destroy(hook.broker);
        hook.broker = NULL;
        return false;
    }
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

static bool process_buffer(mcp_hook_client *client)
{
    size_t consumed = 0U;
    while (consumed < client->length) {
        uint8_t *newline = memchr(client->input + consumed, '\n', client->length - consumed);
        if (newline == NULL) break;
        size_t line_length = (size_t)(newline - (client->input + consumed));
        if (line_length > 0U && client->input[consumed + line_length - 1U] == '\r') line_length--;
        if (line_length > MCP_JSON_MAX_BYTES ||
            !process_line(client, client->input + consumed, line_length)) return false;
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
        state->clients[index] = (mcp_hook_client){.descriptor = descriptor, .input = buffer};
        mcp_protocol_session_reset(&state->clients[index].protocol);
        int flags = fcntl(descriptor, F_GETFL, 0);
        if (flags < 0 || fcntl(descriptor, F_SETFL, flags | O_NONBLOCK) != 0) close_client(&state->clients[index]);
    }
}

static void service_client(mcp_hook_client *client)
{
    if (client->descriptor < 0 || client->input == NULL) return;
    for (;;) {
        ssize_t amount = recv(client->descriptor, client->input + client->length,
                              MCP_JSON_MAX_BYTES + 1U - client->length, 0);
        if (amount < 0 && errno == EINTR) continue;
        if (amount < 0 && (errno == EAGAIN || errno == EWOULDBLOCK)) break;
        if (amount <= 0) { close_client(client); return; }
        client->length += (size_t)amount;
        if (!process_buffer(client)) { close_client(client); return; }
    }
}

void mcp_broker_hook_poll(viewer_app *app)
{
    if (app == NULL || app->smoke_mode || !ensure_hook(app)) return;
    mcp_broker_bind_current(hook.broker);
    accept_clients(&hook);
    for (size_t index = 0U; index < MCP_HOOK_CLIENTS; index++) service_client(&hook.clients[index]);
}

void mcp_broker_hook_shutdown(viewer_app *app)
{
    if (hook.app == NULL || (app != NULL && hook.app != app)) return;
    for (size_t index = 0U; index < MCP_HOOK_CLIENTS; index++) close_client(&hook.clients[index]);
    if (hook.listener >= 0) (void)close(hook.listener);
    if (hook.owns_socket) (void)unlink(hook.socket_path);
    mcp_broker_destroy(hook.broker);
    hook = (mcp_hook_state){.listener = -1};
}
