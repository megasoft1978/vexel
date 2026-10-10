#include "platform/media/platform_image.h"
#include "platform/media/platform_text.h"

#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static bool read_image_file(const char *path, uint8_t **out_bytes, size_t *out_length)
{
    *out_bytes = NULL;
    *out_length = 0U;
    FILE *file = fopen(path, "rb");
    if (file == NULL) {
        return false;
    }
    if (fseek(file, 0L, SEEK_END) != 0) {
        (void)fclose(file);
        return false;
    }
    long file_length = ftell(file);
    if (file_length < 0L || (uint64_t)file_length >
        (uint64_t)PLATFORM_IMAGE_MAX_INPUT_BYTES) {
        (void)fclose(file);
        return false;
    }
    if (fseek(file, 0L, SEEK_SET) != 0) {
        (void)fclose(file);
        return false;
    }
    size_t length = (size_t)file_length;
    uint8_t *bytes = length == 0U ? NULL : malloc(length);
    if (length != 0U && bytes == NULL) {
        (void)fclose(file);
        return false;
    }
    bool success = length == 0U || fread(bytes, 1U, length, file) == length;
    int close_status = fclose(file);
    if (!success || close_status != 0) {
        free(bytes);
        return false;
    }
    *out_bytes = bytes;
    *out_length = length;
    return true;
}

static char path_verb(platform_text_path_verb verb)
{
    switch (verb) {
    case PLATFORM_TEXT_PATH_MOVE: return 'M';
    case PLATFORM_TEXT_PATH_LINE: return 'L';
    case PLATFORM_TEXT_PATH_CUBIC: return 'C';
    case PLATFORM_TEXT_PATH_CLOSE: return 'Z';
    }
    return '?';
}

static int text_command(int argc, char **argv)
{
    if (argc < 3 || argc > 4) {
        fprintf(stderr, "usage: media_probe text UTF8_TEXT [FONT_FAMILY]\n");
        return 2;
    }
    platform_text_slice family = {NULL, 0U};
    size_t family_count = 0U;
    if (argc == 4) {
        family.bytes = (const uint8_t *)argv[3];
        family.length = strlen(argv[3]);
        family_count = 1U;
    }
    platform_text_request request = {
        {(const uint8_t *)argv[2], strlen(argv[2])},
        family_count == 0U ? NULL : &family,
        family_count,
        24.0,
        400.0,
        PLATFORM_TEXT_STYLE_NORMAL,
        0.0
    };
    platform_text_result result = PLATFORM_TEXT_RESULT_INIT;
    platform_media_status status = platform_text_layout(&request, &result);
    if (status != PLATFORM_MEDIA_OK) {
        fprintf(stderr, "%s\n", platform_media_status_name(status));
        return 1;
    }
    printf("{\"advance_width\":%.17g,\"ascent\":%.17g,\"descent\":%.17g,"
           "\"leading\":%.17g,\"glyph_count\":%zu,\"command_count\":%zu,"
           "\"glyphs\":[",
           result.advance_width, result.ascent, result.descent, result.leading,
           result.glyph_count, result.command_count);
    for (size_t i = 0U; i < result.glyph_count; ++i) {
        const platform_text_glyph *glyph = &result.glyphs[i];
        if (i != 0U) putchar(',');
        printf("{\"id\":%u,\"x\":%.17g,\"y\":%.17g,\"advance\":%.17g,"
               "\"commands\":[",
               (unsigned)glyph->glyph_id, glyph->x, glyph->y, glyph->advance);
        for (size_t j = 0U; j < glyph->command_count; ++j) {
            const platform_text_path_command *command =
                &result.commands[glyph->command_start + j];
            if (j != 0U) putchar(',');
            putchar('{');
            printf("\"op\":\"%c\",\"values\":[", path_verb(command->verb));
            size_t value_count = command->verb == PLATFORM_TEXT_PATH_CUBIC ? 6U :
                                 command->verb == PLATFORM_TEXT_PATH_CLOSE ? 0U : 2U;
            for (size_t k = 0U; k < value_count; ++k) {
                if (k != 0U) putchar(',');
                printf("%.17g", command->values[k]);
            }
            printf("]}");
        }
        printf("]}");
    }
    printf("]}\n");
    platform_text_result_destroy(&result);
    return ferror(stdout) == 0 ? 0 : 1;
}

static int write_ppm(const char *path, const platform_image *image)
{
    FILE *file = fopen(path, "wb");
    if (file == NULL) {
        return 1;
    }
    int result = fprintf(file, "P6\n%u %u\n255\n", image->width, image->height);
    bool good = result > 0;
    for (uint32_t y = 0U; good && y < image->height; ++y) {
        const uint8_t *row = image->rgba + (size_t)y * image->row_stride;
        for (uint32_t x = 0U; x < image->width; ++x) {
            const uint8_t *pixel = row + (size_t)x * 4U;
            uint8_t rgb[3];
            for (size_t channel = 0U; channel < 3U; ++channel) {
                unsigned value = (unsigned)pixel[channel] +
                                 (255U - (unsigned)pixel[3]);
                rgb[channel] = (uint8_t)(value > 255U ? 255U : value);
            }
            if (fwrite(rgb, 1U, sizeof(rgb), file) != sizeof(rgb)) {
                good = false;
                break;
            }
        }
    }
    if (fclose(file) != 0) {
        good = false;
    }
    return good ? 0 : 1;
}

static int image_command(int argc, char **argv)
{
    if (argc != 4) {
        fprintf(stderr, "usage: media_probe image INPUT_IMAGE OUTPUT.ppm\n");
        return 2;
    }
    uint8_t *bytes = NULL;
    size_t length = 0U;
    if (!read_image_file(argv[2], &bytes, &length)) {
        fprintf(stderr, "cannot read bounded image input\n");
        return 1;
    }
    platform_image image = PLATFORM_IMAGE_INIT;
    platform_media_status status = platform_image_decode(bytes, length, &image);
    free(bytes);
    if (status != PLATFORM_MEDIA_OK) {
        fprintf(stderr, "%s\n", platform_media_status_name(status));
        return 1;
    }
    int result = write_ppm(argv[3], &image);
    platform_image_destroy(&image);
    return result;
}

int main(int argc, char **argv)
{
    if (argc < 2) {
        fprintf(stderr, "usage: media_probe {text|image} ...\n");
        return 2;
    }
    if (strcmp(argv[1], "text") == 0) {
        return text_command(argc, argv);
    }
    if (strcmp(argv[1], "image") == 0) {
        return image_command(argc, argv);
    }
    fprintf(stderr, "unknown media_probe operation\n");
    return 2;
}
