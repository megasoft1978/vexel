#ifndef VEXEL_SVG_SHAPE_INTERNAL_H
#define VEXEL_SVG_SHAPE_INTERNAL_H

#include "shape.h"

svg_shape_error svg_shape_decode_source(base_bytes source,
                                        svg_shape_document *out,
                                        svg_shape_diagnostic *diagnostic);

#endif
