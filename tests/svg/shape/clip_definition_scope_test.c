#include "shape.h"
#include "shape_render.h"

#include <stdio.h>
#include <string.h>

#define CHECK(condition) do { \
    if (!(condition)) { \
        (void)fprintf(stderr, "clip definition check failed at %d: %s\n", \
            __LINE__, #condition); \
        result = 1; \
        goto cleanup; \
    } \
} while (0)

static const uint8_t nested_clip_source[] =
    "<svg width='10' height='10' viewBox='0 0 10 10'><g>"
    "<clipPath id='left-half'><rect x='0' y='0' width='5' height='10'/>"
    "</clipPath><rect x='0' y='0' width='10' height='10' fill='#ff0000' "
    "clip-path='url(#left-half)'/></g></svg>";

static const uint8_t missing_clip_source[] =
    "<svg width='10' height='10'><g clip-path='url(#missing)'>"
    "<rect x='0' y='0' width='10' height='10'/></g></svg>";

static const char clip_nonzero_source[] =
    "<svg width='10' height='10'><defs><clipPath id='c'><path "
    "d='M0 0H10V10H0Z M2 2H8V8H2Z'/></clipPath></defs><rect "
    "width='10' height='10' fill='red' clip-path='url(#c)'/></svg>";
static const char clip_evenodd_source[] =
    "<svg width='10' height='10'><defs><clipPath id='c'><path "
    "d='M0 0H10V10H0Z M2 2H8V8H2Z' clip-rule='evenodd'/></clipPath></defs>"
    "<rect width='10' height='10' fill='red' clip-path='url(#c)'/></svg>";
static const char clip_inherited_source[] =
    "<svg width='10' height='10'><defs><clipPath id='c'><g "
    "clip-rule='evenodd'><path d='M0 0H10V10H0Z M2 2H8V8H2Z'/></g>"
    "</clipPath></defs><rect width='10' height='10' fill='red' "
    "clip-path='url(#c)'/></svg>";
static const char clip_inline_source[] =
    "<svg width='10' height='10'><defs><clipPath id='c'><path "
    "style='clip-rule:evenodd' d='M0 0H10V10H0Z M2 2H8V8H2Z'/>"
    "</clipPath></defs><rect width='10' height='10' fill='red' "
    "clip-path='url(#c)'/></svg>";
static const char clip_stylesheet_source[] =
    "<svg width='10' height='10'><defs><clipPath id='c'><style>"
    ".evenodd{clip-rule:evenodd}</style><path class='evenodd' "
    "d='M0 0H10V10H0Z M2 2H8V8H2Z'/></clipPath></defs><rect "
    "width='10' height='10' fill='red' clip-path='url(#c)'/></svg>";
static const char clip_reference_rule_source[] =
    "<svg width='10' height='10'><defs><clipPath id='c'><path "
    "d='M0 0H10V10H0Z M2 2H8V8H2Z'/></clipPath></defs><rect "
    "width='10' height='10' fill='red' clip-rule='evenodd' "
    "clip-path='url(#c)'/></svg>";
static const char clip_invalid_rule_source[] =
    "<svg width='10' height='10'><defs><clipPath id='c'><path "
    "clip-rule='invalid' d='M0 0H10V10H0Z'/></clipPath></defs><rect "
    "width='10' height='10' clip-path='url(#c)'/></svg>";
static const char clip_unsupported_style_source[] =
    "<svg width='10' height='10'><defs><clipPath id='c'><path "
    "fill='red' d='M0 0H10V10H0Z'/></clipPath></defs><rect "
    "width='10' height='10' clip-path='url(#c)'/></svg>";

static size_t rgba_index(const raster_tile *tile, uint32_t x, uint32_t y)
{
    return ((size_t)y * tile->width + x) * 4U;
}

static bool center_is_clipped(const char *source, bool clipped)
{
    svg_shape_document doc = {0};
    raster_tile tile = {0};
    svg_shape_diagnostic diagnostic = {0};
    base_bytes bytes = {(const uint8_t *)source, strlen(source)};
    bool ok = svg_shape_decode(bytes, &doc, &diagnostic) ==
        SVG_SHAPE_ERROR_NONE && doc.source.ptr == bytes.ptr &&
        doc.source.len == bytes.len && raster_shape_render(&doc, &tile,
            &diagnostic);
    if (ok) {
        size_t center = rgba_index(&tile, 5U, 5U);
        ok = (tile.rgba[center + 3U] == 0U) == clipped;
    }
    raster_tile_destroy(&tile);
    svg_shape_document_destroy(&doc);
    return ok;
}

int main(void)
{
    int result = 0;
    svg_shape_document doc = {0};
    raster_tile tile = {0};
    svg_shape_diagnostic diagnostic = {0};

    CHECK(svg_shape_decode((base_bytes){nested_clip_source,
        sizeof(nested_clip_source) - 1U}, &doc, &diagnostic) ==
        SVG_SHAPE_ERROR_NONE);
    CHECK(doc.source.ptr == nested_clip_source &&
        doc.source.len == sizeof(nested_clip_source) - 1U);
    CHECK(doc.shape_count == 1U && doc.has_clips);
    CHECK(raster_shape_render(&doc, &tile, &diagnostic));
    size_t inside = rgba_index(&tile, 2U, 5U);
    size_t outside = rgba_index(&tile, 7U, 5U);
    CHECK(tile.rgba[inside] > 0U && tile.rgba[inside + 3U] > 0U);
    CHECK(tile.rgba[outside + 3U] == 0U);
    raster_tile_destroy(&tile);
    svg_shape_document_destroy(&doc);

    CHECK(center_is_clipped(clip_nonzero_source, false));
    CHECK(center_is_clipped(clip_evenodd_source, true));
    CHECK(center_is_clipped(clip_inherited_source, true));
    CHECK(center_is_clipped(clip_inline_source, true));
    CHECK(center_is_clipped(clip_stylesheet_source, true));
    CHECK(center_is_clipped(clip_reference_rule_source, false));

    size_t invalid_rule_offset = (size_t)(strstr(clip_invalid_rule_source,
        "invalid") - clip_invalid_rule_source);
    CHECK(svg_shape_decode((base_bytes){
        (const uint8_t *)clip_invalid_rule_source,
        strlen(clip_invalid_rule_source)}, &doc, &diagnostic) ==
        SVG_SHAPE_ERROR_UNSUPPORTED);
    CHECK(diagnostic.category == SVG_SHAPE_ERROR_UNSUPPORTED &&
        diagnostic.offset == invalid_rule_offset);
    svg_shape_document_destroy(&doc);

    CHECK(svg_shape_decode((base_bytes){
        (const uint8_t *)clip_unsupported_style_source,
        strlen(clip_unsupported_style_source)}, &doc, &diagnostic) ==
        SVG_SHAPE_ERROR_UNSUPPORTED);
    CHECK(diagnostic.category == SVG_SHAPE_ERROR_UNSUPPORTED);
    svg_shape_document_destroy(&doc);

    size_t missing_offset = (size_t)(strstr((const char *)missing_clip_source,
        "url(#missing)") - (const char *)missing_clip_source);
    CHECK(svg_shape_decode((base_bytes){missing_clip_source,
        sizeof(missing_clip_source) - 1U}, &doc, &diagnostic) ==
        SVG_SHAPE_ERROR_UNSUPPORTED);
    CHECK(diagnostic.category == SVG_SHAPE_ERROR_UNSUPPORTED &&
        diagnostic.offset == missing_offset);

cleanup:
    raster_tile_destroy(&tile);
    svg_shape_document_destroy(&doc);
    return result;
}
