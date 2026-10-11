#include "shape_internal.h"
#include "../css/color.h"

#include <math.h>
#include <string.h>

typedef svg_shape_number_reader number_reader;

static bool parse_color(base_bytes src, svg_xml_span s, uint8_t out[4])
{
    if (svg_css_keyword_equals(src, s, "none")) {
        out[0] = out[1] = out[2] = out[3] = 0U;
        return true;
    }
    return svg_css_parse_color(src, s, out);
}

bool svg_shape_parse_style_color(base_bytes source, svg_xml_span value,
                              const svg_css_paint *style, uint8_t rgba[4])
{
    if (!svg_css_keyword_equals(source, value, "currentColor"))
        return parse_color(source, value, rgba);
    svg_xml_span color_value;
    if (!svg_css_paint_value(style, SVG_CSS_COLOR, &color_value)) {
        rgba[0] = rgba[1] = rgba[2] = 0U;
        rgba[3] = 255U;
        return true;
    }
    if (svg_css_keyword_equals(source, color_value, "currentColor")) return false;
    return svg_css_parse_color(source, color_value, rgba);
}

static bool parse_stroke_number(base_bytes src, svg_xml_span span, double *out)
{
    number_reader r = {src.ptr + span.offset, span.length, 0U, span.offset};
    if (!svg_shape_read_number_value(&r, out)) return false;
    svg_shape_skip_number_space(&r);
    return r.i == r.n && isfinite(*out);
}
static bool parse_dasharray(base_bytes src, svg_xml_span span,
                            svg_shape_stroke *stroke, double reference)
{
    if (svg_css_keyword_equals(src, span, "none")) return true;
    number_reader r = {src.ptr + span.offset, span.length, 0U, span.offset};
    while (true) {
        svg_shape_skip_number_space(&r);
        if (r.i == r.n) break;
        if (stroke->dash_count == SVG_SHAPE_MAX_DASH_VALUES) return false;
        double value;
        if (!svg_shape_read_length_value(&r, reference, &value) ||
            value <= 0.0 || value > 1000000.0) return false;
        stroke->dash[stroke->dash_count++] = value;
        size_t before = r.i;
        svg_shape_skip_number_space(&r);
        bool spaced = r.i != before;
        if (r.i < r.n && r.p[r.i] == ',') {
            r.i++; svg_shape_skip_number_space(&r);
            if (r.i == r.n || r.p[r.i] == ',') return false;
        } else if (r.i < r.n && !spaced) return false;
    }
    if (stroke->dash_count == 0U) return false;
    return true;
}

static bool paint_server_value(base_bytes source, svg_xml_span value)
{
    if (value.offset > source.len || value.length > source.len - value.offset)
        return false;
    size_t i = 0U;
    while (i < value.length && (source.ptr[value.offset + i] == ' ' ||
           source.ptr[value.offset + i] == '\t' ||
           source.ptr[value.offset + i] == '\r' ||
           source.ptr[value.offset + i] == '\n')) i++;
    return value.length - i >= 4U &&
           source.ptr[value.offset + i] == 'u' &&
           source.ptr[value.offset + i + 1U] == 'r' &&
           source.ptr[value.offset + i + 2U] == 'l' &&
           source.ptr[value.offset + i + 3U] == '(';
}
bool svg_shape_parse_opacity(base_bytes src, svg_xml_span span, double *out)
{
    number_reader reader = {src.ptr + span.offset, span.length, 0U, span.offset};
    double value;
    if (!svg_shape_read_number_value(&reader, &value)) return false;
    svg_shape_skip_number_space(&reader);
    bool percent = reader.i < reader.n && reader.p[reader.i] == '%';
    if (percent) reader.i++;
    svg_shape_skip_number_space(&reader);
    if (reader.i != reader.n) return false;
    if (percent) value /= 100.0;
    if (!isfinite(value) || value < 0.0 || value > 1.0) return false;
    *out = value;
    return true;
}

bool svg_shape_style_opacity(base_bytes source, const svg_css_paint *style,
                             double *opacity, svg_xml_span *value)
{
    if (opacity == NULL || value == NULL) return false;
    *opacity = 1.0;
    if (!svg_css_paint_value(style, SVG_CSS_OPACITY, value)) return true;
    return svg_shape_parse_opacity(source, *value, opacity);
}

bool svg_shape_style_display_none(base_bytes source,
                                  const svg_css_paint *style,
                                  size_t fallback_offset, bool *is_none,
                                  svg_shape_diagnostic *diagnostic)
{
    if (is_none == NULL) return false;
    *is_none = false;
    svg_xml_span value;
    if (!svg_css_paint_value(style, SVG_CSS_DISPLAY, &value)) return true;
    if (svg_css_keyword_equals(source, value, "none")) {
        *is_none = true;
        return true;
    }
    if (svg_css_keyword_equals(source, value, "inline") ||
        svg_css_keyword_equals(source, value, "block") ||
        svg_css_keyword_equals(source, value, "contents")) return true;
    svg_shape_fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                   value.offset != 0U ? value.offset : fallback_offset);
    return false;
}

void svg_shape_multiply_alpha(uint8_t rgba[4], double opacity)
{
    rgba[3] = (uint8_t)lround((double)rgba[3] * opacity);
}

bool svg_shape_parse_stroke(base_bytes src, svg_shape_stroke *stroke,
                            const svg_css_paint *paint,
                            double percentage_reference,
                            svg_shape_diagnostic *diag)
{
    svg_xml_span span;
    if (svg_css_paint_value(paint, SVG_CSS_STROKE, &span)) {
        if (paint_server_value(src, span)) {
            stroke->paint_server = true;
            stroke->rgba[0] = stroke->rgba[1] = stroke->rgba[2] = 0U;
            stroke->rgba[3] = 255U;
        } else if (!svg_shape_parse_style_color(src, span, paint,
                                                stroke->rgba)) {
            svg_shape_fail(diag, SVG_SHAPE_ERROR_UNSUPPORTED, span.offset);
            return false;
        }
    }
    if (!svg_css_paint_value(paint, SVG_CSS_STROKE, &span)) stroke->rgba[3] = 0U;
    if (svg_css_paint_value(paint, SVG_CSS_STROKE_WIDTH, &span)) {
        if (!svg_shape_read_length(src, span, percentage_reference,
                                   &stroke->width) ||
            stroke->width < 0.0 || stroke->width > 1024.0) {
            svg_shape_fail(diag, SVG_SHAPE_ERROR_NUMBER, span.offset); return false;
        }
    }
    if (svg_css_paint_value(paint, SVG_CSS_STROKE_MITERLIMIT, &span)) {
        if (!parse_stroke_number(src, span, &stroke->miter_limit) || stroke->miter_limit < 1.0 || stroke->miter_limit > 64.0) {
            svg_shape_fail(diag, SVG_SHAPE_ERROR_NUMBER, span.offset); return false;
        }
    }
    if (svg_css_paint_value(paint, SVG_CSS_STROKE_DASHOFFSET, &span)) {
        if (!svg_shape_read_length(src, span, percentage_reference,
                                   &stroke->dash_offset) ||
            fabs(stroke->dash_offset) > 1000000.0) {
            svg_shape_fail(diag, SVG_SHAPE_ERROR_NUMBER, span.offset); return false;
        }
    }
    if (svg_css_paint_value(paint, SVG_CSS_STROKE_LINECAP, &span)) {
        if (svg_css_keyword_equals(src, span, "butt")) stroke->cap = SVG_SHAPE_CAP_BUTT;
        else if (svg_css_keyword_equals(src, span, "square")) stroke->cap = SVG_SHAPE_CAP_SQUARE;
        else if (svg_css_keyword_equals(src, span, "round")) stroke->cap = SVG_SHAPE_CAP_ROUND;
        else { svg_shape_fail(diag, SVG_SHAPE_ERROR_UNSUPPORTED, span.offset); return false; }
    }
    if (svg_css_paint_value(paint, SVG_CSS_STROKE_LINEJOIN, &span)) {
        if (svg_css_keyword_equals(src, span, "miter")) stroke->join = SVG_SHAPE_JOIN_MITER;
        else if (svg_css_keyword_equals(src, span, "bevel")) stroke->join = SVG_SHAPE_JOIN_BEVEL;
        else if (svg_css_keyword_equals(src, span, "round") ||
                 svg_css_keyword_equals(src, span, "arcs"))
            stroke->join = SVG_SHAPE_JOIN_ROUND;
        else if (svg_css_keyword_equals(src, span, "miter-clip"))
            stroke->join = SVG_SHAPE_JOIN_MITER;
        else { svg_shape_fail(diag, SVG_SHAPE_ERROR_UNSUPPORTED, span.offset); return false; }
    }
    if (svg_css_paint_value(paint, SVG_CSS_STROKE_DASHARRAY, &span) &&
        !parse_dasharray(src, span, stroke, percentage_reference)) {
        svg_shape_fail(diag, SVG_SHAPE_ERROR_NUMBER, span.offset); return false;
    }
    if (svg_css_paint_value(paint, SVG_CSS_STROKE_OPACITY, &span)) {
        double opacity;
        if (!svg_shape_parse_opacity(src, span, &opacity)) {
            svg_shape_fail(diag, SVG_SHAPE_ERROR_NUMBER, span.offset);
            return false;
        }
        svg_shape_multiply_alpha(stroke->rgba, opacity);
    }
    if (svg_css_paint_value(paint, SVG_CSS_PAINT_ORDER, &span)) {
        if (svg_css_keyword_equals(src, span, "normal")) {
            stroke->paint_order = 0U;
        } else {
            size_t cursor = 0U;
            bool seen_fill = false, seen_stroke = false, seen_marker = false;
            bool first_is_stroke = false;
            size_t item_count = 0U;
            while (cursor < span.length) {
                while (cursor < span.length &&
                       (src.ptr[span.offset + cursor] == ' ' ||
                        src.ptr[span.offset + cursor] == '\t' ||
                        src.ptr[span.offset + cursor] == '\r' ||
                        src.ptr[span.offset + cursor] == '\n')) cursor++;
                if (cursor == span.length) break;
                size_t start = cursor;
                while (cursor < span.length && src.ptr[span.offset + cursor] != ' ' &&
                       src.ptr[span.offset + cursor] != '\t' &&
                       src.ptr[span.offset + cursor] != '\r' &&
                       src.ptr[span.offset + cursor] != '\n') cursor++;
                svg_xml_span item = {span.offset + start, cursor - start};
                bool *seen = svg_css_keyword_equals(src, item, "fill") ?
                    &seen_fill : svg_css_keyword_equals(src, item, "stroke") ?
                    &seen_stroke : svg_css_keyword_equals(src, item, "markers") ?
                    &seen_marker : NULL;
                if (seen == NULL || *seen || item_count >= 3U) {
                    svg_shape_fail(diag, SVG_SHAPE_ERROR_UNSUPPORTED,
                                   item.offset);
                    return false;
                }
                if (item_count == 0U)
                    first_is_stroke = seen == &seen_stroke;
                *seen = true;
                item_count++;
            }
            if (item_count == 0U) {
                svg_shape_fail(diag, SVG_SHAPE_ERROR_UNSUPPORTED, span.offset);
                return false;
            }
            stroke->paint_order = first_is_stroke ? 1U : 0U;
        }
    }
    if (svg_css_paint_value(paint, SVG_CSS_SHAPE_RENDERING, &span)) {
        if (svg_css_keyword_equals(src, span, "crispEdges"))
            stroke->crisp_edges = true;
        else if (!svg_css_keyword_equals(src, span, "auto") &&
                 !svg_css_keyword_equals(src, span, "optimizeSpeed") &&
                 !svg_css_keyword_equals(src, span, "geometricPrecision")) {
            svg_shape_fail(diag, SVG_SHAPE_ERROR_UNSUPPORTED, span.offset);
            return false;
        }
    }
    return true;
}

static bool uniform_scale(geom_affine transform, double *scale)
{
    if (!isfinite(transform.a) || !isfinite(transform.b) ||
        !isfinite(transform.c) || !isfinite(transform.d)) return false;
    double xx = transform.a * transform.a + transform.b * transform.b;
    double yy = transform.c * transform.c + transform.d * transform.d;
    double xy = transform.a * transform.c + transform.b * transform.d;
    double discriminant = hypot(xx - yy, 2.0 * xy);
    double maximum = sqrt(fmax(0.0, (xx + yy + discriminant) * 0.5));
    double minimum = sqrt(fmax(0.0, (xx + yy - discriminant) * 0.5));
    if (!isfinite(maximum) || maximum <= 0.0 ||
        fabs(maximum - minimum) > maximum * 1e-8) return false;
    *scale = maximum;
    return true;
}

bool svg_shape_apply_vector_effect(base_bytes source,
                                   const svg_css_paint *style,
                                   geom_affine transform,
                                   const svg_shape_document *document,
                                   svg_shape_stroke *stroke,
                                   svg_shape_diagnostic *diagnostic,
                                   size_t fallback_offset)
{
    svg_xml_span value = {0};
    if (!svg_css_paint_value(style, SVG_CSS_VECTOR_EFFECT, &value) ||
        svg_css_keyword_equals(source, value, "none")) return true;
    if (!svg_css_keyword_equals(source, value, "non-scaling-stroke")) {
        svg_shape_fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                       value.offset != 0U ? value.offset : fallback_offset);
        return false;
    }
    double element_scale = 0.0;
    if (document == NULL || stroke == NULL || document->view_width <= 0.0 ||
        document->view_height <= 0.0 ||
        !uniform_scale(transform, &element_scale)) {
        svg_shape_fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, value.offset);
        return false;
    }
    double scale_x = (double)document->width / document->view_width;
    double scale_y = (double)document->height / document->view_height;
    svg_shape_aspect aspect = document->preserve_aspect;
    if (!aspect.set) aspect = (svg_shape_aspect){0.5, 0.5, false, false, true};
    if (!aspect.none) {
        double scale = aspect.slice ? fmax(scale_x, scale_y) :
                                      fmin(scale_x, scale_y);
        scale_x = scale;
        scale_y = scale;
    }
    double viewport_scale = fmax(scale_x, scale_y);
    double minimum_scale = fmin(scale_x, scale_y);
    if (!isfinite(viewport_scale) || viewport_scale <= 0.0 ||
        fabs(viewport_scale - minimum_scale) > viewport_scale * 1e-8) {
        svg_shape_fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, value.offset);
        return false;
    }
    double combined = element_scale * viewport_scale;
    if (!isfinite(combined) || combined <= 0.0) {
        svg_shape_fail(diagnostic, SVG_SHAPE_ERROR_NUMBER, value.offset);
        return false;
    }
    stroke->width /= combined;
    return isfinite(stroke->width) && stroke->width >= 0.0;
}
