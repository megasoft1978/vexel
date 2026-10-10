#include "shape.h"
#include "shape_bounds.h"
#include "shape_render.h"

#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define TEST_PAINTED_SHAPES 32768U
#define TEST_CLIP_DEPTH 60U
#define TEST_CLIP_CHILDREN 64U
#define TEST_SOURCE_CAPACITY (4U * 1024U * 1024U)

bool clip_boundary_accounting_check(void);

static bool append_text(char *buffer, size_t capacity, size_t *length,
                        const char *text, size_t text_length)
{
    if (*length >= capacity || text_length > capacity - *length - 1U)
        return false;
    memcpy(buffer + *length, text, text_length);
    *length += text_length;
    buffer[*length] = '\0';
    return true;
}

static bool append_format(char *buffer, size_t capacity, size_t *length,
                          const char *format, ...)
{
    if (*length >= capacity) return false;
    va_list args;
    va_start(args, format);
    int written = vsnprintf(buffer + *length, capacity - *length, format, args);
    va_end(args);
    if (written < 0 || (size_t)written >= capacity - *length) return false;
    *length += (size_t)written;
    return true;
}

static bool build_pathological_document(char *source, size_t capacity,
                                        size_t *length)
{
    static const char opening[] = "<svg width=\"8\" height=\"8\"><defs>";
    static const char clip_open[] =
        "<clipPath id=\"c%zu\" clipPathUnits=\"userSpaceOnUse\">";
    static const char clip_child[] = "<rect width=\"0\" height=\"1\"/>";
    static const char clip_close[] = "</clipPath>";
    static const char defs_close[] = "</defs>";
    static const char group_open[] = "<g clip-path=\"url(#c%zu)\">";
    static const char painted_shape[] =
        "<rect x=\"1\" y=\"1\" width=\"1\" height=\"1\" fill=\"red\"/>";
    static const char group_close[] = "</g>";
    static const char close_svg[] = "</svg>";
    *length = 0U;
    source[0] = '\0';
    if (!append_text(source, capacity, length, opening, sizeof(opening) - 1U))
        return false;
    for (size_t clip = 0U; clip < TEST_CLIP_DEPTH; clip++) {
        if (!append_format(source, capacity, length, clip_open, clip)) return false;
        for (size_t child = 0U; child < TEST_CLIP_CHILDREN; child++)
            if (!append_text(source, capacity, length, clip_child,
                             sizeof(clip_child) - 1U)) return false;
        if (!append_text(source, capacity, length, clip_close,
                         sizeof(clip_close) - 1U)) return false;
    }
    if (!append_text(source, capacity, length, defs_close,
                     sizeof(defs_close) - 1U)) return false;
    for (size_t clip = 0U; clip < TEST_CLIP_DEPTH; clip++)
        if (!append_format(source, capacity, length, group_open, clip))
            return false;
    for (size_t shape = 0U; shape < TEST_PAINTED_SHAPES; shape++)
        if (!append_text(source, capacity, length, painted_shape,
                         sizeof(painted_shape) - 1U)) return false;
    for (size_t clip = 0U; clip < TEST_CLIP_DEPTH; clip++)
        if (!append_text(source, capacity, length, group_close,
                         sizeof(group_close) - 1U)) return false;
    return append_text(source, capacity, length, close_svg,
                       sizeof(close_svg) - 1U);
}

bool clip_boundary_accounting_check(void)
{
    char *source = malloc(TEST_SOURCE_CAPACITY);
    if (source == NULL) return false;
    size_t length = 0U;
    bool ok = build_pathological_document(source, TEST_SOURCE_CAPACITY, &length);
    svg_shape_document doc = {0};
    svg_shape_diagnostic diagnostic = {0};
    if (ok)
        ok = svg_shape_decode((base_bytes){(const uint8_t *)source, length},
                              &doc, &diagnostic) == SVG_SHAPE_ERROR_NONE &&
             doc.shape_count == TEST_PAINTED_SHAPES &&
             doc.edge_count == TEST_PAINTED_SHAPES * 4U;

    uint8_t *bits = calloc(8U, sizeof(*bits));
    raster_boundary_mask boundary = {.bits = bits, .bit_count = 64U};
    if (bits == NULL) ok = false;
    size_t child_shapes = TEST_CLIP_DEPTH * TEST_CLIP_CHILDREN;
    size_t per_shape = TEST_CLIP_DEPTH +
                       2U * (TEST_CLIP_DEPTH + child_shapes) + 3U;
    size_t expected_work = TEST_PAINTED_SHAPES * per_shape;
    if (ok) {
        bool marked = raster_shape_bounds_mark_clip_boundaries(
            &doc, 1.0, 1.0, 0.0, 0.0, &boundary);
        ok = !marked && boundary.mark_work == expected_work &&
             boundary.mark_work > 200000000U;
    }
    raster_tile tile = {0};
    if (ok) {
        bool rendered = raster_shape_render(&doc, &tile, &diagnostic);
        ok = !rendered && diagnostic.category == SVG_SHAPE_ERROR_LIMIT &&
             tile.rgba == NULL;
    }
    raster_tile_destroy(&tile);
    free(bits);
    svg_shape_document_destroy(&doc);
    free(source);
    return ok;
}
