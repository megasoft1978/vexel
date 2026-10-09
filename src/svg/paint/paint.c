#include "paint.h"
#include "../shape/shape_alloc.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

#define SVG_PAINT_CACHE_SLOT_COUNT (SVG_PAINT_CACHE_MAX_GRADIENTS * 2U)

typedef struct svg_cached_gradient {
    uint32_t token_index;
    svg_shape_paint_kind kind;
    bool object_bounding_box;
    double coordinates[6];
    size_t first_stop;
    size_t stop_count;
} svg_cached_gradient;

struct svg_paint_cache {
    uint32_t *slots;
    svg_cached_gradient *entries;
    size_t entry_count;
    size_t entry_capacity;
};

void svg_paint_cache_destroy(svg_paint_cache *cache)
{
    if (cache == NULL) return;
    svg_shape_free(cache->slots);
    svg_shape_free(cache->entries);
    svg_shape_free(cache);
}

static size_t cache_hash(uint32_t token_index)
{
    uint32_t value = token_index;
    value ^= value >> 16U;
    value *= UINT32_C(0x7feb352d);
    value ^= value >> 15U;
    value *= UINT32_C(0x846ca68b);
    value ^= value >> 16U;
    return (size_t)value & (SVG_PAINT_CACHE_SLOT_COUNT - 1U);
}

static svg_shape_error cache_create(svg_paint_cache **out)
{
    if (out == NULL) return SVG_SHAPE_ERROR_INVALID_ARGUMENT;
    if (*out != NULL) return SVG_SHAPE_ERROR_NONE;
    if (SVG_PAINT_CACHE_SLOT_COUNT > SIZE_MAX / sizeof(uint32_t))
        return SVG_SHAPE_ERROR_LIMIT;
    svg_paint_cache *cache = svg_shape_calloc(1U, sizeof(*cache));
    if (cache == NULL) return SVG_SHAPE_ERROR_OOM;
    cache->slots = svg_shape_calloc(SVG_PAINT_CACHE_SLOT_COUNT,
                                    sizeof(*cache->slots));
    if (cache->slots == NULL) {
        svg_shape_free(cache);
        return SVG_SHAPE_ERROR_OOM;
    }
    *out = cache;
    return SVG_SHAPE_ERROR_NONE;
}

static svg_cached_gradient *cache_find(svg_paint_cache *cache,
                                       uint32_t token_index)
{
    if (cache == NULL || cache->slots == NULL) return NULL;
    size_t slot = cache_hash(token_index);
    for (size_t probes = 0U; probes < SVG_PAINT_CACHE_SLOT_COUNT; ++probes) {
        uint32_t stored = cache->slots[slot];
        if (stored == 0U) return NULL;
        size_t entry = (size_t)(stored - 1U);
        if (entry < cache->entry_count &&
            cache->entries[entry].token_index == token_index)
            return &cache->entries[entry];
        slot = (slot + 1U) & (SVG_PAINT_CACHE_SLOT_COUNT - 1U);
    }
    return NULL;
}

static svg_shape_error cache_reserve_entry(svg_paint_cache *cache)
{
    if (cache->entry_count >= SVG_PAINT_CACHE_MAX_GRADIENTS ||
        cache->entry_count >= UINT32_MAX) return SVG_SHAPE_ERROR_LIMIT;
    if (cache->entry_count == cache->entry_capacity) {
        size_t capacity = cache->entry_capacity == 0U ? 16U :
                          cache->entry_capacity * 2U;
        if (capacity < cache->entry_capacity) return SVG_SHAPE_ERROR_LIMIT;
        if (capacity > SVG_PAINT_CACHE_MAX_GRADIENTS)
            capacity = SVG_PAINT_CACHE_MAX_GRADIENTS;
        if (capacity > SIZE_MAX / sizeof(*cache->entries))
            return SVG_SHAPE_ERROR_LIMIT;
        svg_cached_gradient *grown = svg_shape_realloc(
            cache->entries, capacity * sizeof(*cache->entries));
        if (grown == NULL) return SVG_SHAPE_ERROR_OOM;
        cache->entries = grown;
        cache->entry_capacity = capacity;
    }
    return SVG_SHAPE_ERROR_NONE;
}

static svg_shape_error cache_insert(svg_paint_cache *cache,
                                    const svg_cached_gradient *gradient,
                                    svg_cached_gradient **out)
{
    if (cache->entry_count >= cache->entry_capacity ||
        cache->entry_count >= SVG_PAINT_CACHE_MAX_GRADIENTS ||
        cache->entry_count >= UINT32_MAX) return SVG_SHAPE_ERROR_LIMIT;
    size_t entry = cache->entry_count;
    cache->entries[entry] = *gradient;
    size_t slot = cache_hash(gradient->token_index);
    for (size_t probes = 0U; probes < SVG_PAINT_CACHE_SLOT_COUNT; ++probes) {
        if (cache->slots[slot] == 0U) {
            cache->slots[slot] = (uint32_t)(entry + 1U);
            cache->entry_count++;
            *out = &cache->entries[entry];
            return SVG_SHAPE_ERROR_NONE;
        }
        slot = (slot + 1U) & (SVG_PAINT_CACHE_SLOT_COUNT - 1U);
    }
    return SVG_SHAPE_ERROR_LIMIT;
}

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

static bool gradient_href_attribute(base_bytes source,
                                    const svg_xml_token *tokens, size_t count,
                                    size_t element, size_t *offset)
{
    for (size_t i = element + 1U;
         i < count && tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE; ++i) {
        if (!eq(source, tokens[i].local_name, "href")) continue;
        if (tokens[i].prefix.length == 0U) {
            *offset = tokens[i].name.offset;
            return true;
        }
        if (eq(source, tokens[i].prefix, "xlink")) {
            if (!svg_reference_xlink_is_bound(source, tokens, count)) {
                *offset = tokens[i].name.offset;
                return true;
            }
            /* Bound xlink references are unsupported gradient inheritance. */
            *offset = tokens[i].name.offset;
            return true;
        }
    }
    return false;
}

static bool count_token_visit(size_t *token_visits)
{
    if (token_visits == NULL) return true;
    if (*token_visits == SIZE_MAX) return false;
    (*token_visits)++;
    return true;
}

static bool element_end(const svg_xml_token *tokens, size_t count, size_t start,
                        size_t *token_visits, size_t *end)
{
    if (tokens[start].kind == SVG_XML_TOKEN_EMPTY_ELEMENT) {
        *end = start;
        return true;
    }
    for (size_t i = start + 1U; i < count; ++i) {
        if (!count_token_visit(token_visits)) return false;
        if (tokens[i].kind == SVG_XML_TOKEN_END_ELEMENT &&
            tokens[i].depth == tokens[start].depth) {
            *end = i;
            return true;
        }
    }
    *end = start;
    return true;
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

static bool supported_stop_inline_style(base_bytes source, svg_xml_span style)
{
    size_t i = style.offset, end = style.offset + style.length;
    while (i < end) {
        while (i < end && (source.ptr[i] == ' ' || source.ptr[i] == '\t' ||
                           source.ptr[i] == '\r' || source.ptr[i] == '\n' ||
                           source.ptr[i] == ';')) i++;
        if (i == end) break;
        size_t name = i;
        while (i < end && source.ptr[i] != ':' && source.ptr[i] != ';') i++;
        if (i == end || source.ptr[i] != ':') return false;
        size_t name_end = i;
        while (name_end > name && (source.ptr[name_end - 1U] == ' ' ||
                                   source.ptr[name_end - 1U] == '\t')) name_end--;
        svg_xml_span property = {name, name_end - name};
        if (!eq(source, property, "stop-color") &&
            !eq(source, property, "stop-opacity")) return false;
        i++;
        while (i < end && source.ptr[i] != ';') i++;
        if (i < end) i++;
    }
    return true;
}

static bool parse_stops(svg_css_sheet *css, base_bytes source,
                        const svg_xml_token *tokens, size_t count,
                        size_t gradient, size_t *token_visits,
                        svg_paint_stop stops[SVG_PAINT_MAX_GRADIENT_STOPS],
                        size_t *stop_count, bool *hit_limit, bool *hit_oom)
{
    *hit_limit = false;
    *hit_oom = false;
    size_t end = gradient;
    if (!element_end(tokens, count, gradient, token_visits, &end)) {
        *hit_limit = true;
        return false;
    }
    size_t n = 0U;
    for (size_t i = gradient + 1U; i < end; ++i) {
        if (!count_token_visit(token_visits)) {
            *hit_limit = true;
            return false;
        }
        if ((tokens[i].kind != SVG_XML_TOKEN_START_ELEMENT &&
             tokens[i].kind != SVG_XML_TOKEN_EMPTY_ELEMENT) ||
            tokens[i].depth != tokens[gradient].depth + 1U) continue;
        if (n == SVG_PAINT_MAX_GRADIENT_STOPS) { *hit_limit = true; return false; }
        if (!eq(source, tokens[i].local_name, "stop") || tokens[i].prefix.length != 0U) return false;
        svg_xml_span offset = {0}, stop_color = {0}, opacity = {0}, style = {0};
        (void)attr(source, tokens, count, i, "offset", &offset);
        if (attr(source, tokens, count, i, "style", &style) &&
            !supported_stop_inline_style(source, style)) return false;
        svg_css_paint computed;
        svg_css_error css_error = svg_css_compute(css, source, tokens, count, i,
                                                 NULL, &computed);
        if (css_error != SVG_CSS_OK) {
            *hit_limit = css_error == SVG_CSS_LIMIT;
            *hit_oom = css_error == SVG_CSS_OOM;
            return false;
        }
        for (size_t property = 0U; property < SVG_CSS_PROPERTY_COUNT; ++property) {
            if (property != SVG_CSS_STOP_COLOR &&
                property != SVG_CSS_STOP_OPACITY &&
                computed.values[property].present) return false;
        }
        bool has_stop_color = svg_css_paint_value(
            &computed, SVG_CSS_STOP_COLOR, &stop_color);
        bool has_opacity = svg_css_paint_value(
            &computed, SVG_CSS_STOP_OPACITY, &opacity);
        svg_xml_span class_name = {0};
        bool has_class = attr(source, tokens, count, i, "class", &class_name);
        if (has_class && !has_stop_color && !has_opacity) return false;
        double position = 0.0, alpha = 1.0;
        if (offset.length != 0U && !number(source, offset, &position)) return false;
        if (has_opacity && !number(source, opacity, &alpha)) return false;
        if (position < 0.0 || position > 1.0 || alpha < 0.0 || alpha > 1.0 ||
            (n != 0U && position < stops[n - 1U].offset)) return false;
        if (!has_stop_color) memset(stops[n].rgba, 0, 3U);
        else if (stop_color.length == 0U || !color(source, stop_color, stops[n].rgba))
            return false;
        stops[n].offset = position;
        stops[n].rgba[3] = (uint8_t)lround(alpha * 255.0);
        ++n;
        if (tokens[i].kind == SVG_XML_TOKEN_START_ELEMENT) {
            size_t stop_end = i;
            if (!element_end(tokens, count, i, token_visits, &stop_end)) {
                *hit_limit = true;
                return false;
            }
            i = stop_end;
        }
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
                                       size_t token_count,
                                       svg_reference_index *references,
                                       svg_css_sheet *css,
                                       svg_paint_cache **cache,
                                       size_t *token_visits,
                                       svg_xml_span fill,
                                       const geom_edge *edges, size_t edge_count,
                                       geom_affine user_inverse,
                                       double viewport_width, double viewport_height,
                                       svg_shape_paint *paint,
                                       svg_paint_stop **stops,
                                       size_t *stop_count,
                                       svg_shape_diagnostic *diagnostic)
{
    if (cache == NULL || stops == NULL || stop_count == NULL || paint == NULL) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_INVALID_ARGUMENT,
                                                 fill.offset};
        return SVG_SHAPE_ERROR_INVALID_ARGUMENT;
    }
    if (fill.length < 7U || fill.offset > source.len ||
        fill.length > source.len - fill.offset ||
        memcmp(source.ptr + fill.offset, "url(#", 5U) != 0 ||
        source.ptr[fill.offset + fill.length - 1U] != ')') return SVG_SHAPE_ERROR_UNSUPPORTED;
    svg_xml_span target_id = {fill.offset + 5U, fill.length - 6U};
    size_t target;
    if (!svg_reference_index_find(source, tokens, token_count, references,
                                  target_id, &target)) goto unsupported;
    if (target > UINT32_MAX) goto limit;
    bool linear = eq(source, tokens[target].local_name, "linearGradient");
    bool radial = eq(source, tokens[target].local_name, "radialGradient");
    if ((!linear && !radial) || tokens[target].prefix.length != 0U)
        goto unsupported;

    svg_cached_gradient *cached = cache_find(*cache, (uint32_t)target);
    if (cached == NULL) {
        svg_shape_error cache_error = cache_create(cache);
        if (cache_error == SVG_SHAPE_ERROR_LIMIT) goto limit;
        if (cache_error != SVG_SHAPE_ERROR_NONE) goto oom;

        svg_shape_paint definition = {0};
        definition.kind = linear ? SVG_SHAPE_PAINT_LINEAR :
                                   SVG_SHAPE_PAINT_RADIAL;
        definition.object_bounding_box = true;
        svg_xml_span value;
        if (attr(source, tokens, token_count, target, "gradientUnits", &value)) {
            if (eq(source, value, "userSpaceOnUse"))
                definition.object_bounding_box = false;
            else if (!eq(source, value, "objectBoundingBox"))
                goto unsupported;
        }
        if (attr(source, tokens, token_count, target, "spreadMethod", &value) &&
            !eq(source, value, "pad")) goto unsupported;
        if (attr(source, tokens, token_count, target, "gradientTransform", &value))
            goto unsupported;
        size_t href_offset = 0U;
        if (gradient_href_attribute(source, tokens, token_count, target,
                                    &href_offset)) {
            if (diagnostic != NULL)
                *diagnostic = (svg_shape_diagnostic){
                    SVG_SHAPE_ERROR_UNSUPPORTED, href_offset};
            return SVG_SHAPE_ERROR_UNSUPPORTED;
        }
        if (!gradient_coordinates(source, tokens, token_count, target,
                                  definition.object_bounding_box,
                                  viewport_width, viewport_height, &definition))
            goto unsupported;

        svg_paint_stop local[SVG_PAINT_MAX_GRADIENT_STOPS];
        size_t local_count = 0U;
        bool stop_limit = false, stop_oom = false;
        if (!parse_stops(css, source, tokens, token_count, target,
                         token_visits, local, &local_count, &stop_limit,
                         &stop_oom)) {
            if (stop_limit) goto limit;
            if (stop_oom) goto oom;
            goto unsupported;
        }
        if (*stop_count > SVG_PAINT_MAX_STOPS ||
            local_count > SVG_PAINT_MAX_STOPS - *stop_count) goto limit;
        cache_error = cache_reserve_entry(*cache);
        if (cache_error == SVG_SHAPE_ERROR_LIMIT) goto limit;
        if (cache_error != SVG_SHAPE_ERROR_NONE) goto oom;
        size_t new_stop_count = *stop_count + local_count;
        if (new_stop_count > SIZE_MAX / sizeof(**stops)) goto limit;
        svg_paint_stop *grown = svg_shape_realloc(
            *stops, new_stop_count * sizeof(**stops));
        if (grown == NULL) goto oom;
        size_t first_stop = *stop_count;
        memcpy(grown + first_stop, local, local_count * sizeof(*local));
        *stops = grown;
        *stop_count = new_stop_count;

        svg_cached_gradient pending = {0};
        pending.token_index = (uint32_t)target;
        pending.kind = definition.kind;
        pending.object_bounding_box = definition.object_bounding_box;
        memcpy(pending.coordinates, definition.coordinates,
               sizeof(pending.coordinates));
        pending.first_stop = first_stop;
        pending.stop_count = local_count;
        cache_error = cache_insert(*cache, &pending, &cached);
        if (cache_error == SVG_SHAPE_ERROR_LIMIT) goto limit;
        if (cache_error != SVG_SHAPE_ERROR_NONE) goto oom;
    }

    svg_shape_paint result = {0};
    result.kind = cached->kind;
    result.object_bounding_box = cached->object_bounding_box;
    memcpy(result.coordinates, cached->coordinates, sizeof(result.coordinates));
    result.first_stop = cached->first_stop;
    result.stop_count = cached->stop_count;
    result.user_inverse = user_inverse;
    if (edge_count == 0U) goto unsupported;
    geom_point first_local;
    if (!geom_affine_map_point(user_inverse, edges[0].a, &first_local))
        goto unsupported;
    double min_x = first_local.x, min_y = first_local.y;
    double max_x = min_x, max_y = min_y;
    for (size_t i = 0U; i < edge_count; ++i) {
        const geom_point source_points[2] = {edges[i].a, edges[i].b};
        for (size_t j = 0U; j < 2U; ++j) {
            geom_point point;
            if (!geom_affine_map_point(user_inverse, source_points[j], &point))
                goto unsupported;
            if (point.x < min_x) min_x = point.x;
            if (point.x > max_x) max_x = point.x;
            if (point.y < min_y) min_y = point.y;
            if (point.y > max_y) max_y = point.y;
        }
    }
    result.bounds[0] = min_x; result.bounds[1] = min_y;
    result.bounds[2] = max_x; result.bounds[3] = max_y;
    if (result.object_bounding_box && (max_x <= min_x || max_y <= min_y)) goto unsupported;
    *paint = result;
    return SVG_SHAPE_ERROR_NONE;
limit:
    if (diagnostic != NULL) *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, fill.offset};
    return SVG_SHAPE_ERROR_LIMIT;
oom:
    if (diagnostic != NULL) *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_OOM, fill.offset};
    return SVG_SHAPE_ERROR_OOM;
unsupported:
    if (diagnostic != NULL) *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_UNSUPPORTED, fill.offset};
    return SVG_SHAPE_ERROR_UNSUPPORTED;
}

void svg_paint_sample(const svg_shape_paint *paint, const svg_paint_stop *stops,
                      double x, double y, uint8_t rgba[4])
{
    if (paint == NULL || stops == NULL || paint->stop_count < 2U) return;
    geom_point local;
    if (!geom_affine_map_point(paint->user_inverse, (geom_point){x, y}, &local))
        return;
    double gx = local.x, gy = local.y, t;
    if (paint->object_bounding_box) {
        gx = (local.x - paint->bounds[0]) /
             (paint->bounds[2] - paint->bounds[0]);
        gy = (local.y - paint->bounds[1]) /
             (paint->bounds[3] - paint->bounds[1]);
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
