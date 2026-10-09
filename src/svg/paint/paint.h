#ifndef VEXEL_SVG_PAINT_H
#define VEXEL_SVG_PAINT_H

#include "shape.h"

#define SVG_PAINT_MAX_STOPS 65536U
#define SVG_PAINT_MAX_GRADIENT_STOPS 256U

typedef enum svg_shape_paint_kind {
    SVG_SHAPE_PAINT_SOLID = 0,
    SVG_SHAPE_PAINT_LINEAR,
    SVG_SHAPE_PAINT_RADIAL
} svg_shape_paint_kind;

struct svg_paint_stop {
    double offset;
    uint8_t rgba[4];
};

struct svg_shape_paint {
    svg_shape_paint_kind kind;
    bool object_bounding_box;
    double coordinates[6];
    double bounds[4];
    size_t first_stop;
    size_t stop_count;
};

/* Resolves only supported local linear/radial paint servers. The document owns
 * appended stops; failure leaves both document and paint unchanged. */
svg_shape_error svg_paint_resolve_fill(base_bytes source,
                                       const svg_xml_token *tokens,
                                       size_t token_count,
                                       svg_xml_span fill,
                                       const geom_edge *edges,
                                       size_t edge_count,
                                       double viewport_width,
                                       double viewport_height,
                                       svg_shape_paint *paint,
                                       svg_paint_stop **stops,
                                       size_t *stop_count,
                                       svg_shape_diagnostic *diagnostic);

void svg_paint_sample(const svg_shape_paint *paint,
                      const svg_paint_stop *stops,
                      double x, double y, uint8_t rgba[4]);

#endif
