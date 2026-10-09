#ifndef VEXEL_BASE_BASE_H
#define VEXEL_BASE_BASE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef enum base_status {
    BASE_STATUS_OK = 0,
    BASE_STATUS_INVALID,
    BASE_STATUS_OVERFLOW,
    BASE_STATUS_LIMIT,
    BASE_STATUS_OOM
} base_status;

typedef struct base_bytes {
    const uint8_t *ptr;
    size_t len;
} base_bytes;

typedef struct base_mut_bytes {
    uint8_t *ptr;
    size_t len;
} base_mut_bytes;

typedef void *(*base_alloc_fn)(size_t size, void *context);
typedef void (*base_free_fn)(void *ptr, void *context);

typedef struct base_allocator {
    base_alloc_fn alloc;
    base_free_fn free;
    void *context;
} base_allocator;

typedef struct base_arena base_arena;

typedef struct base_arena_config {
    size_t initial_chunk_bytes;
    size_t capacity_limit;
    base_allocator allocator;
} base_arena_config;

typedef struct base_arena_stats {
    size_t chunk_count;
    size_t payload_capacity;
    size_t used_bytes;
} base_arena_stats;

bool base_size_add(size_t a, size_t b, size_t *out);
bool base_size_mul(size_t a, size_t b, size_t *out);

bool base_bytes_valid(base_bytes bytes);
bool base_mut_bytes_valid(base_mut_bytes bytes);
bool base_bytes_sub(base_bytes bytes, size_t offset, size_t length, base_bytes *out);
bool base_mut_bytes_sub(base_mut_bytes bytes, size_t offset, size_t length, base_mut_bytes *out);
bool base_bytes_equal(base_bytes left, base_bytes right);

base_status base_arena_create(const base_arena_config *config, base_arena **out);
base_status base_arena_push(base_arena *arena, size_t size, size_t alignment, void **out);
bool base_arena_get_stats(const base_arena *arena, base_arena_stats *out);
void base_arena_reset(base_arena *arena);
void base_arena_destroy(base_arena *arena);

#endif
