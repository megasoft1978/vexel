#include "traverse_internal.h"

#include "reference.h"
#include "../clip/clip.h"

#include <math.h>

bool svg_shape_traverse_resolve_use(shape_traversal *traversal, size_t use,
                        geom_affine parent_transform,
                        const svg_css_paint *parent_paint,
                        size_t render_depth)
{
    const svg_xml_token *use_token = &traversal->tokens[use];
    if (!svg_reference_use_attributes_supported(
            traversal->source, traversal->tokens, traversal->token_count, use)) {
        svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
             use_token->name.offset);
        return false;
    }
    svg_css_paint use_style;
    if (!svg_shape_traverse_compute_style(traversal, use, parent_paint, &use_style)) return false;
    bool display_none = false;
    if (!svg_shape_style_display_none(traversal->source, &use_style,
            use_token->name.offset, &display_none, traversal->diagnostic)) return false;
    if (display_none) return true;

    svg_xml_span href = {0};
    bool plain = svg_shape_traverse_attr(traversal->source, traversal->tokens,
                      traversal->token_count, use, "href", NULL, &href);
    bool xlink = svg_shape_traverse_attr(traversal->source, traversal->tokens,
                      traversal->token_count, use, "href", "xlink", &href);
    if (plain == xlink ||
        (xlink && !svg_reference_xlink_is_bound(
                      traversal->source, traversal->tokens,
                      traversal->token_count)) ||
        href.length < 2U || traversal->source.ptr[href.offset] != '#') {
        svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
             use_token->name.offset);
        return false;
    }
    size_t target;
    svg_xml_span id = {href.offset + 1U, href.length - 1U};
    if (!svg_reference_index_find(traversal->source, traversal->tokens,
                                  traversal->token_count,
                                  &traversal->references, id, &target)) {
        svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, href.offset);
        return false;
    }
    for (size_t i = 0U; i < traversal->reference_depth; ++i) {
        if (traversal->active_references[i] == target) {
            svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                 href.offset);
            return false;
        }
    }
    if (traversal->reference_depth >= SHAPE_MAX_REFERENCE_DEPTH ||
        traversal->use_instances >= SHAPE_MAX_USE_INSTANCES) {
        svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT, use_token->name.offset);
        return false;
    }
    traversal->use_instances++;

    const svg_xml_token *target_token = &traversal->tokens[target];
    bool symbol = svg_shape_traverse_element_name(traversal->source, target_token, "symbol");
    if (!symbol && !svg_shape_traverse_element_name(traversal->source, target_token, "g") &&
        !svg_shape_traverse_basic_shape(traversal->source, target_token)) {
        svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
             target_token->name.offset);
        return false;
    }
    size_t target_end = svg_shape_traverse_matching_end(traversal, target);
    if (target_end == SIZE_MAX) {
        svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_XML,
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
        svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_XML,
             target_token->name.offset);
        return false;
    }
    size_t subtree_bytes = end_offset + end_length - subtree_start;
    if (subtree_bytes > SHAPE_MAX_REFERENCE_BYTES ||
        subtree_bytes > SHAPE_MAX_JOB_REFERENCE_BYTES -
                        traversal->reference_bytes) {
        svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
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
        !svg_shape_traverse_account_transform(traversal, local_operations, use_token->name.offset))
        return false;

    svg_xml_span x_span, y_span;
    bool has_x = svg_shape_traverse_attr(traversal->source, traversal->tokens,
                      traversal->token_count, use, "x", NULL, &x_span);
    bool has_y = svg_shape_traverse_attr(traversal->source, traversal->tokens,
                      traversal->token_count, use, "y", NULL, &y_span);
    double x = 0.0, y = 0.0;
    if ((has_x && !svg_shape_read_length(traversal->source, x_span,
                                         traversal->viewport_width, &x)) ||
        (has_y && !svg_shape_read_length(traversal->source, y_span,
                                         traversal->viewport_height, &y))) {
        svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
             has_x ? x_span.offset : y_span.offset);
        return false;
    }

    geom_affine suffix_transform;
    geom_affine local_transform;
    if (symbol) {
        size_t unsupported_offset = 0U;
        if (!svg_shape_symbol_attributes_supported(
                traversal->source, traversal->tokens,
                traversal->token_count, target, &unsupported_offset)) {
            svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                 unsupported_offset);
            return false;
        }
        if (!svg_shape_symbol_mapping(
                traversal->source, traversal->tokens,
                traversal->token_count, target, use,
                traversal->viewport_width, traversal->viewport_height,
                &suffix_transform)) {
            svg_xml_span preserve;
            size_t offset = svg_shape_traverse_attr(traversal->source, traversal->tokens,
                                 traversal->token_count, use,
                                 "preserveAspectRatio", NULL, &preserve) ?
                preserve.offset : use_token->name.offset;
            svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, offset);
            return false;
        }
        if (!geom_affine_compose(use_transform, suffix_transform,
                                 &local_transform)) {
            svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_NUMBER,
                 use_token->name.offset);
            return false;
        }
    } else {
        suffix_transform = (geom_affine){1.0, 0.0, 0.0, 1.0, x, y};
        if (!geom_affine_compose(use_transform, suffix_transform,
                                 &local_transform)) {
            svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_NUMBER,
                 use_token->name.offset);
            return false;
        }
    }
    geom_affine instance_transform;
    if (!geom_affine_compose(parent_transform, local_transform,
                             &instance_transform)) {
        svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_NUMBER,
             use_token->name.offset);
        return false;
    }

    size_t transform_parent = render_depth == 0U ?
        SVG_SHAPE_NO_TRANSFORM_NODE :
        traversal->transform_node_stack[render_depth - 1U];
    size_t use_transform_node = SVG_SHAPE_NO_TRANSFORM_NODE;
    if (!svg_shape_traverse_push_transform_node(
            traversal, use_token->source.offset, render_depth, transform_parent,
            use_transform, suffix_transform, &use_transform_node)) return false;

    if (traversal->element_visits >= SHAPE_MAX_TRAVERSAL_VISITS) {
        svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
             target_token->name.offset);
        return false;
    }
    traversal->element_visits++;

    size_t opacity_group = SVG_SHAPE_NO_GROUP;
    if (!svg_shape_traverse_begin_opacity_group(traversal, &use_style, &opacity_group)) return false;
    size_t first_shape = traversal->builder.doc->shape_count;
    size_t first_edge = traversal->builder.doc->edge_count;
    size_t previous_selection = traversal->selection_element;
    if (traversal->selection_element == SIZE_MAX)
        traversal->selection_element = use;
    traversal->active_references[traversal->reference_depth++] = target;
    bool ok;
    svg_css_paint symbol_style = {0};
    if (symbol) {
        if (render_depth + 1U > SHAPE_MAX_XML_DEPTH) {
            svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
                 target_token->name.offset);
            ok = false;
        } else {
            size_t symbol_transform_node = SVG_SHAPE_NO_TRANSFORM_NODE;
            ok = svg_shape_traverse_push_transform_node(
                    traversal, target_token->source.offset, render_depth,
                    use_transform_node, geom_affine_identity(),
                    geom_affine_identity(), &symbol_transform_node) &&
                 svg_shape_traverse_compute_style(traversal, target, &use_style,
                                                  &symbol_style);
            if (ok) {
                double previous_width = traversal->viewport_width;
                double previous_height = traversal->viewport_height;
                svg_xml_span view_span;
                double view_box[4];
                if (!svg_shape_traverse_attr(traversal->source, traversal->tokens,
                          traversal->token_count, target, "viewBox", NULL,
                          &view_span) ||
                    !svg_shape_read_number_list(traversal->source, view_span,
                                                view_box, 4U)) {
                    svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                         target_token->name.offset);
                    ok = false;
                } else {
                    traversal->viewport_width = view_box[2];
                    traversal->viewport_height = view_box[3];
                }
                svg_xml_span symbol_clip;
                if (ok && svg_css_paint_value(&symbol_style, SVG_CSS_CLIP_PATH,
                                        &symbol_clip) &&
                    !svg_css_keyword_equals(traversal->source, symbol_clip,
                                            "none")) {
                    svg_shape_traverse_fail(traversal->diagnostic,
                         SVG_SHAPE_ERROR_UNSUPPORTED, symbol_clip.offset);
                    ok = false;
                } else if (ok) {
                    ok = svg_shape_traverse_walk_range(traversal, target + 1U, target_end,
                                    instance_transform, &symbol_style,
                                    render_depth + 1U);
                }
                traversal->viewport_width = previous_width;
                traversal->viewport_height = previous_height;
            }
        }
    } else {
        ok = false;
        if (render_depth + 1U <= SHAPE_MAX_XML_DEPTH) {
            /* The reference target is traversed at the use position. Its own
             * definition ancestors are intentionally absent from paint and
             * transform inheritance. */
            ok = target_end < traversal->token_count &&
                 svg_shape_traverse_render_node(traversal, target, target_end,
                             instance_transform, &use_style,
                             render_depth + 1U);
        } else {
            svg_shape_traverse_fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
                 target_token->name.offset);
        }
    }
    traversal->reference_depth--;
    traversal->selection_element = previous_selection;
    svg_xml_span clip_value;
    if (ok && svg_css_paint_value(&use_style, SVG_CSS_CLIP_PATH,
                                  &clip_value) &&
        !svg_css_keyword_equals(traversal->source, clip_value, "none"))
        ok = svg_shape_apply_declared_clip(traversal->source, traversal->tokens,
            traversal->token_count, &traversal->references, traversal->css,
            traversal->builder.doc, clip_value, first_shape, first_edge,
            use_transform_node, instance_transform,
            traversal->diagnostic);
    svg_shape_traverse_finish_opacity_group(traversal, opacity_group);
    return ok;
}

