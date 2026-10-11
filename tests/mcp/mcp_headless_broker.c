#define _DARWIN_C_SOURCE
#define _POSIX_C_SOURCE 200809L

#include "mcp/mcp_broker.h"
#include "mcp/mcp_json.h"
#include "mcp/mcp_protocol.h"
#include "mcp/mcp_socket.h"

#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <unistd.h>

typedef struct headless_document {
    char path[MCP_BROKER_PATH_CAP];
    document_edit *edit;
    document_handle selection;
} headless_document;

static document_edit *current_edit(void *context)
{
    return ((headless_document *)context)->edit;
}

static const char *current_path(void *context)
{
    return ((headless_document *)context)->path;
}

static bool current_selection(void *context, document_handle *out)
{
    if (context == NULL || out == NULL) return false;
    *out = ((headless_document *)context)->selection;
    return true;
}

static bool replace_edit(void *context, document_edit *expected,
                         document_edit *replacement, document_handle selection)
{
    headless_document *state = context;
    if (state == NULL || state->edit != expected || replacement == NULL) return false;
    document_edit_destroy(expected);
    state->edit = replacement;
    state->selection = selection;
    return true;
}

static bool destroy_edit(void *context, document_edit *edit)
{
    headless_document *state = context;
    if (state == NULL || state->edit != edit) return false;
    state->edit = NULL;
    document_edit_destroy(edit);
    return true;
}

static bool set_path(void *context, const char *path)
{
    headless_document *state = context;
    if (state == NULL || path == NULL) return false;
    size_t length = strnlen(path, sizeof(state->path));
    if (length == 0U || length >= sizeof(state->path)) return false;
    memcpy(state->path, path, length + 1U);
    return true;
}

static bool read_source(const char *path, uint8_t **out, size_t *out_length)
{
    int descriptor = open(path, O_RDONLY | O_NOFOLLOW);
    if (descriptor < 0) return false;
    struct stat status;
    if (fstat(descriptor, &status) != 0 || !S_ISREG(status.st_mode) || status.st_size < 0 ||
        (uint64_t)status.st_size > 64U * 1024U * 1024U) {
        (void)close(descriptor);
        return false;
    }
    size_t length = (size_t)status.st_size;
    uint8_t *source = malloc(length == 0U ? 1U : length);
    if (source == NULL) { (void)close(descriptor); return false; }
    size_t cursor = 0U;
    while (cursor < length) {
        ssize_t amount = read(descriptor, source + cursor, length - cursor);
        if (amount < 0 && errno == EINTR) continue;
        if (amount <= 0) { free(source); (void)close(descriptor); return false; }
        cursor += (size_t)amount;
    }
    if (close(descriptor) != 0) { free(source); return false; }
    *out = source;
    *out_length = length;
    return true;
}

static bool send_all(int descriptor, const uint8_t *bytes, size_t length)
{
    size_t cursor = 0U;
    while (cursor < length) {
        ssize_t amount = send(descriptor, bytes + cursor, length - cursor, 0);
        if (amount < 0 && errno == EINTR) continue;
        if (amount <= 0) return false;
        cursor += (size_t)amount;
    }
    return true;
}

static bool serve_client(int descriptor, mcp_broker *broker)
{
    uint8_t *input = malloc(MCP_JSON_MAX_BYTES + 1U);
    if (input == NULL) return false;
    size_t length = 0U;
    mcp_protocol_session session = {0};
    mcp_protocol_backend backend = mcp_broker_protocol_backend(broker);
    bool okay = true;
    for (;;) {
        ssize_t amount = recv(descriptor, input + length, MCP_JSON_MAX_BYTES + 1U - length, 0);
        if (amount < 0 && errno == EINTR) continue;
        if (amount <= 0) break;
        length += (size_t)amount;
        size_t consumed = 0U;
        while (consumed < length) {
            uint8_t *newline = memchr(input + consumed, '\n', length - consumed);
            if (newline == NULL) break;
            size_t line_length = (size_t)(newline - (input + consumed));
            if (line_length > 0U && input[consumed + line_length - 1U] == '\r') line_length--;
            mcp_json_buffer response = {0};
            if (!mcp_json_buffer_init(&response, 256U, MCP_JSON_MAX_BYTES) ||
                !mcp_protocol_handle(&session, input + consumed, line_length, &backend, &response) ||
                (response.length > 0U &&
                 (!send_all(descriptor, (const uint8_t *)response.data, response.length) ||
                  !send_all(descriptor, (const uint8_t *)"\n", 1U)))) okay = false;
            mcp_json_buffer_destroy(&response);
            if (!okay) break;
            consumed = (size_t)(newline - input) + 1U;
        }
        if (!okay) break;
        if (consumed > 0U) {
            length -= consumed;
            memmove(input, input + consumed, length);
        }
        if (length > MCP_JSON_MAX_BYTES) { okay = false; break; }
    }
    free(input);
    return okay;
}

int main(int argc, char **argv)
{
    if (argc != 2) return EXIT_FAILURE;
    char path[MCP_BROKER_PATH_CAP];
    size_t path_length = strnlen(argv[1], sizeof(path));
    if (path_length == 0U || path_length >= sizeof(path)) return EXIT_FAILURE;
    headless_document state = {0};
    memcpy(state.path, argv[1], path_length + 1U);
    uint8_t *source = NULL;
    size_t source_length = 0U;
    if (!read_source(state.path, &source, &source_length)) {
        (void)fprintf(stderr, "headless broker: cannot read fixture %s\n", state.path);
        return EXIT_FAILURE;
    }
    document_edit_result edit = document_edit_create((base_bytes){source, source_length},
                                                       document_edit_default_limits());
    if (edit.error != DOCUMENT_EDIT_OK || edit.edit == NULL) {
        (void)fprintf(stderr, "headless broker: fixture parse failed: %s\n", document_edit_error_name(edit.error));
        free(source);
        return EXIT_FAILURE;
    }
    state.edit = edit.edit;
    (void)document_edit_root(state.edit, &state.selection);
    mcp_broker_host host = {0};
    host.context = &state;
    host.current_edit = current_edit;
    host.current_path = current_path;
    host.current_selection = current_selection;
    host.replace_current_edit = replace_edit;
    host.destroy_current_edit = destroy_edit;
    host.set_current_path = set_path;
    mcp_broker *broker = NULL;
    if (!mcp_broker_create(&host, &broker)) {
        (void)fputs("headless broker: cannot create broker state\n", stderr);
        document_edit_destroy(state.edit);
        free(source);
        return EXIT_FAILURE;
    }
    mcp_broker_bind_current(broker);
    char socket_path[sizeof(((struct sockaddr_un *)0)->sun_path)];
    if (!mcp_socket_path(socket_path, sizeof(socket_path))) {
        (void)fputs("headless broker: invalid socket path\n", stderr);
        mcp_broker_destroy(broker);
        document_edit_destroy(state.edit);
        free(source);
        return EXIT_FAILURE;
    }
    struct sockaddr_un address = {0};
    size_t socket_length = strlen(socket_path);
    address.sun_family = AF_UNIX;
    address.sun_len = (uint8_t)(offsetof(struct sockaddr_un, sun_path) + socket_length + 1U);
    memcpy(address.sun_path, socket_path, socket_length + 1U);
    int listener = socket(AF_UNIX, SOCK_STREAM, 0);
    if (listener < 0) {
        (void)perror("headless broker: socket failed");
        mcp_broker_destroy(broker); free(source); return EXIT_FAILURE;
    }
    if (bind(listener, (struct sockaddr *)&address, sizeof(address)) != 0) {
        (void)perror("headless broker: bind failed");
        (void)close(listener); mcp_broker_destroy(broker); free(source); return EXIT_FAILURE;
    }
    if (chmod(socket_path, S_IRUSR | S_IWUSR) != 0) {
        (void)perror("headless broker: socket chmod failed");
        (void)close(listener); (void)unlink(socket_path); mcp_broker_destroy(broker); free(source); return EXIT_FAILURE;
    }
    if (listen(listener, 4) != 0) {
        (void)perror("headless broker: listen failed");
        if (listener >= 0) (void)close(listener);
        (void)unlink(socket_path);
        mcp_broker_destroy(broker);
        document_edit_destroy(state.edit);
        free(source);
        return EXIT_FAILURE;
    }
    for (;;) {
        int client = accept(listener, NULL, NULL);
        if (client < 0) {
            if (errno == EINTR) continue;
            break;
        }
        uid_t uid;
        gid_t gid;
        if (getpeereid(client, &uid, &gid) == 0 && uid == getuid()) (void)serve_client(client, broker);
        (void)close(client);
    }
    (void)close(listener);
    (void)unlink(socket_path);
    mcp_broker_destroy(broker);
    if (state.edit != NULL) document_edit_destroy(state.edit);
    free(source);
    return EXIT_SUCCESS;
}
