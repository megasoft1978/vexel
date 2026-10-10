#include "shape_internal.h"

#include "reference.h"
#include "../clip/clip.h"
#include "../paint/paint.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

#define SHAPE_MAX_BYTES (16U * 1024U * 1024U)
#define SHAPE_MAX_XML_DEPTH 64U
#define SHAPE_MAX_REFERENCE_DEPTH 16U
#define SHAPE_MAX_USE_INSTANCES 65536U
#define SHAPE_MAX_TRAVERSAL_VISITS 4194304U
#define SHAPE_MAX_TRANSFORM_OPERATIONS 4096U
#define SHAPE_MAX_REFERENCE_BYTES (64U * 1024U)
#define SHAPE_MAX_JOB_REFERENCE_BYTES (1024U * 1024U)

typedef struct shape_traversal {
    base_bytes source;
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
    size_t element_visits;
    size_t paint_token_visits;
    size_t transform_operations;
    bool root_seen;
} shape_traversal;

static void fail(svg_shape_diagnostic *diagnostic, svg_shape_error category,
                 size_t offset)
{
    if (diagnostic != NULL)
        *diagnostic = (svg_shape_diagnostic){category, offset};
}

static bool span_eq(base_bytes source, svg_xml_span span, const char *text)
{
    size_t length = strlen(text);
    return span.offset <= source.len && span.length == length &&
           length <= source.len - span.offset &&
           memcmp(source.ptr + span.offset, text, length) == 0;
}

static bool attr(base_bytes source, svg_xml_token *tokens, size_t count,
                 size_t element, const char *local, const char *prefix,
                 svg_xml_span *value)
{
    for (size_t i = element + 1U;
         i < count && tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE; ++i) {
        if (span_eq(source, tokens[i].local_name, local) &&
            ((prefix == NULL && tokens[i].prefix.length == 0U) ||
             (prefix != NULL && span_eq(source, tokens[i].prefix, prefix)))) {
            if (value != NULL) *value = tokens[i].value;
            return true;
        }
    }
    return false;
}

static bool element_name(base_bytes source, const svg_xml_token *token,
                         const char *name)
{
    return token->prefix.length == 0U &&
           span_eq(source, token->local_name, name);
}

static bool basic_shape(base_bytes source, const svg_xml_token *token)
{
    return element_name(source, token, "rect") ||
           element_name(source, token, "path") ||
           element_name(source, token, "circle") ||
           element_name(source, token, "ellipse") ||
           element_name(source, token, "line") ||
           element_name(source, token, "polygon") ||
           element_name(source, token, "polyline");
}

static size_t matching_end(const shape_traversal *traversal, size_t element)
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

static bool account_transform(shape_traversal *traversal, size_t count,
                              size_t offset)
{
    if (count > SHAPE_MAX_TRANSFORM_OPERATIONS -
                traversal->transform_operations) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT, offset);
        return false;
    }
    traversal->transform_operations += count;
    return true;
}

static bool compute_style(shape_traversal *traversal, size_t element,
                          const svg_css_paint *parent, svg_css_paint *out)
{
    svg_css_error result = svg_css_compute(
        traversal->css, traversal->source, traversal->tokens,
        traversal->token_count, element, parent, out);
    if (result == SVG_CSS_OK) return true;
    fail(traversal->diagnostic,
         result == SVG_CSS_LIMIT ? SVG_SHAPE_ERROR_LIMIT : SVG_SHAPE_ERROR_OOM,
         0U);
    return false;
}

static bool apply_declared_clip(shape_traversal *traversal,
                                svg_xml_span value, size_t first_shape,
                                size_t first_edge, geom_affine transform)
{
    svg_shape_document *document = traversal->builder.doc;
    size_t end_shape = document->shape_count;
    if (first_shape == end_shape) return true;
    geom_affine inverse;
    double bounds[4];
    if (!svg_shape_affine_inverse(transform, &inverse) ||
        first_edge > document->edge_count ||
        !svg_clip_local_bounds(document, first_edge,
                               document->edge_count - first_edge,
                               inverse, bounds)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, value.offset);
        return false;
    }
    svg_shape_clip *clip = NULL;
    svg_shape_error result = svg_clip_resolve(
        traversal->source, traversal->tokens, traversal->token_count,
        &traversal->references, traversal->css, value, document,
        &document->clip_bytes,
        inverse, bounds, &clip, traversal->diagnostic);
    if (result != SVG_SHAPE_ERROR_NONE) return false;
    bool applied = svg_clip_apply_group(document, first_shape, end_shape, clip);
    svg_clip_destroy(clip);
    if (!applied) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT, value.offset);
        return false;
    }
    return true;
}

static bool root_viewport(shape_traversal *traversal, size_t element)
{
    base_bytes source = traversal->source;
    svg_xml_token *tokens = traversal->tokens;
    size_t count = traversal->token_count;
    svg_shape_document *document = traversal->builder.doc;
    svg_xml_span value;
    if (attr(source, tokens, count, element, "xmlns", NULL, &value) &&
        !span_eq(source, value, "http://www.w3.org/2000/svg")) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, value.offset);
        return false;
    }
    svg_xml_span width_span, height_span;
    if (!attr(source, tokens, count, element, "width", NULL, &width_span) ||
        !attr(source, tokens, count, element, "height", NULL, &height_span)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_VIEWPORT,
             tokens[element].name.offset);
        return false;
    }
    double width, height;
    if (!svg_shape_read_dimension(source, width_span, &width)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_NUMBER, width_span.offset);
        return false;
    }
    if (!svg_shape_read_dimension(source, height_span, &height)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_NUMBER, height_span.offset);
        return false;
    }
    if (width < 1.0 || width > 4096.0) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_VIEWPORT, width_span.offset);
        return false;
    }
    if (height < 1.0 || height > 4096.0) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_VIEWPORT, height_span.offset);
        return false;
    }
    document->width = (uint32_t)ceil(width);
    document->height = (uint32_t)ceil(height);
    document->view_x = 0.0;
    document->view_y = 0.0;
    document->view_width = width;
    document->view_height = height;

    if (attr(source, tokens, count, element, "viewBox", NULL, &value)) {
        double view_box[4];
        if (!svg_shape_read_number_list(source, value, view_box, 4U) ||
            view_box[2] <= 0.0 || view_box[3] <= 0.0 ||
            fabs(view_box[0] + view_box[2]) > 1000000.0 ||
            fabs(view_box[1] + view_box[3]) > 1000000.0) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_VIEWPORT, value.offset);
            return false;
        }
        document->view_x = view_box[0];
        document->view_y = view_box[1];
        document->view_width = view_box[2];
        document->view_height = view_box[3];
    }
    return true;
}

static bool symbol_attributes_supported(shape_traversal *traversal,
                                        size_t element,
                                        size_t *unsupported_offset)
{
    base_bytes source = traversal->source;
    for (size_t i = element + 1U;
         i < traversal->token_count &&
         traversal->tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE; ++i) {
        svg_xml_span name = traversal->tokens[i].local_name;
        if (traversal->tokens[i].prefix.length != 0U) {
            if (unsupported_offset != NULL)
                *unsupported_offset = traversal->tokens[i].name.offset;
            return false;
        }
        bool supported = span_eq(source, name, "id") ||
            span_eq(source, name, "class") || span_eq(source, name, "style") ||
            span_eq(source, name, "viewBox") ||
            span_eq(source, name, "preserveAspectRatio") ||
            span_eq(source, name, "width") || span_eq(source, name, "height") ||
            span_eq(source, name, "fill") || span_eq(source, name, "fill-rule") ||
            span_eq(source, name, "stroke") ||
            span_eq(source, name, "stroke-width") ||
            span_eq(source, name, "stroke-linecap") ||
            span_eq(source, name, "stroke-linejoin") ||
            span_eq(source, name, "stroke-miterlimit") ||
            span_eq(source, name, "stroke-dasharray") ||
            span_eq(source, name, "stroke-dashoffset");
        if (!supported) {
            if (unsupported_offset != NULL)
                *unsupported_offset = traversal->tokens[i].name.offset;
            return false;
        }
    }
    return true;
}

static bool symbol_mapping(shape_traversal *traversal, size_t target,
                           size_t use, geom_affine *mapping)
{
    svg_xml_span view_span, width_span, height_span, value;
    if (!attr(traversal->source, traversal->tokens, traversal->token_count,
              target, "viewBox", NULL, &view_span) ||
        !attr(traversal->source, traversal->tokens, traversal->token_count,
              use, "width", NULL, &width_span) ||
        !attr(traversal->source, traversal->tokens, traversal->token_count,
              use, "height", NULL, &height_span)) return false;
    if (attr(traversal->source, traversal->tokens, traversal->token_count,
             use, "preserveAspectRatio", NULL, &value) &&
        !span_eq(traversal->source, value, "xMidYMid meet")) return false;

    double view_box[4], width, height, x = 0.0, y = 0.0;
    if (!svg_shape_read_number_list(traversal->source, view_span, view_box, 4U) ||
        !svg_shape_read_number(traversal->source, width_span, &width) ||
        !svg_shape_read_number(traversal->source, height_span, &height) ||
        view_box[2] <= 0.0 || view_box[3] <= 0.0 ||
        width <= 0.0 || height <= 0.0) return false;
    if (attr(traversal->source, traversal->tokens, traversal->token_count,
             use, "x", NULL, &value) &&
        !svg_shape_read_number(traversal->source, value, &x)) return false;
    if (attr(traversal->source, traversal->tokens, traversal->token_count,
             use, "y", NULL, &value) &&
        !svg_shape_read_number(traversal->source, value, &y)) return false;

    double scale = fmin(width / view_box[2], height / view_box[3]);
    double tx = x + (width - view_box[2] * scale) * 0.5 -
                view_box[0] * scale;
    double ty = y + (height - view_box[3] * scale) * 0.5 -
                view_box[1] * scale;
    *mapping = (geom_affine){scale, 0.0, 0.0, scale, tx, ty};
    return geom_affine_valid(*mapping);
}

static bool walk_range(shape_traversal *traversal, size_t first, size_t limit,
                       geom_affine parent_transform,
                       const svg_css_paint *parent_paint,
                       size_t render_depth);
static bool render_node(shape_traversal *traversal, size_t element,
                        size_t end, geom_affine parent_transform,
                        const svg_css_paint *parent_paint,
                        size_t render_depth);

static bool resolve_use(shape_traversal *traversal, size_t use,
                        geom_affine parent_transform,
                        const svg_css_paint *parent_paint,
                        size_t render_depth)
{
    const svg_xml_token *use_token = &traversal->tokens[use];
    if (!svg_reference_use_attributes_supported(
            traversal->source, traversal->tokens, traversal->token_count, use)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
             use_token->name.offset);
        return false;
    }
    svg_css_paint use_style;
    if (!compute_style(traversal, use, parent_paint, &use_style)) return false;

    svg_xml_span href = {0};
    bool plain = attr(traversal->source, traversal->tokens,
                      traversal->token_count, use, "href", NULL, &href);
    bool xlink = attr(traversal->source, traversal->tokens,
                      traversal->token_count, use, "href", "xlink", &href);
    if (plain == xlink ||
        (xlink && !svg_reference_xlink_is_bound(
                      traversal->source, traversal->tokens,
                      traversal->token_count)) ||
        href.length < 2U || traversal->source.ptr[href.offset] != '#') {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
             use_token->name.offset);
        return false;
    }
    size_t target;
    svg_xml_span id = {href.offset + 1U, href.length - 1U};
    if (!svg_reference_index_find(traversal->source, traversal->tokens,
                                  traversal->token_count,
                                  &traversal->references, id, &target)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, href.offset);
        return false;
    }
    for (size_t i = 0U; i < traversal->reference_depth; ++i) {
        if (traversal->active_references[i] == target) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                 href.offset);
            return false;
        }
    }
    if (traversal->reference_depth >= SHAPE_MAX_REFERENCE_DEPTH ||
        traversal->use_instances >= SHAPE_MAX_USE_INSTANCES) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT, use_token->name.offset);
        return false;
    }
    traversal->use_instances++;

    const svg_xml_token *target_token = &traversal->tokens[target];
    bool symbol = element_name(traversal->source, target_token, "symbol");
    if (!symbol && !element_name(traversal->source, target_token, "g") &&
        !basic_shape(traversal->source, target_token)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
             target_token->name.offset);
        return false;
    }
    size_t target_end = matching_end(traversal, target);
    if (target_end == SIZE_MAX) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_XML,
             target_token->name.offset);
        return false;
    }
    size_t subtree_start = target_token->source.offset;
    size_t end_offset = traversal->tokens[target_end].source.offset;
    size_t end_length = traversal->tokens[target_end].source.length;
    if (subtree_start > traversal->source.len ||
        end_offset > traversal->source.len ||
        end_length > traversal->source.len - end_offset ||
        end_offset + end_length < subtree_start) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_XML,
             target_token->name.offset);
        return false;
    }
    size_t subtree_bytes = end_offset + end_length - subtree_start;
    if (subtree_bytes > SHAPE_MAX_REFERENCE_BYTES ||
        subtree_bytes > SHAPE_MAX_JOB_REFERENCE_BYTES -
                        traversal->reference_bytes) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
             use_token->name.offset);
        return false;
    }
    traversal->reference_bytes += subtree_bytes;

    geom_affine use_transform;
    size_t local_operations = 0U;
    if (!svg_shape_transform_node(traversal->source, traversal->tokens,
                                  traversal->token_count, use,
                                  geom_affine_identity(), &use_transform,
                                  &local_operations, traversal->diagnostic) ||
        !account_transform(traversal, local_operations, use_token->name.offset))
        return false;

    svg_xml_span x_span, y_span;
    bool has_x = attr(traversal->source, traversal->tokens,
                      traversal->token_count, use, "x", NULL, &x_span);
    bool has_y = attr(traversal->source, traversal->tokens,
                      traversal->token_count, use, "y", NULL, &y_span);
    double x = 0.0, y = 0.0;
    if ((has_x && !svg_shape_read_number(traversal->source, x_span, &x)) ||
        (has_y && !svg_shape_read_number(traversal->source, y_span, &y))) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
             has_x ? x_span.offset : y_span.offset);
        return false;
    }

    geom_affine local_transform;
    if (symbol) {
        size_t unsupported_offset = 0U;
        if (!symbol_attributes_supported(traversal, target,
                                         &unsupported_offset)) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                 unsupported_offset);
            return false;
        }
        if (!symbol_mapping(traversal, target, use, &local_transform)) {
            svg_xml_span preserve;
            size_t offset = attr(traversal->source, traversal->tokens,
                                 traversal->token_count, use,
                                 "preserveAspectRatio", NULL, &preserve) ?
                preserve.offset : use_token->name.offset;
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, offset);
            return false;
        }
        if (!geom_affine_compose(use_transform, local_transform,
                                 &local_transform)) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_NUMBER,
                 use_token->name.offset);
            return false;
        }
    } else {
        geom_affine translation = {1.0, 0.0, 0.0, 1.0, x, y};
        if (!geom_affine_compose(use_transform, translation,
                                 &local_transform)) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_NUMBER,
                 use_token->name.offset);
            return false;
        }
    }
    geom_affine instance_transform;
    if (!geom_affine_compose(parent_transform, local_transform,
                             &instance_transform)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_NUMBER,
             use_token->name.offset);
        return false;
    }

    if (traversal->element_visits >= SHAPE_MAX_TRAVERSAL_VISITS) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
             target_token->name.offset);
        return false;
    }
    traversal->element_visits++;

    size_t first_shape = traversal->builder.doc->shape_count;
    size_t first_edge = traversal->builder.doc->edge_count;
    traversal->active_references[traversal->reference_depth++] = target;
    bool ok;
    svg_css_paint symbol_style = {0};
    if (symbol) {
        if (render_depth + 1U > SHAPE_MAX_XML_DEPTH) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
                 target_token->name.offset);
            ok = false;
        } else {
            ok = compute_style(traversal, target, &use_style, &symbol_style);
            if (ok) {
                svg_xml_span symbol_clip;
                if (svg_css_paint_value(&symbol_style, SVG_CSS_CLIP_PATH,
                                        &symbol_clip) &&
                    !svg_css_keyword_equals(traversal->source, symbol_clip,
                                            "none")) {
                    fail(traversal->diagnostic,
                         SVG_SHAPE_ERROR_UNSUPPORTED, symbol_clip.offset);
                    ok = false;
                } else {
                    ok = walk_range(traversal, target + 1U, target_end,
                                    instance_transform, &symbol_style,
                                    render_depth + 1U);
                }
            }
        }
    } else {
        ok = false;
        if (render_depth + 1U <= SHAPE_MAX_XML_DEPTH) {
            /* The reference target is traversed at the use position. Its own
             * definition ancestors are intentionally absent from paint and
             * transform inheritance. */
            ok = target_end < traversal->token_count &&
                 render_node(traversal, target, target_end,
                             instance_transform, &use_style,
                             render_depth + 1U);
        } else {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
                 target_token->name.offset);
        }
    }
    traversal->reference_depth--;
    svg_xml_span clip_value;
    if (ok && svg_css_paint_value(&use_style, SVG_CSS_CLIP_PATH,
                                  &clip_value) &&
        !svg_css_keyword_equals(traversal->source, clip_value, "none"))
        ok = apply_declared_clip(traversal, clip_value, first_shape,
                                 first_edge, instance_transform);
    return ok;
}

static bool render_node(shape_traversal *traversal, size_t element,
                        size_t end, geom_affine parent_transform,
                        const svg_css_paint *parent_paint,
                        size_t render_depth)
{
    const svg_xml_token *token = &traversal->tokens[element];
    bool root = token->depth == 0U && !traversal->root_seen;
    if (render_depth > SHAPE_MAX_XML_DEPTH) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT, token->name.offset);
        return false;
    }
    if (token->prefix.length != 0U ||
        (root && !element_name(traversal->source, token, "svg")) ||
        (!root && !element_name(traversal->source, token, "g") &&
         !element_name(traversal->source, token, "use") &&
         !basic_shape(traversal->source, token))) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
             token->name.offset);
        return false;
    }
    if (root) {
        if (traversal->root_seen) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                 token->name.offset);
            return false;
        }
        traversal->root_seen = true;
    }

    if (element_name(traversal->source, token, "use"))
        return resolve_use(traversal, element, parent_transform,
                           parent_paint, render_depth);

    if (!svg_reference_attributes_supported(
            traversal->source, traversal->tokens, traversal->token_count,
            element, root, token->local_name)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
             token->name.offset);
        return false;
    }
    svg_css_paint style;
    if (!compute_style(traversal, element, parent_paint, &style)) return false;

    geom_affine current;
    size_t local_operations = 0U;
    if (!svg_shape_transform_node(traversal->source, traversal->tokens,
                                  traversal->token_count, element,
                                  parent_transform, &current,
                                  &local_operations, traversal->diagnostic) ||
        !account_transform(traversal, local_operations, token->name.offset))
        return false;
    if (root && !root_viewport(traversal, element)) return false;

    if (!root && !element_name(traversal->source, token, "g") &&
        !svg_shape_builder_add_element(&traversal->builder,
                                       traversal->source, traversal->tokens,
                                       traversal->token_count, element, &style,
                                       current)) return false;

    if (end == element) return true;
    size_t first_shape = traversal->builder.doc->shape_count;
    size_t first_edge = traversal->builder.doc->edge_count;
    if (!walk_range(traversal, element + 1U, end, current, &style,
                    render_depth + 1U)) return false;
    svg_xml_span clip_value;
    if ((root || element_name(traversal->source, token, "g")) &&
        svg_css_paint_value(&style, SVG_CSS_CLIP_PATH, &clip_value) &&
        !svg_css_keyword_equals(traversal->source, clip_value, "none"))
        return apply_declared_clip(traversal, clip_value, first_shape,
                                   first_edge, current);
    return true;
}

static bool walk_range(shape_traversal *traversal, size_t first, size_t limit,
                       geom_affine parent_transform,
                       const svg_css_paint *parent_paint,
                       size_t render_depth)
{
    size_t i = first;
    while (i < limit) {
        svg_xml_token *token = &traversal->tokens[i];
        if (token->kind == SVG_XML_TOKEN_PROCESSING_INSTRUCTION) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                 token->name.offset);
            return false;
        }
        if (token->kind == SVG_XML_TOKEN_CDATA) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                 token->value.offset);
            return false;
        }
        if (token->kind == SVG_XML_TOKEN_TEXT) {
            for (size_t j = 0U; j < token->value.length; ++j) {
                uint8_t byte = traversal->source.ptr[token->value.offset + j];
                if (byte != ' ' && byte != '\t' && byte != '\r' && byte != '\n') {
                    fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
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
        size_t end = matching_end(traversal, i);
        if (end == SIZE_MAX || end >= limit) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_XML,
                 token->name.offset);
            return false;
        }
        if (traversal->element_visits >= SHAPE_MAX_TRAVERSAL_VISITS) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
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
        if (svg_css_is_style_element(traversal->source, token)) {
            i = end + 1U;
            continue;
        }
        if (token->depth > SHAPE_MAX_XML_DEPTH) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
                 token->name.offset);
            return false;
        }
        if (!render_node(traversal, i, end, parent_transform, parent_paint,
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

svg_shape_error svg_shape_decode_source_with_bounds(
    base_bytes source, svg_shape_document *out,
    svg_shape_diagnostic *diagnostic,
    svg_shape_decode_memory_bounds *memory_bounds)
{
    svg_shape_diagnostic local_diagnostic;
    if (diagnostic == NULL) diagnostic = &local_diagnostic;
    if (memory_bounds != NULL)
        *memory_bounds = (svg_shape_decode_memory_bounds){0};
    if (out == NULL || !base_bytes_valid(source)) {
        fail(diagnostic, SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U);
        return SVG_SHAPE_ERROR_INVALID_ARGUMENT;
    }
    *out = (svg_shape_document){0};
    *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_NONE, 0U};
    if (source.len > SHAPE_MAX_BYTES) {
        fail(diagnostic, SVG_SHAPE_ERROR_LIMIT, SHAPE_MAX_BYTES);
        return SVG_SHAPE_ERROR_LIMIT;
    }
    size_t capacity = source.len / 2U + 2U;
    if (capacity > SIZE_MAX / sizeof(svg_xml_token)) {
        fail(diagnostic, SVG_SHAPE_ERROR_LIMIT, 0U);
        return SVG_SHAPE_ERROR_LIMIT;
    }
    svg_xml_token *tokens = svg_shape_calloc(capacity, sizeof(*tokens));
    if (tokens == NULL) {
        fail(diagnostic, SVG_SHAPE_ERROR_OOM, 0U);
        return SVG_SHAPE_ERROR_OOM;
    }
    (void)svg_shape_alloc_set_tag(tokens, SVG_SHAPE_ALLOC_TAG_TOKEN_ARRAY);
    svg_xml_result xml = svg_xml_tokenize(source, svg_xml_default_options(),
                                          tokens, capacity);
    if (xml.diagnostic.category != SVG_XML_ERROR_NONE) {
        svg_shape_free(tokens);
        fail(diagnostic, SVG_SHAPE_ERROR_XML, xml.diagnostic.offset);
        return SVG_SHAPE_ERROR_XML;
    }

    shape_traversal traversal = {
        .source = source,
        .tokens = tokens,
        .token_count = xml.token_count,
        .diagnostic = diagnostic
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
        fail(diagnostic, error, 0U);
        goto cleanup;
    }
    traversal.builder = (svg_shape_builder){
        .doc = out,
        .references = &traversal.references,
        .css = traversal.css,
        .paint_cache = &traversal.paint_cache,
        .paint_token_visits = &traversal.paint_token_visits,
        .diag = diagnostic,
        .transform = geom_affine_identity()
    };
    out->source = source;
    if (!walk_range(&traversal, 0U, xml.token_count,
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
        fail(diagnostic, SVG_SHAPE_ERROR_VIEWPORT, 0U);
        error = SVG_SHAPE_ERROR_VIEWPORT;
        goto cleanup;
    }
    error = SVG_SHAPE_ERROR_NONE;

cleanup:
    svg_paint_cache_destroy(traversal.paint_cache);
    svg_css_sheet_destroy(traversal.css);
    svg_reference_index_destroy(&traversal.references);
    svg_shape_free(tokens);
    if (error != SVG_SHAPE_ERROR_NONE) svg_shape_document_destroy(out);
    return error;
}
