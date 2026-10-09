#include "shape.h"
#include "shape_render.h"

#include <stdio.h>
#include <stdlib.h>
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
    static const uint8_t overshoot[] = "<svg width=\"8\" height=\"8\"><path d=\"M0 0 C100 0 100 0 1 0\"/></svg>";
    if (svg_shape_decode((base_bytes){overshoot, sizeof(overshoot)-1U}, &doc, &diagnostic) != SVG_SHAPE_ERROR_NONE) return 8;
    bool retained_overshoot = false;
    for (size_t i = 0; i < doc.edge_count; i++) {
        if (doc.edges[i].a.x > 1.0 || doc.edges[i].b.x > 1.0) retained_overshoot = true;
    }
    svg_shape_document_destroy(&doc);
    if (!retained_overshoot) return 9;
    size_t source_bytes = 330000U + 256U;
    uint8_t *many_paths = malloc(source_bytes);
    if (many_paths == NULL) return 10;
    static const char path_start[] = "<svg width=\"8\" height=\"8\"><path d=\"M0 0";
    static const char path_middle[] = "\"/><path d=\"M0 0";
    static const char path_end[] = "\"/></svg>";
    size_t offset = sizeof(path_start) - 1U;
    memcpy(many_paths, path_start, offset);
    for (size_t path = 0; path < 2U; path++) {
        for (size_t i = 0; i < 33000U; i++) {
            memcpy(many_paths + offset, " L1 1", 5U);
            offset += 5U;
        }
        if (path == 0U) {
            memcpy(many_paths + offset, path_middle, sizeof(path_middle) - 1U);
            offset += sizeof(path_middle) - 1U;
        }
    }
    memcpy(many_paths + offset, path_end, sizeof(path_end) - 1U);
    offset += sizeof(path_end) - 1U;
    if (svg_shape_decode((base_bytes){many_paths, offset}, &doc, &diagnostic) == SVG_SHAPE_ERROR_NONE) {
        svg_shape_document_destroy(&doc); free(many_paths); return 11;
    }
    free(many_paths);
    if (diagnostic.category != SVG_SHAPE_ERROR_LIMIT) return 12;
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
