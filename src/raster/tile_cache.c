#include "tile_cache.h"

#include <stdlib.h>
#include <string.h>

#define ENTRY_METADATA_BYTES (sizeof(cache_entry))
typedef struct cache_entry { uint64_t key, last_used; raster_tile tile; bool occupied; } cache_entry;
struct raster_tile_cache { size_t cap, max_entries, count, accounted; uint64_t generation, clock; uint32_t scale; cache_entry *entries; };

static void destroy_tile(raster_tile *tile) { free(tile->rgba); memset(tile, 0, sizeof(*tile)); }
static void remove_entry(raster_tile_cache *cache, size_t index) {
    cache_entry *entry = &cache->entries[index];
    if (!entry->occupied) return;
    cache->accounted -= entry->tile.byte_count;
    cache->count--;
    destroy_tile(&entry->tile);
    memset(entry, 0, sizeof(*entry));
}
static size_t find_key(const raster_tile_cache *cache, uint64_t key) {
    for (size_t i = 0; i < cache->max_entries; ++i) if (cache->entries[i].occupied && cache->entries[i].key == key) return i;
    return cache->max_entries;
}
static size_t free_slot(const raster_tile_cache *cache) {
    for (size_t i = 0; i < cache->max_entries; ++i) if (!cache->entries[i].occupied) return i;
    return cache->max_entries;
}
static size_t lru_slot(const raster_tile_cache *cache) {
    size_t selected = cache->max_entries;
    for (size_t i = 0; i < cache->max_entries; ++i) {
        if (!cache->entries[i].occupied) continue;
        if (selected == cache->max_entries || cache->entries[i].last_used < cache->entries[selected].last_used) selected = i;
    }
    return selected;
}
static uint64_t next_tick(raster_tile_cache *cache) {
    if (cache->clock == UINT64_MAX) {
        uint64_t rank = 1;
        for (;;) {
            size_t selected = cache->max_entries;
            for (size_t i = 0; i < cache->max_entries; ++i) {
                cache_entry *e = &cache->entries[i];
                if (!e->occupied || e->last_used == 0) continue;
                if (selected == cache->max_entries || e->last_used < cache->entries[selected].last_used ||
                    (e->last_used == cache->entries[selected].last_used && i < selected)) selected = i;
            }
            if (selected == cache->max_entries) break;
            cache->entries[selected].last_used = rank++;
        }
        cache->clock = rank;
    }
    return ++cache->clock;
}

bool raster_tile_cache_create(size_t byte_cap, size_t max_entries, uint64_t generation,
                              uint32_t scale_bucket, raster_tile_cache **out_cache) {
    if (out_cache == NULL) return false;
    *out_cache = NULL;
    if (byte_cap == 0 || byte_cap > RASTER_TILE_CACHE_MAX_BYTES || max_entries == 0 || max_entries > RASTER_TILE_CACHE_MAX_ENTRIES) return false;
    if (max_entries > SIZE_MAX / ENTRY_METADATA_BYTES) return false;
    size_t metadata_bytes = max_entries * ENTRY_METADATA_BYTES;
    if (metadata_bytes > byte_cap) return false;
    raster_tile_cache *cache = calloc(1, sizeof(*cache));
    if (cache == NULL) return false;
    cache->entries = calloc(max_entries, sizeof(*cache->entries));
    if (cache->entries == NULL) { free(cache); return false; }
    cache->cap = byte_cap; cache->max_entries = max_entries; cache->accounted = metadata_bytes;
    cache->generation = generation; cache->scale = scale_bucket;
    *out_cache = cache;
    return true;
}
void raster_tile_cache_destroy(raster_tile_cache *cache) {
    if (cache == NULL) return;
    for (size_t i = 0; i < cache->max_entries; ++i) remove_entry(cache, i);
    free(cache->entries); free(cache);
}
const raster_tile *raster_tile_cache_get(raster_tile_cache *cache, uint64_t key) {
    if (cache == NULL) return NULL;
    size_t slot = find_key(cache, key);
    if (slot == cache->max_entries) return NULL;
    cache->entries[slot].last_used = next_tick(cache);
    return &cache->entries[slot].tile;
}
bool raster_tile_cache_put(raster_tile_cache *cache, uint64_t key, raster_tile *tile) {
    if (cache == NULL || tile == NULL || tile->rgba == NULL || tile->width == 0 || tile->height == 0 ||
        tile->width > RASTER_TILE_MAX_AXIS || tile->height > RASTER_TILE_MAX_AXIS) return false;
    size_t pixels = (size_t)tile->width * (size_t)tile->height;
    size_t metadata_bytes = cache->max_entries * ENTRY_METADATA_BYTES;
    if (pixels > SIZE_MAX / 4 || tile->byte_count != pixels * 4 ||
        tile->byte_count > cache->cap - metadata_bytes) return false;
    size_t charge = tile->byte_count;
    size_t slot = find_key(cache, key);
    while (cache->accounted - (slot < cache->max_entries ? cache->entries[slot].tile.byte_count : 0U) > cache->cap - charge ||
           (slot == cache->max_entries && cache->count == cache->max_entries)) {
        size_t victim = lru_slot(cache);
        if (victim == cache->max_entries) return false;
        if (victim == slot) slot = cache->max_entries;
        remove_entry(cache, victim);
    }
    if (slot < cache->max_entries) remove_entry(cache, slot);
    else slot = free_slot(cache);
    if (slot == cache->max_entries) return false;
    cache_entry *entry = &cache->entries[slot];
    entry->key = key; entry->tile = *tile; entry->occupied = true; entry->last_used = next_tick(cache);
    cache->count++; cache->accounted += charge;
    memset(tile, 0, sizeof(*tile));
    return true;
}
bool raster_tile_cache_set_epoch(raster_tile_cache *cache, uint64_t generation, uint32_t scale_bucket) {
    if (cache == NULL) return false;
    if (cache->generation == generation && cache->scale == scale_bucket) return true;
    for (size_t i = 0; i < cache->max_entries; ++i) remove_entry(cache, i);
    cache->generation = generation; cache->scale = scale_bucket; cache->clock = 0;
    return true;
}
bool raster_tile_cache_get_stats(const raster_tile_cache *cache, raster_tile_cache_stats *out_stats) {
    if (out_stats == NULL) return false;
    memset(out_stats, 0, sizeof(*out_stats));
    if (cache == NULL) return false;
    *out_stats = (raster_tile_cache_stats){cache->count, cache->accounted, cache->cap, cache->generation, cache->scale};
    return true;
}
