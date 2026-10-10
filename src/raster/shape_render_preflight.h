#ifndef VEXEL_RASTER_SHAPE_RENDER_PREFLIGHT_H
#define VEXEL_RASTER_SHAPE_RENDER_PREFLIGHT_H

#include "shape_bounds.h"
#include "shape_render_internal.h"
#include "shape_scanline.h"
#include "shape_scanline_fill.h"

bool raster_shape_render_work_preflight(
    const svg_shape_document *document,
    const raster_shape_bounds_set *bounds,
    const raster_boundary_mask *boundary,
    raster_scanline_index *scanlines,
    size_t *shape_offsets,
    uint32_t *active_edges,
    size_t active_edge_capacity,
    uint32_t *active_shapes,
    const size_t *stroke_cost,
    const raster_shape_dash_cache *dash_cache,
    raster_fill_scanline_cache *fill_cache,
    double scale_x, double scale_y, double offset_x, double offset_y,
    raster_shape_work_stats *stats);

#endif
