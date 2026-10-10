#include "shape_internal.h"

#include "../clip/clip.h"
#include "../paint/paint.h"

#include <math.h>
#include <string.h>

bool svg_shape_builder_add_custom_element(
    svg_shape_builder *builder, base_bytes source, svg_xml_token *tokens,
    size_t token_count, size_t element, const svg_css_paint *style,
    geom_affine transform, svg_shape_custom_geometry_fn geometry,
    void *geometry_context)
{
    if (builder == NULL || builder->doc == NULL || tokens == NULL ||
        style == NULL || geometry == NULL || element >= token_count) {
        svg_shape_fail(builder == NULL ? NULL : builder->diag,
                       SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U);
        return false;
    }
    svg_xml_span visibility = {0};
    if (svg_css_paint_value(style, SVG_CSS_VISIBILITY, &visibility)) {
        if (svg_css_keyword_equals(source, visibility, "hidden") ||
            svg_css_keyword_equals(source, visibility, "collapse")) return true;
        if (!svg_css_keyword_equals(source, visibility, "visible")) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED,
                           visibility.offset);
            return false;
        }
    }
    builder->offset = tokens[element].name.offset;
    builder->transform = transform;
    uint8_t color[4] = {0U, 0U, 0U, 255U};
    svg_xml_span fill = {0};
    bool has_fill = svg_css_paint_value(style, SVG_CSS_FILL, &fill);
    bool gradient_fill = has_fill && fill.length >= 5U &&
        fill.offset <= source.len && fill.length <= source.len - fill.offset &&
        memcmp(source.ptr + fill.offset, "url(#", 5U) == 0;
    if (has_fill && !gradient_fill &&
        !svg_shape_parse_style_color(source, fill, style, color)) {
        svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED, fill.offset);
        return false;
    }
    svg_xml_span rule = {0};
    if (svg_css_paint_value(style, SVG_CSS_FILL_RULE, &rule) &&
        !svg_css_keyword_equals(source, rule, "evenodd") &&
        !svg_css_keyword_equals(source, rule, "nonzero")) {
        svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED, rule.offset);
        return false;
    }
    bool evenodd = svg_css_paint_value(style, SVG_CSS_FILL_RULE, &rule) &&
                   svg_css_keyword_equals(source, rule, "evenodd");
    if (!svg_shape_builder_begin_shape(builder, evenodd)) return false;
    size_t shape = builder->shape_index;
    size_t first_edge = builder->doc->edge_count;
    svg_xml_span opacity_span = {0};
    if (svg_css_paint_value(style, SVG_CSS_FILL_OPACITY, &opacity_span)) {
        double opacity = 1.0;
        if (!svg_shape_parse_opacity(source, opacity_span, &opacity)) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_NUMBER,
                           opacity_span.offset);
            return false;
        }
        svg_shape_multiply_alpha(color, opacity);
    }
    geom_affine inverse = geom_affine_identity();
    bool invertible = svg_shape_affine_inverse(transform, &inverse);
    if (invertible) builder->doc->shape_stroke_inverse[shape] = inverse;
    double diagonal = hypot(builder->viewport_width, builder->viewport_height) /
                      sqrt(2.0);
    if (!svg_shape_parse_stroke(source, &builder->doc->strokes[shape], style,
                                diagonal, builder->diag)) return false;
    if (!svg_shape_apply_vector_effect(
            source, style, transform, builder->doc,
            &builder->doc->strokes[shape], builder->diag,
            tokens[element].name.offset)) return false;
    if (builder->doc->strokes[shape].rgba[3] != 0U && !invertible) {
        svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED,
                       tokens[element].name.offset);
        return false;
    }
    if (!geometry(builder, geometry_context)) return false;
    for (size_t edge = first_edge; edge < builder->doc->edge_count; ++edge)
        memcpy(builder->doc->colors + edge * 4U, color, sizeof(color));
    builder->doc->shape_edge_start[shape] = first_edge;
    builder->doc->shape_edge_count[shape] =
        builder->doc->edge_count - first_edge;
    if (gradient_fill && builder->doc->paints[shape].kind !=
                             SVG_SHAPE_PAINT_IMAGE) {
        if (!invertible) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED,
                           fill.offset);
            return false;
        }
        bool pattern_resolved = false;
        if (!svg_shape_pattern_resolve(builder, source, tokens, token_count,
                                       fill, *style, shape, transform, false,
                                       &pattern_resolved)) return false;
        if (!pattern_resolved) {
            svg_shape_error error = svg_paint_resolve_fill(
                source, tokens, token_count, builder->references, builder->css,
                builder->paint_cache, builder->paint_token_visits, fill,
                svg_paint_edge_span(builder->doc, first_edge,
                    builder->doc->edge_count - first_edge, NULL),
                builder->doc->edge_count - first_edge, inverse,
                builder->viewport_width, builder->viewport_height,
                &builder->doc->paints[shape], &builder->doc->paint_stops,
                &builder->doc->paint_stop_count, builder->diag);
            if (error != SVG_SHAPE_ERROR_NONE) return false;
        }
    }
    svg_shape_stroke *stroke = &builder->doc->strokes[shape];
    if (stroke->paint_server) {
        svg_xml_span stroke_value = {0};
        bool stroke_resolved = false;
        (void)svg_css_paint_value(style, SVG_CSS_STROKE, &stroke_value);
        if (!svg_shape_pattern_resolve(
                builder, source, tokens, token_count, stroke_value, *style,
                shape, transform, true, &stroke_resolved)) return false;
        if (!stroke_resolved) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED,
                           stroke_value.offset);
            return false;
        }
    }
    svg_xml_span clip = {0};
    if (svg_css_paint_value(style, SVG_CSS_CLIP_PATH, &clip) &&
        !svg_css_keyword_equals(source, clip, "none")) {
        double local_bounds[4];
        if (!invertible || !svg_clip_local_bounds(
                builder->doc, first_edge,
                builder->doc->edge_count - first_edge, inverse, local_bounds)) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED,
                           clip.offset);
            return false;
        }
        svg_shape_error error = svg_clip_resolve(
            source, tokens, token_count, builder->references, builder->css,
            clip, builder->doc, &builder->doc->clip_bytes, inverse,
            local_bounds, &builder->doc->clips[shape], builder->diag);
        if (error != SVG_SHAPE_ERROR_NONE) return false;
    }
    return true;
}

