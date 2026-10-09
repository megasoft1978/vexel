#ifndef VEXEL_RASTER_TILE_CACHE_H
#define VEXEL_RASTER_TILE_CACHE_H

#include "tile.h"
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define RASTER_TILE_CACHE_MAX_BYTES (256U * 1024U * 1024U)
#define RASTER_TILE_CACHE_MAX_ENTRIES 65536U
typedef struct raster_tile_cache raster_tile_cache;
typedef struct raster_tile_cache_stats { size_t entry_count, accounted_bytes, byte_cap; uint64_t generation; uint32_t scale_bucket; } raster_tile_cache_stats;

/* Cache is mutable from one owner thread. max_entries is bounded and nonzero. */
bool raster_tile_cache_create(size_t byte_cap, size_t max_entries, uint64_t generation,
                              uint32_t scale_bucket, raster_tile_cache **out_cache);
void raster_tile_cache_destroy(raster_tile_cache *cache);
/* Returned tile is borrowed until the next mutating call or destruction. */
const raster_tile *raster_tile_cache_get(raster_tile_cache *cache, uint64_t key);
/* On success ownership transfers and tile is cleared. On failure caller retains ownership. */
bool raster_tile_cache_put(raster_tile_cache *cache, uint64_t key, raster_tile *tile);
/* Changing either epoch component clears every entry before return. */
bool raster_tile_cache_set_epoch(raster_tile_cache *cache, uint64_t generation, uint32_t scale_bucket);
bool raster_tile_cache_get_stats(const raster_tile_cache *cache, raster_tile_cache_stats *out_stats);

#endif
