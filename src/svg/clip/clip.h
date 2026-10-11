#ifndef VEXEL_SVG_CLIP_H
#define VEXEL_SVG_CLIP_H

#include "shape.h"
#include "reference.h"
#include "css.h"

typedef struct svg_shape_clip svg_shape_clip;
typedef bool (*svg_clip_boundary_edge_visitor)(void *context, geom_edge edge);
typedef struct svg_clip_work_counts {
    size_t chain_elements;
    size_t child_shapes;
    size_t edges;
} svg_clip_work_counts;

svg_shape_error svg_clip_resolve(base_bytes source,
                                 const svg_xml_token *tokens,
                                 size_t token_count,
                                 svg_reference_index *references,
                                 svg_css_sheet *css,
                                 svg_xml_span value,
                                 svg_shape_document *document,
                                 size_t *document_clip_bytes,
                                 geom_affine user_inverse,
                                 const double local_bounds[4],
                                 size_t owner_node,
                                 svg_shape_clip **out,
                                 svg_shape_diagnostic *diagnostic);
bool svg_clip_local_bounds(const svg_shape_document *document,
                           size_t first_edge, size_t edge_count,
                           geom_affine user_inverse, double bounds[4]);
bool svg_clip_apply_group(svg_shape_document *document,
                          size_t first_shape, size_t end_shape,
                          svg_shape_clip *clip);
svg_shape_clip *svg_clip_create_rect(geom_point p0, geom_point p1,
                                    geom_point p2, geom_point p3,
                                    geom_affine user_inverse,
                                    size_t owner_node);
svg_shape_clip *svg_clip_clone(const svg_shape_clip *clip,
                               size_t *total_bytes, bool *limit);
bool svg_clip_retain(svg_shape_clip *clip);
void svg_clip_update_owner(svg_shape_clip *clip, size_t owner_node,
                           geom_affine user_inverse,
                           geom_affine attribute_inverse,
                           const double local_bounds[4]);
bool svg_clip_contains(const svg_shape_clip *clip, double x, double y);
/* Visits every filled child edge in this clip and its parent chain, mapped back
 * into the painted document's user space. mapping_valid is false when a clip
 * transform cannot be inverted safely; callers should then disable culling. */
bool svg_clip_visit_boundary_edges(const svg_shape_clip *clip,
                                   svg_clip_boundary_edge_visitor visitor,
                                   void *context, bool *mapping_valid);
/* Counts clip-chain entries, child shapes and edges without visiting geometry. */
bool svg_clip_boundary_work_counts(const svg_shape_clip *clip,
                                   svg_clip_work_counts *out);
size_t svg_clip_work_units(const svg_shape_clip *clip);
void svg_clip_destroy(svg_shape_clip *clip);

#endif
