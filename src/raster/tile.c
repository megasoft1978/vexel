#include "raster/tile.h"

#include <stdlib.h>
#include <string.h>

static bool raster_tile_size(uint32_t width, uint32_t height, size_t *out_bytes)
{
    if (out_bytes == NULL || width == 0U || height == 0U ||
        width > RASTER_TILE_MAX_AXIS || height > RASTER_TILE_MAX_AXIS) {
        return false;
    }
    size_t width_size = (size_t)width;
    size_t height_size = (size_t)height;
    if (width_size > SIZE_MAX / height_size) {
        return false;
    }
    size_t pixels = width_size * height_size;
    if (pixels > SIZE_MAX / 4U) {
        return false;
    }
    *out_bytes = pixels * 4U;
    return true;
}

bool raster_tile_create(uint32_t width, uint32_t height, raster_tile *out_tile)
{
    size_t byte_count = 0;
    if (out_tile == NULL) {
        return false;
    }
    *out_tile = (raster_tile){0};
    if (!raster_tile_size(width, height, &byte_count)) {
        return false;
    }
    uint8_t *rgba = malloc(byte_count);
    if (rgba == NULL) {
        return false;
    }
    for (uint32_t y = 0; y < height; y++) {
        for (uint32_t x = 0; x < width; x++) {
            size_t offset = ((size_t)y * width + x) * 4U;
            rgba[offset] = (uint8_t)((x * 17U + y * 3U) & 0xffU);
            rgba[offset + 1U] = (uint8_t)((x * 5U + y * 29U) & 0xffU);
            rgba[offset + 2U] = (uint8_t)((x * 11U + y * 7U) & 0xffU);
            rgba[offset + 3U] = 255U;
        }
    }
    out_tile->width = width;
    out_tile->height = height;
    out_tile->byte_count = byte_count;
    out_tile->rgba = rgba;
    return true;
}

void raster_tile_destroy(raster_tile *tile)
{
    if (tile != NULL) {
        free(tile->rgba);
        memset(tile, 0, sizeof(*tile));
    }
}
