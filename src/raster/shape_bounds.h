#ifndef VEXEL_RASTER_SHAPE_BOUNDS_H
#define VEXEL_RASTER_SHAPE_BOUNDS_H

#include "shape.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef struct raster_pixel_bounds {
    uint32_t x_first;
    uint32_t x_last;
    uint32_t y_first;
    uint32_t y_last;
    bool visible;
} raster_pixel_bounds;

typedef struct raster_shape_bounds {
    double device_left;
    double device_top;
    double device_right;
    double device_bottom;
    raster_pixel_bounds pixels;
} raster_shape_bounds;

typedef struct raster_shape_bounds_set {
    raster_shape_bounds *shapes;
    size_t shape_count;
    size_t pixel_count;
} raster_shape_bounds_set;

typedef struct raster_boundary_mask {
    uint8_t *bits;
    size_t bit_count;
    size_t marked_pixel_count;
    size_t mark_work;
} raster_boundary_mask;

/* Bounds cover every sample point the scalar renderer can paint. */
bool raster_shape_bounds_build(const svg_shape_document *document,
                               double scale_x, double scale_y,
                               double offset_x, double offset_y,
                               raster_shape_bounds_set *out,
                               svg_shape_diagnostic *diagnostic);
void raster_shape_bounds_destroy(raster_shape_bounds_set *bounds);
bool raster_shape_bounds_mark_boundaries(const svg_shape_document *document,
                                         double scale_x, double scale_y,
                                         double offset_x,
                                         double offset_y,
                                         raster_boundary_mask *out,
                                         svg_shape_diagnostic *diagnostic);
bool raster_shape_bounds_mark_clip_boundaries(
    const svg_shape_document *document, double scale_x, double scale_y,
    double offset_x,
    double offset_y, raster_boundary_mask *boundary);
void raster_boundary_mask_destroy(raster_boundary_mask *mask);
bool raster_boundary_mask_test(const raster_boundary_mask *mask, size_t pixel_index);

#endif
