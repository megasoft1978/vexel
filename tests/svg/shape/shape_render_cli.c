#include "shape.h"
#include "shape_render.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>

static void print_peak_rss(bool enabled)
{
    if (!enabled) return;
    struct rusage usage;
    if (getrusage(RUSAGE_SELF, &usage) == 0) printf("peak_rss_bytes=%ld\n", usage.ru_maxrss);
}

int main(int argc, char **argv)
{
    bool metrics = argc == 4 && strcmp(argv[1], "--metrics") == 0;
    if (argc != 3 && !metrics) return 2;
    const char *input_path = argv[metrics ? 2 : 1];
    const char *output_path = argv[metrics ? 3 : 2];
    FILE *input = fopen(input_path, "rb");
    if (input == NULL) { print_peak_rss(metrics); return 2; }
    if (fseek(input, 0, SEEK_END) != 0) { fclose(input); print_peak_rss(metrics); return 2; }
    long length = ftell(input);
    if (length < 0 || length > 16L * 1024L * 1024L) { fclose(input); print_peak_rss(metrics); return 2; }
    rewind(input);
    size_t n = (size_t)length;
    uint8_t *bytes = malloc(n == 0U ? 1U : n);
    if (bytes == NULL || fread(bytes, 1U, n, input) != n) { free(bytes); fclose(input); print_peak_rss(metrics); return 2; }
    fclose(input);
    svg_shape_document doc;
    svg_shape_diagnostic diag;
    if (svg_shape_decode((base_bytes){bytes, n}, &doc, &diag) != SVG_SHAPE_ERROR_NONE) {
        fprintf(stderr, "%s:%zu\n", svg_shape_error_name(diag.category), diag.offset);
        print_peak_rss(metrics);
        free(bytes);
        return 1;
    }
    raster_tile tile;
    if (!raster_shape_render(&doc, &tile, &diag)) {
        fprintf(stderr, "%s:%zu\n", svg_shape_error_name(diag.category), diag.offset);
        print_peak_rss(metrics);
        svg_shape_document_destroy(&doc); free(bytes); return 1;
    }
    FILE *output = fopen(output_path, "wb");
    bool ok = output != NULL && fwrite(tile.rgba, 1U, tile.byte_count, output) == tile.byte_count;
    if (output != NULL && fclose(output) != 0) ok = false;
    if (ok) print_peak_rss(metrics);
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc); free(bytes);
    return ok ? 0 : 2;
}
