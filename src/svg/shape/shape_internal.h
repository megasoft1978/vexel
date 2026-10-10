#ifndef VEXEL_SVG_SHAPE_INTERNAL_H
#define VEXEL_SVG_SHAPE_INTERNAL_H

#include "shape.h"
#include "shape_alloc.h"
#include "css.h"
#include "reference.h"
#include "../paint/paint.h"

typedef struct svg_shape_number_reader {
    const uint8_t *p;
    size_t n, i, base;
} svg_shape_number_reader;

typedef struct svg_shape_builder {
    svg_shape_document *doc;
    svg_reference_index *references;
    svg_css_sheet *css;
    svg_paint_cache **paint_cache;
    size_t *paint_token_visits;
    size_t edge_capacity;
    size_t shape_capacity;
    size_t subpath_capacity;
    size_t shape_index;
    size_t command_count;
    size_t point_count;
    double viewport_width;
    double viewport_height;
    size_t offset;
    svg_shape_diagnostic *diag;
    geom_affine transform;
} svg_shape_builder;

typedef struct svg_shape_decode_memory_bounds {
    size_t reference_index_bytes;
    size_t reference_lookup_probes;
    size_t paint_token_visits;
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
bool svg_shape_transform_parse(base_bytes source, svg_xml_span span,
                               geom_affine *out, size_t *operations);
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
bool svg_shape_read_length(base_bytes source, svg_xml_span span,
                           double percentage_reference, double *out);
bool svg_shape_read_length_value(svg_shape_number_reader *reader,
                                double percentage_reference, double *out);
bool svg_shape_parse_preserve_aspect(base_bytes source, svg_xml_span value,
                                    svg_shape_aspect *out);
bool svg_shape_viewbox_transform(double view_x, double view_y,
                                 double view_width, double view_height,
                                 double x, double y, double width,
                                 double height, svg_shape_aspect aspect,
                                 geom_affine *out);
bool svg_shape_root_viewport(base_bytes source, svg_xml_token *tokens,
                             size_t token_count, size_t element,
                             svg_shape_document *document,
                             double *viewport_width,
                             double *viewport_height,
                             svg_shape_diagnostic *diagnostic);
bool svg_shape_symbol_attributes_supported(base_bytes source,
                                          const svg_xml_token *tokens,
                                          size_t token_count, size_t element,
                                          size_t *unsupported_offset);
bool svg_shape_symbol_mapping(base_bytes source,
                              const svg_xml_token *tokens,
                              size_t token_count, size_t target, size_t use,
                              double viewport_width, double viewport_height,
                              geom_affine *mapping);
bool svg_shape_nested_viewport(base_bytes source,
                               const svg_xml_token *tokens,
                               size_t token_count, size_t element,
                               double parent_width, double parent_height,
                               geom_affine element_transform,
                               geom_affine *child_transform,
                               geom_point clip_points[4],
                               double *child_width, double *child_height,
                               bool *empty,
                               svg_shape_diagnostic *diagnostic);
bool svg_shape_span_eq(base_bytes source, svg_xml_span span,
                       const char *literal);
bool svg_shape_point_bounded(geom_point point);
double svg_shape_transformed_flatness_tolerance(
    const svg_shape_builder *builder);
void svg_shape_fail(svg_shape_diagnostic *diagnostic, svg_shape_error error,
                    size_t offset);
void svg_shape_skip_path_separators(svg_shape_number_reader *reader);
bool svg_shape_path_operand_boundary(svg_shape_number_reader *reader,
                                     bool previous_number,
                                     bool previous_arc_flag, char command,
                                     unsigned argument);
bool svg_shape_read_arc_flag(svg_shape_number_reader *reader, double *out);
bool svg_shape_builder_add_element(svg_shape_builder *builder,
                                   base_bytes source,
                                   svg_xml_token *tokens, size_t token_count,
                                   size_t element,
                                   const svg_css_paint *style,
                                   geom_affine transform);
bool svg_shape_parse_style_color(base_bytes source, svg_xml_span value,
                                 const svg_css_paint *style,
                                 uint8_t rgba[4]);
bool svg_shape_parse_opacity(base_bytes source, svg_xml_span value,
                             double *out);
void svg_shape_multiply_alpha(uint8_t rgba[4], double opacity);
bool svg_shape_parse_stroke(base_bytes source, svg_shape_stroke *stroke,
                            const svg_css_paint *paint,
                            double percentage_reference,
                            svg_shape_diagnostic *diagnostic);
bool svg_shape_apply_declared_clip(base_bytes source,
                                   const svg_xml_token *tokens,
                                   size_t token_count,
                                   svg_reference_index *references,
                                   svg_css_sheet *css,
                                   svg_shape_document *document,
                                   svg_xml_span value,
                                   size_t first_shape, size_t first_edge,
                                   geom_affine transform,
                                   svg_shape_diagnostic *diagnostic);
bool svg_shape_style_opacity(base_bytes source, const svg_css_paint *style,
                             double *opacity, svg_xml_span *value);
bool svg_shape_style_display_none(base_bytes source,
                                  const svg_css_paint *style,
                                  size_t fallback_offset, bool *is_none,
                                  svg_shape_diagnostic *diagnostic);

#endif
