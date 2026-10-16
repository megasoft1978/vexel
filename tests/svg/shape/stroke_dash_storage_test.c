#include "shape.h"
#include "shape_render_internal.h"

#include <math.h>
#include <stdio.h>

#define CHECK(condition) do { \
    if (!(condition)) { \
        (void)fprintf(stderr, "dash storage check failed at %d: %s\n", \
            __LINE__, #condition); \
        result = 1; \
        goto cleanup; \
    } \
} while (0)

static const uint8_t no_dash_source[] =
    "<svg width='8' height='8'><path d='M0 0L7 7' stroke='red'/>"
    "<path d='M0 7L7 0' stroke='blue'/></svg>";

static const uint8_t stored_dash_source[] =
    "<svg width='12' height='12'>"
    "<path d='M0 0L10 0' stroke='red' stroke-dasharray='1 2 3'/>"
    "<path d='M0 1L10 1' stroke='blue' stroke-dasharray='4, 5'/>"
    "<path d='M0 2L10 2' stroke='green'/>"
    "</svg>";

static const uint8_t max_dash_source[] =
    "<svg width='12' height='12'><path d='M0 0L10 0' stroke='red' "
    "stroke-dasharray='1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16'/></svg>";

static const uint8_t invalid_dash_source[] =
    "<svg width='12' height='12'><path d='M0 0L10 0' stroke='red' "
    "stroke-dasharray='1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17'/></svg>";

static bool has_values(const double *actual, const double *expected,
                       size_t count)
{
    if (actual == NULL) return false;
    for (size_t index = 0U; index < count; ++index)
        if (!isfinite(actual[index]) || actual[index] != expected[index])
            return false;
    return true;
}

int main(void)
{
    int result = 0;
    svg_shape_document doc = {0};
    svg_shape_diagnostic diagnostic = {0};
    raster_shape_dash_cache cache = {0};

    CHECK(svg_shape_decode((base_bytes){no_dash_source,
        sizeof(no_dash_source) - 1U}, &doc, &diagnostic) ==
        SVG_SHAPE_ERROR_NONE);
    CHECK(doc.stroke_dash_values == NULL && doc.stroke_dash_value_count == 0U);
    CHECK(svg_shape_stroke_dash_values_for_shape(&doc, 0U) == NULL);
    CHECK(svg_shape_stroke_dash_values_for_shape(&doc, 1U) == NULL);
    svg_shape_document_destroy(&doc);

    CHECK(svg_shape_decode((base_bytes){stored_dash_source,
        sizeof(stored_dash_source) - 1U}, &doc, &diagnostic) ==
        SVG_SHAPE_ERROR_NONE);
    static const double first[] = {1.0, 2.0, 3.0};
    static const double second[] = {4.0, 5.0};
    CHECK(doc.shape_count == 3U && doc.stroke_dash_value_count == 5U);
    CHECK(doc.strokes[0].dash_value_start == 0U &&
        doc.strokes[0].dash_count == 3U);
    CHECK(doc.strokes[1].dash_value_start == 3U &&
        doc.strokes[1].dash_count == 2U);
    CHECK(doc.strokes[2].dash_count == 0U);
    CHECK(has_values(svg_shape_stroke_dash_values_for_shape(&doc, 0U),
        first, 3U));
    CHECK(has_values(svg_shape_stroke_dash_values_for_shape(&doc, 1U),
        second, 2U));
    CHECK(svg_shape_stroke_dash_values_for_shape(&doc, 2U) == NULL);
    svg_shape_document_destroy(&doc);

    CHECK(svg_shape_decode((base_bytes){max_dash_source,
        sizeof(max_dash_source) - 1U}, &doc, &diagnostic) ==
        SVG_SHAPE_ERROR_NONE);
    CHECK(doc.strokes[0].dash_count == SVG_SHAPE_MAX_DASH_VALUES);
    CHECK(doc.stroke_dash_value_count == SVG_SHAPE_MAX_DASH_VALUES);
    CHECK(svg_shape_stroke_dash_values_for_shape(&doc, 0U)[15] == 16.0);
    svg_shape_document_destroy(&doc);

    CHECK(svg_shape_decode((base_bytes){invalid_dash_source,
        sizeof(invalid_dash_source) - 1U}, &doc, &diagnostic) ==
        SVG_SHAPE_ERROR_NUMBER);
    CHECK(diagnostic.category == SVG_SHAPE_ERROR_NUMBER);

    svg_shape_document_destroy(&doc);

    static const uint8_t large_dashed_line[] =
        "<svg width='16' height='16' viewBox='0 0 140000 20'>"
        "<path d='M0 10L140000 10' fill='none' stroke='red' "
        "stroke-dasharray='1 1'/></svg>";
    CHECK(svg_shape_decode((base_bytes){large_dashed_line,
        sizeof(large_dashed_line) - 1U}, &doc, &diagnostic) ==
        SVG_SHAPE_ERROR_NONE);
    CHECK(raster_shape_dash_cache_create(&doc, doc.edges, NULL, &cache,
        &diagnostic));
    CHECK(cache.segment_count > 65536U);
    CHECK(cache.build_steps <= RASTER_DASH_CACHE_MAX_STEPS);
    size_t range_bytes, segment_bytes, join_bytes, total_bytes;
    CHECK(cache.shape_count <= SIZE_MAX / sizeof(*cache.shapes));
    CHECK(cache.segment_capacity <= SIZE_MAX / sizeof(*cache.segments));
    CHECK(cache.join_capacity <= SIZE_MAX / sizeof(*cache.joins));
    range_bytes = cache.shape_count * sizeof(*cache.shapes);
    segment_bytes = cache.segment_capacity * sizeof(*cache.segments);
    join_bytes = cache.join_capacity * sizeof(*cache.joins);
    CHECK(range_bytes <= SIZE_MAX - segment_bytes);
    total_bytes = range_bytes + segment_bytes;
    CHECK(total_bytes <= SIZE_MAX - join_bytes);
    total_bytes += join_bytes;
    CHECK(total_bytes <= SIZE_MAX - cache.grid_bytes);
    total_bytes += cache.grid_bytes;
    CHECK(total_bytes <= RASTER_DASH_CACHE_BYTE_LIMIT);
    raster_shape_dash_cache_destroy(&cache);

cleanup:
    raster_shape_dash_cache_destroy(&cache);
    svg_shape_document_destroy(&doc);
    return result;
}
