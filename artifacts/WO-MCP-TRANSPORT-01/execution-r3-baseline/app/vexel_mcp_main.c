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

static bool set_nonblocking(int descriptor, int *old_flags)
{
    *old_flags = fcntl(descriptor, F_GETFL);
    return *old_flags >= 0 && fcntl(descriptor, F_SETFL, *old_flags | O_NONBLOCK) == 0;
}

static void complete_line(relay_stream *stream, bool client, bool newline, int64_t now)
{
    if (client && newline && stream->length > 0U &&
        stream->line[stream->length - 1U] == '\r') stream->length--;
    stream->line[stream->length++] = '\n';
    stream->ready = true;
    stream->deadline = io_deadline(now);
}

/* Returns false only for an overlong/truncated broker line. Stdio overflow
 * discards through its delimiter and schedules exactly one local parse error. */
static bool consume_chunk(relay_stream *stream, bool client, bool *error_pending,
                          int64_t now)
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
            complete_line(stream, client, true, now);
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
            complete_line(stream, true, false, now);
        }
    }
    return true;
}

static bool read_chunk(int descriptor, relay_stream *stream, int64_t now)
{
    ssize_t amount = read(descriptor, stream->scratch, sizeof(stream->scratch));
    if (amount < 0) return errno == EINTR || errno == EAGAIN || errno == EWOULDBLOCK;
    stream->cursor = 0U;
    stream->count = (size_t)amount;
    stream->eof = amount == 0;
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

static bool relay(int broker, relay_stream *client, relay_stream *output)
{
    bool error_pending = false;
    size_t error_written = 0U;
    int64_t error_deadline = 0;
    int64_t drain_deadline = 0;
    bool half_closed = false;
    for (;;) {
        int64_t now = now_ms();
        if (now < 0) return false;
        if (!consume_chunk(output, false, &error_pending, now)) return false;
        bool had_error = error_pending;
        if (!error_pending && !consume_chunk(client, true, &error_pending, now)) return false;
        if (!had_error && error_pending) error_deadline = io_deadline(now);
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
            (half_closed && !poll_timeout(drain_deadline, now, &timeout))) return false;
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
            } else if (!flush_line(STDOUT_FILENO, output, now)) return false;
        } else if ((descriptors[2].revents & (POLLERR | POLLHUP)) != 0) return false;
        if (client->ready && (descriptors[1].revents & (POLLERR | POLLHUP)) != 0 &&
            (descriptors[1].revents & POLLOUT) == 0) return false;
        if ((descriptors[1].revents & POLLOUT) != 0 && !flush_line(broker, client, now)) return false;
        if (can_read(output) &&
            (descriptors[1].revents & (POLLIN | POLLHUP | POLLERR)) != 0) {
            if (!read_chunk(broker, output, now)) return false;
        }
        if (can_read(client) && !error_pending &&
            (descriptors[0].revents & (POLLIN | POLLHUP | POLLERR)) != 0 &&
            !read_chunk(STDIN_FILENO, client, now)) return false;
    }
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
