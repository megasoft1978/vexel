#define _DARWIN_C_SOURCE
#define _POSIX_C_SOURCE 200809L

#include "mcp/mcp_json.h"
#include "mcp/mcp_protocol.h"
#include "mcp/mcp_socket.h"

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <sys/un.h>
#include <unistd.h>

#define MCP_IO_TIMEOUT_SECONDS 30
#define MCP_IO_CHUNK 8192U

static bool write_all(int descriptor, const uint8_t *bytes, size_t length)
{
    size_t cursor = 0U;
    while (cursor < length) {
        ssize_t amount = write(descriptor, bytes + cursor, length - cursor);
        if (amount < 0 && errno == EINTR) continue;
        if (amount <= 0) return false;
        cursor += (size_t)amount;
    }
    return true;
}

static bool socket_send_line(int descriptor, const uint8_t *line, size_t length)
{
    return write_all(descriptor, line, length) && write_all(descriptor, (const uint8_t *)"\n", 1U);
}

static bool stdout_line(const uint8_t *line, size_t length)
{
    return write_all(STDOUT_FILENO, line, length) && write_all(STDOUT_FILENO, (const uint8_t *)"\n", 1U);
}

static bool send_parse_error(void)
{
    static const uint8_t error[] =
        "{\"jsonrpc\":\"2.0\",\"id\":null,\"error\":{\"code\":-32700,\"message\":\"message exceeds the 2 MiB limit\"}}";
    return stdout_line(error, sizeof(error) - 1U);
}

static bool read_response_line(int descriptor, mcp_json_buffer *response)
{
    uint8_t scratch[MCP_IO_CHUNK];
    for (;;) {
        ssize_t amount = recv(descriptor, scratch, sizeof(scratch), 0);
        if (amount < 0 && errno == EINTR) continue;
        if (amount <= 0) return false;
        size_t count = (size_t)amount;
        uint8_t *newline = memchr(scratch, '\n', count);
        size_t line_length = newline == NULL ? count : (size_t)(newline - scratch);
        if (!mcp_json_buffer_append(response, (const char *)scratch, line_length)) return false;
        if (newline != NULL) return true;
        if (response->length >= MCP_JSON_MAX_BYTES) return false;
    }
}

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

static bool relay_line(int descriptor, const uint8_t *line, size_t length)
{
    bool expects_response = mcp_protocol_request_expects_response(line, length);
    if (!socket_send_line(descriptor, line, length)) return false;
    if (!expects_response) return true;
    mcp_json_buffer response = {0};
    if (!mcp_json_buffer_init(&response, 256U, MCP_JSON_MAX_BYTES)) return false;
    bool okay = read_response_line(descriptor, &response) &&
        stdout_line((const uint8_t *)response.data, response.length);
    mcp_json_buffer_destroy(&response);
    return okay;
}

int main(void)
{
    int broker = connect_broker();
    if (broker < 0) return EXIT_FAILURE;
    uint8_t *line = malloc(MCP_JSON_MAX_BYTES + 1U);
    if (line == NULL) { (void)close(broker); return EXIT_FAILURE; }
    size_t length = 0U;
    bool oversized = false;
    int result = EXIT_SUCCESS;
    for (;;) {
        int byte = fgetc(stdin);
        if (byte == EOF) {
            if (ferror(stdin)) result = EXIT_FAILURE;
            if (oversized) { if (!send_parse_error()) result = EXIT_FAILURE; }
            else if (length > 0U && !relay_line(broker, line, length)) result = EXIT_FAILURE;
            break;
        }
        if (byte == '\n') {
            if (oversized) {
                if (!send_parse_error()) { result = EXIT_FAILURE; break; }
            } else {
                if (length > 0U && line[length - 1U] == '\r') length--;
                if (length > 0U && !relay_line(broker, line, length)) { result = EXIT_FAILURE; break; }
            }
            length = 0U;
            oversized = false;
            continue;
        }
        if (!oversized) {
            if (length >= MCP_JSON_MAX_BYTES) oversized = true;
            else line[length++] = (uint8_t)byte;
        }
    }
    free(line);
    (void)shutdown(broker, SHUT_RDWR);
    (void)close(broker);
    return result;
}
