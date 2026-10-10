#ifndef VEXEL_SVG_PAINT_H
#define VEXEL_SVG_PAINT_H

#include "shape.h"
#include "reference.h"
#include "css.h"

#define SVG_PAINT_MAX_STOPS 65536U
#define SVG_PAINT_MAX_GRADIENT_STOPS 256U
#define SVG_PAINT_CACHE_MAX_GRADIENTS 32768U

typedef struct svg_paint_cache svg_paint_cache;

typedef enum svg_shape_paint_kind {
    SVG_SHAPE_PAINT_SOLID = 0,
    SVG_SHAPE_PAINT_LINEAR,
    SVG_SHAPE_PAINT_RADIAL,
    SVG_SHAPE_PAINT_IMAGE
} svg_shape_paint_kind;
typedef enum svg_shape_spread_method {
    SVG_SHAPE_SPREAD_PAD,
    SVG_SHAPE_SPREAD_REFLECT,
    SVG_SHAPE_SPREAD_REPEAT
} svg_shape_spread_method;

struct svg_paint_stop {
    double offset;
    uint8_t rgba[4];
};

struct svg_shape_paint {
    svg_shape_paint_kind kind;
    svg_shape_spread_method spread_method;
    bool object_bounding_box;
    double coordinates[6];
    double bounds[4];
    geom_affine user_inverse;
    geom_affine gradient_inverse;
    size_t first_stop;
    size_t stop_count;
};

/* Returns an edge pointer only for a nonempty in-range span; formed reports
 * whether pointer arithmetic was used, for deterministic safety tests. */
const geom_edge *svg_paint_edge_span(const svg_shape_document *document,
                                     size_t first_edge, size_t edge_count,
                                     bool *formed);

/* Resolves only supported local linear/radial paint servers. The document owns
 * appended stops; failure leaves both document and paint unchanged. */
svg_shape_error svg_paint_resolve_fill(base_bytes source,
                                       const svg_xml_token *tokens,
                                       size_t token_count,
                                       svg_reference_index *references,
                                       svg_css_sheet *css,
                                       svg_paint_cache **cache,
                                       size_t *token_visits,
                                       svg_xml_span fill,
                                       const geom_edge *edges,
                                       size_t edge_count,
                                       geom_affine user_inverse,
                                       double viewport_width,
                                       double viewport_height,
                                       svg_shape_paint *paint,
                                       svg_paint_stop **stops,
                                       size_t *stop_count,
                                       svg_shape_diagnostic *diagnostic);

void svg_paint_cache_destroy(svg_paint_cache *cache);

void svg_paint_sample(const svg_shape_paint *paint,
                      const svg_paint_stop *stops,
                      double x, double y, uint8_t rgba[4]);

#endif
