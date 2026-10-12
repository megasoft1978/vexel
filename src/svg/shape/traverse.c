#include "traverse_internal.h"

#include "reference.h"
#include "../clip/clip.h"
#include "../paint/paint.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

void svg_shape_traverse_fail(svg_shape_diagnostic *diagnostic, svg_shape_error category,
                 size_t offset)
{
    if (diagnostic != NULL)
        *diagnostic = (svg_shape_diagnostic){category, offset};
}

bool svg_shape_traverse_span_eq(base_bytes source, svg_xml_span span, const char *text)
{
    size_t length = strlen(text);
    return span.offset <= source.len && span.length == length &&
           length <= source.len - span.offset &&
           memcmp(source.ptr + span.offset, text, length) == 0;
}

bool svg_shape_traverse_attr(base_bytes source, svg_xml_token *tokens, size_t count,
                 size_t element, const char *local, const char *prefix,
                 svg_xml_span *value)
{
    for (size_t i = element + 1U;
         i < count && tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE; ++i) {
        if (svg_shape_traverse_span_eq(source, tokens[i].local_name, local) &&
            ((prefix == NULL && tokens[i].prefix.length == 0U) ||
             (prefix != NULL && svg_shape_traverse_span_eq(source, tokens[i].prefix, prefix)))) {
            if (value != NULL) *value = tokens[i].value;
            return true;
        }
    }
    return false;
}

bool svg_shape_traverse_element_name(base_bytes source, const svg_xml_token *token,
                         const char *name)
{
    return token->prefix.length == 0U &&
           svg_shape_traverse_span_eq(source, token->local_name, name);
}

bool svg_shape_traverse_basic_shape(base_bytes source,
                                    const svg_xml_token *token)
{
    return svg_shape_traverse_element_name(source, token, "rect") ||
           svg_shape_traverse_element_name(source, token, "path") ||
           svg_shape_traverse_element_name(source, token, "circle") ||
           svg_shape_traverse_element_name(source, token, "ellipse") ||
           svg_shape_traverse_element_name(source, token, "line") ||
           svg_shape_traverse_element_name(source, token, "polygon") ||
           svg_shape_traverse_element_name(source, token, "polyline") ||
           svg_shape_traverse_element_name(source, token, "image") ||
           svg_shape_traverse_element_name(source, token, "text") ||
           svg_shape_traverse_element_name(source, token, "tspan");
}

static bool metadata_element(base_bytes source, const svg_xml_token *token)
{
    return svg_shape_traverse_element_name(source, token, "title") ||
           svg_shape_traverse_element_name(source, token, "desc") ||
           svg_shape_traverse_element_name(source, token, "metadata") ||
           svg_shape_traverse_element_name(source, token, "meta") ||
           (token->prefix.length != 0U &&
            svg_shape_traverse_span_eq(source, token->prefix, "sodipodi") &&
            svg_shape_traverse_span_eq(source, token->local_name, "namedview"));
}

static bool animation_element(base_bytes source, const svg_xml_token *token)
{
    return svg_shape_traverse_element_name(source, token, "animate") ||
           svg_shape_traverse_element_name(source, token, "animateTransform") ||
           svg_shape_traverse_element_name(source, token, "animateMotion") ||
           svg_shape_traverse_element_name(source, token, "animateColor") ||
           svg_shape_traverse_element_name(source, token, "set");
}

size_t svg_shape_traverse_matching_end(const shape_traversal *traversal,
                                       size_t element)
{
    if (traversal->tokens[element].kind == SVG_XML_TOKEN_EMPTY_ELEMENT)
        return element;
    for (size_t i = element + 1U; i < traversal->token_count; ++i) {
        if (traversal->tokens[i].kind == SVG_XML_TOKEN_END_ELEMENT &&
            traversal->tokens[i].depth == traversal->tokens[element].depth)
            return i;
    }
    return SIZE_MAX;
}

static bool source_element_span(const shape_traversal *traversal,
                                size_t element, size_t *offset,
                                size_t *length)
{
    if (element >= traversal->token_count || offset == NULL || length == NULL)
        return false;
    size_t end = svg_shape_traverse_matching_end(traversal, element);
    if (end == SIZE_MAX) return false;
    size_t start_offset = traversal->tokens[element].source.offset;
    size_t end_offset = traversal->tokens[end].source.offset;
    size_t end_length = traversal->tokens[end].source.length;
    if (start_offset > traversal->source.len || end_offset > traversal->source.len ||
        end_length > traversal->source.len - end_offset ||
        end_offset + end_length < start_offset) return false;
    *offset = start_offset;
    *length = end_offset + end_length - start_offset;
    return true;
}

static bool assign_shape_source(shape_traversal *traversal,
                                size_t first_shape, size_t end_shape,
                                size_t element)
{
    size_t offset = 0U, length = 0U;
    if (!source_element_span(traversal, element, &offset, &length)) {
        svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_XML,
             traversal->tokens[element].name.offset);
        return false;
    }
    return svg_shape_source_assign(traversal->builder.doc, first_shape,
                                   end_shape, offset, length);
}

static bool authored_transform(shape_traversal *traversal, size_t element,
                               geom_affine *out)
{
    *out = geom_affine_identity();
    svg_xml_span value;
    if (!svg_shape_traverse_attr(traversal->source, traversal->tokens,
            traversal->token_count, element, "transform", NULL, &value))
        return true;
    size_t operations = 0U;
    if (svg_shape_transform_parse(traversal->source, value, out, &operations))
        return true;
    svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                            value.offset);
    return false;
}

bool svg_shape_traverse_account_transform(shape_traversal *traversal, size_t count,
                              size_t offset)
{
    if (count > SHAPE_MAX_TRANSFORM_OPERATIONS -
                traversal->transform_operations) {
        svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT, offset);
        return false;
    }
    traversal->transform_operations += count;
    return true;
}

bool svg_shape_traverse_push_transform_node(
    shape_traversal *traversal, size_t source_offset, size_t render_depth,
    size_t parent, geom_affine attribute, geom_affine suffix,
    size_t *out_node)
{
    if (traversal == NULL || out_node == NULL ||
        render_depth > SHAPE_MAX_XML_DEPTH ||
        (parent != SVG_SHAPE_NO_TRANSFORM_NODE &&
         parent >= traversal->builder.doc->transform_node_count) ||
        !geom_affine_valid(attribute) || !geom_affine_valid(suffix)) {
        svg_shape_traverse_fail(traversal == NULL ? NULL : traversal->diagnostic,
                                SVG_SHAPE_ERROR_INVALID_ARGUMENT, source_offset);
        return false;
    }
    svg_shape_document *document = traversal->builder.doc;
    if (document->transform_node_count >= SHAPE_MAX_TRANSFORM_NODES) {
        svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
                                source_offset);
        return false;
    }
    if (document->transform_node_count == traversal->transform_node_capacity) {
        size_t next = traversal->transform_node_capacity == 0U ? 64U :
                      traversal->transform_node_capacity;
        if (next > SHAPE_MAX_TRANSFORM_NODES / 2U)
            next = SHAPE_MAX_TRANSFORM_NODES;
        else next *= 2U;
        if (next <= document->transform_node_count ||
            next > SIZE_MAX / sizeof(*document->transform_nodes)) {
            svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
                                    source_offset);
            return false;
        }
        svg_shape_transform_frame *nodes = svg_shape_realloc(
            document->transform_nodes, next * sizeof(*nodes));
        if (nodes == NULL) {
            svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_OOM,
                                    source_offset);
            return false;
        }
        document->transform_nodes = nodes;
        traversal->transform_node_capacity = next;
    }
    size_t index = document->transform_node_count++;
    document->transform_nodes[index] = (svg_shape_transform_frame){
        .source_offset = source_offset,
        .parent = parent,
        .attribute = attribute,
        .suffix = suffix,
        .motion = geom_affine_identity()
    };
    traversal->transform_node_stack[render_depth] = index;
    *out_node = index;
    return true;
}

bool svg_shape_traverse_compute_style(shape_traversal *traversal, size_t element,
                          const svg_css_paint *parent, svg_css_paint *out)
{
    svg_css_error result = svg_css_compute(
        traversal->css, traversal->source, traversal->tokens,
        traversal->token_count, element, parent, out);
    if (result == SVG_CSS_OK) return true;
    svg_shape_traverse_fail(traversal->diagnostic,
         result == SVG_CSS_LIMIT ? SVG_SHAPE_ERROR_LIMIT :
         result == SVG_CSS_OOM ? SVG_SHAPE_ERROR_OOM :
         SVG_SHAPE_ERROR_UNSUPPORTED, 0U);
    return false;
}

static void apply_marker_shorthand(svg_css_paint *style)
{
    svg_xml_span marker = {0};
    if (!svg_css_paint_value(style, SVG_CSS_MARKER, &marker)) return;
    const svg_css_property longhands[] = {
        SVG_CSS_MARKER_START, SVG_CSS_MARKER_MID, SVG_CSS_MARKER_END};
    for (size_t i = 0U; i < sizeof(longhands) / sizeof(longhands[0]); ++i) {
        if (!style->values[longhands[i]].present)
            style->values[longhands[i]] = (svg_css_value){marker, true};
    }
}

bool svg_shape_traverse_begin_opacity_group(shape_traversal *traversal,
                                const svg_css_paint *style,
                                size_t *group)
{
    *group = SVG_SHAPE_NO_GROUP;
    double opacity = 1.0;
    svg_xml_span value = {0};
    if (!svg_shape_style_opacity(traversal->source, style, &opacity, &value)) {
        svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_NUMBER, value.offset);
        return false;
    }
    bool has_filter = false;
    svg_xml_span filter_value = {0};
    if (svg_css_paint_value(style, SVG_CSS_FILTER, &filter_value) &&
        !svg_css_keyword_equals(traversal->source, filter_value, "none"))
        has_filter = true;
    bool has_mask = false;
    svg_xml_span mask_value = {0};
    if (svg_css_paint_value(style, SVG_CSS_MASK, &mask_value) &&
        !svg_css_keyword_equals(traversal->source, mask_value, "none"))
        has_mask = true;
    uint8_t blend_mode = 0U;
    svg_xml_span blend_value = {0};
    if (svg_css_paint_value(style, SVG_CSS_MIX_BLEND_MODE, &blend_value)) {
        static const char *const names[] = {
            "normal", "multiply", "screen", "darken", "lighten"};
        bool found = false;
        for (size_t i = 0U; i < sizeof(names) / sizeof(names[0]); ++i) {
            if (svg_shape_span_eq(traversal->source, blend_value, names[i])) {
                blend_mode = (uint8_t)i;
                found = true;
                break;
            }
        }
        if (!found) {
            svg_shape_traverse_fail(traversal->diagnostic,
                SVG_SHAPE_ERROR_UNSUPPORTED, blend_value.offset);
            return false;
        }
    }
    bool isolation = false;
    svg_xml_span isolation_value = {0};
    if (svg_css_paint_value(style, SVG_CSS_ISOLATION, &isolation_value)) {
        if (svg_css_keyword_equals(traversal->source, isolation_value, "isolate"))
            isolation = true;
        else if (!svg_css_keyword_equals(traversal->source, isolation_value,
                                         "auto")) {
            svg_shape_traverse_fail(traversal->diagnostic,
                SVG_SHAPE_ERROR_UNSUPPORTED, isolation_value.offset);
            return false;
        }
    }
    if (opacity == 1.0 && !has_filter && !has_mask &&
        blend_mode == 0U && !isolation)
        return true;
    if (traversal->opacity_group_depth >= SHAPE_MAX_XML_DEPTH + 1U ||
        traversal->builder.doc->opacity_group_count >= SVG_SHAPE_MAX_EDGES) {
        svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT, value.offset);
        return false;
    }
    svg_shape_document *document = traversal->builder.doc;
    if (document->opacity_group_count == traversal->opacity_group_capacity) {
        size_t next = traversal->opacity_group_capacity == 0U ? 16U :
                      traversal->opacity_group_capacity * 2U;
        if (next < traversal->opacity_group_capacity ||
            next > SVG_SHAPE_MAX_EDGES) next = SVG_SHAPE_MAX_EDGES;
        if (next > SIZE_MAX / sizeof(*document->opacity_groups)) {
            svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT, value.offset);
            return false;
        }
        svg_shape_opacity_group *grown = svg_shape_realloc(
            document->opacity_groups, next * sizeof(*grown));
        if (grown == NULL) {
            svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_OOM, value.offset);
            return false;
        }
        document->opacity_groups = grown;
        traversal->opacity_group_capacity = next;
    }
    size_t index = document->opacity_group_count++;
    size_t parent = traversal->opacity_group_depth == 0U ? SVG_SHAPE_NO_GROUP :
        traversal->opacity_group_stack[traversal->opacity_group_depth - 1U];
    document->opacity_groups[index] = (svg_shape_opacity_group){
        .first_shape = document->shape_count,
        .end_shape = document->shape_count,
        .parent = parent,
        .opacity = opacity,
        .filter_index = SVG_SHAPE_NO_FILTER,
        .mask_index = SVG_SHAPE_NO_MASK,
        .blend_mode = blend_mode,
        .isolation = isolation};
    if (has_filter) {
        size_t filter_index = SVG_SHAPE_NO_FILTER;
        if (!svg_shape_filter_parse(
                traversal->source, traversal->tokens,
                traversal->token_count, &traversal->references, document,
                filter_value, &filter_index, traversal->diagnostic))
            return false;
        svg_xml_span interpolation = {0};
        if (svg_css_paint_value(style, SVG_CSS_COLOR_INTERPOLATION_FILTERS,
                                &interpolation)) {
            if (svg_css_keyword_equals(traversal->source, interpolation,
                                       "sRGB"))
                document->filters[filter_index].linear_rgb = false;
            else if (!svg_css_keyword_equals(traversal->source, interpolation,
                                              "linearRGB") &&
                     !svg_css_keyword_equals(traversal->source, interpolation,
                                              "auto")) {
                svg_shape_traverse_fail(traversal->diagnostic,
                    SVG_SHAPE_ERROR_UNSUPPORTED, interpolation.offset);
                return false;
            }
        }
        document->opacity_groups[index].filter_index = filter_index;
    }
    traversal->opacity_group_stack[traversal->opacity_group_depth++] = index;
    *group = index;
    return true;
}

void svg_shape_traverse_finish_opacity_group(shape_traversal *traversal, size_t group)
{
    if (group == SVG_SHAPE_NO_GROUP) return;
    svg_shape_document *document = traversal->builder.doc;
    document->opacity_groups[group].end_shape = document->shape_count;
    if (traversal->opacity_group_depth != 0U &&
        traversal->opacity_group_stack[traversal->opacity_group_depth - 1U] == group)
        traversal->opacity_group_depth--;
}

bool svg_shape_traverse_walk_range(shape_traversal *traversal, size_t first, size_t limit,
                       geom_affine parent_transform,
                       const svg_css_paint *parent_paint,
                       size_t render_depth);
bool svg_shape_traverse_render_node(shape_traversal *traversal, size_t element,
                        size_t end, geom_affine parent_transform,
                        const svg_css_paint *parent_paint,
                        size_t render_depth);

bool svg_shape_traverse_render_node(shape_traversal *traversal, size_t element,
                        size_t end, geom_affine parent_transform,
                        const svg_css_paint *parent_paint,
                        size_t render_depth)
{
    const svg_xml_token *token = &traversal->tokens[element];
    bool text_element = svg_shape_traverse_element_name(traversal->source, token, "text");
    bool tspan_element = svg_shape_traverse_element_name(traversal->source, token, "tspan");
    bool link_element = svg_shape_traverse_element_name(traversal->source, token, "a");
    bool switch_element = svg_shape_traverse_element_name(traversal->source, token, "switch");
    bool text_container = text_element || tspan_element;
    bool root = token->depth == 0U && !traversal->root_seen;
    if (render_depth > SHAPE_MAX_XML_DEPTH) {
        svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT, token->name.offset);
        return false;
    }
    if (token->prefix.length != 0U ||
        svg_shape_traverse_element_name(traversal->source, token, "textPath") ||
        (root && !svg_shape_traverse_element_name(traversal->source, token, "svg")) ||
        (!root && !svg_shape_traverse_element_name(traversal->source, token, "g") &&
         !svg_shape_traverse_element_name(traversal->source, token, "svg") &&
         !svg_shape_traverse_element_name(traversal->source, token, "use") &&
         !link_element && !switch_element &&
         !svg_shape_traverse_basic_shape(traversal->source, token))) {
        svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
             token->name.offset);
        return false;
    }
    if (tspan_element && traversal->text_state == NULL) {
        svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
             token->name.offset);
        return false;
    }
    if (root) {
        if (traversal->root_seen) {
            svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                 token->name.offset);
            return false;
        }
        traversal->root_seen = true;
    }

    if (svg_shape_traverse_element_name(traversal->source, token, "use"))
        return svg_shape_traverse_resolve_use(traversal, element, parent_transform,
                           parent_paint, render_depth);

    if (!svg_shape_traverse_attributes_supported(traversal, element, root)) {
        svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
             token->name.offset);
        return false;
    }
    svg_css_paint style;
    if (!svg_shape_traverse_compute_style(traversal, element, parent_paint, &style)) return false;
    apply_marker_shorthand(&style);
    bool display_none = false;
    if (!svg_shape_style_display_none(traversal->source, &style,
            token->name.offset, &display_none, traversal->diagnostic)) return false;
    if (display_none) return true;

    geom_affine current;
    size_t local_operations = 0U;
    if (!svg_shape_transform_node(traversal->source, traversal->tokens,
                                  traversal->token_count, element,
                                  parent_transform, &current,
                                  &local_operations, traversal->diagnostic) ||
        !svg_shape_traverse_account_transform(traversal, local_operations, token->name.offset))
        return false;
    if (root && !svg_shape_root_viewport(
            traversal->source, traversal->tokens, traversal->token_count,
            element, traversal->builder.doc, &traversal->viewport_width,
            &traversal->viewport_height, traversal->diagnostic)) return false;

    bool nested_svg = !root && svg_shape_traverse_element_name(traversal->source, token, "svg");
    geom_point viewport_clip[4] = {{0}};
    double child_width = traversal->viewport_width;
    double child_height = traversal->viewport_height;
    bool viewport_empty = false;
    geom_affine viewport_transform = geom_affine_identity();
    geom_affine viewport_owner_transform = current;
    if (nested_svg) {
        if (!svg_shape_nested_viewport(
                traversal->source, traversal->tokens,
                traversal->token_count, element, traversal->viewport_width,
                traversal->viewport_height, current, &current,
                &viewport_transform, viewport_clip, &child_width, &child_height,
                &viewport_empty,
                traversal->diagnostic)) return false;
        if (viewport_empty) return true;
    }

    geom_affine local_attribute;
    size_t transform_parent = render_depth == 0U ?
        SVG_SHAPE_NO_TRANSFORM_NODE :
        traversal->transform_node_stack[render_depth - 1U];
    size_t transform_node = SVG_SHAPE_NO_TRANSFORM_NODE;
    if (!authored_transform(traversal, element, &local_attribute) ||
        !svg_shape_traverse_push_transform_node(
            traversal, token->source.offset, render_depth, transform_parent,
            local_attribute, viewport_transform, &transform_node)) return false;
    traversal->builder.transform_node = transform_node;

    size_t opacity_group = SVG_SHAPE_NO_GROUP;
    if (!svg_shape_traverse_begin_opacity_group(traversal, &style, &opacity_group)) return false;
    traversal->builder.viewport_width = traversal->viewport_width;
    traversal->builder.viewport_height = traversal->viewport_height;

    size_t direct_first_shape = traversal->builder.doc->shape_count;
    size_t direct_first_edge = traversal->builder.doc->edge_count;
    if (!root && !svg_shape_traverse_element_name(traversal->source, token, "g") &&
        !nested_svg && !link_element && !switch_element) {
        if (text_container) {
            svg_shape_text_state *previous_state = traversal->text_state;
            if (traversal->text_depth > SHAPE_MAX_XML_DEPTH) {
                svg_shape_traverse_fail(traversal->diagnostic,
                    SVG_SHAPE_ERROR_LIMIT, token->name.offset);
                svg_shape_traverse_finish_opacity_group(traversal, opacity_group);
                return false;
            }
            if (text_element) {
                if (previous_state != NULL) {
                    svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                         token->name.offset);
                    svg_shape_traverse_finish_opacity_group(traversal, opacity_group);
                    return false;
                }
                traversal->text_states[traversal->text_depth] =
                    (svg_shape_text_state){0};
                traversal->text_state =
                    &traversal->text_states[traversal->text_depth];
            }
            svg_shape_text_state *state = traversal->text_state;
            if (state == NULL) {
                svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
                     token->name.offset);
                traversal->text_state = previous_state;
                svg_shape_traverse_finish_opacity_group(traversal, opacity_group);
                return false;
            }
            size_t handle = traversal->selection_element == SIZE_MAX ?
                            element : traversal->selection_element;
            size_t source_offset = 0U, source_length = 0U;
            if (!source_element_span(traversal, handle, &source_offset,
                                     &source_length)) {
                svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_XML,
                     token->name.offset);
                traversal->text_state = previous_state;
                svg_shape_traverse_finish_opacity_group(traversal, opacity_group);
                return false;
            }
            bool parent_preserve = traversal->text_depth == 0U ? false :
                traversal->text_preserve_stack[traversal->text_depth - 1U];
            bool preserve = parent_preserve;
            svg_css_paint text_style = style;
            text_style.values[SVG_CSS_CLIP_PATH] = (svg_css_value){0};
            if (!svg_shape_text_render_element(
                    &traversal->builder, traversal->source, traversal->tokens,
                    traversal->token_count, element, end, &text_style, current,
                    source_offset, source_length, state, parent_preserve,
                    &preserve)) {
                traversal->text_state = previous_state;
                svg_shape_traverse_finish_opacity_group(traversal, opacity_group);
                return false;
            }
            if (traversal->text_depth >= SHAPE_MAX_XML_DEPTH + 1U) {
                svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
                     token->name.offset);
                traversal->text_state = previous_state;
                svg_shape_traverse_finish_opacity_group(traversal, opacity_group);
                return false;
            }
            traversal->text_preserve_stack[traversal->text_depth++] = preserve;
            if (text_element) traversal->text_state = state;
        } else if (svg_shape_traverse_element_name(traversal->source, token, "image")) {
            size_t first = traversal->builder.doc->shape_count;
            if (!svg_shape_builder_add_image_element(
                    &traversal->builder, traversal->source,
                    traversal->tokens, traversal->token_count, element,
                    &style, current) ||
                !assign_shape_source(traversal, first,
                    traversal->builder.doc->shape_count,
                    traversal->selection_element == SIZE_MAX ? element :
                    traversal->selection_element)) {
                svg_shape_traverse_finish_opacity_group(traversal, opacity_group);
                return false;
            }
        } else {
            size_t first = traversal->builder.doc->shape_count;
            if (!svg_shape_builder_add_element(
                    &traversal->builder, traversal->source,
                    traversal->tokens, traversal->token_count, element,
                    &style, current) ||
                !assign_shape_source(traversal, first,
                    traversal->builder.doc->shape_count,
                    traversal->selection_element == SIZE_MAX ? element :
                    traversal->selection_element) ||
                !svg_shape_traverse_attach_markers(
                    traversal, element, first,
                    traversal->builder.doc->shape_count, &style,
                    render_depth)) {
                svg_shape_traverse_finish_opacity_group(traversal, opacity_group);
                return false;
            }
        }
    }
    for (size_t shape = direct_first_shape;
         shape < traversal->builder.doc->shape_count; ++shape)
        traversal->builder.doc->shape_sources[shape].transform_node =
            transform_node;

    if (end == element) {
        if (text_container && traversal->text_depth != 0U) {
            traversal->text_depth--;
            if (text_element) traversal->text_state = NULL;
        }
        if (!svg_shape_traverse_attach_mask(traversal, element, opacity_group,
                                             &style, current)) {
            svg_shape_traverse_finish_opacity_group(traversal, opacity_group);
            return false;
        }
        svg_shape_traverse_finish_opacity_group(traversal, opacity_group);
        return true;
    }
    size_t first_shape = text_container ? direct_first_shape :
                         traversal->builder.doc->shape_count;
    size_t first_edge = text_container ? direct_first_edge :
                        traversal->builder.doc->edge_count;
    double previous_width = traversal->viewport_width;
    double previous_height = traversal->viewport_height;
    if (nested_svg) {
        traversal->viewport_width = child_width;
        traversal->viewport_height = child_height;
    }
    traversal->builder.viewport_width = traversal->viewport_width;
    traversal->builder.viewport_height = traversal->viewport_height;
    size_t child_first = element + 1U;
    size_t child_limit = end;
    bool selected_child = true;
    if (switch_element && !svg_shape_traverse_select_switch_child(
            traversal, element, end, &child_first, &child_limit,
            &selected_child)) {
        traversal->viewport_width = previous_width;
        traversal->viewport_height = previous_height;
        traversal->builder.viewport_width = previous_width;
        traversal->builder.viewport_height = previous_height;
        svg_shape_traverse_finish_opacity_group(traversal, opacity_group);
        return false;
    }
    bool walked = !selected_child || svg_shape_traverse_walk_range(
        traversal, child_first, child_limit, current, &style, render_depth + 1U);
    traversal->viewport_width = previous_width;
    traversal->viewport_height = previous_height;
    traversal->builder.viewport_width = previous_width;
    traversal->builder.viewport_height = previous_height;
    if (!walked) {
        if (text_container && traversal->text_depth != 0U) {
            traversal->text_depth--;
            if (text_element) traversal->text_state = NULL;
        }
        svg_shape_traverse_finish_opacity_group(traversal, opacity_group);
        return false;
    }
    if (nested_svg && traversal->builder.doc->shape_count != first_shape) {
        geom_affine viewport_owner_inverse;
        geom_point local_clip[4];
        if (!svg_shape_affine_inverse(viewport_owner_transform,
                                      &viewport_owner_inverse)) {
            svg_shape_traverse_fail(traversal->diagnostic,
                SVG_SHAPE_ERROR_UNSUPPORTED, token->name.offset);
            svg_shape_traverse_finish_opacity_group(traversal, opacity_group);
            return false;
        }
        for (size_t point = 0U; point < 4U; ++point) {
            if (!geom_affine_map_point(viewport_owner_inverse,
                                       viewport_clip[point],
                                       &local_clip[point])) {
                svg_shape_traverse_fail(traversal->diagnostic,
                    SVG_SHAPE_ERROR_UNSUPPORTED, token->name.offset);
                svg_shape_traverse_finish_opacity_group(traversal,
                                                        opacity_group);
                return false;
            }
        }
        svg_shape_clip *viewport = svg_clip_create_rect(
            local_clip[0], local_clip[1], local_clip[2], local_clip[3],
            viewport_owner_inverse, transform_node);
        if (viewport == NULL) {
            svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_OOM,
                 token->name.offset);
            svg_shape_traverse_finish_opacity_group(traversal, opacity_group);
            return false;
        }
        bool applied = svg_clip_apply_group(
            traversal->builder.doc, first_shape,
            traversal->builder.doc->shape_count, viewport);
        svg_clip_destroy(viewport);
        if (!applied) {
            svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
                 token->name.offset);
            svg_shape_traverse_finish_opacity_group(traversal, opacity_group);
            return false;
        }
        traversal->builder.doc->has_clips = true;
    }
    svg_xml_span clip_value;
    if ((root || svg_shape_traverse_element_name(traversal->source, token, "g") || nested_svg ||
         link_element || switch_element || text_container) &&
        svg_css_paint_value(&style, SVG_CSS_CLIP_PATH, &clip_value) &&
        !svg_css_keyword_equals(traversal->source, clip_value, "none")) {
        bool clipped = svg_shape_apply_declared_clip(traversal->source, traversal->tokens,
            traversal->token_count, &traversal->references, traversal->css,
            traversal->builder.doc, clip_value, first_shape, first_edge,
            transform_node, current, traversal->diagnostic);
        bool masked = clipped && svg_shape_traverse_attach_mask(
            traversal, element, opacity_group, &style, current);
        svg_shape_traverse_finish_opacity_group(traversal, opacity_group);
        return clipped && masked;
    }
    if (text_container && traversal->text_depth != 0U) {
        traversal->text_depth--;
        if (text_element) traversal->text_state = NULL;
    }
    if (!svg_shape_traverse_attach_mask(traversal, element, opacity_group,
                                         &style, current)) {
        svg_shape_traverse_finish_opacity_group(traversal, opacity_group);
        return false;
    }
    svg_shape_traverse_finish_opacity_group(traversal, opacity_group);
    return true;
}

bool svg_shape_traverse_walk_range(shape_traversal *traversal, size_t first, size_t limit,
                       geom_affine parent_transform,
                       const svg_css_paint *parent_paint,
                       size_t render_depth)
{
    size_t i = first;
    while (i < limit) {
        svg_xml_token *token = &traversal->tokens[i];
        if (token->kind == SVG_XML_TOKEN_PROCESSING_INSTRUCTION) {
            svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                 token->name.offset);
            return false;
        }
        if (token->kind == SVG_XML_TOKEN_CDATA) {
            if (traversal->text_state != NULL) {
                i++;
                continue;
            }
            svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                 token->value.offset);
            return false;
        }
        if (token->kind == SVG_XML_TOKEN_TEXT) {
            if (traversal->text_state != NULL) {
                i++;
                continue;
            }
            for (size_t j = 0U; j < token->value.length; ++j) {
                uint8_t byte = traversal->source.ptr[token->value.offset + j];
                if (byte != ' ' && byte != '\t' && byte != '\r' && byte != '\n') {
                    svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                         token->value.offset + j);
                    return false;
                }
            }
            i++;
            continue;
        }
        if (token->kind != SVG_XML_TOKEN_START_ELEMENT &&
            token->kind != SVG_XML_TOKEN_EMPTY_ELEMENT) {
            i++;
            continue;
        }
        size_t end = svg_shape_traverse_matching_end(traversal, i);
        if (end == SIZE_MAX || end >= limit) {
            svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_XML,
                 token->name.offset);
            return false;
        }
        if (traversal->element_visits >= SHAPE_MAX_TRAVERSAL_VISITS) {
            svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
                 token->name.offset);
            return false;
        }
        traversal->element_visits++;

        size_t definition_end;
        if (svg_reference_definition_end(traversal->source, traversal->tokens,
                                         traversal->token_count, i,
                                         &definition_end)) {
            i = end + 1U;
            continue;
        }
        if (metadata_element(traversal->source, token)) {
            i = end + 1U;
            continue;
        }
        if (animation_element(traversal->source, token)) {
            i = end + 1U;
            continue;
        }
        if (svg_css_is_style_element(traversal->source, token)) {
            i = end + 1U;
            continue;
        }
        if (token->depth > SHAPE_MAX_XML_DEPTH) {
            svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
                 token->name.offset);
            return false;
        }
        if (!svg_shape_traverse_render_node(traversal, i, end, parent_transform, parent_paint,
                         render_depth)) return false;
        i = end + 1U;
    }
    return true;
}
