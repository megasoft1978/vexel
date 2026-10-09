#ifndef VEXEL_SVG_REFERENCE_H
#define VEXEL_SVG_REFERENCE_H

#include "base.h"
#include "shape.h"
#include "xml_tokenizer.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define SVG_REFERENCE_MAX_IDS 65536U
#define SVG_REFERENCE_ID_TABLE_CAPACITY 131072U

typedef struct svg_reference_index {
    uint32_t *slots;
    size_t id_count;
    size_t lookup_probes;
} svg_reference_index;

/* Locate the matching close token for a non-empty definition container. */
bool svg_reference_definition_end(base_bytes source, const svg_xml_token *tokens,
                                  size_t count, size_t start, size_t *end);
bool svg_reference_attributes_supported(base_bytes source, const svg_xml_token *tokens,
                                       size_t count, size_t element, bool root,
                                       svg_xml_span kind);

/* Fixed-capacity open-addressed table. A nonempty index always allocates the
 * same number of slots, independent of source length or the number of IDs. */
svg_shape_error svg_reference_index_create(base_bytes source,
                                           const svg_xml_token *tokens,
                                           size_t count,
                                           svg_reference_index *index,
                                           svg_shape_diagnostic *diagnostic);
void svg_reference_index_destroy(svg_reference_index *index);
bool svg_reference_index_find(base_bytes source, const svg_xml_token *tokens,
                              size_t count, svg_reference_index *index,
                              svg_xml_span id, size_t *element);
size_t svg_reference_index_storage_bytes(const svg_reference_index *index);
size_t svg_reference_index_lookup_probes(const svg_reference_index *index);
bool svg_reference_xlink_is_bound(base_bytes source, const svg_xml_token *tokens,
                                  size_t count);
bool svg_reference_use_attributes_supported(base_bytes source,
                                            const svg_xml_token *tokens,
                                            size_t count, size_t element);

#endif
