#include "shape.h"
#include "shape_render.h"

#include <stdio.h>
#include <stdlib.h>

#define INPUT_LIMIT (16U * 1024U * 1024U)

svg_shape_media_providers svg_app_media_providers(void);

int main(int argc, char **argv)
{
    if (argc != 3) return 2;
    FILE *input = fopen(argv[1], "rb");
    if (input == NULL || fseek(input, 0L, SEEK_END) != 0) {
        if (input != NULL) (void)fclose(input);
        return 2;
    }
    long file_length = ftell(input);
    if (file_length < 0L || (unsigned long)file_length > INPUT_LIMIT ||
        fseek(input, 0L, SEEK_SET) != 0) {
        (void)fclose(input);
        return 2;
    }
    size_t length = (size_t)file_length;
    uint8_t *source = malloc(length == 0U ? 1U : length);
    if (source == NULL || fread(source, 1U, length, input) != length) {
        free(source);
        (void)fclose(input);
        return 2;
    }
    if (fclose(input) != 0) {
        free(source);
        return 2;
    }

    svg_shape_media_providers providers = svg_app_media_providers();
    svg_shape_document document = {0};
    svg_shape_diagnostic diagnostic = {0};
    svg_shape_error error = svg_shape_decode_with_media(
        (base_bytes){source, length}, &providers, &document, &diagnostic);
    if (error != SVG_SHAPE_ERROR_NONE) {
        fprintf(stderr, "%s:%zu\n", svg_shape_error_name(error),
                diagnostic.offset);
        free(source);
        return 1;
    }
    raster_tile tile = {0};
    if (!raster_shape_render(&document, &tile, &diagnostic)) {
        fprintf(stderr, "%s:%zu\n",
                svg_shape_error_name(diagnostic.category), diagnostic.offset);
        svg_shape_document_destroy(&document);
        free(source);
        return 1;
    }
    FILE *output = fopen(argv[2], "wb");
    bool okay = output != NULL &&
        fwrite(tile.rgba, 1U, tile.byte_count, output) == tile.byte_count;
    if (output != NULL && fclose(output) != 0) okay = false;
    raster_tile_destroy(&tile);
    svg_shape_document_destroy(&document);
    free(source);
    return okay ? 0 : 2;
}
