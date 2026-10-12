#include "test_reference.h"
#include "shape.h"
#include "shape_internal.h"
#include "reference.h"
#include "shape_render.h"
#include "shape_alloc.h"

#include <CommonCrypto/CommonDigest.h>
#include <math.h>
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

static bool build_allocation_source(size_t padding, size_t uses,
                                    uint8_t **out, size_t *out_length)
{
    static const char prefix[] =
        "<svg width=\"8\" height=\"8\"><defs><linearGradient id=\"paint\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient><clipPath id=\"clip\" clipPathUnits=\"userSpaceOnUse\"><rect width=\"8\" height=\"8\"/></clipPath><g id=\"target\"><rect width=\"8\" height=\"8\" fill=\"url(#paint)\"/></g></defs><!--";
    static const char middle[] = "-->";
    static const char use[] = "<use href=\"#target\"/>";
    static const char suffix[] =
        "<rect width=\"8\" height=\"8\" fill=\"url(#paint)\" clip-path=\"url(#clip)\"/></svg>";
    if (out == NULL || out_length == NULL || uses == 0U) return false;
    size_t length = sizeof(prefix) - 1U;
    if (padding > SIZE_MAX - length) return false;
    length += padding;
    if (sizeof(middle) - 1U > SIZE_MAX - length) return false;
    length += sizeof(middle) - 1U;
    if (uses > (SIZE_MAX - length) / (sizeof(use) - 1U)) return false;
    length += uses * (sizeof(use) - 1U);
    if (sizeof(suffix) - 1U > SIZE_MAX - length) return false;
    length += sizeof(suffix) - 1U;
    uint8_t *source = malloc(length);
    if (source == NULL) return false;
    size_t cursor = 0U;
    memcpy(source + cursor, prefix, sizeof(prefix) - 1U);
    cursor += sizeof(prefix) - 1U;
    memset(source + cursor, 'p', padding);
    cursor += padding;
    memcpy(source + cursor, middle, sizeof(middle) - 1U);
    cursor += sizeof(middle) - 1U;
    for (size_t i = 0U; i < uses; ++i) {
        memcpy(source + cursor, use, sizeof(use) - 1U);
        cursor += sizeof(use) - 1U;
    }
    memcpy(source + cursor, suffix, sizeof(suffix) - 1U);
    cursor += sizeof(suffix) - 1U;
    *out = source;
    *out_length = cursor;
    return cursor == length;
}

static bool measure_allocation_source(const uint8_t *source, size_t length,
                                     svg_shape_alloc_stats *stats,
                                     size_t *index_bytes, size_t *lookup_probes)
{
    if (source == NULL || stats == NULL || index_bytes == NULL ||
        lookup_probes == NULL || length > UINT32_MAX) return false;
    unsigned char before[CC_SHA256_DIGEST_LENGTH], after[CC_SHA256_DIGEST_LENGTH];
    if (CC_SHA256(source, (CC_LONG)length, before) == NULL) return false;
    svg_shape_document document = {0};
    svg_shape_diagnostic diagnostic = {0};
    svg_shape_decode_memory_bounds bounds = {0};
    raster_tile tile = {0};
    svg_shape_alloc_tracking_start();
    svg_shape_error error = svg_shape_decode_source_with_bounds(
        (base_bytes){source, length}, &document, &diagnostic, &bounds);
    bool ok = error == SVG_SHAPE_ERROR_NONE &&
              raster_shape_render(&document, &tile, &diagnostic);
    svg_shape_alloc_tracking_stop(stats);
    *index_bytes = bounds.reference_index_bytes;
    *lookup_probes = bounds.reference_lookup_probes;
    if (ok) ok = document.source.ptr == source && document.source.len == length &&
        CC_SHA256(source, (CC_LONG)length, after) != NULL &&
        memcmp(before, after, sizeof(before)) == 0;
    raster_tile_destroy(&tile);
    svg_shape_document_destroy(&document);
    return ok;
}

static bool decode_reference_memory_bounds_check(void)
{
    static const char prefix[] =
        "<svg width=\"8\" height=\"8\"><defs><linearGradient id=\"paint\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient><clipPath id=\"clip\" clipPathUnits=\"userSpaceOnUse\"><rect width=\"8\" height=\"8\"/></clipPath><g id=\"target\"><rect width=\"8\" height=\"8\" fill=\"url(#paint)\"/></g></defs><!--";
    static const char use[] = "<use href=\"#target\"/>";
    static const char middle[] = "-->";
    static const char suffix[] =
        "<rect width=\"8\" height=\"8\" fill=\"url(#paint)\" clip-path=\"url(#clip)\"/></svg>";
    const size_t use_count_many = 32U;
    const size_t padding_one = 64U * 1024U;
    size_t use_length = sizeof(use) - 1U;
    if (use_length == 0U || use_count_many - 1U > padding_one / use_length)
        return false;
    size_t padding_many = padding_one - (use_count_many - 1U) * use_length;
    size_t base_length = sizeof(prefix) - 1U + sizeof(middle) - 1U +
                         sizeof(suffix) - 1U;
    if (base_length > SIZE_MAX - padding_one - use_length) return false;
    size_t one_length = base_length + padding_one + use_length;
    size_t many_length = base_length + padding_many + use_count_many * use_length;
    if (one_length != many_length) return false;
    uint8_t *small = NULL, *padded_one = NULL, *padded_many = NULL;
    size_t small_length = 0U, one_size = 0U, many_size = 0U;
    if (!build_allocation_source(0U, 1U, &small, &small_length) ||
        !build_allocation_source(padding_one, 1U, &padded_one, &one_size) ||
        !build_allocation_source(padding_many, use_count_many,
                                 &padded_many, &many_size)) {
        free(small); free(padded_one); free(padded_many);
        return false;
    }
    if (one_size != many_size) {
        free(small); free(padded_one); free(padded_many);
        return false;
    }
    svg_shape_alloc_stats stats[3] = {{0}};
    size_t index_bytes[3] = {0U}, lookup_probes[3] = {0U};
    bool ok = measure_allocation_source(small, small_length, &stats[0],
                                        &index_bytes[0], &lookup_probes[0]) &&
              measure_allocation_source(padded_one, one_size, &stats[1],
                                        &index_bytes[1], &lookup_probes[1]) &&
              measure_allocation_source(padded_many, many_size, &stats[2],
                                        &index_bytes[2], &lookup_probes[2]);
    if (ok) {
        for (size_t i = 0U; i < 3U; ++i) {
            ok = ok && !stats[i].overflow && stats[i].peak_live_bytes != 0U &&
                 stats[i].total_allocated_bytes >= stats[i].peak_live_bytes &&
                 stats[i].allocation_count != 0U && stats[i].clip_copy_bytes != 0U &&
                 stats[i].output_bytes == 8U * 8U * 4U &&
                 stats[i].reference_index_bytes == index_bytes[i] &&
                 index_bytes[i] == SVG_REFERENCE_ID_TABLE_CAPACITY * sizeof(uint32_t) &&
                 lookup_probes[i] != 0U;
        }
        size_t small_non_token = stats[0].total_allocated_bytes -
            stats[0].token_array_bytes;
        size_t padded_non_token = stats[1].total_allocated_bytes -
            stats[1].token_array_bytes;
        size_t small_capacity = small_length / 2U + 2U;
        size_t padded_capacity = one_size / 2U + 2U;
        size_t expected_baseline_growth =
            (padded_capacity - small_capacity) * sizeof(svg_xml_token);
        size_t use_growth_budget = (use_count_many - 1U) * 2048U;
        ok = ok && stats[1].token_array_bytes > stats[0].token_array_bytes &&
             stats[1].token_array_bytes - stats[0].token_array_bytes ==
                 expected_baseline_growth &&
             small_non_token == padded_non_token &&
             stats[1].token_array_bytes == stats[2].token_array_bytes &&
             stats[1].reference_index_bytes == stats[2].reference_index_bytes &&
             stats[2].clip_copy_bytes == stats[1].clip_copy_bytes &&
             stats[2].clip_copy_bytes <= 512U * 1024U &&
             stats[2].total_allocated_bytes >= stats[1].total_allocated_bytes &&
             stats[2].total_allocated_bytes - stats[1].total_allocated_bytes <=
                 use_growth_budget &&
             stats[2].peak_live_bytes >= stats[1].peak_live_bytes &&
             stats[2].peak_live_bytes - stats[1].peak_live_bytes <=
                 use_growth_budget &&
             lookup_probes[2] > lookup_probes[1];
    }
    free(small); free(padded_one); free(padded_many);
    return ok;
}

static bool group_clip_scratch_resolved_once_check(void)
{
    static const char prefix[] =
        "<svg width=\"8\" height=\"8\"><defs><clipPath id=\"c\" clipPathUnits=\"objectBoundingBox\"><rect width=\"0.5\" height=\"1\"/></clipPath></defs><g clip-path=\"url(#c)\">";
    static const char shape[] = "<rect width=\"4\" height=\"4\" fill=\"red\"/>";
    static const char suffix[] = "</g></svg>";
    const size_t counts[] = {1U, 16U};
    svg_shape_alloc_stats stats[2] = {{0}};
    size_t index_bytes[2] = {0U}, lookup_probes[2] = {0U};
    bool ok = true;
    for (size_t test = 0U; test < 2U && ok; ++test) {
        size_t shape_bytes = sizeof(shape) - 1U;
        size_t length = sizeof(prefix) - 1U + sizeof(suffix) - 1U;
        if (counts[test] > (SIZE_MAX - length) / shape_bytes) return false;
        length += counts[test] * shape_bytes;
        uint8_t *source = malloc(length);
        if (source == NULL) return false;
        size_t cursor = 0U;
        memcpy(source + cursor, prefix, sizeof(prefix) - 1U);
        cursor += sizeof(prefix) - 1U;
        for (size_t i = 0U; i < counts[test]; ++i) {
            memcpy(source + cursor, shape, shape_bytes);
            cursor += shape_bytes;
        }
        memcpy(source + cursor, suffix, sizeof(suffix) - 1U);
        cursor += sizeof(suffix) - 1U;
        ok = cursor == length && measure_allocation_source(
            source, length, &stats[test], &index_bytes[test],
            &lookup_probes[test]);
        free(source);
    }
    return ok && !stats[0].overflow && !stats[1].overflow &&
        stats[0].clip_copy_bytes != 0U &&
        stats[0].clip_copy_bytes == stats[1].clip_copy_bytes &&
        stats[0].output_bytes == 8U * 8U * 4U &&
        stats[1].output_bytes == 8U * 8U * 4U &&
        stats[0].reference_index_bytes == stats[1].reference_index_bytes &&
        lookup_probes[0] != 0U && lookup_probes[1] != 0U;
}

static uint8_t pixel_alpha(const raster_tile *tile, uint32_t x, uint32_t y);

static bool sha_same(const char *s, size_t n, const unsigned char *before)
{
    unsigned char after[CC_SHA256_DIGEST_LENGTH];
    return CC_SHA256(s, (CC_LONG)n, after) != NULL &&
           memcmp(before, after, sizeof(after)) == 0;
}

static bool decode_use_target_size(size_t target_size,
                                   svg_shape_error expected_error)
{
    static const char prefix[] =
        "<svg width=\"1\" height=\"1\"><defs><g id=\"x\"><!--";
    static const char suffix[] = "--></g></defs><use href=\"#x\"/></svg>";
    static const size_t fixed_target_size =
        sizeof("<g id=\"x\">") - 1U + sizeof("<!--") - 1U +
        sizeof("-->") - 1U + sizeof("</g>") - 1U;
    if (target_size < fixed_target_size) return false;
    size_t payload_size = target_size - fixed_target_size;
    size_t source_size = sizeof(prefix) - 1U;
    if (payload_size > SIZE_MAX - source_size) return false;
    source_size += payload_size;
    if (sizeof(suffix) - 1U > SIZE_MAX - source_size) return false;
    source_size += sizeof(suffix) - 1U;
    char *source = malloc(source_size);
    if (source == NULL) return false;
    size_t cursor = 0U;
    memcpy(source + cursor, prefix, sizeof(prefix) - 1U);
    cursor += sizeof(prefix) - 1U;
    memset(source + cursor, 'a', payload_size);
    cursor += payload_size;
    memcpy(source + cursor, suffix, sizeof(suffix) - 1U);
    cursor += sizeof(suffix) - 1U;
    svg_shape_document document = {0};
    svg_shape_diagnostic diagnostic = {0};
    unsigned char before[CC_SHA256_DIGEST_LENGTH];
    if (CC_SHA256(source, (CC_LONG)cursor, before) == NULL) { free(source); return false; }
    svg_shape_error error = svg_shape_decode(
        (base_bytes){(const uint8_t *)source, cursor}, &document, &diagnostic);
    bool ok = sha_same(source, cursor, before) && error == expected_error &&
        (expected_error != SVG_SHAPE_ERROR_LIMIT ||
         diagnostic.category == SVG_SHAPE_ERROR_LIMIT);
    if (!ok) fprintf(stderr, "use boundary %zu: expected %d got %d at %zu\n",
                      target_size, (int)expected_error, (int)error,
                      diagnostic.offset);
    svg_shape_document_destroy(&document);
    free(source);
    return ok;
}

static bool build_clip_boundary_source(size_t child_bytes,
                                       size_t declaring_elements,
                                       char **out_source, size_t *out_length)
{
    static const char prefix[] =
        "<svg width=\"1\" height=\"1\"><defs><clipPath id=\"c\" clipPathUnits=\"objectBoundingBox\">";
    static const char child_prefix[] = "<!--";
    static const char child_middle[] = "--><rect width=\"1\" height=\"1\"/>";
    static const char suffix[] = "</clipPath></defs>";
    static const char declaration[] =
        "<rect width=\"1\" height=\"1\" clip-path=\"url(#c)\"/>";
    static const char document_suffix[] = "</svg>";
    const size_t fixed_child = sizeof(child_prefix) - 1U +
                               sizeof(child_middle) - 1U;
    if (out_source == NULL || out_length == NULL || child_bytes < fixed_child ||
        declaring_elements == 0U) return false;
    size_t payload = child_bytes - fixed_child;
    size_t length = sizeof(prefix) - 1U;
    if (child_bytes > SIZE_MAX - length) return false;
    length += child_bytes;
    if (sizeof(suffix) - 1U > SIZE_MAX - length) return false;
    length += sizeof(suffix) - 1U;
    size_t declaration_bytes = sizeof(declaration) - 1U;
    if (declaring_elements > (SIZE_MAX - length) / declaration_bytes) return false;
    length += declaring_elements * declaration_bytes;
    if (sizeof(document_suffix) - 1U > SIZE_MAX - length) return false;
    length += sizeof(document_suffix) - 1U;
    char *source = malloc(length);
    if (source == NULL) return false;
    size_t cursor = 0U;
    memcpy(source + cursor, prefix, sizeof(prefix) - 1U);
    cursor += sizeof(prefix) - 1U;
    memcpy(source + cursor, child_prefix, sizeof(child_prefix) - 1U);
    cursor += sizeof(child_prefix) - 1U;
    memset(source + cursor, 'b', payload);
    cursor += payload;
    memcpy(source + cursor, child_middle, sizeof(child_middle) - 1U);
    cursor += sizeof(child_middle) - 1U;
    memcpy(source + cursor, suffix, sizeof(suffix) - 1U);
    cursor += sizeof(suffix) - 1U;
    for (size_t i = 0U; i < declaring_elements; ++i) {
        memcpy(source + cursor, declaration, declaration_bytes);
        cursor += declaration_bytes;
    }
    memcpy(source + cursor, document_suffix, sizeof(document_suffix) - 1U);
    cursor += sizeof(document_suffix) - 1U;
    *out_source = source;
    *out_length = cursor;
    return cursor == length;
}

static bool decode_clip_boundary(size_t child_bytes, size_t declarations,
                                 svg_shape_error expected_error)
{
    char *source = NULL;
    size_t source_length = 0U;
    if (!build_clip_boundary_source(child_bytes, declarations, &source,
                                    &source_length)) return false;
    svg_shape_document document = {0};
    svg_shape_diagnostic diagnostic = {0};
    unsigned char before[CC_SHA256_DIGEST_LENGTH];
    if (CC_SHA256(source, (CC_LONG)source_length, before) == NULL) { free(source); return false; }
    svg_shape_error error = svg_shape_decode(
        (base_bytes){(const uint8_t *)source, source_length}, &document,
        &diagnostic);
    bool ok = sha_same(source, source_length, before) && error == expected_error &&
        (expected_error != SVG_SHAPE_ERROR_LIMIT ||
         diagnostic.category == SVG_SHAPE_ERROR_LIMIT);
    if (!ok) fprintf(stderr, "clip boundary child=%zu declarations=%zu: expected %d got %d at %zu\n",
                      child_bytes, declarations, (int)expected_error,
                      (int)error, diagnostic.offset);
    svg_shape_document_destroy(&document);
    free(source);
    return ok;
}

static bool reference_boundary_checks(void)
{
    const size_t reference_limit = 64U * 1024U;
    if (!decode_use_target_size(reference_limit, SVG_SHAPE_ERROR_NONE)) {
        fprintf(stderr, "use exact source span failed\n"); return false;
    }
    if (!decode_use_target_size(reference_limit + 1U, SVG_SHAPE_ERROR_LIMIT)) {
        fprintf(stderr, "use oversized source span failed\n"); return false;
    }

    static const char object_box_wrapper[] =
        "<svg width=\"1\" height=\"1\" viewBox=\"0 0 1 1\">";
    const size_t wrapper_bytes = sizeof(object_box_wrapper) - 1U;
    const size_t scratch_limit = 512U * 1024U;
    const size_t suffix_bytes = sizeof("</svg>") - 1U;
    if (wrapper_bytes + suffix_bytes + 1U >= reference_limit) return false;
    const size_t exact_aggregate_child = reference_limit - wrapper_bytes -
                                        suffix_bytes - 1U;
    if (!decode_clip_boundary(reference_limit, 1U, SVG_SHAPE_ERROR_NONE)) {
        fprintf(stderr, "clip exact source span failed\n"); return false;
    }
    if (!decode_clip_boundary(reference_limit + 1U, 1U,
                              SVG_SHAPE_ERROR_LIMIT)) {
        fprintf(stderr, "clip oversized source span failed\n"); return false;
    }
    if (exact_aggregate_child > reference_limit ||
        (wrapper_bytes + exact_aggregate_child + suffix_bytes + 1U) * 8U !=
            scratch_limit) {
        fprintf(stderr, "clip scratch calculation failed: %zu\n",
                exact_aggregate_child); return false;
    }
    if (!decode_clip_boundary(exact_aggregate_child, 8U,
                              SVG_SHAPE_ERROR_NONE)) {
        fprintf(stderr, "clip exact aggregate failed\n"); return false;
    }
    if (!decode_clip_boundary(exact_aggregate_child, 9U,
                              SVG_SHAPE_ERROR_LIMIT)) {
        fprintf(stderr, "clip over aggregate failed\n"); return false;
    }
    return true;
}

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
    for (size_t i = 0U; composed && i < 65536U; ++i) {
        wrote = snprintf(repeated + used, repeated_capacity - used, "<use href=\"#g\"/>");
        if (wrote < 0 || (size_t)wrote >= repeated_capacity - used) composed = false;
        else used += (size_t)wrote;
    }
    static const char tail[] = "</svg>";
    if (composed && sizeof(tail) - 1U < repeated_capacity - used) {
        memcpy(repeated + used, tail, sizeof(tail) - 1U);
        size_t instance_source_len = used + sizeof(tail) - 1U;
        composed = svg_shape_decode(
            (base_bytes){(const uint8_t *)repeated, instance_source_len}, &doc,
            &diag) == SVG_SHAPE_ERROR_NONE && doc.shape_count == 0U;
        svg_shape_document_destroy(&doc);
        wrote = snprintf(repeated + used, repeated_capacity - used,
                         "<use href=\"#g\"/>");
        if (wrote < 0 || (size_t)wrote >= repeated_capacity - used) composed = false;
        else used += (size_t)wrote;
        if (composed && sizeof(tail) - 1U < repeated_capacity - used) {
            memcpy(repeated + used, tail, sizeof(tail) - 1U);
            used += sizeof(tail) - 1U;
            composed = svg_shape_decode(
                (base_bytes){(const uint8_t *)repeated, used}, &doc, &diag) ==
                    SVG_SHAPE_ERROR_LIMIT &&
                diag.category == SVG_SHAPE_ERROR_LIMIT;
        } else composed = false;
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

static bool use_transform_order_check(void)
{
    static const char scaled[] =
        "<svg width=\"16\" height=\"16\"><defs><rect id=\"r\" width=\"1\" height=\"1\"/></defs><use href=\"#r\" x=\"4\" y=\"3\" transform=\"scale(2)\"/></svg>";
    static const char rotated[] =
        "<svg width=\"16\" height=\"16\"><defs><rect id=\"r\" width=\"2\" height=\"1\"/></defs><g transform=\"translate(8 0)\"><use href=\"#r\" x=\"2\" y=\"1\" transform=\"rotate(90)\"/></g></svg>";
    static const char symbol[] =
        "<svg width=\"16\" height=\"16\"><defs><symbol id=\"s\" viewBox=\"0 0 2 2\"><rect width=\"1\" height=\"1\"/></symbol></defs><use href=\"#s\" x=\"2\" y=\"1\" width=\"4\" height=\"4\" transform=\"scale(2)\"/></svg>";
    const char *cases[] = {scaled, rotated, symbol};
    const geom_point expected[] = {{8.0, 6.0}, {7.0, 2.0}, {4.0, 2.0}};
    for (size_t i = 0U; i < sizeof(cases) / sizeof(cases[0]); ++i) {
        svg_shape_document doc = {0};
        svg_shape_diagnostic diagnostic = {0};
        base_bytes source = {(const uint8_t *)cases[i], strlen(cases[i])};
        if (svg_shape_decode(source, &doc, &diagnostic) != SVG_SHAPE_ERROR_NONE ||
            doc.edge_count == 0U ||
            fabs(doc.edges[0].a.x - expected[i].x) > 1e-9 ||
            fabs(doc.edges[0].a.y - expected[i].y) > 1e-9) {
            svg_shape_document_destroy(&doc);
            return false;
        }
        svg_shape_document_destroy(&doc);
    }
    return true;
}

static bool indexed_paint_lookup_check(void)
{
    static const char prefix[] = "<svg width=\"16\" height=\"16\">";
    static const char shape[] = "<rect width=\"1\" height=\"1\" fill=\"url(#late)\"/>";
    static const char suffix[] =
        "<defs><linearGradient id=\"late\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient></defs></svg>";
    const size_t shape_count = 512U;
    size_t capacity = sizeof(prefix) - 1U +
        shape_count * (sizeof(shape) - 1U) + sizeof(suffix) - 1U;
    uint8_t *source = malloc(capacity);
    if (source == NULL) return false;
    size_t length = sizeof(prefix) - 1U;
    memcpy(source, prefix, length);
    for (size_t i = 0U; i < shape_count; ++i) {
        memcpy(source + length, shape, sizeof(shape) - 1U);
        length += sizeof(shape) - 1U;
    }
    memcpy(source + length, suffix, sizeof(suffix) - 1U);
    length += sizeof(suffix) - 1U;
    svg_shape_document document = {0};
    svg_shape_diagnostic diagnostic = {0};
    svg_shape_decode_memory_bounds bounds = {0};
    bool ok = svg_shape_decode_source_with_bounds(
        (base_bytes){source, length}, &document, &diagnostic, &bounds) ==
            SVG_SHAPE_ERROR_NONE &&
        document.shape_count == shape_count &&
        bounds.reference_lookup_probes >= shape_count &&
        bounds.reference_lookup_probes <= shape_count * 2U;
    svg_shape_document_destroy(&document);
    free(source);
    return ok;
}

static bool cached_gradient_scan_check(void)
{
    static const char prefix[] =
        "<svg width=\"8\" height=\"8\"><defs><linearGradient id=\"g\">"
        "<stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/>";
    static const char comment[] = "<!-- padding -->";
    static const char middle[] = "</linearGradient></defs>";
    static const char shape[] = "<rect width=\"1\" height=\"1\" fill=\"url(#g)\"/>";
    static const char suffix[] = "</svg>";
    const size_t comment_count = 4096U;
    const size_t shape_count = 512U;
    size_t length = sizeof(prefix) - 1U;
    if (comment_count > (SIZE_MAX - length) / (sizeof(comment) - 1U)) return false;
    length += comment_count * (sizeof(comment) - 1U);
    if (sizeof(middle) - 1U > SIZE_MAX - length) return false;
    length += sizeof(middle) - 1U;
    if (shape_count > (SIZE_MAX - length) / (sizeof(shape) - 1U)) return false;
    length += shape_count * (sizeof(shape) - 1U);
    if (sizeof(suffix) - 1U > SIZE_MAX - length) return false;
    length += sizeof(suffix) - 1U;
    uint8_t *source = malloc(length);
    if (source == NULL) return false;
    size_t cursor = 0U;
    memcpy(source + cursor, prefix, sizeof(prefix) - 1U);
    cursor += sizeof(prefix) - 1U;
    for (size_t i = 0U; i < comment_count; ++i) {
        memcpy(source + cursor, comment, sizeof(comment) - 1U);
        cursor += sizeof(comment) - 1U;
    }
    memcpy(source + cursor, middle, sizeof(middle) - 1U);
    cursor += sizeof(middle) - 1U;
    for (size_t i = 0U; i < shape_count; ++i) {
        memcpy(source + cursor, shape, sizeof(shape) - 1U);
        cursor += sizeof(shape) - 1U;
    }
    memcpy(source + cursor, suffix, sizeof(suffix) - 1U);
    cursor += sizeof(suffix) - 1U;
    svg_shape_document document = {0};
    svg_shape_diagnostic diagnostic = {0};
    svg_shape_decode_memory_bounds bounds = {0};
    bool ok = cursor == length && svg_shape_decode_source_with_bounds(
        (base_bytes){source, length}, &document, &diagnostic, &bounds) ==
            SVG_SHAPE_ERROR_NONE &&
        document.shape_count == shape_count &&
        document.paint_stop_count == 2U &&
        bounds.paint_token_visits >= comment_count &&
        bounds.paint_token_visits <= 8U *
            (comment_count + shape_count + 64U);
    svg_shape_document_destroy(&document);
    free(source);
    return ok;
}

bool svg_shape_reference_tests(void)
{
    if (!source_bytes_unchanged_check()) { fprintf(stderr, "source preservation failed\n"); return false; }
    if (!svg_shape_w5_reference_tests()) { fprintf(stderr, "switch language selection failed\n"); return false; }
    if (!decode_reference_memory_bounds_check()) { fprintf(stderr, "allocation bounds failed\n"); return false; }
    if (!group_clip_scratch_resolved_once_check()) { fprintf(stderr, "group clip allocation failed\n"); return false; }
    if (!reference_boundary_checks()) { fprintf(stderr, "reference boundaries failed\n"); return false; }
    if (!reference_rejection_checks()) { fprintf(stderr, "reference rejection failed\n"); return false; }
    if (!use_transform_order_check()) { fprintf(stderr, "use transform failed\n"); return false; }
    if (!indexed_paint_lookup_check()) { fprintf(stderr, "indexed lookup failed\n"); return false; }
    if (!cached_gradient_scan_check()) { fprintf(stderr, "gradient cache scan work failed\n"); return false; }
    return true;
}
