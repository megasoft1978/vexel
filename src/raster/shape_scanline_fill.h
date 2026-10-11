#ifndef VEXEL_RASTER_SHAPE_SCANLINE_FILL_H
#define VEXEL_RASTER_SHAPE_SCANLINE_FILL_H

#include "shape.h"
#include "shape_scanline.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define RASTER_FILL_SCANLINE_SAMPLE_ROWS RASTER_SCANLINE_VALUES_PER_ROW
#define RASTER_FILL_SCANLINE_MAX_EDGES 4096U

typedef struct raster_fill_scanline_interval {
    double left;
    double right;
} raster_fill_scanline_interval;

typedef struct raster_fill_scanline_crossing {
    double x;
    int32_t winding_delta;
    uint32_t edge_index;
} raster_fill_scanline_crossing;

typedef struct raster_fill_pixel_run {
    uint32_t first_x;
    uint32_t end_x;
    bool center_only;
} raster_fill_pixel_run;

typedef struct raster_fill_scanline_cache {
    bool *eligible_shapes;
    size_t shape_count;
    size_t max_edge_count;
    size_t interval_stride;
    raster_fill_scanline_crossing *crossings;
    raster_fill_scanline_interval *intervals;
    size_t interval_counts[RASTER_FILL_SCANLINE_SAMPLE_ROWS];
    raster_fill_pixel_run *pixel_runs;
    size_t pixel_run_capacity;
    size_t pixel_run_count;
} raster_fill_scanline_cache;

bool raster_fill_scanline_cache_create(const svg_shape_document *document,
                                       raster_fill_scanline_cache *out_cache,
                                       svg_shape_diagnostic *diagnostic);
void raster_fill_scanline_cache_destroy(raster_fill_scanline_cache *cache);
bool raster_fill_scanline_cache_is_eligible(
    const raster_fill_scanline_cache *cache, size_t shape);
const uint8_t *raster_fill_scanline_cache_color(
    const svg_shape_document *document, size_t shape);
bool raster_fill_scanline_cache_build_row(
    raster_fill_scanline_cache *cache, const svg_shape_document *document,
    size_t shape, uint32_t pixel_y, double scale_y, double offset_y,
    const size_t *shape_offsets, const uint32_t *active_edges, size_t stride,
    size_t *out_edge_visits, size_t *out_work_units);
bool raster_fill_scanline_cache_build_pixel_runs(
    raster_fill_scanline_cache *cache, const svg_shape_document *document,
    const raster_pixel_bounds *pixel_bounds, uint32_t pixel_y,
    const raster_boundary_mask *boundary, double scale_x, double scale_y,
    double offset_x, double offset_y, size_t *out_work_units);
const raster_fill_pixel_run *raster_fill_scanline_cache_pixel_runs(
    const raster_fill_scanline_cache *cache, size_t *out_run_count);
bool raster_fill_scanline_interval_pixel_range(
    const svg_shape_document *document, const raster_pixel_bounds *pixel_bounds,
    double scale_x, double offset_x, double sample_offset,
    const raster_fill_scanline_interval *interval,
    size_t *out_first_x, size_t *out_end_x, size_t *out_comparisons);
bool raster_fill_scanline_cache_contains(
    const raster_fill_scanline_cache *cache, size_t sample_row, double x,
    size_t *out_comparisons);

#endif
