#include "shape.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

#define CHECK(condition) do { \
    if (!(condition)) { \
        (void)fprintf(stderr, "path number check failed at %d: %s\n", \
            __LINE__, #condition); \
        return 1; \
    } \
} while (0)

static size_t find_bytes(const uint8_t *source, size_t length,
    size_t start, const char *needle)
{
    size_t needle_length = strlen(needle);
    if (start > length || needle_length > length - start) return SIZE_MAX;
    for (size_t i = start; i <= length - needle_length; ++i)
        if (memcmp(source + i, needle, needle_length) == 0) return i;
    return SIZE_MAX;
}

int main(void)
{
    static const uint8_t compact[] =
        "<svg width='8' height='8'><path d='M-.4.6L1.2.4Z'/></svg>";
    static const uint8_t separated[] =
        "<svg width='8' height='8'><path d='M-0.4 0.6 L1.2 0.4 Z'/></svg>";
    svg_shape_document a = {0}, b = {0};
    svg_shape_diagnostic diagnostic = {0};
    CHECK(svg_shape_decode((base_bytes){compact, sizeof(compact) - 1U},
        &a, &diagnostic) == SVG_SHAPE_ERROR_NONE);
    CHECK(svg_shape_decode((base_bytes){separated, sizeof(separated) - 1U},
        &b, &diagnostic) == SVG_SHAPE_ERROR_NONE);
    CHECK(a.source.ptr == compact && a.source.len == sizeof(compact) - 1U);
    CHECK(b.source.ptr == separated && b.source.len == sizeof(separated) - 1U);
    CHECK(a.path_number_count == 4U && b.path_number_count == 4U);
    CHECK(a.edge_count == b.edge_count);
    const double expected[] = {-0.4, 0.6, 1.2, 0.4};
    const char *compact_literals[] = {"-.4", ".6", "1.2", ".4"};
    size_t search_from = 0U;
    for (size_t i = 0U; i < 4U; ++i) {
        const svg_shape_path_number *number = &a.path_numbers[i];
        size_t literal_offset = find_bytes(compact, sizeof(compact) - 1U,
            search_from,
            compact_literals[i]);
        CHECK(literal_offset != SIZE_MAX);
        CHECK(number->offset == literal_offset &&
            number->length == strlen(compact_literals[i]));
        search_from = literal_offset + number->length;
        CHECK(memcmp(compact + number->offset, compact_literals[i],
            number->length) == 0);
        CHECK(fabs(number->value - expected[i]) < 1e-12);
        CHECK(fabs(b.path_numbers[i].value - number->value) < 1e-12);
    }
    for (size_t i = 0U; i < a.edge_count; ++i) {
        CHECK(fabs(a.edges[i].a.x - b.edges[i].a.x) < 1e-12);
        CHECK(fabs(a.edges[i].a.y - b.edges[i].a.y) < 1e-12);
        CHECK(fabs(a.edges[i].b.x - b.edges[i].b.x) < 1e-12);
        CHECK(fabs(a.edges[i].b.y - b.edges[i].b.y) < 1e-12);
    }
    svg_shape_document_destroy(&a);
    svg_shape_document_destroy(&b);

    static const uint8_t malformed[] =
        "<svg width='8' height='8'><path d='M.4..6'/></svg>";
    size_t malformed_group = find_bytes(malformed, sizeof(malformed) - 1U, 0U,
        ".4..6");
    CHECK(malformed_group != SIZE_MAX);
    CHECK(svg_shape_decode((base_bytes){malformed, sizeof(malformed) - 1U},
        &a, &diagnostic) == SVG_SHAPE_ERROR_PATH);
    CHECK(diagnostic.offset == malformed_group);
    svg_shape_document_destroy(&a);
    return 0;
}
