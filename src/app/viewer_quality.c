#include "viewer_quality.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

#define VIEWER_QUALITY_MAX_AXIS 8192U

static bool tile_bytes(uint32_t width, uint32_t height, size_t *out_bytes)
{
    if (out_bytes == NULL || width == 0U || height == 0U ||
        width > VIEWER_SHARP_TILE_AXIS || height > VIEWER_SHARP_TILE_AXIS) return false;
    size_t w = (size_t)width;
    size_t h = (size_t)height;
    if (w > SIZE_MAX / h) return false;
    size_t pixels = w * h;
    if (pixels > SIZE_MAX / 4U) return false;
    *out_bytes = pixels * 4U;
    return true;
}

static void tile_release(viewer_sharp_tile *tile)
{
    if (tile == NULL) return;
    free(tile->rgba);
    *tile = (viewer_sharp_tile){0};
}

bool viewer_tile_cache_init(viewer_tile_cache *cache, uint64_t generation)
{
    if (cache == NULL || generation == 0U) return false;
    *cache = (viewer_tile_cache){.generation = generation};
    return true;
}

void viewer_tile_cache_clear(viewer_tile_cache *cache, uint64_t generation)
{
    if (cache == NULL) return;
    for (size_t index = 0U; index < VIEWER_SHARP_CACHE_ENTRIES; index++)
        tile_release(&cache->entries[index].tile);
    *cache = (viewer_tile_cache){.generation = generation};
}

void viewer_tile_cache_destroy(viewer_tile_cache *cache)
{
    viewer_tile_cache_clear(cache, 0U);
}

static bool region_matches(const viewer_sharp_tile *tile, uint64_t generation,
                           uint32_t grid_x, uint32_t grid_y)
{
    return tile->rgba != NULL && tile->region.generation == generation &&
        tile->region.grid_x == grid_x && tile->region.grid_y == grid_y;
}

const viewer_sharp_tile *viewer_tile_cache_get(viewer_tile_cache *cache,
                                                uint64_t generation,
                                                uint32_t grid_x,
                                                uint32_t grid_y)
{
    if (cache == NULL || cache->generation != generation) return NULL;
    for (size_t index = 0U; index < VIEWER_SHARP_CACHE_ENTRIES; index++) {
        viewer_tile_cache_entry *entry = &cache->entries[index];
        if (region_matches(&entry->tile, generation, grid_x, grid_y)) {
            if (cache->clock < UINT64_MAX) entry->last_use = ++cache->clock;
            return &entry->tile;
        }
    }
    return NULL;
}

static size_t cache_victim(const viewer_tile_cache *cache)
{
    size_t oldest = SIZE_MAX;
    uint64_t oldest_use = UINT64_MAX;
    for (size_t index = 0U; index < VIEWER_SHARP_CACHE_ENTRIES; index++) {
        const viewer_tile_cache_entry *entry = &cache->entries[index];
        if (entry->tile.rgba == NULL) return index;
        if (entry->last_use < oldest_use) {
            oldest_use = entry->last_use;
            oldest = index;
        }
    }
    return oldest;
}

bool viewer_tile_cache_put(viewer_tile_cache *cache, viewer_sharp_tile *tile)
{
    if (cache == NULL || tile == NULL || tile->rgba == NULL ||
        tile->region.generation == 0U || tile->region.generation != cache->generation) return false;
    size_t expected = 0U;
    if (!tile_bytes(tile->region.width, tile->region.height, &expected) ||
        expected != tile->byte_count || expected > VIEWER_SHARP_CACHE_BYTES) return false;

    size_t target = SIZE_MAX;
    for (size_t index = 0U; index < VIEWER_SHARP_CACHE_ENTRIES; index++) {
        viewer_sharp_tile *resident = &cache->entries[index].tile;
        if (resident->rgba == NULL) {
            if (target == SIZE_MAX) target = index;
        } else if (resident->region.grid_x == tile->region.grid_x &&
                   resident->region.grid_y == tile->region.grid_y) {
            target = index;
            break;
        }
    }
    if (target == SIZE_MAX) target = cache_victim(cache);
    if (target == SIZE_MAX) return false;
    viewer_tile_cache_entry *entry = &cache->entries[target];
    if (entry->tile.rgba != NULL) {
        if (cache->accounted_bytes < entry->tile.byte_count || cache->count == 0U) return false;
        cache->accounted_bytes -= entry->tile.byte_count;
        cache->count--;
        tile_release(&entry->tile);
    }
    if (expected > VIEWER_SHARP_CACHE_BYTES - cache->accounted_bytes) {
        while (expected > VIEWER_SHARP_CACHE_BYTES - cache->accounted_bytes) {
            size_t victim = cache_victim(cache);
            if (victim == SIZE_MAX || victim == target ||
                cache->entries[victim].tile.rgba == NULL) return false;
            viewer_tile_cache_entry *evicted = &cache->entries[victim];
            if (cache->accounted_bytes < evicted->tile.byte_count || cache->count == 0U) return false;
            cache->accounted_bytes -= evicted->tile.byte_count;
            cache->count--;
            tile_release(&evicted->tile);
        }
    }
    entry->tile = *tile;
    if (cache->clock < UINT64_MAX) cache->clock++;
    entry->last_use = cache->clock;
    cache->accounted_bytes += expected;
    cache->count++;
    *tile = (viewer_sharp_tile){0};
    return true;
}

bool viewer_tile_plan_init(viewer_tile_plan *plan, uint32_t width, uint32_t height,
                           uint64_t generation, const ui_viewer_navigation *nav)
{
    if (plan == NULL || nav == NULL || width == 0U || height == 0U ||
        width > VIEWER_QUALITY_MAX_AXIS || height > VIEWER_QUALITY_MAX_AXIS ||
        generation == 0U || !isfinite(nav->scale) || !(nav->scale > 0.0) ||
        !isfinite(nav->pan_x) || !isfinite(nav->pan_y) ||
        !isfinite(nav->document_width) || !(nav->document_width > 0.0) ||
        !isfinite(nav->document_height) || !(nav->document_height > 0.0)) return false;
    double right = nav->pan_x + nav->document_width * nav->scale;
    double bottom = nav->pan_y + nav->document_height * nav->scale;
    if (!isfinite(right) || !isfinite(bottom)) return false;
    *plan = (viewer_tile_plan){
        .viewport_width = width,
        .viewport_height = height,
        .columns = (width + VIEWER_SHARP_TILE_AXIS - 1U) / VIEWER_SHARP_TILE_AXIS,
        .rows = (height + VIEWER_SHARP_TILE_AXIS - 1U) / VIEWER_SHARP_TILE_AXIS,
        .generation = generation,
        .document_left = nav->pan_x,
        .document_top = nav->pan_y,
        .document_right = right,
        .document_bottom = bottom
    };
    return true;
}

static bool cached_region(const viewer_tile_cache *cache, uint64_t generation,
                          uint32_t grid_x, uint32_t grid_y)
{
    if (cache == NULL || cache->generation != generation) return false;
    for (size_t index = 0U; index < VIEWER_SHARP_CACHE_ENTRIES; index++) {
        if (region_matches(&cache->entries[index].tile, generation, grid_x, grid_y)) return true;
    }
    return false;
}

static uint32_t pixel_start(double edge)
{
    double value = ceil(edge - 0.5);
    if (!(value > 0.0)) return 0U;
    if (value >= (double)UINT32_MAX) return UINT32_MAX;
    return (uint32_t)value;
}

bool viewer_tile_plan_next(viewer_tile_plan *plan, const viewer_tile_cache *cache,
                           uint64_t work_per_pixel, viewer_tile_region *out)
{
    if (plan == NULL || out == NULL || work_per_pixel == 0U) return false;
    size_t total = (size_t)plan->columns * (size_t)plan->rows;
    while (plan->next_index < total) {
        size_t index = plan->next_index++;
        uint32_t grid_x = (uint32_t)(index % plan->columns);
        uint32_t grid_y = (uint32_t)(index / plan->columns);
        uint32_t tile_x = grid_x * VIEWER_SHARP_TILE_AXIS;
        uint32_t tile_y = grid_y * VIEWER_SHARP_TILE_AXIS;
        uint32_t tile_right = tile_x + VIEWER_SHARP_TILE_AXIS;
        uint32_t tile_bottom = tile_y + VIEWER_SHARP_TILE_AXIS;
        if (tile_right > plan->viewport_width) tile_right = plan->viewport_width;
        if (tile_bottom > plan->viewport_height) tile_bottom = plan->viewport_height;
        double left = fmax((double)tile_x, fmax(0.0, plan->document_left));
        double top = fmax((double)tile_y, fmax(0.0, plan->document_top));
        double right = fmin((double)tile_right,
                            fmin((double)plan->viewport_width, plan->document_right));
        double bottom = fmin((double)tile_bottom,
                             fmin((double)plan->viewport_height, plan->document_bottom));
        uint32_t x0 = pixel_start(left);
        uint32_t y0 = pixel_start(top);
        uint32_t x1 = pixel_start(right);
        uint32_t y1 = pixel_start(bottom);
        if (x1 > tile_right) x1 = tile_right;
        if (y1 > tile_bottom) y1 = tile_bottom;
        if (x0 < tile_x) x0 = tile_x;
        if (y0 < tile_y) y0 = tile_y;
        if (x0 >= x1 || y0 >= y1 || cached_region(cache, plan->generation, grid_x, grid_y))
            continue;
        uint64_t pixels = (uint64_t)(x1 - x0) * (uint64_t)(y1 - y0);
        if (pixels > VIEWER_SHARP_WORK_CAP / work_per_pixel ||
            plan->planned_work > VIEWER_SHARP_WORK_CAP - pixels * work_per_pixel) return false;
        plan->planned_work += pixels * work_per_pixel;
        *out = (viewer_tile_region){plan->generation, grid_x, grid_y, x0, y0,
                                    x1 - x0, y1 - y0};
        return true;
    }
    return false;
}

bool viewer_generation_advance(uint64_t current, uint64_t *out_next)
{
    if (current == 0U || current == UINT64_MAX || out_next == NULL) return false;
    *out_next = current + 1U;
    return true;
}

bool viewer_generation_is_current(uint64_t current, uint64_t job_generation)
{
    return current != 0U && current == job_generation;
}
