#ifndef VEXEL_SVG_REFERENCE_H
#define VEXEL_SVG_REFERENCE_H

#include "base.h"
#include "shape.h"
#include "xml_tokenizer.h"

#include <stdbool.h>
#include <stddef.h>

/* Locate the matching close token for a non-empty definition container. */
bool svg_reference_definition_end(base_bytes source, const svg_xml_token *tokens,
                                  size_t count, size_t start, size_t *end);
bool svg_reference_attributes_supported(base_bytes source, const svg_xml_token *tokens,
                                       size_t count, size_t element, bool root,
                                       svg_xml_span kind);

/* Validate bounded local IDs and reference syntax before expansion. */
svg_shape_error svg_reference_validate(base_bytes source, const svg_xml_token *tokens,
                                       size_t count, svg_shape_diagnostic *diagnostic);
bool svg_reference_xlink_is_bound(base_bytes source, const svg_xml_token *tokens,
                                  size_t count);

/* Creates an owned, bounded SVG source with local use instances expanded. */
svg_shape_error svg_reference_expand(base_bytes source, base_bytes *expanded,
                                     svg_shape_diagnostic *diagnostic);
typedef struct svg_reference_source_map_entry {
    size_t generated_offset, length, source_offset;
} svg_reference_source_map_entry;
typedef struct svg_reference_source_map {
    svg_reference_source_map_entry *entries;
    size_t count, fallback_offset;
} svg_reference_source_map;
svg_shape_error svg_reference_expand_mapped(base_bytes source, base_bytes *expanded,
                                            svg_reference_source_map *map,
                                            svg_shape_diagnostic *diagnostic);
size_t svg_reference_map_offset(const svg_reference_source_map *map,
                                size_t generated_offset);
void svg_reference_source_map_destroy(svg_reference_source_map *map);

#endif
