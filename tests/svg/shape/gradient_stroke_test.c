#include "shape.h"
#include "shape_render.h"

#include <stdio.h>
#include <string.h>

#define CHECK(condition) do { \
    if (!(condition)) { \
        (void)fprintf(stderr, "gradient stroke check failed at %d: %s\n", \
            __LINE__, #condition); \
        result = 1; \
        goto cleanup; \
    } \
} while (0)

static const uint8_t linear_source[] =
    "<svg width='20' height='20' viewBox='0 0 20 20'><defs>"
    "<linearGradient id='g' x1='0%' y1='0%' x2='100%' y2='0%'>"
    "<stop stop-color='#ff0000'/><stop offset='1' stop-color='#0000ff'/>"
    "</linearGradient></defs><rect x='2' y='2' width='16' height='16' "
    "fill='none' stroke='url(#g)' stroke-width='4' stroke-opacity='.5'/>"
    "</svg>";

static const uint8_t implicit_solid_source[] =
    "<svg width='8' height='8'><rect x='1' y='1' width='6' height='6'/>"
    "</svg>";

static const uint8_t solid_stroke_source[] =
    "<svg width='8' height='8'><path d='M1 1L7 7' stroke='#123456'/>"
    "</svg>";

static const uint8_t gradient_fill_source[] =
    "<svg width='8' height='8'><defs><linearGradient id='g'>"
    "<stop stop-color='#ff0000'/><stop offset='1' stop-color='#0000ff'/>"
    "</linearGradient></defs><rect width='8' height='8' fill='url(#g)'/>"
    "</svg>";

static const uint8_t radial_source[] =
    "<svg width='20' height='20' viewBox='0 0 20 20'><defs>"
    "<radialGradient id='g'><stop stop-color='#ff0000'/>"
    "<stop offset='1' stop-color='#0000ff'/></radialGradient></defs>"
    "<circle cx='10' cy='10' r='6' fill='none' stroke='url(#g)' "
    "stroke-width='4'/></svg>";

static const uint8_t overlapping_two_source[] =
    "<svg width='16' height='16' viewBox='0 0 16 16'><defs>"
    "<linearGradient id='g'><stop stop-color='#f4c96b'/>"
    "<stop offset='1' stop-color='#3289a7'/></linearGradient></defs>"
    "<g fill='none' stroke='url(#g)'>"
    "<path d='M4 4h4v2h-4z'/><path d='M4 4h4v2h-4z'/>"
    "</g></svg>";

static const uint8_t overlapping_ten_source[] =
    "<svg width='16' height='16' viewBox='0 0 16 16'><defs>"
    "<linearGradient id='g'><stop stop-color='#f4c96b'/>"
    "<stop offset='1' stop-color='#3289a7'/></linearGradient></defs>"
    "<g fill='none' stroke='url(#g)'>"
    "<path d='M4 4h4v2h-4z'/><path d='M4 4h4v2h-4z'/>"
    "<path d='M4 4h4v2h-4z'/><path d='M4 4h4v2h-4z'/>"
    "<path d='M4 4h4v2h-4z'/><path d='M4 4h4v2h-4z'/>"
    "<path d='M4 4h4v2h-4z'/><path d='M4 4h4v2h-4z'/>"
    "<path d='M4 4h4v2h-4z'/><path d='M4 4h4v2h-4z'/>"
    "</g></svg>";

static size_t rgba_index(const raster_tile *tile, uint32_t x, uint32_t y)
{
    return ((size_t)y * tile->width + x) * 4U;
}

int main(void)
{
    int result = 0;
    svg_shape_document doc = {0};
    raster_tile tile = {0};
    svg_shape_diagnostic diagnostic = {0};

    CHECK(svg_shape_decode((base_bytes){implicit_solid_source,
        sizeof(implicit_solid_source) - 1U}, &doc, &diagnostic) ==
        SVG_SHAPE_ERROR_NONE);
    CHECK(doc.paints == NULL && doc.paint_count == 0U &&
        svg_shape_paint_for_shape(&doc, 0U) == NULL);
    svg_shape_document_destroy(&doc);

    CHECK(svg_shape_decode((base_bytes){solid_stroke_source,
        sizeof(solid_stroke_source) - 1U}, &doc, &diagnostic) ==
        SVG_SHAPE_ERROR_NONE);
    CHECK(doc.stroke_paints == NULL && doc.stroke_paint_count == 0U &&
        svg_shape_stroke_paint_for_shape(&doc, 0U) == NULL);
    svg_shape_document_destroy(&doc);

    CHECK(svg_shape_decode((base_bytes){gradient_fill_source,
        sizeof(gradient_fill_source) - 1U}, &doc, &diagnostic) ==
        SVG_SHAPE_ERROR_NONE);
    CHECK(doc.paint_count == 1U &&
        svg_shape_paint_for_shape(&doc, 0U) != NULL);
    svg_shape_document_destroy(&doc);

    CHECK(svg_shape_decode((base_bytes){linear_source,
        sizeof(linear_source) - 1U}, &doc, &diagnostic) == SVG_SHAPE_ERROR_NONE);
    CHECK(doc.source.ptr == linear_source &&
        doc.source.len == sizeof(linear_source) - 1U);
    CHECK(doc.stroke_paint_count == 1U &&
        doc.stroke_paint_indices_by_shape[0] == 1U &&
        svg_shape_stroke_paint_for_shape(&doc, 0U) != NULL);
    CHECK(raster_shape_render(&doc, &tile, &diagnostic));
    size_t left = rgba_index(&tile, 4U, 3U);
    size_t right = rgba_index(&tile, 15U, 3U);
    CHECK(tile.rgba[left + 3U] >= 126U && tile.rgba[left + 3U] <= 129U);
    CHECK(tile.rgba[right + 3U] >= 126U && tile.rgba[right + 3U] <= 129U);
    CHECK(tile.rgba[left] > tile.rgba[left + 2U]);
    CHECK(tile.rgba[right + 2U] > tile.rgba[right]);
    raster_tile_destroy(&tile);
    svg_shape_document_destroy(&doc);

    CHECK(svg_shape_decode((base_bytes){radial_source,
        sizeof(radial_source) - 1U}, &doc, &diagnostic) == SVG_SHAPE_ERROR_NONE);
    CHECK(raster_shape_render(&doc, &tile, &diagnostic));
    size_t outer = rgba_index(&tile, 10U, 4U);
    CHECK(tile.rgba[outer + 3U] > 0U &&
        tile.rgba[outer + 2U] > tile.rgba[outer]);
    raster_tile_destroy(&tile);
    svg_shape_document_destroy(&doc);

    CHECK(svg_shape_decode((base_bytes){overlapping_two_source,
        sizeof(overlapping_two_source) - 1U}, &doc, &diagnostic) ==
        SVG_SHAPE_ERROR_NONE);
    CHECK(raster_shape_render(&doc, &tile, &diagnostic));
    size_t overlap = rgba_index(&tile, 5U, 4U);
    CHECK(tile.rgba[overlap + 3U] >= 191U &&
        tile.rgba[overlap + 3U] <= 193U);
    raster_tile_destroy(&tile);
    svg_shape_document_destroy(&doc);

    CHECK(svg_shape_decode((base_bytes){overlapping_ten_source,
        sizeof(overlapping_ten_source) - 1U}, &doc, &diagnostic) ==
        SVG_SHAPE_ERROR_NONE);
    CHECK(raster_shape_render(&doc, &tile, &diagnostic));
    overlap = rgba_index(&tile, 5U, 4U);
    CHECK(tile.rgba[overlap + 3U] >= 253U);
    raster_tile_destroy(&tile);
    svg_shape_document_destroy(&doc);

    static const uint8_t missing[] =
        "<svg width='8' height='8'><path d='M1 1L7 7' "
        "stroke='url(#missing)' stroke-width='2'/></svg>";
    size_t missing_offset = (size_t)(strstr((const char *)missing,
        "url(#missing)") - (const char *)missing);
    CHECK(svg_shape_decode((base_bytes){missing, sizeof(missing) - 1U},
        &doc, &diagnostic) == SVG_SHAPE_ERROR_UNSUPPORTED);
    CHECK(diagnostic.offset == missing_offset);

cleanup:
    raster_tile_destroy(&tile);
    svg_shape_document_destroy(&doc);
    return result;
}
