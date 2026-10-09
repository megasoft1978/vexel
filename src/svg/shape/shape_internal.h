#ifndef VEXEL_SVG_SHAPE_INTERNAL_H
#define VEXEL_SVG_SHAPE_INTERNAL_H

#include "shape.h"
#include "shape_alloc.h"
#include "css.h"
#include "reference.h"

typedef struct svg_shape_number_reader {
    const uint8_t *p;
    size_t n, i, base;
} svg_shape_number_reader;

typedef struct svg_shape_builder {
    svg_shape_document *doc;
    svg_reference_index *references;
    svg_css_sheet *css;
    size_t edge_capacity;
    size_t shape_capacity;
    size_t shape_index;
    size_t command_count;
    size_t point_count;
    size_t offset;
    svg_shape_diagnostic *diag;
    geom_affine transform;
} svg_shape_builder;

typedef struct svg_shape_decode_memory_bounds {
    size_t reference_index_bytes;
    size_t reference_lookup_probes;
} svg_shape_decode_memory_bounds;

svg_shape_error svg_shape_decode_source(base_bytes source,
                                        svg_shape_document *out,
                                        svg_shape_diagnostic *diagnostic);
svg_shape_error svg_shape_decode_source_with_bounds(
    base_bytes source, svg_shape_document *out,
    svg_shape_diagnostic *diagnostic,
    svg_shape_decode_memory_bounds *memory_bounds);

bool svg_shape_transform_node(base_bytes source, svg_xml_token *tokens,
                              size_t count, size_t element,
                              geom_affine parent, geom_affine *out,
                              size_t *operations,
                              svg_shape_diagnostic *diagnostic);
bool svg_shape_affine_inverse(geom_affine matrix, geom_affine *inverse);
void svg_shape_skip_number_space(svg_shape_number_reader *reader);
bool svg_shape_next_number_separator(svg_shape_number_reader *reader);
bool svg_shape_read_number_value(svg_shape_number_reader *reader,
                                 double *out);
bool svg_shape_read_number(base_bytes source, svg_xml_span span, double *out);
bool svg_shape_read_number_list(base_bytes source, svg_xml_span span,
                                double *values, size_t count);
bool svg_shape_read_dimension(base_bytes source, svg_xml_span span,
                              double *out);
bool svg_shape_builder_add_element(svg_shape_builder *builder,
                                   base_bytes source,
                                   svg_xml_token *tokens, size_t token_count,
                                   size_t element,
                                   const svg_css_paint *style,
                                   geom_affine transform);

#endif
