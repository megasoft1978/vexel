#include "base.h"

#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(expression) do { \
    if (!(expression)) { \
        fprintf(stderr, "%s:%d: check failed: %s\n", __FILE__, __LINE__, #expression); \
        return false; \
    } \
} while (0)

typedef struct allocator_state {
    size_t allocations;
    size_t frees;
    size_t live;
    size_t fail_on_call;
} allocator_state;

static void *counting_alloc(size_t size, void *context)
{
    allocator_state *state = context;
    state->allocations += 1;
    if (state->fail_on_call != 0 && state->allocations == state->fail_on_call) {
        return NULL;
    }
    void *memory = malloc(size);
    if (memory != NULL) {
        state->live += 1;
    }
    return memory;
}

static void counting_free(void *ptr, void *context)
{
    allocator_state *state = context;
    if (ptr != NULL) {
        state->frees += 1;
        state->live -= 1;
        free(ptr);
    }
}

static uint64_t next_random(uint64_t *state)
{
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    return *state;
}

static bool test_arithmetic(void)
{
    size_t result = 99;
    CHECK(base_size_add(0, 0, &result) && result == 0);
    CHECK(base_size_add(SIZE_MAX, 0, &result) && result == SIZE_MAX);
    CHECK(!base_size_add(SIZE_MAX, 1, &result) && result == 0);
    CHECK(!base_size_add(1, 2, NULL));
    CHECK(base_size_mul(0, SIZE_MAX, &result) && result == 0);
    CHECK(base_size_mul(SIZE_MAX, 1, &result) && result == SIZE_MAX);
    CHECK(!base_size_mul(SIZE_MAX, 2, &result) && result == 0);
    CHECK(!base_size_mul(2, 2, NULL));

    uint64_t seed = UINT64_C(0x4d595df4d0f33173);
    for (size_t i = 0; i < 100000; ++i) {
        size_t a = (size_t)next_random(&seed);
        size_t b = (size_t)next_random(&seed);
        bool add_fits = b <= SIZE_MAX - a;
        result = 123;
        CHECK(base_size_add(a, b, &result) == add_fits);
        CHECK(result == (add_fits ? a + b : 0));
        bool mul_fits = a == 0 || b <= SIZE_MAX / a;
        result = 123;
        CHECK(base_size_mul(a, b, &result) == mul_fits);
        CHECK(result == (mul_fits ? a * b : 0));
    }
    return true;
}

static bool test_slices(void)
{
    uint8_t bytes[64];
    uint8_t copy[64];
    for (size_t i = 0; i < sizeof(bytes); ++i) {
        bytes[i] = (uint8_t)(i * 17U);
        copy[i] = bytes[i];
    }
    base_bytes source = {bytes, sizeof(bytes)};
    base_mut_bytes mutable_source = {bytes, sizeof(bytes)};
    CHECK(base_bytes_valid((base_bytes){NULL, 0}));
    CHECK(!base_bytes_valid((base_bytes){NULL, 1}));
    CHECK(base_mut_bytes_valid((base_mut_bytes){NULL, 0}));
    CHECK(!base_mut_bytes_valid((base_mut_bytes){NULL, 1}));
    CHECK(base_bytes_equal(source, (base_bytes){copy, sizeof(copy)}));
    CHECK(base_bytes_equal((base_bytes){NULL, 0}, (base_bytes){NULL, 0}));
    CHECK(!base_bytes_equal(source, (base_bytes){copy, sizeof(copy) - 1}));
    copy[7] ^= 1U;
    CHECK(!base_bytes_equal(source, (base_bytes){copy, sizeof(copy)}));

    uint64_t seed = UINT64_C(0x9e3779b97f4a7c15);
    for (size_t i = 0; i < 10000; ++i) {
        size_t offset = (size_t)(next_random(&seed) % (sizeof(bytes) + 1));
        size_t length = (size_t)(next_random(&seed) % (sizeof(bytes) - offset + 1));
        base_bytes sub = {bytes, 1};
        base_mut_bytes mut_sub = {bytes, 1};
        CHECK(base_bytes_sub(source, offset, length, &sub));
        CHECK(base_mut_bytes_sub(mutable_source, offset, length, &mut_sub));
        CHECK(sub.len == length && mut_sub.len == length);
        if (length == 0) {
            CHECK(sub.ptr == NULL && mut_sub.ptr == NULL);
        } else {
            CHECK(sub.ptr == bytes + offset && mut_sub.ptr == bytes + offset);
            CHECK(memcmp(sub.ptr, source.ptr + offset, length) == 0);
        }
    }
    base_bytes out = {bytes, sizeof(bytes)};
    CHECK(!base_bytes_sub(source, SIZE_MAX, 1, &out));
    CHECK(out.ptr == NULL && out.len == 0);
    CHECK(!base_bytes_sub(source, 0, SIZE_MAX, &out));
    CHECK(!base_bytes_sub((base_bytes){NULL, 2}, 0, 0, &out));
    CHECK(!base_bytes_sub(source, 0, 0, NULL));
    base_mut_bytes mut_out = {bytes, sizeof(bytes)};
    CHECK(!base_mut_bytes_sub(mutable_source, 65, 0, &mut_out));
    CHECK(mut_out.ptr == NULL && mut_out.len == 0);
    return true;
}

static bool test_arena_alignment_reset_and_limit(void)
{
    base_arena_config config = {16, 96, {NULL, NULL, NULL}};
    base_arena *arena = NULL;
    CHECK(base_arena_create(&config, &arena) == BASE_STATUS_OK && arena != NULL);
    base_arena_stats stats;
    CHECK(base_arena_get_stats(arena, &stats));
    CHECK(stats.chunk_count == 0 && stats.payload_capacity == 0 && stats.used_bytes == 0);

    void *empty = (void *)(uintptr_t)1;
    CHECK(base_arena_push(arena, 0, 4096, &empty) == BASE_STATUS_OK && empty == NULL);
    CHECK(base_arena_push(arena, 1, 3, &empty) == BASE_STATUS_INVALID && empty == NULL);
    CHECK(base_arena_push(arena, 1, 8192, &empty) == BASE_STATUS_INVALID && empty == NULL);

    void *pointers[8] = {NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL};
    const size_t alignments[8] = {1, 2, 4, 8, 16, 64, 256, 4096};
    for (size_t i = 0; i < 8; ++i) {
        CHECK(base_arena_push(arena, 1, alignments[i], &pointers[i]) == BASE_STATUS_OK);
        CHECK(pointers[i] != NULL);
        CHECK((uintptr_t)pointers[i] % alignments[i] == 0);
    }
    CHECK(base_arena_get_stats(arena, &stats));
    CHECK(stats.chunk_count >= 1 && stats.payload_capacity <= 96);
    CHECK(stats.used_bytes <= stats.payload_capacity);
    printf("arena_alignment_boundary: chunks=%zu payload_capacity=%zu used_bytes=%zu\n",
           stats.chunk_count, stats.payload_capacity, stats.used_bytes);
    CHECK(base_arena_push(arena, 48, 1, &empty) == BASE_STATUS_LIMIT && empty == NULL);

    size_t retained_chunks = stats.chunk_count;
    size_t retained_capacity = stats.payload_capacity;
    base_arena_reset(arena);
    CHECK(base_arena_get_stats(arena, &stats));
    CHECK(stats.chunk_count == retained_chunks && stats.payload_capacity == retained_capacity);
    CHECK(stats.used_bytes == 0);
    CHECK(base_arena_push(arena, 8, 8, &empty) == BASE_STATUS_OK);
    CHECK((uintptr_t)empty % 8 == 0);
    CHECK(base_arena_get_stats(arena, &stats) && stats.chunk_count == retained_chunks);
    base_arena_destroy(arena);

    config.initial_chunk_bytes = 9;
    config.capacity_limit = 8;
    arena = (base_arena *)(uintptr_t)1;
    CHECK(base_arena_create(&config, &arena) == BASE_STATUS_LIMIT && arena == NULL);
    return true;
}

static bool test_allocator_failure_and_cleanup(void)
{
    allocator_state state = {0, 0, 0, 1};
    base_arena_config config = {8, 24, {counting_alloc, counting_free, &state}};
    base_arena *arena = (base_arena *)(uintptr_t)1;
    CHECK(base_arena_create(&config, &arena) == BASE_STATUS_OOM && arena == NULL);
    CHECK(state.allocations == 1 && state.frees == 0 && state.live == 0);

    state.fail_on_call = 0;
    CHECK(base_arena_create(&config, &arena) == BASE_STATUS_OK && arena != NULL);
    CHECK(state.allocations == 2 && state.live == 1);
    void *pointer = NULL;
    CHECK(base_arena_push(arena, 8, 1, &pointer) == BASE_STATUS_OK && pointer != NULL);
    base_arena_stats before;
    base_arena_stats after;
    CHECK(base_arena_get_stats(arena, &before));

    state.fail_on_call = state.allocations + 1;
    pointer = (void *)(uintptr_t)1;
    CHECK(base_arena_push(arena, 8, 1, &pointer) == BASE_STATUS_OOM && pointer == NULL);
    CHECK(base_arena_get_stats(arena, &after));
    CHECK(before.chunk_count == after.chunk_count &&
          before.payload_capacity == after.payload_capacity && before.used_bytes == after.used_bytes);
    CHECK(state.live == 2 && state.frees == 0);

    state.fail_on_call = 0;
    CHECK(base_arena_push(arena, 8, 1, &pointer) == BASE_STATUS_OK && pointer != NULL);
    CHECK(base_arena_push(arena, 8, 1, &pointer) == BASE_STATUS_OK && pointer != NULL);
    CHECK(base_arena_get_stats(arena, &after));
    CHECK(after.chunk_count == 3 && after.payload_capacity == 24);
    CHECK(base_arena_push(arena, 1, 1, &pointer) == BASE_STATUS_LIMIT && pointer == NULL);
    base_arena_destroy(arena);
    CHECK(state.live == 0);
    CHECK(state.frees == 4);
    CHECK(state.allocations == 6);
    printf("arena_allocator_failure: allocations=%zu frees=%zu live=%zu\n",
           state.allocations, state.frees, state.live);

    config.allocator.free = NULL;
    arena = (base_arena *)(uintptr_t)1;
    CHECK(base_arena_create(&config, &arena) == BASE_STATUS_INVALID && arena == NULL);

    config.initial_chunk_bytes = 1;
    config.capacity_limit = SIZE_MAX;
    config.allocator = (base_allocator){NULL, NULL, NULL};
    CHECK(base_arena_create(&config, &arena) == BASE_STATUS_OK && arena != NULL);
    CHECK(base_arena_get_stats(arena, &before));
    CHECK(base_arena_push(arena, SIZE_MAX, 1, &pointer) == BASE_STATUS_OVERFLOW);
    CHECK(pointer == NULL && base_arena_get_stats(arena, &after));
    CHECK(before.chunk_count == after.chunk_count &&
          before.payload_capacity == after.payload_capacity && before.used_bytes == after.used_bytes);
    base_arena_destroy(arena);
    return true;
}

int main(void)
{
    if (!test_arithmetic() || !test_slices() || !test_arena_alignment_reset_and_limit() ||
        !test_allocator_failure_and_cleanup()) {
        return EXIT_FAILURE;
    }
    puts("base contract tests passed");
    return EXIT_SUCCESS;
}
