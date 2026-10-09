#ifndef VEXEL_RASTER_SHAPE_RENDER_INTERNAL_H
#define VEXEL_RASTER_SHAPE_RENDER_INTERNAL_H

#include "shape_render.h"

#include <stddef.h>
#include <stdint.h>

typedef struct raster_shape_work_stats {
    uint64_t shape_pixels;
    uint64_t shape_samples;
    uint64_t fill_edge_tests;
    uint64_t stroke_tests;
    uint64_t scanline_edge_entries;
    uint64_t shape_row_visits;
    uint64_t boundary_pixel_marks;
    uint64_t total_work;
} raster_shape_work_stats;

/* Internal measurement entry point used by deterministic tests and benchmarks. */
bool raster_shape_render_with_stats(const svg_shape_document *document,
                                    raster_tile *out_tile,
                                    svg_shape_diagnostic *diagnostic,
                                    raster_shape_work_stats *work);

/* Test adapter for synthetic-alpha comparisons with the frozen scalar oracle.
 * Production rendering always uses source-over compositing. */
bool raster_shape_render_with_stats_scalar_oracle_alpha(
    const svg_shape_document *document, raster_tile *out_tile,
    svg_shape_diagnostic *diagnostic, raster_shape_work_stats *work);

#endif
