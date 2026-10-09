#include "base.h"

#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#define BASE_ARENA_MAX_ALIGNMENT ((size_t)4096)

typedef struct base_arena_chunk {
    struct base_arena_chunk *next;
    uint8_t *data;
    size_t capacity;
    size_t used;
} base_arena_chunk;

struct base_arena {
    base_allocator allocator;
    base_arena_chunk *first;
    base_arena_chunk *last;
    size_t initial_chunk_bytes;
    size_t capacity_limit;
    size_t payload_capacity;
    size_t chunk_count;
};

bool base_size_add(size_t a, size_t b, size_t *out)
{
    if (out == NULL) {
        return false;
    }
    *out = 0;
    if (b > SIZE_MAX - a) {
        return false;
    }
    *out = a + b;
    return true;
}

bool base_size_mul(size_t a, size_t b, size_t *out)
{
    if (out == NULL) {
        return false;
    }
    *out = 0;
    if (a != 0 && b > SIZE_MAX / a) {
        return false;
    }
    *out = a * b;
    return true;
}

bool base_bytes_valid(base_bytes bytes)
{
    return bytes.ptr != NULL || bytes.len == 0;
}

bool base_mut_bytes_valid(base_mut_bytes bytes)
{
    return bytes.ptr != NULL || bytes.len == 0;
}

bool base_bytes_sub(base_bytes bytes, size_t offset, size_t length, base_bytes *out)
{
    if (out == NULL) {
        return false;
    }
    out->ptr = NULL;
    out->len = 0;
    if (!base_bytes_valid(bytes) || offset > bytes.len || length > bytes.len - offset) {
        return false;
    }
    if (length != 0) {
        out->ptr = bytes.ptr + offset;
        out->len = length;
    }
    return true;
}

bool base_mut_bytes_sub(base_mut_bytes bytes, size_t offset, size_t length, base_mut_bytes *out)
{
    if (out == NULL) {
        return false;
    }
    out->ptr = NULL;
    out->len = 0;
    if (!base_mut_bytes_valid(bytes) || offset > bytes.len || length > bytes.len - offset) {
        return false;
    }
    if (length != 0) {
        out->ptr = bytes.ptr + offset;
        out->len = length;
    }
    return true;
}

bool base_bytes_equal(base_bytes left, base_bytes right)
{
    if (!base_bytes_valid(left) || !base_bytes_valid(right) || left.len != right.len) {
        return false;
    }
    return left.len == 0 || memcmp(left.ptr, right.ptr, left.len) == 0;
}

static void *base_default_alloc(size_t size, void *context)
{
    (void)context;
    return malloc(size);
}

static void base_default_free(void *ptr, void *context)
{
    (void)context;
    free(ptr);
}

static size_t base_align_offset(const uint8_t *ptr, size_t alignment)
{
    uintptr_t address = (uintptr_t)ptr;
    uintptr_t mask = (uintptr_t)(alignment - 1);
    uintptr_t padding = ((uintptr_t)0 - address) & mask;
    return (size_t)padding;
}

static base_arena_chunk *base_arena_chunk_create(base_arena *arena, size_t capacity)
{
    size_t allocation_size = 0;
    size_t total = 0;
    if (!base_size_add(sizeof(base_arena_chunk), capacity, &total) ||
        !base_size_add(total, BASE_ARENA_MAX_ALIGNMENT - 1, &allocation_size)) {
        return NULL;
    }

    base_arena_chunk *chunk = arena->allocator.alloc(allocation_size, arena->allocator.context);
    if (chunk == NULL) {
        return NULL;
    }
    chunk->next = NULL;
    chunk->capacity = capacity;
    chunk->used = 0;
    uint8_t *storage = (uint8_t *)(chunk + 1);
    size_t padding = base_align_offset(storage, BASE_ARENA_MAX_ALIGNMENT);
    chunk->data = storage + padding;
    return chunk;
}

base_status base_arena_create(const base_arena_config *config, base_arena **out)
{
    if (out == NULL) {
        return BASE_STATUS_INVALID;
    }
    *out = NULL;
    if (config == NULL || config->initial_chunk_bytes == 0 ||
        ((config->allocator.alloc == NULL) != (config->allocator.free == NULL))) {
        return BASE_STATUS_INVALID;
    }
    if (config->initial_chunk_bytes > config->capacity_limit) {
        return BASE_STATUS_LIMIT;
    }

    base_allocator allocator = config->allocator;
    if (allocator.alloc == NULL) {
        allocator.alloc = base_default_alloc;
        allocator.free = base_default_free;
        allocator.context = NULL;
    }
    base_arena *arena = allocator.alloc(sizeof(*arena), allocator.context);
    if (arena == NULL) {
        return BASE_STATUS_OOM;
    }
    arena->allocator = allocator;
    arena->first = NULL;
    arena->last = NULL;
    arena->initial_chunk_bytes = config->initial_chunk_bytes;
    arena->capacity_limit = config->capacity_limit;
    arena->payload_capacity = 0;
    arena->chunk_count = 0;
    *out = arena;
    return BASE_STATUS_OK;
}

static bool base_arena_chunk_fits(const base_arena_chunk *chunk, size_t size, size_t alignment,
                                  size_t *offset)
{
    size_t padding = base_align_offset(chunk->data + chunk->used, alignment);
    size_t aligned_offset = 0;
    size_t end = 0;
    if (!base_size_add(chunk->used, padding, &aligned_offset) ||
        !base_size_add(aligned_offset, size, &end) || end > chunk->capacity) {
        return false;
    }
    *offset = aligned_offset;
    return true;
}

static base_status base_arena_chunk_failure_status(size_t capacity)
{
    size_t total = 0;
    if (!base_size_add(sizeof(base_arena_chunk), capacity, &total) ||
        !base_size_add(total, BASE_ARENA_MAX_ALIGNMENT - 1, &total)) {
        return BASE_STATUS_OVERFLOW;
    }
    return BASE_STATUS_OOM;
}

static base_status base_arena_append_chunk(base_arena *arena, size_t size, size_t alignment,
                                           void **out)
{
    size_t remaining = arena->capacity_limit - arena->payload_capacity;
    if (size > remaining) {
        return BASE_STATUS_LIMIT;
    }
    size_t capacity = arena->initial_chunk_bytes;
    if (capacity > remaining) capacity = remaining;
    if (capacity < size) capacity = size;
    base_arena_chunk *chunk = base_arena_chunk_create(arena, capacity);
    if (chunk == NULL) return base_arena_chunk_failure_status(capacity);
    size_t offset = 0;
    if (!base_arena_chunk_fits(chunk, size, alignment, &offset)) {
        arena->allocator.free(chunk, arena->allocator.context);
        return BASE_STATUS_OVERFLOW;
    }
    chunk->used = offset + size;
    if (arena->last == NULL) {
        arena->first = chunk;
    } else {
        arena->last->next = chunk;
    }
    arena->last = chunk;
    arena->payload_capacity += capacity;
    arena->chunk_count += 1;
    *out = chunk->data + offset;
    return BASE_STATUS_OK;
}

base_status base_arena_push(base_arena *arena, size_t size, size_t alignment, void **out)
{
    if (out == NULL) return BASE_STATUS_INVALID;
    *out = NULL;
    if (arena == NULL || alignment == 0 || alignment > BASE_ARENA_MAX_ALIGNMENT ||
        (alignment & (alignment - 1)) != 0) return BASE_STATUS_INVALID;
    if (size == 0) return BASE_STATUS_OK;

    for (base_arena_chunk *candidate = arena->first; candidate != NULL; candidate = candidate->next) {
        size_t offset = 0;
        if (base_arena_chunk_fits(candidate, size, alignment, &offset)) {
            candidate->used = offset + size;
            *out = candidate->data + offset;
            return BASE_STATUS_OK;
        }
    }
    return base_arena_append_chunk(arena, size, alignment, out);
}

bool base_arena_get_stats(const base_arena *arena, base_arena_stats *out)
{
    if (out == NULL) {
        return false;
    }
    out->chunk_count = 0;
    out->payload_capacity = 0;
    out->used_bytes = 0;
    if (arena == NULL) {
        return false;
    }
    out->chunk_count = arena->chunk_count;
    out->payload_capacity = arena->payload_capacity;
    for (const base_arena_chunk *chunk = arena->first; chunk != NULL; chunk = chunk->next) {
        out->used_bytes += chunk->used;
    }
    return true;
}

void base_arena_reset(base_arena *arena)
{
    if (arena == NULL) {
        return;
    }
    for (base_arena_chunk *chunk = arena->first; chunk != NULL; chunk = chunk->next) {
        chunk->used = 0;
    }
}

void base_arena_destroy(base_arena *arena)
{
    if (arena == NULL) {
        return;
    }
    base_allocator allocator = arena->allocator;
    base_arena_chunk *chunk = arena->first;
    while (chunk != NULL) {
        base_arena_chunk *next = chunk->next;
        allocator.free(chunk, allocator.context);
        chunk = next;
    }
    allocator.free(arena, allocator.context);
}
