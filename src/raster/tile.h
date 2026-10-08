#ifndef VEXEL_RASTER_TILE_H
#define VEXEL_RASTER_TILE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define RASTER_TILE_MAX_AXIS 4096U

typedef struct raster_tile {
    uint32_t width;
    uint32_t height;
    size_t byte_count;
    uint8_t *rgba; /* owned by the tile; release with raster_tile_destroy */
} raster_tile;

/* Allocate a bounded deterministic opaque RGBA test tile. */
bool raster_tile_create(uint32_t width, uint32_t height, raster_tile *out_tile);
void raster_tile_destroy(raster_tile *tile);

#endif
