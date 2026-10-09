#include "tile_cache.h"

#include <stdio.h>
#include <stdlib.h>

#define CHECK(condition) do { if (!(condition)) { fprintf(stderr, "check failed at %s:%d: %s\n", __FILE__, __LINE__, #condition); return 1; } } while (0)

static raster_tile make_tile(uint32_t color) {
    raster_tile tile = {0};
    if (!raster_tile_create(1, 1, &tile)) return tile;
    tile.rgba[0] = (uint8_t)color;
    return tile;
}

int spatial_cache_contract_tests(void);

int spatial_cache_contract_tests(void) {
    raster_tile_cache *cache = NULL;
    CHECK(!raster_tile_cache_create(0, 2, 1, 1, &cache));
    CHECK(!raster_tile_cache_create(RASTER_TILE_CACHE_MAX_BYTES + 1U, 2, 1, 1, &cache));
    CHECK(!raster_tile_cache_create(8192, 500, 1, 1, &cache));
    CHECK(raster_tile_cache_create(1024, 2, 1, 1, &cache));
    raster_tile_cache_stats initial_stats;
    CHECK(raster_tile_cache_get_stats(cache, &initial_stats));
    CHECK(initial_stats.accounted_bytes <= initial_stats.byte_cap);
    raster_tile bad = {0};
    CHECK(raster_tile_create(17, 17, &bad));
    uint8_t *owned = bad.rgba;
    CHECK(!raster_tile_cache_put(cache, 1, &bad) && bad.rgba == owned);
    raster_tile_destroy(&bad);
    raster_tile a = make_tile(1), b = make_tile(2), c = make_tile(3);
    uint8_t *a_ptr = a.rgba;
    CHECK(raster_tile_cache_put(cache, 1, &a) && a.rgba == NULL);
    CHECK(raster_tile_cache_put(cache, 2, &b) && b.rgba == NULL);
    CHECK(raster_tile_cache_get(cache, 1) != NULL);
    CHECK(raster_tile_cache_put(cache, 3, &c) && c.rgba == NULL);
    CHECK(raster_tile_cache_get(cache, 1) != NULL && raster_tile_cache_get(cache, 2) == NULL);
    CHECK(raster_tile_cache_get(cache, 3) != NULL);
    CHECK(a_ptr != NULL);
    CHECK(raster_tile_cache_set_epoch(cache, 2, 1));
    CHECK(raster_tile_cache_get(cache, 1) == NULL && raster_tile_cache_get(cache, 3) == NULL);
    raster_tile d = make_tile(4);
    CHECK(raster_tile_cache_put(cache, 4, &d));
    CHECK(raster_tile_cache_set_epoch(cache, 2, 2));
    CHECK(raster_tile_cache_get(cache, 4) == NULL);
    raster_tile_cache_destroy(cache);

    const size_t cap = 8192, max_entries = 100;
    CHECK(raster_tile_cache_create(cap, max_entries, 7, 3, &cache));
    size_t max_accounted = 0;
    for (uint64_t i = 0; i < 10000; ++i) {
        raster_tile tile = make_tile((uint32_t)i);
        CHECK(tile.rgba != NULL && raster_tile_cache_put(cache, i, &tile) && tile.rgba == NULL);
        raster_tile_cache_stats stats;
        CHECK(raster_tile_cache_get_stats(cache, &stats));
        CHECK(stats.accounted_bytes <= cap && stats.entry_count <= max_entries);
        if (stats.accounted_bytes > max_accounted) max_accounted = stats.accounted_bytes;
    }
    raster_tile_cache_stats stats;
    CHECK(raster_tile_cache_get_stats(cache, &stats));
    printf("{\"insertions\":10000,\"entry_count\":%zu,\"accounted_bytes\":%zu,\"byte_cap\":%zu,\"max_accounted_bytes\":%zu,\"max_entries\":%zu,\"ownership_success\":true,\"ownership_failure\":true,\"deterministic_lru\":true,\"generation_invalidation\":true,\"scale_invalidation\":true}\n",
           stats.entry_count, stats.accounted_bytes, cap, max_accounted, max_entries);
    raster_tile_cache_destroy(cache);
    return 0;
}
