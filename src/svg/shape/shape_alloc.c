#include "shape_alloc.h"

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#define SVG_ALLOC_TRACKER_CAPACITY 65536U

typedef struct svg_alloc_record {
    void *pointer;
    size_t size;
    svg_shape_alloc_tag tag;
    uint8_t state;
} svg_alloc_record;

static svg_alloc_record records[SVG_ALLOC_TRACKER_CAPACITY];
static svg_shape_alloc_stats current;
static bool active;

static size_t pointer_hash(void *pointer)
{
    size_t value = (size_t)(uintptr_t)pointer >> 4U;
    value ^= value >> 17U;
    value *= (size_t)0xed5ad4bbU;
    value ^= value >> 11U;
    return value & (SVG_ALLOC_TRACKER_CAPACITY - 1U);
}

static void add_saturating(size_t *target, size_t amount)
{
    if (amount > SIZE_MAX - *target) {
        *target = SIZE_MAX;
        current.overflow = true;
    } else {
        *target += amount;
    }
}

static size_t find_record(void *pointer, bool insertion)
{
    if (pointer == NULL) return SVG_ALLOC_TRACKER_CAPACITY;
    size_t slot = pointer_hash(pointer), deleted = SVG_ALLOC_TRACKER_CAPACITY;
    for (size_t probes = 0U; probes < SVG_ALLOC_TRACKER_CAPACITY; ++probes) {
        svg_alloc_record *record = &records[slot];
        if (record->state == 0U)
            return insertion && deleted != SVG_ALLOC_TRACKER_CAPACITY ?
                deleted : slot;
        if (record->state == 2U) {
            if (deleted == SVG_ALLOC_TRACKER_CAPACITY) deleted = slot;
        } else if (record->pointer == pointer) {
            return slot;
        }
        slot = (slot + 1U) & (SVG_ALLOC_TRACKER_CAPACITY - 1U);
    }
    return insertion ? deleted : SVG_ALLOC_TRACKER_CAPACITY;
}

static void track_add(void *pointer, size_t size)
{
    if (!active || pointer == NULL) return;
    size_t slot = find_record(pointer, true);
    if (slot == SVG_ALLOC_TRACKER_CAPACITY) {
        current.overflow = true;
        return;
    }
    svg_alloc_record *record = &records[slot];
    if (record->state == 1U) {
        if (record->size <= current.live_bytes)
            current.live_bytes -= record->size;
        else
            current.live_bytes = 0U;
    }
    *record = (svg_alloc_record){pointer, size, SVG_SHAPE_ALLOC_TAG_NONE, 1U};
    add_saturating(&current.total_allocated_bytes, size);
    add_saturating(&current.allocation_count, 1U);
    add_saturating(&current.live_bytes, size);
    if (current.live_bytes > current.peak_live_bytes)
        current.peak_live_bytes = current.live_bytes;
}

static void track_remove(void *pointer)
{
    if (!active || pointer == NULL) return;
    size_t slot = find_record(pointer, false);
    if (slot == SVG_ALLOC_TRACKER_CAPACITY) return;
    svg_alloc_record *record = &records[slot];
    if (record->size <= current.live_bytes)
        current.live_bytes -= record->size;
    else {
        current.live_bytes = 0U;
        current.overflow = true;
    }
    record->pointer = NULL;
    record->size = 0U;
    record->tag = SVG_SHAPE_ALLOC_TAG_NONE;
    record->state = 2U;
}

void svg_shape_alloc_tracking_start(void)
{
    memset(records, 0, sizeof(records));
    current = (svg_shape_alloc_stats){0};
    active = true;
}

void svg_shape_alloc_tracking_stop(svg_shape_alloc_stats *stats)
{
    active = false;
    if (stats != NULL) *stats = current;
}

void *svg_shape_malloc(size_t size)
{
    void *pointer = malloc(size);
    track_add(pointer, size);
    return pointer;
}

void *svg_shape_calloc(size_t count, size_t size)
{
    void *pointer = calloc(count, size);
    size_t bytes = 0U;
    if (size == 0U || count <= SIZE_MAX / size) bytes = count * size;
    else current.overflow = true;
    track_add(pointer, bytes);
    return pointer;
}

void *svg_shape_realloc(void *pointer, size_t size)
{
    if (!active) return realloc(pointer, size);
    size_t previous_size = 0U;
    size_t previous_slot = find_record(pointer, false);
    bool tracked = previous_slot != SVG_ALLOC_TRACKER_CAPACITY;
    if (tracked) previous_size = records[previous_slot].size;
    void *grown = realloc(pointer, size);
    if (grown == NULL) return NULL;
    if (tracked) track_remove(pointer);
    track_add(grown, size);
    (void)previous_size;
    return grown;
}

void svg_shape_free(void *pointer)
{
    track_remove(pointer);
    free(pointer);
}

bool svg_shape_alloc_set_tag(void *pointer, svg_shape_alloc_tag tag)
{
    if (!active || pointer == NULL) return false;
    size_t slot = find_record(pointer, false);
    if (slot == SVG_ALLOC_TRACKER_CAPACITY) return false;
    svg_alloc_record *record = &records[slot];
    size_t *counter = tag == SVG_SHAPE_ALLOC_TAG_TOKEN_ARRAY ?
        &current.token_array_bytes :
        tag == SVG_SHAPE_ALLOC_TAG_REFERENCE_INDEX ?
        &current.reference_index_bytes :
        tag == SVG_SHAPE_ALLOC_TAG_CLIP_COPY ? &current.clip_copy_bytes :
        tag == SVG_SHAPE_ALLOC_TAG_OUTPUT ? &current.output_bytes : NULL;
    if (counter == NULL) return false;
    add_saturating(counter, record->size);
    record->tag = tag;
    return true;
}

void svg_shape_alloc_note_external(void *pointer, size_t size)
{
    track_add(pointer, size);
}

void svg_shape_alloc_forget_external(void *pointer)
{
    track_remove(pointer);
}
