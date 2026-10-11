#ifndef VEXEL_RASTER_SHAPE_FILTER_H
#define VEXEL_RASTER_SHAPE_FILTER_H

#include "shape.h"
#include "tile.h"

#include <stdbool.h>
#include <stddef.h>

bool raster_shape_filter_apply(const svg_shape_document *document,
                               size_t filter_index, raster_tile *tile,
                               size_t available_scratch,
                               svg_shape_diagnostic *diagnostic);

#endif
