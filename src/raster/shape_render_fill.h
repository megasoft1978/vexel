#ifndef VEXEL_RASTER_SHAPE_RENDER_FILL_H
#define VEXEL_RASTER_SHAPE_RENDER_FILL_H

#include "shape_render_internal.h"
#include "shape_scanline_fill.h"

bool raster_shape_render_scanline_fill_shape(
    const svg_shape_document *document, size_t shape,
    const raster_pixel_bounds *pixel_bounds, uint32_t pixel_y,
    const raster_boundary_mask *boundary,
    const raster_fill_scanline_cache *fill_cache,
    double scale_x, double scale_y, double offset_x, double offset_y,
    const geom_edge *local_edges, const bool *dash_seams,
    const raster_shape_dash_cache *dash_cache, bool scalar_oracle_alpha,
    uint8_t *center_samples, uint8_t *subpixel_samples);

#endif
