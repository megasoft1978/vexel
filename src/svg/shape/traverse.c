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
           (token->prefix.length != 0U &&
            svg_shape_traverse_span_eq(source, token->prefix, "sodipodi") &&
            svg_shape_traverse_span_eq(source, token->local_name, "namedview"));
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
    if (opacity == 1.0) return true;
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
        document->shape_count, document->shape_count, parent, opacity};
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

    if (!svg_reference_attributes_supported(
            traversal->source, traversal->tokens, traversal->token_count,
            element, root, token->local_name)) {
        svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
             token->name.offset);
        return false;
    }
    svg_css_paint style;
    if (!svg_shape_traverse_compute_style(traversal, element, parent_paint, &style)) return false;
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
    if (nested_svg) {
        if (!svg_shape_nested_viewport(
                traversal->source, traversal->tokens,
                traversal->token_count, element, traversal->viewport_width,
                traversal->viewport_height, current, &current, viewport_clip,
                &child_width, &child_height, &viewport_empty,
                traversal->diagnostic)) return false;
        if (viewport_empty) return true;
    }

    size_t opacity_group = SVG_SHAPE_NO_GROUP;
    if (!svg_shape_traverse_begin_opacity_group(traversal, &style, &opacity_group)) return false;
    traversal->builder.viewport_width = traversal->viewport_width;
    traversal->builder.viewport_height = traversal->viewport_height;

    size_t direct_first_shape = traversal->builder.doc->shape_count;
    size_t direct_first_edge = traversal->builder.doc->edge_count;
    if (!root && !svg_shape_traverse_element_name(traversal->source, token, "g") &&
        !nested_svg) {
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
                    traversal->selection_element)) {
                svg_shape_traverse_finish_opacity_group(traversal, opacity_group);
                return false;
            }
        }
    }

    if (end == element) {
        if (text_container && traversal->text_depth != 0U) {
            traversal->text_depth--;
            if (text_element) traversal->text_state = NULL;
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
    bool walked = svg_shape_traverse_walk_range(traversal, element + 1U, end, current, &style,
                             render_depth + 1U);
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
        svg_shape_clip *viewport = svg_clip_create_rect(
            viewport_clip[0], viewport_clip[1], viewport_clip[2],
            viewport_clip[3]);
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
         text_container) &&
        svg_css_paint_value(&style, SVG_CSS_CLIP_PATH, &clip_value) &&
        !svg_css_keyword_equals(traversal->source, clip_value, "none")) {
        bool clipped = svg_shape_apply_declared_clip(traversal->source, traversal->tokens,
            traversal->token_count, &traversal->references, traversal->css,
            traversal->builder.doc, clip_value, first_shape, first_edge,
            current, traversal->diagnostic);
        svg_shape_traverse_finish_opacity_group(traversal, opacity_group);
        return clipped;
    }
    if (text_container && traversal->text_depth != 0U) {
        traversal->text_depth--;
        if (text_element) traversal->text_state = NULL;
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

svg_shape_error svg_shape_decode_source(base_bytes source,
                                        svg_shape_document *out,
                                        svg_shape_diagnostic *diagnostic)
{
    return svg_shape_decode_source_with_bounds(source, out, diagnostic, NULL);
}

svg_shape_error svg_shape_decode_source_with_media(
    base_bytes source, const svg_shape_media_providers *providers,
    svg_shape_document *out, svg_shape_diagnostic *diagnostic)
{
    return svg_shape_decode_source_with_media_and_bounds(
        source, providers, out, diagnostic, NULL);
}

svg_shape_error svg_shape_decode_source_with_bounds(
    base_bytes source, svg_shape_document *out,
    svg_shape_diagnostic *diagnostic,
    svg_shape_decode_memory_bounds *memory_bounds)
{
    return svg_shape_decode_source_with_media_and_bounds(
        source, NULL, out, diagnostic, memory_bounds);
}

svg_shape_error svg_shape_decode_source_with_media_and_bounds(
    base_bytes source, const svg_shape_media_providers *providers,
    svg_shape_document *out, svg_shape_diagnostic *diagnostic,
    svg_shape_decode_memory_bounds *memory_bounds)
{
    svg_shape_diagnostic local_diagnostic;
    if (diagnostic == NULL) diagnostic = &local_diagnostic;
    if (memory_bounds != NULL)
        *memory_bounds = (svg_shape_decode_memory_bounds){0};
    if (out == NULL || !base_bytes_valid(source)) {
        svg_shape_traverse_fail(diagnostic, SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U);
        return SVG_SHAPE_ERROR_INVALID_ARGUMENT;
    }
    *out = (svg_shape_document){0};
    *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_NONE, 0U};
    if (source.len > SHAPE_MAX_BYTES) {
        svg_shape_traverse_fail(diagnostic, SVG_SHAPE_ERROR_LIMIT, SHAPE_MAX_BYTES);
        return SVG_SHAPE_ERROR_LIMIT;
    }
    size_t capacity = source.len / 2U + 2U;
    if (capacity > SIZE_MAX / sizeof(svg_xml_token)) {
        svg_shape_traverse_fail(diagnostic, SVG_SHAPE_ERROR_LIMIT, 0U);
        return SVG_SHAPE_ERROR_LIMIT;
    }
    svg_xml_token *tokens = svg_shape_calloc(capacity, sizeof(*tokens));
    if (tokens == NULL) {
        svg_shape_traverse_fail(diagnostic, SVG_SHAPE_ERROR_OOM, 0U);
        return SVG_SHAPE_ERROR_OOM;
    }
    (void)svg_shape_alloc_set_tag(tokens, SVG_SHAPE_ALLOC_TAG_TOKEN_ARRAY);
    svg_xml_result xml = svg_xml_tokenize(source, svg_xml_default_options(),
                                          tokens, capacity);
    if (xml.diagnostic.category != SVG_XML_ERROR_NONE) {
        svg_shape_free(tokens);
        svg_shape_traverse_fail(diagnostic, SVG_SHAPE_ERROR_XML, xml.diagnostic.offset);
        return SVG_SHAPE_ERROR_XML;
    }

    shape_traversal traversal = {
        .source = source,
        .tokens = tokens,
        .token_count = xml.token_count,
        .diagnostic = diagnostic,
        .media = providers,
        .selection_element = SIZE_MAX
    };
    svg_shape_error error = svg_reference_index_create(
        source, tokens, xml.token_count, &traversal.references, diagnostic);
    if (error != SVG_SHAPE_ERROR_NONE) goto cleanup;
    if (memory_bounds != NULL)
        memory_bounds->reference_index_bytes =
            svg_reference_index_storage_bytes(&traversal.references);
    svg_css_error css_error = svg_css_sheet_create(
        source, tokens, xml.token_count, &traversal.css);
    if (css_error != SVG_CSS_OK) {
        error = css_error == SVG_CSS_LIMIT ? SVG_SHAPE_ERROR_LIMIT :
                                            SVG_SHAPE_ERROR_OOM;
        svg_shape_traverse_fail(diagnostic, error, 0U);
        goto cleanup;
    }
    if (providers != NULL && providers->layout_text != NULL) {
        traversal.text_cache = svg_shape_text_cache_create(providers);
        if (traversal.text_cache == NULL) {
            svg_shape_traverse_fail(diagnostic, SVG_SHAPE_ERROR_OOM, 0U);
            error = SVG_SHAPE_ERROR_OOM;
            goto cleanup;
        }
    }
    traversal.builder = (svg_shape_builder){
        .doc = out,
        .references = &traversal.references,
        .css = traversal.css,
        .paint_cache = &traversal.paint_cache,
        .paint_token_visits = &traversal.paint_token_visits,
        .diag = diagnostic,
        .transform = geom_affine_identity(),
        .media = providers,
        .text_cache = traversal.text_cache
    };
    out->source = source;
    if (providers != NULL) {
        out->media_context = providers->context;
        out->release_image = providers->release_image;
    }
    if (!svg_shape_traverse_walk_range(&traversal, 0U, xml.token_count,
                    geom_affine_identity(), NULL, 0U)) {
        error = diagnostic->category;
        goto cleanup;
    }
    if (memory_bounds != NULL)
        memory_bounds->reference_lookup_probes =
            svg_reference_index_lookup_probes(&traversal.references);
    if (memory_bounds != NULL)
        memory_bounds->paint_token_visits = traversal.paint_token_visits;
    if (!traversal.root_seen) {
        svg_shape_traverse_fail(diagnostic, SVG_SHAPE_ERROR_VIEWPORT, 0U);
        error = SVG_SHAPE_ERROR_VIEWPORT;
        goto cleanup;
    }
    error = SVG_SHAPE_ERROR_NONE;

cleanup:
    svg_shape_text_cache_destroy(traversal.text_cache);
    svg_paint_cache_destroy(traversal.paint_cache);
    svg_css_sheet_destroy(traversal.css);
    svg_reference_index_destroy(&traversal.references);
    svg_shape_free(tokens);
    if (error != SVG_SHAPE_ERROR_NONE) svg_shape_document_destroy(out);
    return error;
}
