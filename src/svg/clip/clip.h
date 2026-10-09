#ifndef VEXEL_SVG_CLIP_H
#define VEXEL_SVG_CLIP_H

#include "shape.h"

typedef struct svg_shape_clip svg_shape_clip;

svg_shape_error svg_clip_resolve(base_bytes source,
                                 const svg_xml_token *tokens,
                                 size_t token_count,
                                 svg_xml_span value,
                                 const svg_shape_document *document,
                                 size_t *document_clip_bytes,
                                 size_t target_shape,
                                 svg_shape_clip **out,
                                 svg_shape_diagnostic *diagnostic);
bool svg_clip_contains(const svg_shape_clip *clip, double x, double y);
size_t svg_clip_work_units(const svg_shape_clip *clip);
void svg_clip_destroy(svg_shape_clip *clip);

#endif
