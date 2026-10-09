#include "test_reference.h"
#include "shape.h"
#include "shape_internal.h"
#include "reference.h"
#include "shape_render.h"

#include <CommonCrypto/CommonDigest.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static bool render_source(const char *source, raster_tile *tile,
                          svg_shape_document *doc,
                          svg_shape_diagnostic *diagnostic)
{
    size_t length = strlen(source);
    if (svg_shape_decode((base_bytes){(const uint8_t *)source, length}, doc,
                         diagnostic) != SVG_SHAPE_ERROR_NONE) return false;
    if (!raster_shape_render(doc, tile, diagnostic)) {
        svg_shape_document_destroy(doc);
        return false;
    }
    return true;
}

static uint8_t pixel_alpha(const raster_tile *tile, uint32_t x, uint32_t y);

static bool source_bytes_unchanged_check(void)
{
    uint8_t source[] = "<svg width=\"8\" height=\"8\"><defs><rect id=\"r\" width=\"6\" height=\"6\" fill=\"red\"/></defs><use href=\"#r\"/></svg>";
    const size_t source_len = sizeof(source) - 1U;
    unsigned char before[CC_SHA256_DIGEST_LENGTH], after[CC_SHA256_DIGEST_LENGTH];
    if (CC_SHA256(source, (CC_LONG)source_len, before) == NULL) return false;
    raster_tile tile = {0};
    svg_shape_document doc = {0};
    svg_shape_diagnostic diagnostic;
    bool ok = render_source((const char *)source, &tile, &doc, &diagnostic);
    if (ok) ok = CC_SHA256(source, (CC_LONG)source_len, after) != NULL &&
                 memcmp(before, after, sizeof(before)) == 0;
    raster_tile_destroy(&tile);
    svg_shape_document_destroy(&doc);
    return ok;
}

static bool decode_reference_memory_bounds_check(void)
{
    static const char prefix[] =
        "<svg width=\"8\" height=\"8\"><defs><rect id=\"r\" width=\"2\" height=\"2\"/></defs><!--";
    static const char suffix[] = "--><use href=\"#r\"/></svg>";
    static const uint8_t small[] =
        "<svg width=\"8\" height=\"8\"><defs><rect id=\"r\" width=\"2\" height=\"2\"/></defs><use href=\"#r\"/></svg>";
    const size_t padding = 128U * 1024U;
    size_t large_length = sizeof(prefix) - 1U + padding + sizeof(suffix) - 1U;
    uint8_t *large = malloc(large_length);
    if (large == NULL) return false;
    size_t cursor = sizeof(prefix) - 1U;
    memcpy(large, prefix, cursor);
    memset(large + cursor, 'p', padding);
    cursor += padding;
    memcpy(large + cursor, suffix, sizeof(suffix) - 1U);

    base_bytes inputs[2] = {
        {small, sizeof(small) - 1U},
        {large, large_length}
    };
    bool ok = true;
    for (size_t i = 0U; ok && i < 2U; ++i) {
        unsigned char before[CC_SHA256_DIGEST_LENGTH];
        unsigned char after[CC_SHA256_DIGEST_LENGTH];
        svg_shape_decode_memory_bounds bounds = {0};
        svg_shape_document document = {0};
        svg_shape_diagnostic diagnostic = {0};
        ok = CC_SHA256(inputs[i].ptr, (CC_LONG)inputs[i].len, before) != NULL &&
             svg_shape_decode_source_with_bounds(inputs[i], &document,
                                                  &diagnostic, &bounds) ==
                 SVG_SHAPE_ERROR_NONE;
        if (ok) {
            size_t token_capacity = inputs[i].len / 2U + 2U;
            size_t expected_tokens = token_capacity * sizeof(svg_xml_token);
            ok = document.source.ptr == inputs[i].ptr &&
                 document.source.len == inputs[i].len &&
                 document.shape_count == 1U &&
                 bounds.token_array_bytes == expected_tokens &&
                 bounds.reference_index_bytes ==
                     SVG_REFERENCE_ID_TABLE_CAPACITY * sizeof(uint32_t) &&
                 bounds.rewritten_source_bytes == 0U &&
                 bounds.source_map_bytes == 0U &&
                 CC_SHA256(inputs[i].ptr, (CC_LONG)inputs[i].len, after) != NULL &&
                 memcmp(before, after, sizeof(before)) == 0;
        }
        svg_shape_document_destroy(&document);
    }
    free(large);
    return ok;
}

static uint8_t pixel_alpha(const raster_tile *tile, uint32_t x, uint32_t y);

static bool reference_rejection_checks(void)
{
    static const char *cases[] = {
        "<svg width=\"8\" height=\"8\"><use href=\"#missing\"/></svg>",
        "<svg width=\"8\" height=\"8\"><use href=\"https://example.invalid/a.svg#x\"/></svg>",
        "<svg width=\"8\" height=\"8\"><g id=\"x\"><use href=\"#x\"/></g></svg>",
        "<svg width=\"8\" height=\"8\"><g id=\"x\"/><rect id=\"x\" width=\"1\" height=\"1\"/></svg>",
        "<svg width=\"8\" height=\"8\"><use xlink:href=\"#x\"/><rect id=\"x\" width=\"1\" height=\"1\"/></svg>"
    };
    for (size_t i = 0; i < sizeof(cases) / sizeof(cases[0]); ++i) {
        svg_shape_document doc = {0};
        svg_shape_diagnostic diag = {0};
        base_bytes source = {(const uint8_t *)cases[i], strlen(cases[i])};
        if (svg_shape_decode(source, &doc, &diag) != SVG_SHAPE_ERROR_UNSUPPORTED ||
            diag.category != SVG_SHAPE_ERROR_UNSUPPORTED || diag.offset >= source.len ||
            doc.source.ptr != NULL || doc.edge_count != 0U) return false;
        svg_shape_document_destroy(&doc);
    }
    static const char unbound[] = "<svg width=\"8\" height=\"8\"><use xlink:href=\"#x\"/><rect id=\"x\" width=\"1\" height=\"1\"/></svg>";
    svg_shape_document doc = {0};
    svg_shape_diagnostic diag = {0};
    if (svg_shape_decode((base_bytes){(const uint8_t *)unbound, strlen(unbound)}, &doc, &diag) !=
        SVG_SHAPE_ERROR_UNSUPPORTED) return false;
    static const char nested_unbound[] = "<svg width=\"8\" height=\"8\"><defs><g id=\"x\"><use xlink:href=\"#y\"/></g><rect id=\"y\" width=\"1\" height=\"1\"/></defs><use href=\"#x\"/></svg>";
    if (svg_shape_decode((base_bytes){(const uint8_t *)nested_unbound, strlen(nested_unbound)}, &doc, &diag) !=
        SVG_SHAPE_ERROR_UNSUPPORTED || diag.offset >= strlen(nested_unbound)) return false;
    char too_deep[1024];
    size_t deep_length = 0U;
    for (size_t i = 0U; i < 66U; ++i) {
        memcpy(too_deep + deep_length, "<g>", 3U);
        deep_length += 3U;
    }
    for (size_t i = 0U; i < 66U; ++i) {
        memcpy(too_deep + deep_length, "</g>", 4U);
        deep_length += 4U;
    }
    static const char depth_prefix[] = "<svg width=\"8\" height=\"8\">";
    static const char depth_suffix[] = "</svg>";
    memmove(too_deep + sizeof(depth_prefix) - 1U, too_deep, deep_length);
    memcpy(too_deep, depth_prefix, sizeof(depth_prefix) - 1U);
    deep_length += sizeof(depth_prefix) - 1U;
    memcpy(too_deep + deep_length, depth_suffix, sizeof(depth_suffix) - 1U);
    deep_length += sizeof(depth_suffix) - 1U;
    if (svg_shape_decode((base_bytes){(const uint8_t *)too_deep, deep_length}, &doc, &diag) !=
        SVG_SHAPE_ERROR_LIMIT || diag.category != SVG_SHAPE_ERROR_LIMIT) return false;
    size_t oversized_capacity = 70U * 1024U;
    char *oversized = malloc(oversized_capacity);
    if (oversized == NULL) return false;
    static const char oversized_prefix[] = "<svg width=\"8\" height=\"8\"><defs><g id=\"x\"><!--";
    static const char oversized_suffix[] = "--><rect width=\"1\" height=\"1\"/></g></defs><use href=\"#x\"/></svg>";
    size_t oversized_len = sizeof(oversized_prefix) - 1U;
    memcpy(oversized, oversized_prefix, oversized_len);
    memset(oversized + oversized_len, 'a', 65U * 1024U);
    oversized_len += 65U * 1024U;
    memcpy(oversized + oversized_len, oversized_suffix, sizeof(oversized_suffix) - 1U);
    oversized_len += sizeof(oversized_suffix) - 1U;
    bool bounded = svg_shape_decode((base_bytes){(const uint8_t *)oversized, oversized_len},
                                    &doc, &diag) == SVG_SHAPE_ERROR_LIMIT &&
                   diag.category == SVG_SHAPE_ERROR_LIMIT;
    free(oversized);
    if (!bounded) return false;
    size_t aggregate_capacity = 2U * 1024U * 1024U;
    char *aggregate = malloc(aggregate_capacity);
    if (aggregate == NULL) return false;
    static const char aggregate_prefix[] = "<svg width=\"8\" height=\"8\"><defs><g id=\"x\"><!--";
    static const char aggregate_middle[] = "--><rect width=\"1\" height=\"1\"/></g></defs>";
    static const char aggregate_use[] = "<use href=\"#x\"/>";
    static const char aggregate_suffix[] = "</svg>";
    size_t aggregate_len = sizeof(aggregate_prefix) - 1U;
    memcpy(aggregate, aggregate_prefix, aggregate_len);
    memset(aggregate + aggregate_len, 'b', 48U * 1024U);
    aggregate_len += 48U * 1024U;
    memcpy(aggregate + aggregate_len, aggregate_middle, sizeof(aggregate_middle) - 1U);
    aggregate_len += sizeof(aggregate_middle) - 1U;
    for (size_t i = 0U; i < 22U; ++i) {
        memcpy(aggregate + aggregate_len, aggregate_use, sizeof(aggregate_use) - 1U);
        aggregate_len += sizeof(aggregate_use) - 1U;
    }
    memcpy(aggregate + aggregate_len, aggregate_suffix, sizeof(aggregate_suffix) - 1U);
    aggregate_len += sizeof(aggregate_suffix) - 1U;
    bounded = svg_shape_decode((base_bytes){(const uint8_t *)aggregate, aggregate_len},
                               &doc, &diag) == SVG_SHAPE_ERROR_LIMIT &&
              diag.category == SVG_SHAPE_ERROR_LIMIT;
    free(aggregate);
    if (!bounded) return false;
    static const char xlink[] = "<svg xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"8\" height=\"8\"><use xlink:href=\"#x\" fill=\"red\"/><rect id=\"x\" x=\"1\" y=\"1\" width=\"3\" height=\"3\"/></svg>";
    raster_tile use_tile = {0};
    if (!render_source(xlink, &use_tile, &doc, &diag) ||
        pixel_alpha(&use_tile, 2U, 2U) != 255U || pixel_alpha(&use_tile, 0U, 0U) != 0U ||
        doc.source.ptr != (const uint8_t *)xlink || doc.source.len != strlen(xlink)) {
        raster_tile_destroy(&use_tile); svg_shape_document_destroy(&doc); return false;
    }
    raster_tile_destroy(&use_tile); svg_shape_document_destroy(&doc);
    static const char translated[] = "<svg width=\"8\" height=\"8\"><defs><rect id=\"r\" width=\"2\" height=\"2\" fill=\"blue\"/></defs><use href=\"#r\" x=\"3\" y=\"4\"/></svg>";
    raster_tile translated_tile = {0};
    if (!render_source(translated, &translated_tile, &doc, &diag) ||
        pixel_alpha(&translated_tile, 3U, 4U) != 255U ||
        pixel_alpha(&translated_tile, 2U, 4U) != 0U) {
        raster_tile_destroy(&translated_tile); svg_shape_document_destroy(&doc); return false;
    }
    raster_tile_destroy(&translated_tile); svg_shape_document_destroy(&doc);
    static const char symbol[] = "<svg width=\"8\" height=\"8\"><defs><symbol id=\"s\" viewBox=\"0 0 2 2\"><rect width=\"2\" height=\"2\"/></symbol></defs><use href=\"#s\" x=\"2\" y=\"2\" width=\"4\" height=\"4\" fill=\"green\"/></svg>";
    raster_tile symbol_tile = {0};
    if (!render_source(symbol, &symbol_tile, &doc, &diag) ||
        pixel_alpha(&symbol_tile, 2U, 2U) != 255U ||
        pixel_alpha(&symbol_tile, 1U, 2U) != 0U ||
        pixel_alpha(&symbol_tile, 6U, 6U) != 0U) {
        raster_tile_destroy(&symbol_tile); svg_shape_document_destroy(&doc); return false;
    }
    raster_tile_destroy(&symbol_tile); svg_shape_document_destroy(&doc);
    static const char nested_transforms[] =
        "<svg width=\"12\" height=\"10\"><defs><g id=\"target\" transform=\"translate(1 0)\"><g transform=\"translate(1 0)\"><rect width=\"2\" height=\"2\"/></g></g></defs><g transform=\"translate(2 1)\"><use href=\"#target\" x=\"1\" y=\"2\" transform=\"translate(1 0)\" fill=\"red\"/></g></svg>";
    raster_tile nested_tile = {0};
    if (!render_source(nested_transforms, &nested_tile, &doc, &diag) ||
        pixel_alpha(&nested_tile, 6U, 3U) != 255U ||
        pixel_alpha(&nested_tile, 5U, 3U) != 0U ||
        pixel_alpha(&nested_tile, 8U, 5U) != 0U) {
        raster_tile_destroy(&nested_tile); svg_shape_document_destroy(&doc);
        return false;
    }
    raster_tile_destroy(&nested_tile); svg_shape_document_destroy(&doc);
    static const char css_parent[] =
        "<svg width=\"12\" height=\"10\"><style>.host{fill:blue}</style><defs><rect id=\"target\" width=\"3\" height=\"3\"/></defs><g class=\"host\" transform=\"translate(3 2)\"><use href=\"#target\" x=\"2\" y=\"1\"/></g></svg>";
    raster_tile css_parent_tile = {0};
    if (!render_source(css_parent, &css_parent_tile, &doc, &diag) ||
        pixel_alpha(&css_parent_tile, 5U, 3U) != 255U ||
        pixel_alpha(&css_parent_tile, 4U, 3U) != 0U) {
        raster_tile_destroy(&css_parent_tile); svg_shape_document_destroy(&doc);
        return false;
    }
    size_t css_pixel = ((size_t)3U * css_parent_tile.width + 5U) * 4U;
    if (css_parent_tile.rgba[css_pixel] != 0U ||
        css_parent_tile.rgba[css_pixel + 1U] != 0U ||
        css_parent_tile.rgba[css_pixel + 2U] != 255U) {
        raster_tile_destroy(&css_parent_tile); svg_shape_document_destroy(&doc);
        return false;
    }
    raster_tile_destroy(&css_parent_tile); svg_shape_document_destroy(&doc);
    static const char paint_clip_reference[] =
        "<svg width=\"12\" height=\"10\"><defs><linearGradient id=\"paint\" gradientUnits=\"userSpaceOnUse\" x1=\"0\" y1=\"0\" x2=\"8\" y2=\"0\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient><clipPath id=\"clip\" clipPathUnits=\"userSpaceOnUse\"><rect x=\"1\" y=\"0\" width=\"3\" height=\"8\"/></clipPath><g id=\"target\"><rect width=\"8\" height=\"8\" fill=\"url(#paint)\" clip-path=\"url(#clip)\"/></g></defs><use href=\"#target\"/></svg>";
    raster_tile reference_paint_clip = {0};
    if (!render_source(paint_clip_reference, &reference_paint_clip, &doc,
                       &diag) ||
        pixel_alpha(&reference_paint_clip, 2U, 3U) != 255U ||
        pixel_alpha(&reference_paint_clip, 5U, 3U) != 0U) {
        raster_tile_destroy(&reference_paint_clip);
        svg_shape_document_destroy(&doc);
        return false;
    }
    size_t paint_pixel = ((size_t)3U * reference_paint_clip.width + 2U) * 4U;
    if (reference_paint_clip.rgba[paint_pixel] == 0U ||
        reference_paint_clip.rgba[paint_pixel + 2U] == 0U) {
        raster_tile_destroy(&reference_paint_clip);
        svg_shape_document_destroy(&doc);
        return false;
    }
    raster_tile_destroy(&reference_paint_clip); svg_shape_document_destroy(&doc);
    static const char cycle[] = "<svg width=\"8\" height=\"8\"><defs><g id=\"a\"><use href=\"#b\"/></g><g id=\"b\"><use href=\"#a\"/></g></defs><use href=\"#a\"/></svg>";
    if (svg_shape_decode((base_bytes){(const uint8_t *)cycle, strlen(cycle)}, &doc, &diag) !=
        SVG_SHAPE_ERROR_UNSUPPORTED || diag.offset >= strlen(cycle)) return false;
    static const char mapped_error[] = "<svg width=\"8\" height=\"8\"><defs><g id=\"bad\"><foreign/></g></defs><use href=\"#bad\"/></svg>";
    if (svg_shape_decode((base_bytes){(const uint8_t *)mapped_error, strlen(mapped_error)}, &doc, &diag) !=
        SVG_SHAPE_ERROR_UNSUPPORTED || diag.offset != (size_t)(strstr(mapped_error, "foreign") - mapped_error)) return false;
    char exactly_deep[4096];
    size_t exactly_used = 0U;
    int exactly_wrote = snprintf(exactly_deep, sizeof(exactly_deep),
                                 "<svg width=\"8\" height=\"8\"><defs>");
    if (exactly_wrote < 0 || (size_t)exactly_wrote >= sizeof(exactly_deep))
        return false;
    exactly_used = (size_t)exactly_wrote;
    for (size_t i = 0U; i < 16U; ++i) {
        exactly_wrote = snprintf(exactly_deep + exactly_used,
                                 sizeof(exactly_deep) - exactly_used,
                                 "<g id=\"d%zu\">", i);
        if (exactly_wrote < 0 ||
            (size_t)exactly_wrote >= sizeof(exactly_deep) - exactly_used)
            return false;
        exactly_used += (size_t)exactly_wrote;
        if (i < 15U)
            exactly_wrote = snprintf(exactly_deep + exactly_used,
                                     sizeof(exactly_deep) - exactly_used,
                                     "<use href=\"#d%zu\"/>", i + 1U);
        else
            exactly_wrote = snprintf(exactly_deep + exactly_used,
                                     sizeof(exactly_deep) - exactly_used,
                                     "<rect width=\"1\" height=\"1\"/>");
        if (exactly_wrote < 0 ||
            (size_t)exactly_wrote >= sizeof(exactly_deep) - exactly_used)
            return false;
        exactly_used += (size_t)exactly_wrote;
        memcpy(exactly_deep + exactly_used, "</g>", 4U);
        exactly_used += 4U;
    }
    static const char exactly_tail[] = "</defs><use href=\"#d0\"/></svg>";
    if (sizeof(exactly_tail) - 1U >= sizeof(exactly_deep) - exactly_used)
        return false;
    memcpy(exactly_deep + exactly_used, exactly_tail, sizeof(exactly_tail) - 1U);
    exactly_used += sizeof(exactly_tail) - 1U;
    if (svg_shape_decode((base_bytes){(const uint8_t *)exactly_deep, exactly_used},
                         &doc, &diag) != SVG_SHAPE_ERROR_NONE ||
        doc.shape_count != 1U) return false;
    svg_shape_document_destroy(&doc);
    char deep[4096];
    size_t used = 0U;
    int wrote = snprintf(deep, sizeof(deep), "<svg width=\"8\" height=\"8\"><defs>");
    if (wrote < 0 || (size_t)wrote >= sizeof(deep)) return false;
    used = (size_t)wrote;
    for (size_t i = 0U; i < 17U; ++i) {
        wrote = snprintf(deep + used, sizeof(deep) - used, "<g id=\"g%zu\">", i);
        if (wrote < 0 || (size_t)wrote >= sizeof(deep) - used) return false;
        used += (size_t)wrote;
        if (i < 16U) wrote = snprintf(deep + used, sizeof(deep) - used, "<use href=\"#g%zu\"/>", i + 1U);
        else wrote = snprintf(deep + used, sizeof(deep) - used, "<rect width=\"1\" height=\"1\"/>");
        if (wrote < 0 || (size_t)wrote >= sizeof(deep) - used) return false;
        used += (size_t)wrote;
        memcpy(deep + used, "</g>", 4U); used += 4U;
    }
    static const char deep_tail[] = "</defs><use href=\"#g0\"/></svg>";
    memcpy(deep + used, deep_tail, sizeof(deep_tail) - 1U); used += sizeof(deep_tail) - 1U;
    if (svg_shape_decode((base_bytes){(const uint8_t *)deep, used}, &doc, &diag) !=
        SVG_SHAPE_ERROR_LIMIT || diag.category != SVG_SHAPE_ERROR_LIMIT) return false;
    size_t repeated_capacity = 4U * 1024U * 1024U;
    char *repeated = malloc(repeated_capacity);
    if (repeated == NULL) return false;
    wrote = snprintf(repeated, repeated_capacity,
                     "<svg width=\"1\" height=\"1\"><defs><g id=\"g\"/></defs>");
    used = wrote < 0 ? repeated_capacity : (size_t)wrote;
    bool composed = used < repeated_capacity;
    for (size_t i = 0U; composed && i < 65537U; ++i) {
        wrote = snprintf(repeated + used, repeated_capacity - used, "<use href=\"#g\"/>");
        if (wrote < 0 || (size_t)wrote >= repeated_capacity - used) composed = false;
        else used += (size_t)wrote;
    }
    static const char tail[] = "</svg>";
    if (composed && sizeof(tail) - 1U < repeated_capacity - used) {
        memcpy(repeated + used, tail, sizeof(tail) - 1U); used += sizeof(tail) - 1U;
        composed = svg_shape_decode((base_bytes){(const uint8_t *)repeated, used}, &doc, &diag) ==
                   SVG_SHAPE_ERROR_LIMIT && diag.category == SVG_SHAPE_ERROR_LIMIT;
    } else composed = false;
    free(repeated);
    if (!composed) return false;
    static const char definition_only[] = "<svg width=\"8\" height=\"8\"><defs><g id=\"x\"><rect width=\"2\" height=\"2\"/></g><use href=\"#x\"/></defs><rect width=\"1\" height=\"1\"/></svg>";
    if (svg_shape_decode((base_bytes){(const uint8_t *)definition_only,
                                     strlen(definition_only)}, &doc, &diag) !=
        SVG_SHAPE_ERROR_NONE || doc.shape_count != 1U ||
        doc.source.ptr != (const uint8_t *)definition_only ||
        doc.source.len != strlen(definition_only)) return false;
    svg_shape_document_destroy(&doc);
    return true;
}


static uint8_t pixel_alpha(const raster_tile *tile, uint32_t x, uint32_t y)
{
    return tile->rgba[((size_t)y * tile->width + x) * 4U + 3U];
}

bool svg_shape_reference_tests(void)
{
    return source_bytes_unchanged_check() &&
           decode_reference_memory_bounds_check() &&
           reference_rejection_checks();
}
