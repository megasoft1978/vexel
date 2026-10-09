#ifndef VEXEL_SVG_CLIP_H
#define VEXEL_SVG_CLIP_H

#include "shape.h"
#include "reference.h"
#include "css.h"

typedef struct svg_shape_clip svg_shape_clip;

svg_shape_error svg_clip_resolve(base_bytes source,
                                 const svg_xml_token *tokens,
                                 size_t token_count,
                                 svg_reference_index *references,
                                 svg_css_sheet *css,
                                 svg_xml_span value,
                                 const svg_shape_document *document,
                                 size_t *document_clip_bytes,
                                 geom_affine user_inverse,
                                 const double local_bounds[4],
                                 svg_shape_clip **out,
                                 svg_shape_diagnostic *diagnostic);
bool svg_clip_local_bounds(const svg_shape_document *document,
                           size_t first_edge, size_t edge_count,
                           geom_affine user_inverse, double bounds[4]);
bool svg_clip_apply_group(svg_shape_document *document,
                          size_t first_shape, size_t end_shape,
                          svg_shape_clip *clip);
bool svg_clip_contains(const svg_shape_clip *clip, double x, double y);
size_t svg_clip_work_units(const svg_shape_clip *clip);
void svg_clip_destroy(svg_shape_clip *clip);

#endif
