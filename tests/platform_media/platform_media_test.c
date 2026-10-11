#include "platform/media/platform_image.h"
#include "platform/media/platform_text.h"

#include <math.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define REQUIRE(condition) do { \
    if (!(condition)) { \
        fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
        return false; \
    } \
} while (0)

static bool make_path(char *out, size_t capacity, const char *directory,
                      const char *name)
{
    size_t directory_length = strlen(directory);
    size_t name_length = strlen(name);
    if (directory_length > SIZE_MAX - name_length - 2U ||
        directory_length + name_length + 2U > capacity) {
        return false;
    }
    memcpy(out, directory, directory_length);
    out[directory_length] = '/';
    memcpy(out + directory_length + 1U, name, name_length);
    out[directory_length + name_length + 1U] = '\0';
    return true;
}

static bool read_fixture(const char *directory, const char *name,
                         uint8_t **out_bytes, size_t *out_length)
{
    char path[4096];
    if (!make_path(path, sizeof(path), directory, name)) {
        return false;
    }
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
    bool good = length == 0U || fread(bytes, 1U, length, file) == length;
    int close_status = fclose(file);
    if (!good || close_status != 0) {
        free(bytes);
        return false;
    }
    *out_bytes = bytes;
    *out_length = length;
    return true;
}

static bool path_commands_equal(const platform_text_path_command *left,
                                const platform_text_path_command *right)
{
    return left->verb == right->verb &&
           memcmp(left->values, right->values, sizeof(left->values)) == 0;
}

static bool glyphs_equal(const platform_text_glyph *left,
                         const platform_text_glyph *right)
{
    return left->glyph_id == right->glyph_id && left->x == right->x &&
           left->y == right->y && left->advance == right->advance &&
           left->command_start == right->command_start &&
           left->command_count == right->command_count;
}

static bool text_tests(void)
{
    static const uint8_t text_bytes[] = "Vexel";
    static const uint8_t family_name[] = "sans-serif";
    platform_text_slice family = {family_name, sizeof(family_name) - 1U};
    platform_text_request request = {
        {text_bytes, sizeof(text_bytes) - 1U}, &family, 1U,
        28.0, 400.0, PLATFORM_TEXT_STYLE_NORMAL, 0.25
    };
    platform_text_result first = PLATFORM_TEXT_RESULT_INIT;
    platform_text_result second = PLATFORM_TEXT_RESULT_INIT;
    REQUIRE(platform_text_layout(&request, &first) == PLATFORM_MEDIA_OK);
    REQUIRE(first.glyph_count == 5U);
    REQUIRE(first.command_count > 0U);
    REQUIRE(first.command_count <= PLATFORM_TEXT_MAX_PATH_SEGMENTS);
    REQUIRE(first.advance_width > 30.0 && first.advance_width < 160.0);
    REQUIRE(first.ascent > 0.0 && first.descent >= 0.0);
    double advance_sum = 0.0;
    double min_x = INFINITY;
    double min_y = INFINITY;
    double max_x = -INFINITY;
    double max_y = -INFINITY;
    for (size_t i = 0U; i < first.glyph_count; ++i) {
        const platform_text_glyph *glyph = &first.glyphs[i];
        REQUIRE(glyph->command_start <= first.command_count);
        REQUIRE(glyph->command_count <= first.command_count - glyph->command_start);
        REQUIRE(isfinite(glyph->x) && isfinite(glyph->y) &&
                isfinite(glyph->advance));
        advance_sum += glyph->advance;
        for (size_t j = 0U; j < glyph->command_count; ++j) {
            const platform_text_path_command *command =
                &first.commands[glyph->command_start + j];
            size_t coordinate_count = command->verb == PLATFORM_TEXT_PATH_CUBIC ? 6U :
                                     command->verb == PLATFORM_TEXT_PATH_CLOSE ? 0U : 2U;
            REQUIRE(command->verb >= PLATFORM_TEXT_PATH_MOVE &&
                    command->verb <= PLATFORM_TEXT_PATH_CLOSE);
            for (size_t k = 0U; k < coordinate_count; ++k) {
                double value = command->values[k];
                REQUIRE(isfinite(value));
                if ((k % 2U) == 0U) {
                    if (value < min_x) min_x = value;
                    if (value > max_x) max_x = value;
                } else {
                    if (value < min_y) min_y = value;
                    if (value > max_y) max_y = value;
                }
            }
        }
    }
    REQUIRE(fabs(advance_sum - first.advance_width) < 2.0);
    REQUIRE(min_x < max_x && min_y < max_y);
    REQUIRE(min_x > -10.0 && max_x < first.advance_width + 12.0);
    REQUIRE(min_y > -first.descent - 2.0 && max_y < first.ascent + 2.0);

    REQUIRE(platform_text_layout(&request, &second) == PLATFORM_MEDIA_OK);
    REQUIRE(first.glyph_count == second.glyph_count);
    REQUIRE(first.command_count == second.command_count);
    REQUIRE(first.advance_width == second.advance_width);
    REQUIRE(first.ascent == second.ascent && first.descent == second.descent &&
            first.leading == second.leading);
    for (size_t i = 0U; i < first.glyph_count; ++i) {
        REQUIRE(glyphs_equal(&first.glyphs[i], &second.glyphs[i]));
    }
    for (size_t i = 0U; i < first.command_count; ++i) {
        REQUIRE(path_commands_equal(&first.commands[i], &second.commands[i]));
    }
    double anchor = 1.0;
    REQUIRE(platform_text_anchor_offset(first.advance_width,
        PLATFORM_TEXT_ANCHOR_START, &anchor) == PLATFORM_MEDIA_OK);
    REQUIRE(anchor == 0.0);
    REQUIRE(platform_text_anchor_offset(first.advance_width,
        PLATFORM_TEXT_ANCHOR_MIDDLE, &anchor) == PLATFORM_MEDIA_OK);
    REQUIRE(anchor == -first.advance_width * 0.5);
    REQUIRE(platform_text_anchor_offset(first.advance_width,
        PLATFORM_TEXT_ANCHOR_END, &anchor) == PLATFORM_MEDIA_OK);
    REQUIRE(anchor == -first.advance_width);
    platform_text_result_destroy(&first);
    platform_text_result_destroy(&second);

    platform_text_request empty = {{NULL, 0U}, NULL, 0U, 18.0, 400.0,
                                   PLATFORM_TEXT_STYLE_NORMAL, 0.0};
    platform_text_result empty_result = PLATFORM_TEXT_RESULT_INIT;
    REQUIRE(platform_text_layout(&empty, &empty_result) == PLATFORM_MEDIA_OK);
    REQUIRE(empty_result.glyph_count == 0U && empty_result.command_count == 0U);
    REQUIRE(isfinite(empty_result.ascent) && empty_result.ascent > 0.0 &&
            isfinite(empty_result.descent) && empty_result.descent >= 0.0);
    platform_text_result_destroy(&empty_result);

    static const uint8_t combining_text[] = {'e', 0xccU, 0x81U};
    request.text = (platform_text_slice){combining_text, sizeof(combining_text)};
    request.font_families = NULL;
    request.font_family_count = 0U;
    platform_text_result combining = PLATFORM_TEXT_RESULT_INIT;
    REQUIRE(platform_text_layout(&request, &combining) == PLATFORM_MEDIA_OK);
    REQUIRE(combining.glyph_count > 0U && combining.advance_width > 0.0);
    platform_text_result_destroy(&combining);

    static const uint8_t rtl_text[] = {
        0xd8U, 0xb3U, 0xd9U, 0x84U, 0xd8U, 0xa7U, 0xd9U, 0x85U
    };
    request.text = (platform_text_slice){rtl_text, sizeof(rtl_text)};
    platform_text_result rtl = PLATFORM_TEXT_RESULT_INIT;
    REQUIRE(platform_text_layout(&request, &rtl) == PLATFORM_MEDIA_OK);
    REQUIRE(rtl.glyph_count > 0U && rtl.advance_width > 0.0);
    platform_text_result_destroy(&rtl);

    static const uint8_t emoji_text[] = {0xf0U, 0x9fU, 0x98U, 0x80U};
    request.text = (platform_text_slice){emoji_text, sizeof(emoji_text)};
    platform_text_result emoji = PLATFORM_TEXT_RESULT_INIT;
    REQUIRE(platform_text_layout(&request, &emoji) == PLATFORM_MEDIA_UNSUPPORTED);
    REQUIRE(emoji.glyph_count == 0U && emoji.commands == NULL);

    static const uint8_t invalid_utf8[] = {0xf0U, 0x28U, 0x8cU, 0x28U};
    request.text = (platform_text_slice){invalid_utf8, sizeof(invalid_utf8)};
    platform_text_result invalid = PLATFORM_TEXT_RESULT_INIT;
    REQUIRE(platform_text_layout(&request, &invalid) == PLATFORM_MEDIA_INVALID_UTF8);

    uint8_t *too_long = malloc((size_t)PLATFORM_TEXT_MAX_BYTES + 1U);
    REQUIRE(too_long != NULL);
    memset(too_long, (int)'a', (size_t)PLATFORM_TEXT_MAX_BYTES + 1U);
    request.text = (platform_text_slice){too_long,
                                         (size_t)PLATFORM_TEXT_MAX_BYTES + 1U};
    platform_text_result limited = PLATFORM_TEXT_RESULT_INIT;
    REQUIRE(platform_text_layout(&request, &limited) == PLATFORM_MEDIA_LIMIT);
    free(too_long);

    size_t glyph_boundary_length = (size_t)PLATFORM_TEXT_MAX_GLYPHS + 1U;
    uint8_t *boundary_text = malloc(glyph_boundary_length);
    REQUIRE(boundary_text != NULL);
    memset(boundary_text, (int)' ', glyph_boundary_length);
    request.text = (platform_text_slice){boundary_text,
                                         (size_t)PLATFORM_TEXT_MAX_GLYPHS};
    platform_text_result max_glyphs = PLATFORM_TEXT_RESULT_INIT;
    REQUIRE(platform_text_layout(&request, &max_glyphs) == PLATFORM_MEDIA_OK);
    REQUIRE(max_glyphs.glyph_count == PLATFORM_TEXT_MAX_GLYPHS);
    REQUIRE(max_glyphs.command_count == 0U);
    platform_text_result_destroy(&max_glyphs);
    request.text.length = glyph_boundary_length;
    REQUIRE(platform_text_layout(&request, &limited) == PLATFORM_MEDIA_LIMIT);

    memset(boundary_text, (int)'a', (size_t)PLATFORM_TEXT_MAX_GLYPHS);
    request.text.length = (size_t)PLATFORM_TEXT_MAX_GLYPHS;
    REQUIRE(platform_text_layout(&request, &limited) == PLATFORM_MEDIA_LIMIT);
    free(boundary_text);

    static const uint8_t missing_family_name[] = "vexel-no-such-font-family";
    static const uint8_t fallback_name[] = "sans-serif";
    platform_text_slice missing_families[] = {
        {missing_family_name, sizeof(missing_family_name) - 1U},
        {fallback_name, sizeof(fallback_name) - 1U}
    };
    request.text = (platform_text_slice){text_bytes, sizeof(text_bytes) - 1U};
    request.font_families = missing_families;
    request.font_family_count = 2U;
    platform_text_result fallback = PLATFORM_TEXT_RESULT_INIT;
    REQUIRE(platform_text_layout(&request, &fallback) == PLATFORM_MEDIA_OK);
    REQUIRE(fallback.glyph_count > 0U);
    platform_text_result_destroy(&fallback);

    REQUIRE(strcmp(platform_media_status_name(PLATFORM_MEDIA_ANIMATED),
                   "animated") == 0);
    return true;
}

static const char BASE64_ALPHABET[] =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

static bool encode_data_uri(const uint8_t *bytes, size_t length,
                            uint8_t **out_uri, size_t *out_length)
{
    size_t groups = (length + 2U) / 3U;
    if (groups > (SIZE_MAX - PLATFORM_IMAGE_DATA_URI_PREFIX_LENGTH) / 4U) {
        return false;
    }
    size_t encoded_length = groups * 4U;
    size_t uri_length = PLATFORM_IMAGE_DATA_URI_PREFIX_LENGTH + encoded_length;
    uint8_t *uri = malloc(uri_length);
    if (uri == NULL) {
        return false;
    }
    memcpy(uri, PLATFORM_IMAGE_DATA_URI_PREFIX,
           PLATFORM_IMAGE_DATA_URI_PREFIX_LENGTH);
    size_t input = 0U;
    size_t output = PLATFORM_IMAGE_DATA_URI_PREFIX_LENGTH;
    while (input < length) {
        size_t remaining = length - input;
        uint32_t value = (uint32_t)bytes[input] << 16U;
        if (remaining > 1U) value |= (uint32_t)bytes[input + 1U] << 8U;
        if (remaining > 2U) value |= bytes[input + 2U];
        uri[output++] = (uint8_t)BASE64_ALPHABET[(value >> 18U) & 63U];
        uri[output++] = (uint8_t)BASE64_ALPHABET[(value >> 12U) & 63U];
        uri[output++] = remaining > 1U ?
            (uint8_t)BASE64_ALPHABET[(value >> 6U) & 63U] : (uint8_t)'=';
        uri[output++] = remaining > 2U ?
            (uint8_t)BASE64_ALPHABET[value & 63U] : (uint8_t)'=';
        input += remaining >= 3U ? 3U : remaining;
    }
    *out_uri = uri;
    *out_length = uri_length;
    return true;
}

static uint64_t random_next(uint64_t *state)
{
    uint64_t value = *state;
    value ^= value << 13U;
    value ^= value >> 7U;
    value ^= value << 17U;
    *state = value;
    return value;
}

static bool base64_tests(const uint8_t *png, size_t png_length)
{
    uint8_t *uri = NULL;
    size_t uri_length = 0U;
    REQUIRE(encode_data_uri(png, png_length, &uri, &uri_length));
    platform_image_bytes decoded = PLATFORM_IMAGE_BYTES_INIT;
    REQUIRE(platform_image_decode_data_uri(uri, uri_length, &decoded) ==
            PLATFORM_MEDIA_OK);
    REQUIRE(decoded.length == png_length);
    REQUIRE(memcmp(decoded.bytes, png, png_length) == 0);
    platform_image_bytes_destroy(&decoded);
    free(uri);

    static const uint8_t bad_prefix[] = "data:image/jpeg;base64,AAAA";
    REQUIRE(platform_image_decode_data_uri(bad_prefix, sizeof(bad_prefix) - 1U,
        &decoded) == PLATFORM_MEDIA_INVALID_ARGUMENT);
    static const uint8_t bad_alphabet[] = "data:image/png;base64,AA?=";
    REQUIRE(platform_image_decode_data_uri(bad_alphabet,
        sizeof(bad_alphabet) - 1U, &decoded) == PLATFORM_MEDIA_INVALID_ARGUMENT);
    static const uint8_t bad_pad_bits[] = "data:image/png;base64,AB==";
    REQUIRE(platform_image_decode_data_uri(bad_pad_bits,
        sizeof(bad_pad_bits) - 1U, &decoded) == PLATFORM_MEDIA_INVALID_ARGUMENT);
    static const uint8_t whitespace[] = "data:image/png;base64,AA A";
    REQUIRE(platform_image_decode_data_uri(whitespace, sizeof(whitespace) - 1U,
        &decoded) == PLATFORM_MEDIA_INVALID_ARGUMENT);
    static const uint8_t empty_payload[] = "data:image/png;base64,";
    REQUIRE(platform_image_decode_data_uri(empty_payload,
        sizeof(empty_payload) - 1U, &decoded) == PLATFORM_MEDIA_INVALID_ARGUMENT);

    uint8_t *oversized_uri = malloc((size_t)PLATFORM_IMAGE_MAX_DATA_URI_BYTES + 1U);
    REQUIRE(oversized_uri != NULL);
    memset(oversized_uri, (int)'A',
           (size_t)PLATFORM_IMAGE_MAX_DATA_URI_BYTES + 1U);
    REQUIRE(platform_image_decode_data_uri(oversized_uri,
        (size_t)PLATFORM_IMAGE_MAX_DATA_URI_BYTES + 1U, &decoded) ==
        PLATFORM_MEDIA_LIMIT);
    free(oversized_uri);

    uint64_t seed = UINT64_C(0x8f3c5a71d2946be1);
    uint8_t candidate[PLATFORM_IMAGE_DATA_URI_PREFIX_LENGTH + 128U];
    uint8_t payload[48];
    size_t valid_cases = 0U;
    for (size_t iteration = 0U; iteration < 200000U; ++iteration) {
        memcpy(candidate, PLATFORM_IMAGE_DATA_URI_PREFIX,
               PLATFORM_IMAGE_DATA_URI_PREFIX_LENGTH);
        size_t payload_length = (size_t)(random_next(&seed) % 48U) + 1U;
        for (size_t i = 0U; i < payload_length; ++i) {
            payload[i] = (uint8_t)random_next(&seed);
        }
        uint8_t *generated = NULL;
        size_t generated_length = 0U;
        REQUIRE(encode_data_uri(payload, payload_length, &generated,
                                &generated_length));
        if ((iteration & 3U) == 0U) {
            memcpy(candidate, generated, generated_length);
            platform_media_status status = platform_image_decode_data_uri(
                candidate, generated_length, &decoded);
            REQUIRE(status == PLATFORM_MEDIA_OK);
            REQUIRE(decoded.length == payload_length);
            REQUIRE(memcmp(decoded.bytes, payload, payload_length) == 0);
            platform_image_bytes_destroy(&decoded);
            valid_cases++;
        } else if ((iteration & 3U) == 1U) {
            memcpy(candidate, generated, generated_length);
            candidate[generated_length - 2U] = (uint8_t)'?';
            platform_media_status status = platform_image_decode_data_uri(
                candidate, generated_length, &decoded);
            REQUIRE(status != PLATFORM_MEDIA_OK);
        } else if ((iteration & 3U) == 2U) {
            memcpy(candidate, generated, generated_length);
            candidate[generated_length - 1U] = (uint8_t)'=';
            platform_media_status status = platform_image_decode_data_uri(
                candidate, generated_length, &decoded);
            if (status == PLATFORM_MEDIA_OK) {
                platform_image_bytes_destroy(&decoded);
            }
        } else {
            size_t random_length = PLATFORM_IMAGE_DATA_URI_PREFIX_LENGTH +
                                   (size_t)(random_next(&seed) % 96U);
            for (size_t i = PLATFORM_IMAGE_DATA_URI_PREFIX_LENGTH;
                 i < random_length; ++i) {
                candidate[i] = (uint8_t)random_next(&seed);
            }
            platform_media_status status = platform_image_decode_data_uri(
                candidate, random_length, &decoded);
            if (status == PLATFORM_MEDIA_OK) {
                platform_image_bytes_destroy(&decoded);
            }
        }
        free(generated);
    }
    REQUIRE(valid_cases == 50000U);
    printf("platform-media base64 fuzz: 200000 seeded cases, seed 0x8f3c5a71d2946be1\n");
    return true;
}

static bool image_tests(const char *fixture_directory)
{
    uint8_t *png = NULL;
    size_t png_length = 0U;
    REQUIRE(read_fixture(fixture_directory, "sample.png", &png, &png_length));
    platform_image decoded = PLATFORM_IMAGE_INIT;
    REQUIRE(platform_image_decode(png, png_length, &decoded) == PLATFORM_MEDIA_OK);
    REQUIRE(decoded.width == 3U && decoded.height == 2U);
    REQUIRE(decoded.row_stride == 12U && decoded.byte_count == 24U);
    REQUIRE(decoded.rgba[0] == 255U && decoded.rgba[1] == 0U &&
            decoded.rgba[2] == 0U && decoded.rgba[3] == 255U);
    REQUIRE(decoded.rgba[4] == 0U && decoded.rgba[5] >= 127U &&
            decoded.rgba[5] <= 129U && decoded.rgba[7] == 128U);
    REQUIRE(decoded.rgba[12] == 10U && decoded.rgba[13] == 20U &&
            decoded.rgba[14] == 30U && decoded.rgba[15] == 255U);
    platform_image_destroy(&decoded);

    uint8_t *jpeg = NULL;
    size_t jpeg_length = 0U;
    REQUIRE(read_fixture(fixture_directory, "sample.jpg", &jpeg, &jpeg_length));
    REQUIRE(platform_image_decode(jpeg, jpeg_length, &decoded) == PLATFORM_MEDIA_OK);
    REQUIRE(decoded.width == 3U && decoded.height == 2U);
    platform_image_destroy(&decoded);

    uint8_t *gif = NULL;
    size_t gif_length = 0U;
    REQUIRE(read_fixture(fixture_directory, "static.gif", &gif, &gif_length));
    REQUIRE(platform_image_decode(gif, gif_length, &decoded) == PLATFORM_MEDIA_OK);
    REQUIRE(decoded.width == 1U && decoded.height == 1U);
    platform_image_destroy(&decoded);

    uint8_t *animated_gif = NULL;
    size_t animated_length = 0U;
    REQUIRE(read_fixture(fixture_directory, "animated.gif", &animated_gif,
                         &animated_length));
    REQUIRE(platform_image_decode(animated_gif, animated_length, &decoded) ==
            PLATFORM_MEDIA_ANIMATED);

    REQUIRE(platform_image_decode(NULL, 0U, &decoded) == PLATFORM_MEDIA_CORRUPT);
    static const uint8_t corrupt[] = {
        0xa5U, 0xa5U, 0x00U, 0xffU, 0x7fU, 0x13U, 0x29U, 0x40U
    };
    REQUIRE(platform_image_decode(corrupt, sizeof(corrupt), &decoded) ==
            PLATFORM_MEDIA_CORRUPT);
    static const uint8_t truncated[] = {0x89U, 'P', 'N', 'G', 0x0dU, 0x0aU};
    REQUIRE(platform_image_decode(truncated, sizeof(truncated), &decoded) !=
            PLATFORM_MEDIA_OK);
    uint8_t *oversized_dimension = NULL;
    size_t oversized_dimension_length = 0U;
    REQUIRE(read_fixture(fixture_directory, "oversized-dimension.png",
                         &oversized_dimension, &oversized_dimension_length));
    REQUIRE(platform_image_decode(oversized_dimension, oversized_dimension_length,
                                  &decoded) == PLATFORM_MEDIA_LIMIT);
    uint8_t *oversized_pixels = NULL;
    size_t oversized_pixels_length = 0U;
    REQUIRE(read_fixture(fixture_directory, "oversized-pixels.png",
                         &oversized_pixels, &oversized_pixels_length));
    REQUIRE(platform_image_decode(oversized_pixels, oversized_pixels_length,
                                  &decoded) == PLATFORM_MEDIA_LIMIT);
    uint8_t *too_large = malloc((size_t)PLATFORM_IMAGE_MAX_INPUT_BYTES + 1U);
    REQUIRE(too_large != NULL);
    memset(too_large, 0, (size_t)PLATFORM_IMAGE_MAX_INPUT_BYTES + 1U);
    REQUIRE(platform_image_decode(too_large,
        (size_t)PLATFORM_IMAGE_MAX_INPUT_BYTES + 1U, &decoded) ==
        PLATFORM_MEDIA_LIMIT);
    free(too_large);

    uint8_t sample_pixels[] = {
        0U, 0U, 0U, 255U,
        100U, 20U, 10U, 255U
    };
    platform_image_view view = {sample_pixels, sizeof(sample_pixels), 8U, 2U, 1U};
    REQUIRE(platform_image_scale_bilinear(view, 3U, 1U, &decoded) ==
            PLATFORM_MEDIA_OK);
    REQUIRE(decoded.width == 3U && decoded.height == 1U);
    REQUIRE(decoded.rgba[0] == 0U && decoded.rgba[4] == 50U &&
            decoded.rgba[5] == 10U && decoded.rgba[8] == 100U);
    platform_image_destroy(&decoded);
    REQUIRE(platform_image_scale_bilinear(view, 1U, 1U, &decoded) ==
            PLATFORM_MEDIA_OK);
    REQUIRE(decoded.rgba[0] == 50U && decoded.rgba[1] == 10U);
    platform_image_destroy(&decoded);
    platform_image_view invalid_view = {sample_pixels, sizeof(sample_pixels),
                                        7U, 2U, 1U};
    REQUIRE(platform_image_scale_bilinear(invalid_view, 1U, 1U, &decoded) ==
            PLATFORM_MEDIA_INVALID_ARGUMENT);
    REQUIRE(platform_image_scale_bilinear(view,
        PLATFORM_IMAGE_MAX_DIMENSION + 1U, 1U, &decoded) == PLATFORM_MEDIA_LIMIT);

    REQUIRE(base64_tests(png, png_length));
    free(png);
    free(jpeg);
    free(gif);
    free(animated_gif);
    free(oversized_dimension);
    free(oversized_pixels);
    return true;
}

int main(int argc, char **argv)
{
    if (argc != 2) {
        fprintf(stderr, "usage: platform_media_test FIXTURE_DIRECTORY\n");
        return 2;
    }
    if (!text_tests() || !image_tests(argv[1])) {
        return 1;
    }
    puts("platform-media tests: pass");
    return 0;
}
