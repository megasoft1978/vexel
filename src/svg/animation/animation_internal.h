#ifndef VEXEL_SVG_ANIMATION_INTERNAL_H
#define VEXEL_SVG_ANIMATION_INTERNAL_H

#include "animation.h"

bool svg_anim_span_eq(base_bytes source, svg_xml_span span, const char *text);
bool svg_anim_token_name(base_bytes source, const svg_xml_token *token,
                         const char *name);
bool svg_anim_attr(base_bytes source, const svg_xml_token *tokens,
                   size_t token_count, size_t element, const char *name,
                   svg_xml_span *out);
svg_xml_span svg_anim_attr_or_empty(base_bytes source,
                                    const svg_xml_token *tokens,
                                    size_t token_count, size_t element,
                                    const char *name);
bool svg_anim_parse_number(base_bytes source, svg_xml_span span, double *out);
bool svg_anim_parse_time(base_bytes source, svg_xml_span span, double *out);
bool svg_anim_path_structures_match(base_bytes source, svg_xml_span left,
                                    svg_xml_span right);
svg_anim_attribute svg_anim_attribute_kind(base_bytes source,
                                           svg_xml_span name,
                                           bool *is_transform,
                                           bool *is_motion);
void svg_anim_add_diagnostic(svg_anim_document *document,
                             svg_anim_diagnostic_kind kind, size_t offset);
bool svg_anim_element_span(const svg_xml_token *tokens, size_t token_count,
                           size_t element, size_t *offset, size_t *length);
bool svg_anim_parse_sync_time(base_bytes source, svg_xml_span span,
                              svg_anim_track *track, bool is_begin);
bool svg_anim_parse_value(base_bytes source, svg_xml_span span,
                          svg_anim_attribute attribute, svg_anim_kind kind,
                          const char *transform_type, svg_anim_value *out);
bool svg_anim_split_value_spans(base_bytes source, svg_xml_span values,
                                svg_xml_span *out, size_t capacity,
                                size_t *out_count);
bool svg_anim_base_attribute_value(base_bytes source,
                                   const svg_xml_token *tokens,
                                   size_t token_count, size_t target,
                                   svg_anim_attribute attribute,
                                   svg_anim_kind kind,
                                   const char *transform_type,
                                   svg_anim_value *out);
bool svg_anim_parse_key_times(base_bytes source, svg_xml_span span,
                              svg_anim_track *track);
bool svg_anim_parse_splines(base_bytes source, svg_xml_span span,
                            svg_anim_track *track);
bool svg_anim_parse_repeat(base_bytes source, svg_xml_span span,
                           svg_anim_track *track);
bool svg_anim_parse_calc_fill(base_bytes source,
                              const svg_xml_token *tokens,
                              size_t token_count, size_t element,
                              svg_anim_track *track,
                              size_t *diagnostic_offset,
                              svg_anim_document *document);
bool svg_anim_parse_motion_path(base_bytes source, svg_xml_span path,
                               svg_anim_track *track);
bool svg_anim_resolve_motion_path(base_bytes source,
                                  const svg_xml_token *tokens,
                                  size_t token_count, size_t animation,
                                  svg_xml_span *out_path);

#define span_eq svg_anim_span_eq
#define token_name svg_anim_token_name
#define attr svg_anim_attr
#define attr_or_empty svg_anim_attr_or_empty
#define source_number svg_anim_parse_number
#define time_value svg_anim_parse_time
#define path_structures_match svg_anim_path_structures_match
#define attribute_kind svg_anim_attribute_kind
#define add_diagnostic svg_anim_add_diagnostic
#define element_span svg_anim_element_span
#define parse_sync_time svg_anim_parse_sync_time
#define parse_value svg_anim_parse_value
#define split_value_spans svg_anim_split_value_spans
#define base_attribute_value svg_anim_base_attribute_value
#define parse_key_times svg_anim_parse_key_times
#define parse_splines svg_anim_parse_splines
#define parse_repeat svg_anim_parse_repeat
#define parse_calc_fill svg_anim_parse_calc_fill

#endif
