#include "paint.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

static bool eq(base_bytes source, svg_xml_span span, const char *text)
{
    size_t length = strlen(text);
    return span.offset <= source.len && span.length == length &&
           length <= source.len - span.offset &&
           memcmp(source.ptr + span.offset, text, length) == 0;
}

static bool attr(base_bytes source, const svg_xml_token *tokens, size_t count,
                 size_t element, const char *name, svg_xml_span *value)
{
    for (size_t i = element + 1U; i < count && tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE; ++i) {
        if (tokens[i].prefix.length == 0U && eq(source, tokens[i].local_name, name)) {
            *value = tokens[i].value;
            return true;
        }
    }
    return false;
}

static size_t element_end(const svg_xml_token *tokens, size_t count, size_t start)
{
    if (tokens[start].kind == SVG_XML_TOKEN_EMPTY_ELEMENT) return start;
    for (size_t i = start + 1U; i < count; ++i) {
        if (tokens[i].kind == SVG_XML_TOKEN_END_ELEMENT &&
            tokens[i].depth == tokens[start].depth) return i;
    }
    return start;
}

static bool number(base_bytes source, svg_xml_span span, double *out)
{
    if (span.offset > source.len || span.length == 0U ||
        span.length >= 96U || span.length > source.len - span.offset) return false;
    char text[96];
    memcpy(text, source.ptr + span.offset, span.length);
    text[span.length] = '\0';
    char *end = NULL;
    *out = strtod(text, &end);
    if (end == text || !isfinite(*out) || fabs(*out) > 1000000.0) return false;
    bool percent = *end == '%';
    if (percent) ++end;
    if (*end != '\0') return false;
    if (percent) *out /= 100.0;
    return true;
}

static bool color(base_bytes source, svg_xml_span span, uint8_t rgba[4])
{
    if (eq(source, span, "red")) { rgba[0] = 255U; rgba[1] = 0U; rgba[2] = 0U; }
    else if (eq(source, span, "green")) { rgba[0] = 0U; rgba[1] = 128U; rgba[2] = 0U; }
    else if (eq(source, span, "blue")) { rgba[0] = 0U; rgba[1] = 0U; rgba[2] = 255U; }
    else if (eq(source, span, "black")) { rgba[0] = rgba[1] = rgba[2] = 0U; }
    else if (eq(source, span, "white")) { rgba[0] = rgba[1] = rgba[2] = 255U; }
    else if (span.length == 4U && source.ptr[span.offset] == '#') {
        for (size_t i = 0U; i < 3U; ++i) {
            uint8_t c = source.ptr[span.offset + i + 1U];
            int value = c >= '0' && c <= '9' ? c - '0' :
                c >= 'a' && c <= 'f' ? c - 'a' + 10 :
                c >= 'A' && c <= 'F' ? c - 'A' + 10 : -1;
            if (value < 0) return false;
            rgba[i] = (uint8_t)(value * 17);
        }
    } else if (span.length == 7U && source.ptr[span.offset] == '#') {
        for (size_t i = 0U; i < 3U; ++i) {
            uint8_t a = source.ptr[span.offset + 1U + i * 2U];
            uint8_t b = source.ptr[span.offset + 2U + i * 2U];
            int hi = a >= '0' && a <= '9' ? a - '0' : a >= 'a' && a <= 'f' ? a - 'a' + 10 : a >= 'A' && a <= 'F' ? a - 'A' + 10 : -1;
            int lo = b >= '0' && b <= '9' ? b - '0' : b >= 'a' && b <= 'f' ? b - 'a' + 10 : b >= 'A' && b <= 'F' ? b - 'A' + 10 : -1;
            if (hi < 0 || lo < 0) return false;
            rgba[i] = (uint8_t)(hi * 16 + lo);
        }
    } else return false;
    rgba[3] = 255U;
    return true;
}

static bool parse_stops(base_bytes source, const svg_xml_token *tokens,
                        size_t count, size_t gradient, svg_paint_stop stops[SVG_PAINT_MAX_GRADIENT_STOPS],
                        size_t *stop_count, bool *hit_limit)
{
    *hit_limit = false;
    size_t end = element_end(tokens, count, gradient);
    size_t n = 0U;
    for (size_t i = gradient + 1U; i < end; ++i) {
        if ((tokens[i].kind != SVG_XML_TOKEN_START_ELEMENT &&
             tokens[i].kind != SVG_XML_TOKEN_EMPTY_ELEMENT) ||
            tokens[i].depth != tokens[gradient].depth + 1U) continue;
        if (n == SVG_PAINT_MAX_GRADIENT_STOPS) { *hit_limit = true; return false; }
        if (!eq(source, tokens[i].local_name, "stop") || tokens[i].prefix.length != 0U) return false;
        svg_xml_span offset = {0}, stop_color = {0}, opacity = {0};
        (void)attr(source, tokens, count, i, "offset", &offset);
        bool has_stop_color = attr(source, tokens, count, i, "stop-color", &stop_color);
        (void)attr(source, tokens, count, i, "stop-opacity", &opacity);
        double position = 0.0, alpha = 1.0;
        if (offset.length != 0U && !number(source, offset, &position)) return false;
        if (opacity.length != 0U && !number(source, opacity, &alpha)) return false;
        if (position < 0.0 || position > 1.0 || alpha < 0.0 || alpha > 1.0 ||
            (n != 0U && position < stops[n - 1U].offset)) return false;
        if (!has_stop_color) memset(stops[n].rgba, 0, 3U);
        else if (stop_color.length == 0U || !color(source, stop_color, stops[n].rgba))
            return false;
        stops[n].offset = position;
        stops[n].rgba[3] = (uint8_t)lround(alpha * 255.0);
        ++n;
        if (tokens[i].kind == SVG_XML_TOKEN_START_ELEMENT) i = element_end(tokens, count, i);
    }
    if (n < 2U) return false;
    *stop_count = n;
    return true;
}

static bool gradient_coordinates(base_bytes source, const svg_xml_token *tokens,
                                 size_t count, size_t gradient, bool object_box,
                                 double viewport_width, double viewport_height,
                                 svg_shape_paint *paint)
{
    const char *names[5];
    double defaults[5];
    if (paint->kind == SVG_SHAPE_PAINT_LINEAR) {
        names[0] = "x1"; names[1] = "y1"; names[2] = "x2"; names[3] = "y2";
        defaults[0] = 0.0;
        defaults[1] = 0.0;
        defaults[2] = object_box ? 1.0 : viewport_width;
        defaults[3] = 0.0;
    } else {
        names[0] = "cx"; names[1] = "cy"; names[2] = "r"; names[3] = "fx"; names[4] = "fy";
        double radius = hypot(viewport_width, viewport_height) / sqrt(2.0);
        defaults[0] = object_box ? 0.5 : viewport_width * 0.5;
        defaults[1] = object_box ? 0.5 : viewport_height * 0.5;
        defaults[2] = object_box ? 0.5 : radius * 0.5;
        defaults[3] = defaults[0];
        defaults[4] = defaults[1];
    }
    size_t count_coordinates = paint->kind == SVG_SHAPE_PAINT_LINEAR ? 4U : 5U;
    for (size_t i = 0U; i < count_coordinates; ++i) {
        svg_xml_span value;
        paint->coordinates[i] = defaults[i];
        if (!attr(source, tokens, count, gradient, names[i], &value)) continue;
        if (!number(source, value, &paint->coordinates[i])) return false;
        if (!object_box && value.length != 0U && source.ptr[value.offset + value.length - 1U] == '%') {
            double scale;
            if (paint->kind == SVG_SHAPE_PAINT_LINEAR)
                scale = (i == 0U || i == 2U) ? viewport_width : viewport_height;
            else if (i == 0U || i == 3U) scale = viewport_width;
            else if (i == 1U || i == 4U) scale = viewport_height;
            else scale = hypot(viewport_width, viewport_height) / sqrt(2.0);
            paint->coordinates[i] *= scale;
        }
        if (object_box && (paint->coordinates[i] < 0.0 || paint->coordinates[i] > 1.0)) return false;
    }
    if (paint->kind == SVG_SHAPE_PAINT_RADIAL) {
        svg_xml_span focal;
        if (!attr(source, tokens, count, gradient, "fx", &focal))
            paint->coordinates[3] = paint->coordinates[0];
        if (!attr(source, tokens, count, gradient, "fy", &focal))
            paint->coordinates[4] = paint->coordinates[1];
    }
    if (paint->kind == SVG_SHAPE_PAINT_RADIAL &&
        (paint->coordinates[2] <= 0.0 || paint->coordinates[3] != paint->coordinates[0] ||
         paint->coordinates[4] != paint->coordinates[1])) return false;
    return true;
}

svg_shape_error svg_paint_resolve_fill(base_bytes source,
                                       const svg_xml_token *tokens,
                                       size_t token_count, svg_xml_span fill,
                                       const geom_edge *edges, size_t edge_count,
                                       double viewport_width, double viewport_height,
                                       svg_shape_paint *paint,
                                       svg_paint_stop **stops,
                                       size_t *stop_count,
                                       svg_shape_diagnostic *diagnostic)
{
    if (fill.length < 7U || fill.offset > source.len ||
        fill.length > source.len - fill.offset ||
        memcmp(source.ptr + fill.offset, "url(#", 5U) != 0 ||
        source.ptr[fill.offset + fill.length - 1U] != ')') return SVG_SHAPE_ERROR_UNSUPPORTED;
    svg_xml_span target_id = {fill.offset + 5U, fill.length - 6U};
    size_t target = SIZE_MAX;
    for (size_t i = 0U; i < token_count; ++i) {
        if (tokens[i].kind != SVG_XML_TOKEN_START_ELEMENT &&
            tokens[i].kind != SVG_XML_TOKEN_EMPTY_ELEMENT) continue;
        svg_xml_span id;
        if (attr(source, tokens, token_count, i, "id", &id) && id.length == target_id.length &&
            memcmp(source.ptr + id.offset, source.ptr + target_id.offset, id.length) == 0) {
            target = i;
            break;
        }
    }
    if (target == SIZE_MAX) goto unsupported;
    svg_shape_paint result = {0};
    if (eq(source, tokens[target].local_name, "linearGradient")) result.kind = SVG_SHAPE_PAINT_LINEAR;
    else if (eq(source, tokens[target].local_name, "radialGradient")) result.kind = SVG_SHAPE_PAINT_RADIAL;
    else goto unsupported;
    if (tokens[target].prefix.length != 0U) goto unsupported;
    result.object_bounding_box = true;
    svg_xml_span value;
    if (attr(source, tokens, token_count, target, "gradientUnits", &value)) {
        if (eq(source, value, "userSpaceOnUse")) result.object_bounding_box = false;
        else if (!eq(source, value, "objectBoundingBox")) goto unsupported;
    }
    if (attr(source, tokens, token_count, target, "spreadMethod", &value) && !eq(source, value, "pad")) goto unsupported;
    if (attr(source, tokens, token_count, target, "gradientTransform", &value) ||
        attr(source, tokens, token_count, target, "href", &value)) goto unsupported;
    if (!gradient_coordinates(source, tokens, token_count, target,
                              result.object_bounding_box, viewport_width,
                              viewport_height, &result)) goto unsupported;
    svg_paint_stop local[SVG_PAINT_MAX_GRADIENT_STOPS];
    size_t local_count = 0U;
    bool stop_limit = false;
    if (!parse_stops(source, tokens, token_count, target, local, &local_count, &stop_limit)) {
        if (stop_limit) goto limit;
        goto unsupported;
    }
    if (local_count > SVG_PAINT_MAX_STOPS - *stop_count) goto limit;
    if (edge_count == 0U) goto unsupported;
    double min_x = edges[0].a.x, min_y = edges[0].a.y, max_x = min_x, max_y = min_y;
    for (size_t i = 0U; i < edge_count; ++i) {
        const geom_point p[2] = {edges[i].a, edges[i].b};
        for (size_t j = 0U; j < 2U; ++j) {
            if (p[j].x < min_x) min_x = p[j].x;
            if (p[j].x > max_x) max_x = p[j].x;
            if (p[j].y < min_y) min_y = p[j].y;
            if (p[j].y > max_y) max_y = p[j].y;
        }
    }
    result.bounds[0] = min_x; result.bounds[1] = min_y;
    result.bounds[2] = max_x; result.bounds[3] = max_y;
    if (result.object_bounding_box && (max_x <= min_x || max_y <= min_y)) goto unsupported;
    svg_paint_stop *grown = realloc(*stops, (*stop_count + local_count) * sizeof(**stops));
    if (grown == NULL) {
        if (diagnostic != NULL) *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_OOM, fill.offset};
        return SVG_SHAPE_ERROR_OOM;
    }
    memcpy(grown + *stop_count, local, local_count * sizeof(*local));
    result.first_stop = *stop_count;
    result.stop_count = local_count;
    *stops = grown;
    *stop_count += local_count;
    *paint = result;
    return SVG_SHAPE_ERROR_NONE;
limit:
    if (diagnostic != NULL) *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, fill.offset};
    return SVG_SHAPE_ERROR_LIMIT;
unsupported:
    if (diagnostic != NULL) *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_UNSUPPORTED, fill.offset};
    return SVG_SHAPE_ERROR_UNSUPPORTED;
}

void svg_paint_sample(const svg_shape_paint *paint, const svg_paint_stop *stops,
                      double x, double y, uint8_t rgba[4])
{
    if (paint == NULL || stops == NULL || paint->stop_count < 2U) return;
    double gx = x, gy = y, t;
    if (paint->object_bounding_box) {
        gx = (x - paint->bounds[0]) / (paint->bounds[2] - paint->bounds[0]);
        gy = (y - paint->bounds[1]) / (paint->bounds[3] - paint->bounds[1]);
    }
    if (paint->kind == SVG_SHAPE_PAINT_LINEAR) {
        double dx = paint->coordinates[2] - paint->coordinates[0];
        double dy = paint->coordinates[3] - paint->coordinates[1];
        double length = dx * dx + dy * dy;
        t = length == 0.0 ? 1.0 : ((gx - paint->coordinates[0]) * dx +
                                   (gy - paint->coordinates[1]) * dy) / length;
    } else {
        double dx = gx - paint->coordinates[0], dy = gy - paint->coordinates[1];
        t = hypot(dx, dy) / paint->coordinates[2];
    }
    if (t <= stops[paint->first_stop].offset) {
        memcpy(rgba, stops[paint->first_stop].rgba, 4U);
        return;
    }
    size_t last = paint->first_stop + paint->stop_count - 1U;
    if (t >= stops[last].offset) {
        memcpy(rgba, stops[last].rgba, 4U);
        return;
    }
    size_t right = paint->first_stop + 1U;
    while (right <= last && t > stops[right].offset) ++right;
    size_t left = right - 1U;
    double range = stops[right].offset - stops[left].offset;
    double amount = range <= 0.0 ? 1.0 : (t - stops[left].offset) / range;
    for (size_t c = 0U; c < 4U; ++c)
        rgba[c] = (uint8_t)lround((1.0 - amount) * stops[left].rgba[c] + amount * stops[right].rgba[c]);
}
