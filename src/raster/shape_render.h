#ifndef VEXEL_RASTER_SHAPE_RENDER_H
#define VEXEL_RASTER_SHAPE_RENDER_H

#include "shape.h"
#include "tile.h"

#include <stdbool.h>

/* Deterministic scalar 4x4 subpixel fill. out_tile must be empty; caller owns it on success. */
bool raster_shape_render(const svg_shape_document *document, raster_tile *out_tile,
                         svg_shape_diagnostic *diagnostic);
/* Evaluates source-backed SMIL tracks at an explicit document time and renders
 * an owned overlay without mutating the decoded document or its source bytes. */
bool raster_shape_render_at(const svg_shape_document *document,
                            double time_seconds, raster_tile *out_tile,
                            svg_shape_diagnostic *diagnostic);

#endif
