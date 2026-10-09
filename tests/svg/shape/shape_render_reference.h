#ifndef VEXEL_TEST_SHAPE_RENDER_REFERENCE_H
#define VEXEL_TEST_SHAPE_RENDER_REFERENCE_H

#include "shape_render.h"

/* Frozen pre-culling scalar oracle. Compiled only into test executables. */
bool raster_shape_render_reference(const svg_shape_document *document,
                                   raster_tile *out_tile,
                                   svg_shape_diagnostic *diagnostic);

#endif
