#ifndef VEXEL_APP_VIEWER_QUALITY_H
#define VEXEL_APP_VIEWER_QUALITY_H

#include "ui/viewer_controls.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define VIEWER_SHARP_TILE_AXIS 256U
#define VIEWER_SHARP_CACHE_ENTRIES 128U
#define VIEWER_SHARP_CACHE_BYTES (32U * 1024U * 1024U)
#define VIEWER_SHARP_WORK_CAP UINT64_C(200000000)

typedef struct viewer_tile_region {
    uint64_t generation;
    uint32_t grid_x;
    uint32_t grid_y;
    uint32_t x;
    uint32_t y;
    uint32_t width;
    uint32_t height;
} viewer_tile_region;

typedef struct viewer_sharp_tile {
    viewer_tile_region region;
    size_t byte_count;
    uint8_t *rgba;
} viewer_sharp_tile;

typedef struct viewer_tile_cache_entry {
    viewer_sharp_tile tile;
    uint64_t last_use;
} viewer_tile_cache_entry;

typedef struct viewer_tile_cache {
    viewer_tile_cache_entry entries[VIEWER_SHARP_CACHE_ENTRIES];
    size_t count;
    size_t accounted_bytes;
    uint64_t generation;
    uint64_t clock;
} viewer_tile_cache;

typedef struct viewer_tile_plan {
    uint32_t viewport_width;
    uint32_t viewport_height;
    uint32_t columns;
    uint32_t rows;
    size_t next_index;
    uint64_t generation;
    uint64_t planned_work;
    double document_left;
    double document_top;
    double document_right;
    double document_bottom;
} viewer_tile_plan;

bool viewer_tile_cache_init(viewer_tile_cache *cache, uint64_t generation);
void viewer_tile_cache_clear(viewer_tile_cache *cache, uint64_t generation);
void viewer_tile_cache_destroy(viewer_tile_cache *cache);
/* Borrowed result: invalidated when its tile is replaced, evicted, or the cache clears. */
const viewer_sharp_tile *viewer_tile_cache_get(viewer_tile_cache *cache,
                                                uint64_t generation,
                                                uint32_t grid_x,
                                                uint32_t grid_y);
/* On success the cache owns rgba and resets tile; on failure the caller retains it. */
bool viewer_tile_cache_put(viewer_tile_cache *cache, viewer_sharp_tile *tile);

bool viewer_tile_plan_init(viewer_tile_plan *plan, uint32_t viewport_width,
                           uint32_t viewport_height, uint64_t generation,
                           const ui_viewer_navigation *navigation);
/* Returns false when exhausted or over budget; plan borrows no navigation storage. */
bool viewer_tile_plan_next(viewer_tile_plan *plan, const viewer_tile_cache *cache,
                           uint64_t work_per_pixel, viewer_tile_region *out_region);
bool viewer_generation_advance(uint64_t current, uint64_t *out_next);
bool viewer_generation_is_current(uint64_t current, uint64_t job_generation);

#endif
