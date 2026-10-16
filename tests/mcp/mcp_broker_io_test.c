#define _DARWIN_C_SOURCE
#define _POSIX_C_SOURCE 200809L

#include "app/mcp_broker_io.h"
#include "mcp/mcp_json.h"

#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <time.h>
#include <unistd.h>

#define CHECK(value) do { if (!(value)) { \
    (void)fprintf(stderr, "mcp broker IO check failed at line %d: %s\n", \
        __LINE__, #value); return EXIT_FAILURE; } } while (0)

typedef struct harness {
    mcp_broker_io *pool;
    size_t lines;
    size_t lengths[32];
    uint8_t first_bytes[32];
    size_t disconnects;
    mcp_broker_io_reason last_reason;
    bool swap_on_line;
    int replacement_descriptor;
    mcp_broker_io_owner replacement_owner;
} harness;

static bool on_line(void *context, mcp_broker_io_owner owner,
                    const uint8_t *line, size_t length)
{
    (void)owner;
    harness *state = context;
    if (length > 0U && state->swap_on_line && line[0] == 'X') {
        state->swap_on_line = false;
        if (!mcp_broker_io_close(state->pool, owner,
            MCP_BROKER_IO_REQUESTED)) return false;
        return mcp_broker_io_attach(state->pool, state->replacement_descriptor,
            &state->replacement_owner);
    }
    if (state->lines < 32U) {
        state->lengths[state->lines] = length;
        state->first_bytes[state->lines] = length == 0U ? 0U : line[0];
    }
    ++state->lines;
    return true;
}

static void on_disconnect(void *context, mcp_broker_io_owner owner,
                          mcp_broker_io_reason reason)
{
    (void)owner;
    harness *state = context;
    ++state->disconnects;
    state->last_reason = reason;
}

static bool make_pool(harness *state, mcp_broker_io **pool)
{
    mcp_broker_io_callbacks callbacks = {on_line, on_disconnect, state};
    bool okay = mcp_broker_io_create(&callbacks, pool);
    if (okay) state->pool = *pool;
    return okay;
}

static bool attach_pair(mcp_broker_io *pool, int pair[2],
                        mcp_broker_io_owner *owner)
{
    if (socketpair(AF_UNIX, SOCK_STREAM, 0, pair) != 0) return false;
    return mcp_broker_io_attach(pool, pair[0], owner);
}

static bool set_nonblocking(int descriptor)
{
    int flags = fcntl(descriptor, F_GETFL, 0);
    return flags >= 0 && fcntl(descriptor, F_SETFL, flags | O_NONBLOCK) == 0;
}

static bool send_nonblocking(mcp_broker_io *pool, int descriptor,
                             const uint8_t *bytes, size_t length)
{
    size_t offset = 0U;
    for (size_t rounds = 0U; offset < length && rounds < 10000U; ++rounds) {
        ssize_t amount = send(descriptor, bytes + offset, length - offset, 0);
        if (amount > 0) offset += (size_t)amount;
        else if (amount < 0 && errno != EAGAIN && errno != EWOULDBLOCK &&
                 errno != EINTR) return false;
        (void)mcp_broker_io_poll(pool);
    }
    return offset == length;
}

static size_t drain(int descriptor, uint8_t *out, size_t capacity)
{
    size_t length = 0U;
    while (length < capacity) {
        ssize_t amount = recv(descriptor, out + length, capacity - length, 0);
        if (amount > 0) length += (size_t)amount;
        else if (amount < 0 && errno == EINTR) continue;
        else break;
    }
    return length;
}

int main(void)
{
    harness state = {0};
    mcp_broker_io *pool = NULL;
    CHECK(make_pool(&state, &pool));
    int pair[2];
    mcp_broker_io_owner owner = 0U;
    CHECK(attach_pair(pool, pair, &owner));
    CHECK(owner != 0U && mcp_broker_io_is_connected(pool, owner));
    CHECK(set_nonblocking(pair[1]));
    CHECK(send(pair[1], "{\"a\":", 5U, 0) == 5);
    (void)mcp_broker_io_poll(pool);
    CHECK(state.lines == 0U);
    CHECK(send(pair[1], "1}\r\n{\"b\":2}\n", 12U, 0) == 12);
    mcp_broker_io_counters counters = mcp_broker_io_poll(pool);
    CHECK(state.lines == 1U && state.lengths[0] == 7U && counters.lines == 1U);
    (void)mcp_broker_io_poll(pool);
    CHECK(state.lines == 2U && state.lengths[1] == 7U);

    char buffered[9U * 4U];
    for (size_t index = 0U; index < 9U; ++index)
        memcpy(buffered + index * 4U, "{}\n ", 4U);
    CHECK(send(pair[1], buffered, sizeof(buffered), 0) == (ssize_t)sizeof(buffered));
    counters = mcp_broker_io_poll(pool);
    CHECK(counters.lines == 1U && state.lines == 3U);
    for (size_t index = 0U; index < 7U; ++index) {
        counters = mcp_broker_io_poll(pool);
        CHECK(counters.lines <= MCP_BROKER_IO_LINE_BUDGET);
    }
    CHECK(state.lines == 10U);
    (void)mcp_broker_io_poll(pool);
    CHECK(state.lines == 11U);
    CHECK(counters.read_bytes + counters.written_bytes <=
        MCP_BROKER_IO_POLL_BYTE_BUDGET &&
        counters.per_client_bytes[0] <= MCP_BROKER_IO_CLIENT_BUDGET &&
        counters.syscalls <= MCP_BROKER_IO_SYSCALL_BUDGET);

    int pair2[2];
    mcp_broker_io_owner owner2 = 0U;
    CHECK(attach_pair(pool, pair2, &owner2));
    CHECK(owner2 != owner);
    CHECK(set_nonblocking(pair2[1]));
    CHECK(send(pair[1], "{\"f\":1}\n{\"f\":2}\n", 16U, 0) == 16);
    CHECK(send(pair2[1], "{\"s\":1}\n", 8U, 0) == 8);
    counters = mcp_broker_io_poll(pool);
    CHECK(counters.lines == 2U && state.lines == 13U);

    int large_pair[2];
    mcp_broker_io_owner large_owner = 0U;
    CHECK(attach_pair(pool, large_pair, &large_owner));
    CHECK(set_nonblocking(large_pair[1]));
    size_t oversized_length = MCP_JSON_MAX_BYTES + 2U;
    uint8_t *oversized = malloc(oversized_length);
    CHECK(oversized != NULL);
    memset(oversized, 'x', oversized_length);
    oversized[oversized_length - 1U] = '\n';
    CHECK(send_nonblocking(pool, large_pair[1], oversized, oversized_length));
    free(oversized);
    CHECK(!mcp_broker_io_is_connected(pool, large_owner));
    CHECK(state.last_reason == MCP_BROKER_IO_OVERSIZED);
    (void)close(large_pair[1]);

    const uint8_t key[] = "task-1";
    CHECK(mcp_broker_io_enqueue(pool, owner, (const uint8_t *)"{\"n\":1}", 7U,
        MCP_BROKER_IO_PROGRESS, key, sizeof(key) - 1U) == MCP_BROKER_IO_QUEUED);
    CHECK(mcp_broker_io_enqueue(pool, owner, (const uint8_t *)"{\"n\":2}", 7U,
        MCP_BROKER_IO_PROGRESS, key, sizeof(key) - 1U) == MCP_BROKER_IO_QUEUED);
    CHECK(mcp_broker_io_enqueue(pool, owner, (const uint8_t *)"{\"done\":true}", 13U,
        MCP_BROKER_IO_TERMINAL, NULL, 0U) == MCP_BROKER_IO_QUEUED);
    (void)mcp_broker_io_poll(pool);
    uint8_t output[256];
    size_t output_length = drain(pair[1], output, sizeof(output));
    (void)mcp_broker_io_poll(pool);
    output_length += drain(pair[1], output + output_length,
        sizeof(output) - output_length);
    CHECK(output_length == strlen("{\"n\":2}\n{\"done\":true}\n"));
    CHECK(memcmp(output, "{\"n\":2}\n{\"done\":true}\n", output_length) == 0);

    int partial_pair[2];
    mcp_broker_io_owner partial_owner = 0U;
    CHECK(attach_pair(pool, partial_pair, &partial_owner));
    CHECK(set_nonblocking(partial_pair[1]));
    int send_buffer = 1024;
    CHECK(setsockopt(partial_pair[0], SOL_SOCKET, SO_SNDBUF, &send_buffer,
        sizeof(send_buffer)) == 0);
    size_t large_length = 60002U;
    uint8_t *large_json = malloc(large_length);
    uint8_t *captured = malloc(large_length + 64U);
    CHECK(large_json != NULL && captured != NULL);
    large_json[0] = '"';
    memset(large_json + 1U, 'a', large_length - 2U);
    large_json[large_length - 1U] = '"';
    CHECK(mcp_broker_io_enqueue(pool, partial_owner, large_json, large_length,
        MCP_BROKER_IO_PROGRESS, key, sizeof(key) - 1U) == MCP_BROKER_IO_QUEUED);
    counters = mcp_broker_io_poll(pool);
    CHECK(counters.written_bytes < large_length + 1U);
    struct timespec stalled_start = {0};
    struct timespec stalled_finish = {0};
    CHECK(clock_gettime(CLOCK_MONOTONIC, &stalled_start) == 0);
    mcp_broker_io_counters stalled_counters = mcp_broker_io_poll(pool);
    CHECK(clock_gettime(CLOCK_MONOTONIC, &stalled_finish) == 0);
    uint64_t stalled_ns = (uint64_t)(stalled_finish.tv_sec - stalled_start.tv_sec) *
        1000000000U + (uint64_t)(stalled_finish.tv_nsec - stalled_start.tv_nsec);
    CHECK(stalled_ns < 100000000U && stalled_counters.written_bytes == 0U);
    CHECK(mcp_broker_io_enqueue(pool, partial_owner, (const uint8_t *)"{\"p\":1}", 7U,
        MCP_BROKER_IO_PROGRESS, key, sizeof(key) - 1U) == MCP_BROKER_IO_QUEUED);
    CHECK(mcp_broker_io_enqueue(pool, partial_owner, (const uint8_t *)"{\"p\":2}", 7U,
        MCP_BROKER_IO_PROGRESS, key, sizeof(key) - 1U) == MCP_BROKER_IO_QUEUED);
    size_t captured_length = 0U;
    for (size_t round = 0U; round < 128U &&
         captured_length < large_length + 1U + strlen("{\"p\":2}\n");
         ++round) {
        counters = mcp_broker_io_poll(pool);
        captured_length += drain(partial_pair[1], captured + captured_length,
            large_length + 64U - captured_length);
    }
    CHECK(captured_length == large_length + 1U + strlen("{\"p\":2}\n"));
    CHECK(memcmp(captured, large_json, large_length) == 0 &&
        captured[large_length] == '\n' &&
        memcmp(captured + large_length + 1U, "{\"p\":2}\n",
            strlen("{\"p\":2}\n")) == 0);
    free(large_json);
    free(captured);

    for (size_t index = 0U; index < MCP_BROKER_IO_CLIENT_QUEUE_MESSAGES; ++index)
        CHECK(mcp_broker_io_enqueue(pool, owner2, (const uint8_t *)"{}", 2U,
            MCP_BROKER_IO_TERMINAL, NULL, 0U) == MCP_BROKER_IO_QUEUED);
    CHECK(mcp_broker_io_enqueue(pool, owner2, (const uint8_t *)"{}", 2U,
        MCP_BROKER_IO_TERMINAL, NULL, 0U) == MCP_BROKER_IO_CLOSED);
    CHECK(!mcp_broker_io_is_connected(pool, owner2));
    CHECK(state.last_reason == MCP_BROKER_IO_QUEUE_FULL);

    int bytes_pair[2];
    mcp_broker_io_owner bytes_owner = 0U;
    CHECK(attach_pair(pool, bytes_pair, &bytes_owner));
    uint8_t *maximum_json = malloc(MCP_JSON_MAX_BYTES);
    CHECK(maximum_json != NULL);
    maximum_json[0] = '"';
    memset(maximum_json + 1U, 'x', MCP_JSON_MAX_BYTES - 2U);
    maximum_json[MCP_JSON_MAX_BYTES - 1U] = '"';
    int byte_send_buffer = 1024;
    CHECK(setsockopt(bytes_pair[0], SOL_SOCKET, SO_SNDBUF, &byte_send_buffer,
        sizeof(byte_send_buffer)) == 0);
    CHECK(mcp_broker_io_enqueue(pool, bytes_owner, maximum_json,
        MCP_JSON_MAX_BYTES, MCP_BROKER_IO_TERMINAL, NULL, 0U) ==
        MCP_BROKER_IO_QUEUED);
    (void)mcp_broker_io_poll(pool);
    CHECK(mcp_broker_io_enqueue(pool, bytes_owner, maximum_json,
        MCP_JSON_MAX_BYTES, MCP_BROKER_IO_TERMINAL, NULL, 0U) ==
        MCP_BROKER_IO_CLOSED);
    CHECK(state.last_reason == MCP_BROKER_IO_QUEUE_FULL);
    free(maximum_json);
    (void)close(bytes_pair[1]);

    int closed_reader_pair[2];
    mcp_broker_io_owner closed_reader_owner = 0U;
    CHECK(attach_pair(pool, closed_reader_pair, &closed_reader_owner));
    CHECK(mcp_broker_io_enqueue(pool, closed_reader_owner,
        (const uint8_t *)"{\"e\":1}", 7U, MCP_BROKER_IO_TERMINAL,
        NULL, 0U) == MCP_BROKER_IO_QUEUED);
    (void)close(closed_reader_pair[1]);
    (void)mcp_broker_io_poll(pool);
    CHECK(!mcp_broker_io_is_connected(pool, closed_reader_owner));
    CHECK(state.last_reason == MCP_BROKER_IO_SOCKET_ERROR);

    int eof_buffered_pair[2];
    mcp_broker_io_owner eof_buffered_owner = 0U;
    size_t lines_before_eof = state.lines;
    CHECK(attach_pair(pool, eof_buffered_pair, &eof_buffered_owner));
    CHECK(send(eof_buffered_pair[1], "{}\n{}\n", 6U, 0) == 6);
    (void)close(eof_buffered_pair[1]);
    (void)mcp_broker_io_poll(pool);
    (void)mcp_broker_io_poll(pool);
    (void)mcp_broker_io_poll(pool);
    CHECK(state.lines == lines_before_eof + 2U);
    CHECK(!mcp_broker_io_is_connected(pool, eof_buffered_owner));
    CHECK(state.last_reason == MCP_BROKER_IO_EOF);

    int reserve_pair[2];
    mcp_broker_io_owner reserve_owner = 0U;
    CHECK(attach_pair(pool, reserve_pair, &reserve_owner));
    uint8_t *maximum_progress = malloc(MCP_BROKER_IO_PROGRESS_BYTES - 1U);
    CHECK(maximum_progress != NULL);
    maximum_progress[0] = '"';
    memset(maximum_progress + 1U, 'p', MCP_BROKER_IO_PROGRESS_BYTES - 3U);
    maximum_progress[MCP_BROKER_IO_PROGRESS_BYTES - 2U] = '"';
    CHECK(mcp_broker_io_enqueue(pool, reserve_owner, maximum_progress,
        MCP_BROKER_IO_PROGRESS_BYTES - 1U, MCP_BROKER_IO_PROGRESS,
        key, sizeof(key) - 1U) == MCP_BROKER_IO_QUEUED);
    CHECK(mcp_broker_io_enqueue(pool, reserve_owner, (const uint8_t *)"{}", 2U,
        MCP_BROKER_IO_TERMINAL, NULL, 0U) == MCP_BROKER_IO_QUEUED);
    free(maximum_progress);
    (void)close(reserve_pair[1]);

    int invalid_pair[2];
    mcp_broker_io_owner invalid_owner = 0U;
    CHECK(attach_pair(pool, invalid_pair, &invalid_owner));
    CHECK(mcp_broker_io_enqueue(pool, invalid_owner, (const uint8_t *)"{}",
        MCP_JSON_MAX_BYTES + 1U, MCP_BROKER_IO_TERMINAL, NULL, 0U) ==
        MCP_BROKER_IO_CLOSED);
    CHECK(state.last_reason == MCP_BROKER_IO_INVALID_FRAME);
    (void)close(invalid_pair[1]);

    int close_pair[2];
    mcp_broker_io_owner close_owner = 0U;
    CHECK(attach_pair(pool, close_pair, &close_owner));
    size_t close_count = state.disconnects;
    CHECK(!mcp_broker_io_close(pool, UINT64_MAX, MCP_BROKER_IO_REQUESTED));
    CHECK(mcp_broker_io_close(pool, close_owner, MCP_BROKER_IO_REQUESTED));
    CHECK(!mcp_broker_io_close(pool, close_owner, MCP_BROKER_IO_REQUESTED));
    CHECK(state.disconnects == close_count + 1U &&
        state.last_reason == MCP_BROKER_IO_REQUESTED);
    (void)close(close_pair[1]);

    harness swap_state = {0};
    mcp_broker_io *swap_pool = NULL;
    CHECK(make_pool(&swap_state, &swap_pool));
    int old_pair[2];
    int replacement_pair[2];
    mcp_broker_io_owner old_owner = 0U;
    CHECK(attach_pair(swap_pool, old_pair, &old_owner));
    CHECK(socketpair(AF_UNIX, SOCK_STREAM, 0, replacement_pair) == 0);
    swap_state.replacement_descriptor = replacement_pair[0];
    swap_state.replacement_owner = 0U;
    swap_state.swap_on_line = true;
    close_count = swap_state.disconnects;
    CHECK(send(old_pair[1], "X\n", 2U, 0) == 2);
    (void)mcp_broker_io_poll(swap_pool);
    CHECK(!mcp_broker_io_is_connected(swap_pool, old_owner));
    CHECK(swap_state.replacement_owner != 0U &&
        mcp_broker_io_is_connected(swap_pool, swap_state.replacement_owner));
    CHECK(swap_state.disconnects == close_count + 1U &&
        swap_state.last_reason == MCP_BROKER_IO_REQUESTED);
    CHECK(send(replacement_pair[1], "Y\n", 2U, 0) == 2);
    (void)mcp_broker_io_poll(swap_pool);
    CHECK(swap_state.lines == 1U && swap_state.first_bytes[0] == 'Y');
    (void)close(old_pair[1]);
    (void)close(replacement_pair[1]);
    mcp_broker_io_destroy(swap_pool);

    harness duplicate_state = {0};
    mcp_broker_io *duplicate_pool = NULL;
    CHECK(make_pool(&duplicate_state, &duplicate_pool));
    int first_duplicate_pair[2];
    int second_duplicate_pair[2];
    mcp_broker_io_owner first_duplicate_owner = 0U;
    mcp_broker_io_owner second_duplicate_owner = 0U;
    CHECK(attach_pair(duplicate_pool, first_duplicate_pair,
        &first_duplicate_owner));
    CHECK(attach_pair(duplicate_pool, second_duplicate_pair,
        &second_duplicate_owner));
    int first_descriptor = first_duplicate_pair[0];
    int second_descriptor = second_duplicate_pair[0];
    CHECK(mcp_broker_io_close(duplicate_pool, first_duplicate_owner,
        MCP_BROKER_IO_REQUESTED));
    int filler_descriptor = open("/dev/null", O_RDONLY);
    CHECK(filler_descriptor == first_descriptor);
    mcp_broker_io_owner rejected_owner = UINT64_C(0xfeedface);
    CHECK(!mcp_broker_io_attach(duplicate_pool, second_descriptor,
        &rejected_owner));
    CHECK(rejected_owner == UINT64_C(0xfeedface) &&
        mcp_broker_io_is_connected(duplicate_pool, second_duplicate_owner) &&
        fcntl(second_descriptor, F_GETFD) >= 0);
    CHECK(send(second_duplicate_pair[1], "D\n", 2U, 0) == 2);
    (void)mcp_broker_io_poll(duplicate_pool);
    CHECK(duplicate_state.lines == 1U && duplicate_state.first_bytes[0] == 'D');
    CHECK(mcp_broker_io_close(duplicate_pool, second_duplicate_owner,
        MCP_BROKER_IO_REQUESTED));
    (void)close(second_duplicate_pair[1]);
    int reuse_probe = open("/dev/null", O_RDONLY);
    CHECK(reuse_probe == second_descriptor);
    mcp_broker_io_destroy(duplicate_pool);
    CHECK(fcntl(reuse_probe, F_GETFD) >= 0);
    (void)close(reuse_probe);
    (void)close(filler_descriptor);
    (void)close(first_duplicate_pair[1]);

    (void)printf("stalled-peer poll: %llu ns, %u syscalls, %zu written bytes\n",
        (unsigned long long)stalled_ns, stalled_counters.syscalls,
        stalled_counters.written_bytes);

    (void)close(pair[1]);
    (void)mcp_broker_io_poll(pool);
    CHECK(state.disconnects >= 1U);
    mcp_broker_io_destroy(pool);
    (void)close(pair2[1]);
    CHECK(state.last_reason == MCP_BROKER_IO_SHUTDOWN ||
        state.last_reason == MCP_BROKER_IO_EOF);
    CHECK(make_pool(&state, &pool));
    int pair3[2];
    mcp_broker_io_owner reused = 0U;
    CHECK(attach_pair(pool, pair3, &reused));
    CHECK(reused != owner && reused != owner2);
    mcp_broker_io_destroy(pool);
    (void)close(pair3[1]);

    harness budget_state = {0};
    mcp_broker_io *budget_pool = NULL;
    CHECK(make_pool(&budget_state, &budget_pool));
    int budget_pairs[MCP_BROKER_IO_CLIENTS][2];
    mcp_broker_io_owner budget_owners[MCP_BROKER_IO_CLIENTS];
    uint8_t budget_input[MCP_BROKER_IO_CLIENT_BUDGET / 2U];
    uint8_t budget_output[sizeof(budget_input) - 1U];
    for (size_t index = 0U; index < sizeof(budget_input) / 3U; ++index)
        memcpy(budget_input + index * 3U, "{}\n", 3U);
    memset(budget_input + (sizeof(budget_input) / 3U) * 3U, 'x',
        sizeof(budget_input) % 3U);
    budget_output[0] = '"';
    memset(budget_output + 1U, 'o', sizeof(budget_output) - 2U);
    budget_output[sizeof(budget_output) - 1U] = '"';
    for (size_t index = 0U; index < MCP_BROKER_IO_CLIENTS; ++index) {
        CHECK(attach_pair(budget_pool, budget_pairs[index], &budget_owners[index]));
        CHECK(mcp_broker_io_enqueue(budget_pool, budget_owners[index],
            budget_output, sizeof(budget_output), MCP_BROKER_IO_TERMINAL,
            NULL, 0U) == MCP_BROKER_IO_QUEUED);
        CHECK(send(budget_pairs[index][1], budget_input, sizeof(budget_input), 0) ==
            (ssize_t)sizeof(budget_input));
    }
    counters = mcp_broker_io_poll(budget_pool);
    CHECK(counters.read_bytes + counters.written_bytes ==
            MCP_BROKER_IO_POLL_BYTE_BUDGET &&
        counters.read_bytes == MCP_BROKER_IO_POLL_BYTE_BUDGET / 2U &&
        counters.written_bytes == MCP_BROKER_IO_POLL_BYTE_BUDGET / 2U &&
        counters.lines == MCP_BROKER_IO_LINE_BUDGET &&
        counters.syscalls == MCP_BROKER_IO_CLIENTS * 2U);
    for (size_t index = 0U; index < MCP_BROKER_IO_CLIENTS; ++index)
        CHECK(counters.per_client_bytes[index] == MCP_BROKER_IO_CLIENT_BUDGET);
    counters = mcp_broker_io_poll(budget_pool);
    CHECK(counters.read_bytes + counters.written_bytes == 0U &&
        counters.lines == MCP_BROKER_IO_LINE_BUDGET);
    for (size_t index = 0U; index < MCP_BROKER_IO_CLIENTS; ++index)
        (void)close(budget_pairs[index][1]);
    mcp_broker_io_destroy(budget_pool);
    return EXIT_SUCCESS;
}
