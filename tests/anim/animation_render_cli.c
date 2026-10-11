#include "shape.h"
#include "shape_render.h"

#include <errno.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define ANIMATION_RENDER_SOURCE_LIMIT (16U * 1024U * 1024U)
#define ANIMATION_RENDER_TIME_LIMIT 3600.0

static bool read_file(const char *path, uint8_t **out_bytes, size_t *out_length)
{
    FILE *file = fopen(path, "rb");
    if (file == NULL) return false;
    bool okay = fseek(file, 0L, SEEK_END) == 0;
    long length = okay ? ftell(file) : -1L;
    if (length <= 0L || (unsigned long)length > ANIMATION_RENDER_SOURCE_LIMIT ||
        fseek(file, 0L, SEEK_SET) != 0) okay = false;
    uint8_t *bytes = okay ? malloc((size_t)length) : NULL;
    if (bytes == NULL) okay = false;
    if (okay) okay = fread(bytes, 1U, (size_t)length, file) == (size_t)length;
    if (fclose(file) != 0) okay = false;
    if (!okay) {
        free(bytes);
        return false;
    }
    *out_bytes = bytes;
    *out_length = (size_t)length;
    return true;
}

static bool parse_time(const char *text, double *out)
{
    if (text == NULL || out == NULL || text[0] == '\0') return false;
    errno = 0;
    char *end = NULL;
    double value = strtod(text, &end);
    if (errno != 0 || end == text || *end != '\0' || !isfinite(value) ||
        value < 0.0 || value > ANIMATION_RENDER_TIME_LIMIT) return false;
    *out = value;
    return true;
}

int main(int argc, char **argv)
{
    if (argc != 4) return 2;
    double seconds = 0.0;
    uint8_t *source = NULL;
    size_t source_length = 0U;
    if (!parse_time(argv[2], &seconds) || !read_file(argv[1], &source, &source_length))
        return 2;
    svg_shape_document document = {0};
    svg_shape_diagnostic diagnostic = {SVG_SHAPE_ERROR_NONE, 0U};
    svg_shape_error error = svg_shape_decode((base_bytes){source, source_length},
                                              &document, &diagnostic);
    if (error != SVG_SHAPE_ERROR_NONE) {
        (void)fprintf(stderr, "%s at byte %zu\n", svg_shape_error_name(error),
                      diagnostic.offset);
        free(source);
        return 1;
    }
    raster_tile tile = {0};
    bool okay = raster_shape_render_at(&document, seconds, &tile, &diagnostic);
    if (!okay) {
        (void)fprintf(stderr, "%s at byte %zu\n",
            svg_shape_error_name(diagnostic.category), diagnostic.offset);
    }
    FILE *output = okay ? fopen(argv[3], "wb") : NULL;
    okay = okay && output != NULL && fwrite(tile.rgba, 1U, tile.byte_count, output) ==
        tile.byte_count;
    if (output != NULL && fclose(output) != 0) okay = false;
    raster_tile_destroy(&tile);
    svg_shape_document_destroy(&document);
    free(source);
    return okay ? 0 : 1;
}
