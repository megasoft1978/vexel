#include "shape.h"
#include "shape_render_internal.h"
#include "shape_render_reference.h"

#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static bool read_file(const char *path, uint8_t **out_bytes, size_t *out_length)
{
    *out_bytes = NULL;
    *out_length = 0U;
    FILE *file = fopen(path, "rb");
    if (file == NULL) return false;
    bool ok = fseek(file, 0, SEEK_END) == 0;
    long length = ok ? ftell(file) : -1L;
    if (length < 0L || length > 16L * 1024L * 1024L) ok = false;
    if (ok && fseek(file, 0, SEEK_SET) != 0) ok = false;
    size_t size = ok ? (size_t)length : 0U;
    uint8_t *bytes = ok ? malloc(size == 0U ? 1U : size) : NULL;
    if (ok && bytes == NULL) ok = false;
    if (ok && fread(bytes, 1U, size, file) != size) ok = false;
    if (fclose(file) != 0) ok = false;
    if (!ok) { free(bytes); return false; }
    *out_bytes = bytes;
    *out_length = size;
    return true;
}

static void make_translucent(svg_shape_document *doc)
{
    for (size_t edge = 0U; edge < doc->edge_count; edge++) {
        if (doc->colors[edge * 4U + 3U] != 0U)
            doc->colors[edge * 4U + 3U] =
                (uint8_t)(48U + ((size_t)doc->shape_ids[edge] * 73U) % 192U);
    }
    for (size_t shape = 0U; shape < doc->shape_count; shape++) {
        if (doc->strokes[shape].rgba[3] != 0U)
            doc->strokes[shape].rgba[3] = (uint8_t)(64U + (shape * 47U) % 160U);
    }
}

int main(int argc, char **argv)
{
    bool translucent = argc == 4 && strcmp(argv[1], "--translucent") == 0;
    if (argc != (translucent ? 4 : 3)) return 2;
    const char *input_path = argv[translucent ? 2 : 1];
    const char *output_path = argv[translucent ? 3 : 2];
    uint8_t *bytes = NULL;
    size_t length = 0U;
    if (!read_file(input_path, &bytes, &length)) return 2;
    svg_shape_document doc = {0};
    svg_shape_diagnostic diag = {0};
    if (svg_shape_decode((base_bytes){bytes, length}, &doc, &diag) != SVG_SHAPE_ERROR_NONE) {
        fprintf(stderr, "%s:%zu\n", svg_shape_error_name(diag.category), diag.offset);
        free(bytes);
        return 1;
    }
    if (translucent) make_translucent(&doc);
    raster_tile tile = {0};
    raster_shape_work_stats work = {0};
    if (!raster_shape_render_with_stats(&doc, &tile, &diag, &work)) {
        fprintf(stderr, "%s:%zu\n", svg_shape_error_name(diag.category), diag.offset);
        svg_shape_document_destroy(&doc);
        free(bytes);
        return 1;
    }
    raster_tile reference_tile = {0};
    if (translucent &&
        (!raster_shape_render_reference(&doc, &reference_tile, &diag) ||
         reference_tile.byte_count != tile.byte_count ||
         memcmp(reference_tile.rgba, tile.rgba, tile.byte_count) != 0)) {
        size_t mismatch = 0U;
        while (mismatch < tile.byte_count && reference_tile.rgba != NULL &&
               tile.rgba[mismatch] == reference_tile.rgba[mismatch]) mismatch++;
        fprintf(stderr, "reference-mismatch:%zu\n", mismatch);
        raster_tile_destroy(&reference_tile);
        raster_tile_destroy(&tile);
        svg_shape_document_destroy(&doc);
        free(bytes);
        return 3;
    }
    FILE *output = fopen(output_path, "wb");
    bool ok = output != NULL && fwrite(tile.rgba, 1U, tile.byte_count, output) == tile.byte_count;
    if (output != NULL && fclose(output) != 0) ok = false;
    if (ok) {
        fprintf(stderr, "shape_pixels=%" PRIu64 "\n", work.shape_pixels);
        fprintf(stderr, "shape_samples=%" PRIu64 "\n", work.shape_samples);
        fprintf(stderr, "fill_edge_tests=%" PRIu64 "\n", work.fill_edge_tests);
        fprintf(stderr, "stroke_tests=%" PRIu64 "\n", work.stroke_tests);
        fprintf(stderr, "scanline_edge_entries=%" PRIu64 "\n", work.scanline_edge_entries);
        fprintf(stderr, "shape_row_visits=%" PRIu64 "\n", work.shape_row_visits);
        fprintf(stderr, "boundary_pixel_marks=%" PRIu64 "\n", work.boundary_pixel_marks);
        fprintf(stderr, "total_work=%" PRIu64 "\n", work.total_work);
    }
    raster_tile_destroy(&reference_tile);
    raster_tile_destroy(&tile);
    svg_shape_document_destroy(&doc);
    free(bytes);
    return ok ? 0 : 2;
}
