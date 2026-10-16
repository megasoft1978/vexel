#ifndef VEXEL_SVG_SHAPE_TRAVERSE_INTERNAL_H
#define VEXEL_SVG_SHAPE_TRAVERSE_INTERNAL_H

#include "shape_internal.h"

/* The tokenizer streams, so the source limit is the tokenizer's own 1 GiB input
 * limit; token storage is bounded separately and taken from the stream. */
#define SHAPE_MAX_SOURCE_BYTES (1024U * 1024U * 1024U)
/* Token-storage budget for the decode path: 256 MiB, which at
 * sizeof(svg_xml_token) == 96 bytes holds SHAPE_MAX_TOKEN_BYTES /
 * sizeof(svg_xml_token) = 2,796,202 tokens (floor of 2,796,202.67). */
#define SHAPE_MAX_TOKEN_BYTES (256U * 1024U * 1024U)
#define SHAPE_MAX_BYTES SHAPE_MAX_SOURCE_BYTES
#define SHAPE_MAX_XML_DEPTH 64U
#define SHAPE_MAX_REFERENCE_DEPTH 16U
#define SHAPE_MAX_USE_INSTANCES 65536U
#define SHAPE_MAX_TRAVERSAL_VISITS 4194304U
/* Accumulated transform operations across the whole document. Real drawings
 * carry a transform on most elements, so 4096 refused ordinary files; the bound
 * stays finite so a pathological input is still refused. */
#define SHAPE_MAX_TRANSFORM_OPERATIONS (1024U * 1024U)
#define SHAPE_MAX_TRANSFORM_NODES 4194304U
#define SHAPE_MAX_REFERENCE_BYTES (64U * 1024U)
#define SHAPE_MAX_JOB_REFERENCE_BYTES (1024U * 1024U)

/* Test-only: svg_shape_decode with an explicit token-storage budget in bytes.
 * svg_shape_decode itself uses SHAPE_MAX_TOKEN_BYTES. */
svg_shape_error svg_shape_decode_source_with_token_budget(
    base_bytes source, size_t token_budget_bytes, svg_shape_document *out,
    svg_shape_diagnostic *diagnostic);

/* Root-document sink for ignored constructs. `seen` is a lazily allocated bit
 * per token so a revisited element (use, switch, mask, pattern) counts once. */
typedef struct svg_shape_ignored_state {
    svg_shape_document *document;
    uint8_t *seen;
} svg_shape_ignored_state;

typedef struct shape_traversal {
    base_bytes source;
    base_bytes system_language;
    svg_xml_token *tokens;
    size_t token_count;
    svg_css_sheet *css;
    svg_paint_cache *paint_cache;
    svg_reference_index references;
    svg_shape_builder builder;
    svg_shape_diagnostic *diagnostic;
    size_t active_references[SHAPE_MAX_REFERENCE_DEPTH];
    size_t reference_depth;
    size_t use_instances;
    size_t reference_bytes;
    size_t active_masks[8];
    size_t active_mask_depth;
    size_t mask_instances;
    size_t active_patterns[8];
    size_t active_pattern_depth;
    size_t pattern_instances;
    size_t pattern_bytes;
    size_t active_markers[8];
    size_t active_marker_depth;
    size_t marker_instances;
    size_t marker_style_token_visits;
    size_t element_visits;
    size_t paint_token_visits;
    size_t transform_operations;
    const svg_shape_media_providers *media;
    svg_shape_text_state *text_state;
    svg_shape_text_state text_states[SHAPE_MAX_XML_DEPTH + 1U];
    size_t text_depth;
    bool text_preserve_stack[SHAPE_MAX_XML_DEPTH + 1U];
    size_t selection_element;
    svg_shape_text_cache *text_cache;
    size_t opacity_group_stack[SHAPE_MAX_XML_DEPTH + 1U];
    size_t opacity_group_depth;
    size_t opacity_group_capacity;
    size_t transform_node_stack[SHAPE_MAX_XML_DEPTH + 1U];
    size_t transform_node_capacity;
    double viewport_width;
    double viewport_height;
    svg_shape_ignored_state *ignored; /* shared with nested mask/pattern traversals */
    svg_xml_cancel_check_fn cancel_check;
    void *cancel_context;
    bool canceled;
    bool root_seen;
} shape_traversal;

void svg_shape_traverse_fail(svg_shape_diagnostic *diagnostic,
                             svg_shape_error category, size_t offset);
static inline bool svg_shape_traverse_cancelled(shape_traversal *traversal,
                                                size_t offset)
{
    if (traversal->cancel_check == NULL ||
        !traversal->cancel_check(traversal->cancel_context)) return false;
    traversal->canceled = true;
    svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT, offset);
    return true;
}
static inline bool svg_shape_traverse_matching_end_cancellable(
    shape_traversal *traversal, size_t element, size_t *out_end)
{
    if (traversal->tokens[element].kind == SVG_XML_TOKEN_EMPTY_ELEMENT) {
        *out_end = element;
        return true;
    }
    *out_end = SIZE_MAX;
    for (size_t index = element + 1U; index < traversal->token_count; ++index) {
        if ((index & 255U) == 0U && traversal->cancel_check != NULL &&
            traversal->cancel_check(traversal->cancel_context)) {
            traversal->canceled = true;
            svg_shape_traverse_fail(traversal->diagnostic,
                SVG_SHAPE_ERROR_LIMIT, traversal->tokens[index].source.offset);
            return false;
        }
        if (traversal->tokens[index].kind == SVG_XML_TOKEN_END_ELEMENT &&
            traversal->tokens[index].depth == traversal->tokens[element].depth) {
            *out_end = index;
            break;
        }
    }
    return true;
}
bool svg_shape_traverse_span_eq(base_bytes source, svg_xml_span span,
                                const char *text);
bool svg_shape_traverse_attr(base_bytes source, svg_xml_token *tokens,
                             size_t count, size_t element, const char *local,
                             const char *prefix, svg_xml_span *value);
bool svg_shape_traverse_element_name(base_bytes source,
                                     const svg_xml_token *token,
                                     const char *name);
bool svg_shape_traverse_basic_shape(base_bytes source,
                                    const svg_xml_token *token);
size_t svg_shape_traverse_matching_end(const shape_traversal *traversal,
                                       size_t element);
bool svg_shape_traverse_account_transform(shape_traversal *traversal,
                                          size_t count, size_t offset);
bool svg_shape_traverse_push_transform_node(
    shape_traversal *traversal, size_t source_offset, size_t render_depth,
    size_t parent, geom_affine attribute, geom_affine suffix,
    size_t *out_node);
bool svg_shape_traverse_compute_style(shape_traversal *traversal,
                                      size_t element,
                                      const svg_css_paint *parent,
                                      svg_css_paint *out);
bool svg_shape_traverse_begin_opacity_group(shape_traversal *traversal,
                                            const svg_css_paint *style,
                                            size_t *group);
bool svg_shape_traverse_attach_mask(shape_traversal *traversal,
                                    size_t element, size_t group,
                                    const svg_css_paint *style,
                                    geom_affine transform);
bool svg_shape_traverse_select_switch_child(shape_traversal *traversal,
                                           size_t element, size_t end,
                                           size_t *child_first,
                                           size_t *child_limit, bool *found);
bool svg_shape_traverse_attach_markers(shape_traversal *traversal,
                                       size_t element, size_t first_shape,
                                       size_t end_shape,
                                       const svg_css_paint *style,
                                       size_t render_depth);
void svg_shape_traverse_finish_opacity_group(shape_traversal *traversal,
                                             size_t group);
bool svg_shape_traverse_resolve_use(shape_traversal *traversal, size_t use,
                                    geom_affine parent_transform,
                                    const svg_css_paint *parent_paint,
                                    size_t render_depth);
bool svg_shape_traverse_render_node(shape_traversal *traversal,
                                    size_t element, size_t end,
                                    geom_affine parent_transform,
                                    const svg_css_paint *parent_paint,
                                    size_t render_depth);
/* Like svg_reference_attributes_supported, but unknown non-namespaced attributes
 * and writing-mode are ignored and recorded instead of failing. */
bool svg_shape_traverse_attributes_supported(shape_traversal *traversal,
                                             size_t element, bool root);
void svg_shape_ignored_scan(base_bytes source, const svg_xml_token *tokens,
                            size_t token_count, svg_shape_document *document);
bool svg_shape_traverse_walk_range(shape_traversal *traversal, size_t first,
                                   size_t limit,
                                   geom_affine parent_transform,
                                   const svg_css_paint *parent_paint,
                                   size_t render_depth);

#endif
