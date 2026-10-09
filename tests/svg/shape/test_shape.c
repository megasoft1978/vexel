#include "shape.h"
#include "shape_render.h"

#include <stdio.h>
#include <string.h>

int main(void)
{
    static const uint8_t source[] = "<svg width=\"8\" height=\"8\"><rect x=\"1\" y=\"1\" width=\"6\" height=\"6\" fill=\"#f00\"/></svg>";
    svg_shape_document doc; svg_shape_diagnostic diagnostic;
    if (svg_shape_decode((base_bytes){source, sizeof(source)-1U}, &doc, &diagnostic) != SVG_SHAPE_ERROR_NONE) return 1;
    raster_tile tile;
    if (!raster_shape_render(&doc, &tile, &diagnostic)) return 2;
    size_t inside = ((size_t)3U * 8U + 3U) * 4U;
    size_t outside = ((size_t)0U * 8U + 0U) * 4U;
    bool valid = tile.rgba[inside] == 255U && tile.rgba[inside+1U] == 0U && tile.rgba[inside+3U] == 255U && tile.rgba[outside+3U] == 0U && doc.source.ptr == source && doc.source.len == sizeof(source)-1U;
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    static const uint8_t bad[] = "<svg width=\"8\" height=\"8\"><path d=\"M0 0 A3 3 0 2 0 4 4\"/></svg>";
    if (svg_shape_decode((base_bytes){bad, sizeof(bad)-1U}, &doc, &diagnostic) == SVG_SHAPE_ERROR_NONE) { svg_shape_document_destroy(&doc); return 3; }
    if (diagnostic.category != SVG_SHAPE_ERROR_PATH || diagnostic.offset == 0U || !valid) return 4;
    static const uint8_t blank[] = "<svg width=\"8\" height=\"8\"/>";
    if (svg_shape_decode((base_bytes){blank, sizeof(blank)-1U}, &doc, &diagnostic) != SVG_SHAPE_ERROR_NONE) return 5;
    if (!raster_shape_render(&doc, &tile, &diagnostic)) { svg_shape_document_destroy(&doc); return 6; }
    bool transparent = true;
    for (size_t i = 3U; i < tile.byte_count; i += 4U) transparent = transparent && tile.rgba[i] == 0U;
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!transparent) return 7;
    puts("shape decoder and scalar raster checks passed");
    return 0;
}
