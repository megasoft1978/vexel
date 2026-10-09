#ifndef VEXEL_RASTER_SHAPE_RENDER_H
#define VEXEL_RASTER_SHAPE_RENDER_H

#include "shape.h"
#include "tile.h"

#include <stdbool.h>

/* Deterministic scalar 4x4 subpixel fill. out_tile must be empty; caller owns it on success. */
bool raster_shape_render(const svg_shape_document *document, raster_tile *out_tile,
                         svg_shape_diagnostic *diagnostic);

#endif
