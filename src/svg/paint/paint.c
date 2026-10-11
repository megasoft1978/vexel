#include "paint.h"
#include "../shape/shape_alloc.h"
#include "../shape/shape_internal.h"
#include "../css/color.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

#define SVG_PAINT_CACHE_SLOT_COUNT (SVG_PAINT_CACHE_MAX_GRADIENTS * 2U)
#define SVG_PAINT_MAX_TOKEN_VISITS 4194304U

typedef struct svg_cached_gradient {
    uint32_t token_index;
    double viewport_width;
    double viewport_height;
    svg_shape_paint_kind kind;
    svg_shape_spread_method spread_method;
    bool object_bounding_box;
    double coordinates[6];
    geom_affine gradient_inverse;
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

const geom_edge *svg_paint_edge_span(const svg_shape_document *document,
                                     size_t first_edge, size_t edge_count,
                                     bool *formed)
{
    if (formed != NULL) *formed = false;
    if (edge_count == 0U || document == NULL || document->edges == NULL ||
        first_edge > document->edge_count ||
        edge_count > document->edge_count - first_edge) return NULL;
    if (formed != NULL) *formed = true;
    return &document->edges[first_edge];
}

static size_t cache_hash(uint32_t token_index, double viewport_width,
                        double viewport_height)
{
    uint64_t width_bits = 0U, height_bits = 0U;
    memcpy(&width_bits, &viewport_width, sizeof(width_bits));
    memcpy(&height_bits, &viewport_height, sizeof(height_bits));
    uint64_t value = (uint64_t)token_index ^ width_bits ^
                     (height_bits + UINT64_C(0x9e3779b97f4a7c15));
    value ^= value >> 30U;
    value *= UINT64_C(0xbf58476d1ce4e5b9);
    value ^= value >> 27U;
    value *= UINT64_C(0x94d049bb133111eb);
    value ^= value >> 31U;
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
                                       uint32_t token_index,
                                       double viewport_width,
                                       double viewport_height)
{
    if (cache == NULL || cache->slots == NULL) return NULL;
    size_t slot = cache_hash(token_index, viewport_width, viewport_height);
    for (size_t probes = 0U; probes < SVG_PAINT_CACHE_SLOT_COUNT; ++probes) {
        uint32_t stored = cache->slots[slot];
        if (stored == 0U) return NULL;
        size_t entry = (size_t)(stored - 1U);
        if (entry < cache->entry_count &&
            cache->entries[entry].token_index == token_index &&
            cache->entries[entry].viewport_width == viewport_width &&
            cache->entries[entry].viewport_height == viewport_height)
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
    size_t slot = cache_hash(gradient->token_index,
                             gradient->viewport_width,
                             gradient->viewport_height);
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

static bool count_token_visit(size_t *token_visits)
{
    if (token_visits == NULL) return true;
    if (*token_visits >= SVG_PAINT_MAX_TOKEN_VISITS) return false;
    (*token_visits)++;
    return true;
}

static bool gradient_attributes_supported(base_bytes source,
                                          const svg_xml_token *tokens,
                                          size_t count, size_t gradient,
                                          bool linear,
                                          size_t *token_visits,
                                          bool *hit_limit)
{
    *hit_limit = false;
    for (size_t i = gradient + 1U;
         i < count && tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE; ++i) {
        if (!count_token_visit(token_visits)) {
            *hit_limit = true;
            return false;
        }
        svg_xml_span name = tokens[i].local_name;
        bool href = eq(source, name, "href") &&
            (tokens[i].prefix.length == 0U ||
             (eq(source, tokens[i].prefix, "xlink") &&
              svg_reference_xlink_is_bound(source, tokens, count)));
        bool common = tokens[i].prefix.length == 0U &&
            (eq(source, name, "id") || eq(source, name, "class") ||
             eq(source, name, "style") || eq(source, name, "gradientUnits") ||
             eq(source, name, "spreadMethod") ||
             eq(source, name, "gradientTransform"));
        bool coordinate = tokens[i].prefix.length == 0U &&
            (linear ? (eq(source, name, "x1") || eq(source, name, "y1") ||
                       eq(source, name, "x2") || eq(source, name, "y2")) :
                      (eq(source, name, "cx") || eq(source, name, "cy") ||
                       eq(source, name, "r") || eq(source, name, "fx") ||
                       eq(source, name, "fy")));
        if (!href && !common && !coordinate) return false;
    }
    return true;
}

static bool gradient_href(base_bytes source, const svg_xml_token *tokens,
                          size_t count, size_t gradient, bool *present,
                          svg_xml_span *value)
{
    *present = false;
    for (size_t i = gradient + 1U;
         i < count && tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE; ++i) {
        if (!eq(source, tokens[i].local_name, "href")) continue;
        if (*present ||
            (tokens[i].prefix.length != 0U &&
             (!eq(source, tokens[i].prefix, "xlink") ||
              !svg_reference_xlink_is_bound(source, tokens, count))) ||
            tokens[i].value.offset > source.len ||
            tokens[i].value.length < 2U ||
            tokens[i].value.length > source.len - tokens[i].value.offset ||
            source.ptr[tokens[i].value.offset] != '#') return false;
        *present = true;
        *value = tokens[i].value;
    }
    return true;
}

static svg_css_error gradient_inherited_color(
    svg_css_sheet *css, base_bytes source, const svg_xml_token *tokens,
    size_t count, size_t gradient, size_t *token_visits,
    svg_css_paint *color_style)
{
    size_t ancestors[64];
    size_t ancestor_count = 0U;
    for (size_t i = 0U; i <= gradient; ++i) {
        if (!count_token_visit(token_visits)) return SVG_CSS_LIMIT;
        if (tokens[i].kind != SVG_XML_TOKEN_START_ELEMENT &&
            tokens[i].kind != SVG_XML_TOKEN_EMPTY_ELEMENT) continue;
        while (ancestor_count != 0U &&
               tokens[ancestors[ancestor_count - 1U]].depth >= tokens[i].depth)
            ancestor_count--;
        if (ancestor_count == sizeof(ancestors) / sizeof(ancestors[0]))
            return SVG_CSS_LIMIT;
        ancestors[ancestor_count++] = i;
    }
    svg_css_paint inherited = svg_css_initial_paint();
    for (size_t i = 0U; i < ancestor_count; ++i) {
        svg_css_paint computed;
        svg_css_error error = svg_css_compute(css, source, tokens, count,
            ancestors[i], i == 0U ? NULL : &inherited, &computed);
        if (error != SVG_CSS_OK) return error;
        inherited = computed;
    }
    *color_style = (svg_css_paint){0};
    if (inherited.values[SVG_CSS_COLOR].present)
        color_style->values[SVG_CSS_COLOR] = inherited.values[SVG_CSS_COLOR];
    return SVG_CSS_OK;
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
    return svg_css_parse_color(source, span, rgba);
}

static bool resolve_stop_color(base_bytes source, svg_xml_span span,
                               const svg_css_paint *style,
                               uint8_t rgba[4])
{
    if (!svg_css_keyword_equals(source, span, "currentColor"))
        return color(source, span, rgba);
    svg_xml_span color_value;
    if (!svg_css_paint_value(style, SVG_CSS_COLOR, &color_value) ||
        svg_css_keyword_equals(source, color_value, "currentColor")) {
        rgba[0] = rgba[1] = rgba[2] = 0U;
        rgba[3] = 255U;
        return true;
    }
    return color(source, color_value, rgba);
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
            !eq(source, property, "stop-opacity") &&
            !eq(source, property, "color")) return false;
        i++;
        while (i < end && source.ptr[i] != ';') i++;
        if (i < end) i++;
    }
    return true;
}

static bool parse_stops(svg_css_sheet *css, base_bytes source,
                        const svg_xml_token *tokens, size_t count,
                        size_t gradient, const svg_css_paint *gradient_style,
                        size_t *token_visits,
                        svg_paint_stop stops[SVG_PAINT_MAX_GRADIENT_STOPS],
                        size_t *stop_count, bool *has_stop_elements,
                        bool *hit_limit, bool *hit_oom)
{
    *has_stop_elements = false;
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
        *has_stop_elements = true;
        if (!eq(source, tokens[i].local_name, "stop") ||
            tokens[i].prefix.length != 0U) return false;
        if (n == SVG_PAINT_MAX_GRADIENT_STOPS) { *hit_limit = true; return false; }
        svg_xml_span offset = {0}, stop_color = {0}, opacity = {0}, style = {0};
        (void)attr(source, tokens, count, i, "offset", &offset);
        if (attr(source, tokens, count, i, "style", &style) &&
            !supported_stop_inline_style(source, style)) return false;
        svg_css_paint computed;
        svg_css_error css_error = svg_css_compute(css, source, tokens, count, i,
                                                 gradient_style, &computed);
        if (css_error != SVG_CSS_OK) {
            *hit_limit = css_error == SVG_CSS_LIMIT;
            *hit_oom = css_error == SVG_CSS_OOM;
            return false;
        }
        for (size_t property = 0U; property < SVG_CSS_PROPERTY_COUNT; ++property) {
            if (property != SVG_CSS_STOP_COLOR &&
                property != SVG_CSS_STOP_OPACITY && property != SVG_CSS_COLOR &&
                computed.values[property].present) return false;
        }
        bool has_stop_color = svg_css_paint_value(
            &computed, SVG_CSS_STOP_COLOR, &stop_color);
        bool has_opacity = svg_css_paint_value(
            &computed, SVG_CSS_STOP_OPACITY, &opacity);
        double position = 0.0, alpha = 1.0;
        if (offset.length != 0U && !number(source, offset, &position)) return false;
        if (has_opacity && !number(source, opacity, &alpha)) return false;
        if (position < 0.0 || position > 1.0 || alpha < 0.0 || alpha > 1.0 ||
            (n != 0U && position < stops[n - 1U].offset)) return false;
        if (!has_stop_color) {
            stops[n].rgba[0] = stops[n].rgba[1] = stops[n].rgba[2] = 0U;
            stops[n].rgba[3] = 255U;
        } else if (stop_color.length == 0U ||
                   !resolve_stop_color(source, stop_color, &computed,
                                       stops[n].rgba))
            return false;
        stops[n].offset = position;
        stops[n].rgba[3] = (uint8_t)lround((double)stops[n].rgba[3] * alpha);
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
                                 size_t count, const size_t *chain,
                                 size_t chain_count, bool object_box,
                                 double viewport_width, double viewport_height,
                                 svg_shape_paint *paint)
{
    static const char *const linear_names[4] = {"x1", "y1", "x2", "y2"};
    static const char *const radial_names[5] = {"cx", "cy", "r", "fx", "fy"};
    const char *const *names = paint->kind == SVG_SHAPE_PAINT_LINEAR ?
        linear_names : radial_names;
    double diagonal = hypot(viewport_width, viewport_height) / sqrt(2.0);
    double defaults[5] = {0.0};
    if (paint->kind == SVG_SHAPE_PAINT_LINEAR) {
        defaults[2] = object_box ? 1.0 : viewport_width;
    } else {
        defaults[0] = object_box ? 0.5 : viewport_width * 0.5;
        defaults[1] = object_box ? 0.5 : viewport_height * 0.5;
        defaults[2] = object_box ? 0.5 : diagonal * 0.5;
        defaults[3] = defaults[0];
        defaults[4] = defaults[1];
    }
    size_t coordinate_count = paint->kind == SVG_SHAPE_PAINT_LINEAR ? 4U : 5U;
    bool coordinate_found[5] = {false, false, false, false, false};
    for (size_t coordinate = 0U; coordinate < coordinate_count; ++coordinate) {
        paint->coordinates[coordinate] = defaults[coordinate];
        for (size_t definition = 0U; definition < chain_count; ++definition) {
            svg_xml_span value;
            if (!attr(source, tokens, count, chain[definition],
                      names[coordinate], &value)) continue;
            coordinate_found[coordinate] = true;
            double reference = object_box ? 1.0 :
                (paint->kind == SVG_SHAPE_PAINT_LINEAR ?
                    ((coordinate & 1U) == 0U ? viewport_width : viewport_height) :
                    coordinate == 2U ? diagonal :
                    (coordinate == 0U || coordinate == 3U ?
                        viewport_width : viewport_height));
            if (object_box) {
                size_t end = value.offset + value.length;
                for (size_t at = value.offset; at < end; ++at) {
                    uint8_t byte = source.ptr[at];
                    if ((byte >= 'a' && byte <= 'z') ||
                        (byte >= 'A' && byte <= 'Z')) return false;
                }
            }
            if (!svg_shape_read_length(source, value, reference,
                                       &paint->coordinates[coordinate]))
                return false;
            break;
        }
    }
    if (paint->kind == SVG_SHAPE_PAINT_RADIAL && !coordinate_found[3]) paint->coordinates[3] = paint->coordinates[0];
    if (paint->kind == SVG_SHAPE_PAINT_RADIAL && !coordinate_found[4]) paint->coordinates[4] = paint->coordinates[1];
    return paint->kind != SVG_SHAPE_PAINT_RADIAL ||
           (paint->coordinates[2] > 0.0 && isfinite(paint->coordinates[2]));
}

static bool collect_gradient_chain(base_bytes source,
                                   const svg_xml_token *tokens,
                                   size_t token_count,
                                   svg_reference_index *references,
                                   size_t target, bool linear,
                                   size_t *token_visits, size_t chain[16],
                                   size_t *chain_count, bool *hit_limit)
{
    *chain_count = 0U;
    *hit_limit = false;
    size_t current = target;
    while (true) {
        if (*chain_count == 16U) {
            *hit_limit = true;
            return false;
        }
        bool current_linear = eq(source, tokens[current].local_name,
                                 "linearGradient");
        bool current_radial = eq(source, tokens[current].local_name,
                                 "radialGradient");
        if ((!current_linear && !current_radial) || current_linear != linear ||
            tokens[current].prefix.length != 0U) return false;
        bool attr_limit = false;
        if (!gradient_attributes_supported(source, tokens, token_count,
                current, linear, token_visits, &attr_limit)) {
            *hit_limit = attr_limit;
            return false;
        }
        chain[(*chain_count)++] = current;
        bool has_href = false;
        svg_xml_span href = {0};
        if (!gradient_href(source, tokens, token_count, current,
                           &has_href, &href)) return false;
        if (!has_href) return true;
        svg_xml_span id = {href.offset + 1U, href.length - 1U};
        size_t parent;
        if (!svg_reference_index_find(source, tokens, token_count,
                                      references, id, &parent)) return false;
        for (size_t i = 0U; i < *chain_count; ++i)
            if (chain[i] == parent) return false;
        current = parent;
    }
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

    svg_cached_gradient *cached = cache_find(*cache, (uint32_t)target,
                                             viewport_width,
                                             viewport_height);
    if (cached == NULL) {
        svg_shape_error cache_error = cache_create(cache);
        if (cache_error == SVG_SHAPE_ERROR_LIMIT) goto limit;
        if (cache_error != SVG_SHAPE_ERROR_NONE) goto oom;

        size_t chain[16], chain_count = 0U;
        bool chain_limit = false;
        if (!collect_gradient_chain(source, tokens, token_count, references,
                target, linear, token_visits, chain, &chain_count,
                &chain_limit)) {
            if (chain_limit) goto limit;
            goto unsupported;
        }
        svg_shape_paint definition = {0};
        definition.kind = linear ? SVG_SHAPE_PAINT_LINEAR :
                                   SVG_SHAPE_PAINT_RADIAL;
        definition.object_bounding_box = true;
        definition.spread_method = SVG_SHAPE_SPREAD_PAD;
        definition.gradient_inverse = geom_affine_identity();
        bool found_units = false, found_spread = false, found_transform = false;
        for (size_t i = 0U; i < chain_count; ++i) {
            svg_xml_span value;
            if (!found_units && attr(source, tokens, token_count, chain[i],
                                     "gradientUnits", &value)) {
                if (eq(source, value, "userSpaceOnUse"))
                    definition.object_bounding_box = false;
                else if (!eq(source, value, "objectBoundingBox"))
                    goto unsupported;
                found_units = true;
            }
            if (!found_spread && attr(source, tokens, token_count, chain[i],
                                      "spreadMethod", &value)) {
                if (eq(source, value, "reflect"))
                    definition.spread_method = SVG_SHAPE_SPREAD_REFLECT;
                else if (eq(source, value, "repeat"))
                    definition.spread_method = SVG_SHAPE_SPREAD_REPEAT;
                else if (!eq(source, value, "pad"))
                    goto unsupported;
                found_spread = true;
            }
            if (!found_transform && attr(source, tokens, token_count, chain[i],
                                         "gradientTransform", &value)) {
                geom_affine transform;
                size_t operations = 0U;
                if (!svg_shape_transform_parse(source, value, &transform,
                                               &operations) ||
                    !svg_shape_affine_inverse(transform,
                                              &definition.gradient_inverse))
                    goto unsupported;
                (void)operations;
                found_transform = true;
            }
        }
        if (!gradient_coordinates(source, tokens, token_count, chain,
                chain_count, definition.object_bounding_box, viewport_width,
                viewport_height, &definition)) goto unsupported;

        svg_paint_stop local[SVG_PAINT_MAX_GRADIENT_STOPS];
        size_t local_count = 0U;
        bool inherited_stops = false;
        for (size_t i = 0U; i < chain_count; ++i) {
            svg_css_paint parent_color = {0};
            svg_css_error css_error = gradient_inherited_color(
                css, source, tokens, token_count, chain[i], token_visits,
                &parent_color);
            if (css_error == SVG_CSS_LIMIT) goto limit;
            if (css_error == SVG_CSS_OOM) goto oom;
            if (css_error != SVG_CSS_OK) goto unsupported;
            bool has_stop_elements = false, stop_limit = false, stop_oom = false;
            if (parse_stops(css, source, tokens, token_count, chain[i],
                    &parent_color, token_visits, local, &local_count,
                    &has_stop_elements, &stop_limit, &stop_oom)) {
                inherited_stops = true;
                break;
            }
            if (stop_limit) goto limit;
            if (stop_oom) goto oom;
            if (has_stop_elements) goto unsupported;
        }
        if (!inherited_stops) goto unsupported;
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
        pending.viewport_width = viewport_width;
        pending.viewport_height = viewport_height;
        pending.kind = definition.kind;
        pending.spread_method = definition.spread_method;
        pending.object_bounding_box = definition.object_bounding_box;
        memcpy(pending.coordinates, definition.coordinates,
               sizeof(pending.coordinates));
        pending.gradient_inverse = definition.gradient_inverse;
        pending.first_stop = first_stop;
        pending.stop_count = local_count;
        cache_error = cache_insert(*cache, &pending, &cached);
        if (cache_error == SVG_SHAPE_ERROR_LIMIT) goto limit;
        if (cache_error != SVG_SHAPE_ERROR_NONE) goto oom;
    }

    svg_shape_paint result = {0};
    result.kind = cached->kind;
    result.spread_method = cached->spread_method;
    result.object_bounding_box = cached->object_bounding_box;
    memcpy(result.coordinates, cached->coordinates, sizeof(result.coordinates));
    result.first_stop = cached->first_stop;
    result.stop_count = cached->stop_count;
    result.user_inverse = user_inverse;
    result.gradient_inverse = cached->gradient_inverse;
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
    geom_point transformed;
    if (!geom_affine_map_point(paint->gradient_inverse,
                               (geom_point){gx, gy}, &transformed)) return;
    gx = transformed.x;
    gy = transformed.y;
    if (paint->kind == SVG_SHAPE_PAINT_LINEAR) {
        double dx = paint->coordinates[2] - paint->coordinates[0];
        double dy = paint->coordinates[3] - paint->coordinates[1];
        double length = dx * dx + dy * dy;
        t = length == 0.0 ? 1.0 : ((gx - paint->coordinates[0]) * dx +
                                   (gy - paint->coordinates[1]) * dy) / length;
    } else {
        double focal_x = paint->coordinates[3];
        double focal_y = paint->coordinates[4];
        double center_dx = paint->coordinates[0] - focal_x;
        double center_dy = paint->coordinates[1] - focal_y;
        double point_dx = gx - focal_x;
        double point_dy = gy - focal_y;
        double radius = paint->coordinates[2];
        double a = center_dx * center_dx + center_dy * center_dy -
                   radius * radius;
        double b = -2.0 * (point_dx * center_dx + point_dy * center_dy);
        double c = point_dx * point_dx + point_dy * point_dy;
        if (fabs(a) <= 1e-15) {
            if (fabs(b) <= 1e-15) return;
            t = -c / b;
        } else {
            double discriminant = b * b - 4.0 * a * c;
            if (discriminant < 0.0 || !isfinite(discriminant)) return;
            double root = sqrt(discriminant);
            double t0 = (-b - root) / (2.0 * a);
            double t1 = (-b + root) / (2.0 * a);
            t = fmax(t0, t1);
            if (t < 0.0) t = fmin(t0, t1);
        }
    }
    if (!isfinite(t)) return;
    if (paint->spread_method == SVG_SHAPE_SPREAD_REPEAT) {
        t -= floor(t);
    } else if (paint->spread_method == SVG_SHAPE_SPREAD_REFLECT) {
        t = fmod(t, 2.0);
        if (t < 0.0) t += 2.0;
        if (t > 1.0) t = 2.0 - t;
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
