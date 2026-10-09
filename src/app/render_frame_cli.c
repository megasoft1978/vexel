#include "render_frame_cli.h"

#include "frame_compose.h"
#include "viewer_load.h"

#include <errno.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define VIEWER_CLI_PATH_MAX 4096U
#define VIEWER_CLI_NUMBER_MAX 63U
#define VIEWER_CLI_PAN_MAX 10000000.0

typedef struct render_options {
    const char *input_path;
    const char *output_path;
    uint32_t width;
    uint32_t height;
    double zoom;
    double pan_x;
    double pan_y;
    bool has_zoom;
    bool has_pan;
} render_options;

static size_t bounded_length(const char *text, size_t limit)
{
    size_t length = 0U;
    if (text == NULL) return 0U;
    while (length <= limit && text[length] != '\0') length++;
    return length;
}

static bool parse_positive_integer(const char *text, uint32_t maximum, uint32_t *out)
{
    if (text == NULL || text[0] == '\0' || out == NULL || text[0] == '-') return false;
    errno = 0;
    char *end = NULL;
    unsigned long value = strtoul(text, &end, 10);
    if (errno != 0 || end == text || *end != '\0' || value == 0UL || value > maximum) {
        return false;
    }
    *out = (uint32_t)value;
    return true;
}

static bool parse_number_slice(const char *text, size_t length, double *out)
{
    if (text == NULL || out == NULL || length == 0U || length > VIEWER_CLI_NUMBER_MAX) {
        return false;
    }
    char buffer[VIEWER_CLI_NUMBER_MAX + 1U];
    memcpy(buffer, text, length);
    buffer[length] = '\0';
    errno = 0;
    char *end = NULL;
    double value = strtod(buffer, &end);
    if (errno != 0 || end == buffer || *end != '\0' || !isfinite(value)) {
        return false;
    }
    *out = value;
    return true;
}

static bool parse_number(const char *text, double *out)
{
    size_t length = bounded_length(text, VIEWER_CLI_NUMBER_MAX);
    return length > 0U && length <= VIEWER_CLI_NUMBER_MAX &&
        parse_number_slice(text, length, out);
}

static bool parse_size(const char *text, uint32_t *width, uint32_t *height)
{
    if (text == NULL) return false;
    size_t length = bounded_length(text, 32U);
    if (length == 0U || length > 32U) return false;
    const char *separator = memchr(text, 'x', length);
    if (separator == NULL || memchr(separator + 1, 'x',
                                    length - (size_t)(separator + 1 - text)) != NULL) {
        return false;
    }
    size_t left_length = (size_t)(separator - text);
    size_t right_length = length - left_length - 1U;
    if (left_length == 0U || right_length == 0U || left_length > 8U || right_length > 8U) {
        return false;
    }
    char left[9] = {0};
    char right[9] = {0};
    memcpy(left, text, left_length);
    memcpy(right, separator + 1, right_length);
    return parse_positive_integer(left, 8192U, width) &&
        parse_positive_integer(right, 8192U, height);
}

static bool parse_pan(const char *text, double *pan_x, double *pan_y)
{
    if (text == NULL) return false;
    size_t length = bounded_length(text, VIEWER_CLI_NUMBER_MAX * 2U + 1U);
    if (length == 0U || length > VIEWER_CLI_NUMBER_MAX * 2U + 1U) return false;
    const char *separator = memchr(text, ',', length);
    if (separator == NULL || memchr(separator + 1, ',',
                                    length - (size_t)(separator + 1 - text)) != NULL) {
        return false;
    }
    size_t x_length = (size_t)(separator - text);
    size_t y_length = length - x_length - 1U;
    return parse_number_slice(text, x_length, pan_x) &&
        parse_number_slice(separator + 1, y_length, pan_y) &&
        fabs(*pan_x) <= VIEWER_CLI_PAN_MAX && fabs(*pan_y) <= VIEWER_CLI_PAN_MAX;
}

static bool parse_options(int argc, char *argv[], render_options *options)
{
    if (argc < 6 || argv == NULL || options == NULL ||
        strcmp(argv[1], "--render-frame") != 0) return false;
    size_t path_length = bounded_length(argv[2], VIEWER_CLI_PATH_MAX);
    if (path_length == 0U || path_length > VIEWER_CLI_PATH_MAX) return false;
    *options = (render_options){.input_path = argv[2], .zoom = 1.0};
    bool has_size = false, has_output = false;
    for (int index = 3; index < argc; index++) {
        const char *option = argv[index];
        if (index + 1 >= argc) return false;
        const char *value = argv[++index];
        if (strcmp(option, "--size") == 0 && !has_size) {
            has_size = parse_size(value, &options->width, &options->height);
            if (!has_size) return false;
        } else if (strcmp(option, "--out") == 0 && !has_output) {
            size_t output_length = bounded_length(value, VIEWER_CLI_PATH_MAX);
            if (output_length == 0U || output_length > VIEWER_CLI_PATH_MAX) return false;
            options->output_path = value;
            has_output = true;
        } else if (strcmp(option, "--zoom") == 0 && !options->has_zoom) {
            if (!parse_number(value, &options->zoom) ||
                options->zoom < UI_VIEWER_MIN_SCALE || options->zoom > UI_VIEWER_MAX_SCALE) {
                return false;
            }
            options->has_zoom = true;
        } else if (strcmp(option, "--pan") == 0 && !options->has_pan) {
            if (!parse_pan(value, &options->pan_x, &options->pan_y)) return false;
            options->has_pan = true;
        } else {
            return false;
        }
    }
    return has_size && has_output;
}

static bool write_ppm(const char *path, const uint8_t *rgba, uint32_t width, uint32_t height)
{
    FILE *file = fopen(path, "wb");
    if (file == NULL) return false;
    char header[64];
    int header_length = snprintf(header, sizeof(header), "P6\n%u %u\n255\n", width, height);
    bool okay = header_length > 0 && (size_t)header_length < sizeof(header) &&
        fwrite(header, 1U, (size_t)header_length, file) == (size_t)header_length;
    uint8_t row[8192U * 3U];
    for (uint32_t y = 0U; okay && y < height; y++) {
        for (uint32_t x = 0U; x < width; x++) {
            size_t source_offset = ((size_t)y * width + x) * 4U;
            size_t output_offset = (size_t)x * 3U;
            row[output_offset] = rgba[source_offset];
            row[output_offset + 1U] = rgba[source_offset + 1U];
            row[output_offset + 2U] = rgba[source_offset + 2U];
        }
        okay = fwrite(row, 1U, (size_t)width * 3U, file) == (size_t)width * 3U;
    }
    if (fclose(file) != 0) okay = false;
    return okay;
}

static void report_load_failure(const char *path, const viewer_loaded_document *document)
{
    if (document->shape_error != SVG_SHAPE_ERROR_NONE) {
        const char *name = svg_shape_error_name(document->shape_error);
        (void)fprintf(stderr, "render-frame: %s: %s at byte %zu\n", path,
                      name == NULL ? "SVG error" : name, document->error_offset);
    } else {
        (void)fprintf(stderr, "render-frame: %s: %s\n", path,
                      document->load_error[0] == '\0' ? "document load failed" :
                      document->load_error);
    }
}

int app_run_render_frame(int argc, char *argv[])
{
    render_options options = {0};
    if (!parse_options(argc, argv, &options)) {
        (void)fputs("render-frame: expected <file.svg> --size WxH --out <file.ppm> "
                    "[--zoom N] [--pan X,Y] (size 1..8192, zoom 0.05..64, "
                    "pan within +/-10000000 pixels)\n", stderr);
        return 2;
    }
    viewer_loaded_document document = {0};
    viewer_load_status load_status = viewer_load_document(options.input_path, NULL, &document);
    bool loaded = load_status == VIEWER_LOAD_OK;
    ui_viewer_navigation navigation = {0};
    if (loaded && (!ui_viewer_navigation_init(&navigation, (double)options.width,
            (double)options.height, (double)document.tile.width,
            (double)document.tile.height) || !ui_viewer_navigation_fit(&navigation))) {
        document.shape_error = SVG_SHAPE_ERROR_VIEWPORT;
        loaded = false;
    }
    if (loaded && options.has_zoom) {
        double factor = options.zoom / navigation.scale;
        if (!ui_viewer_navigation_zoom(&navigation, factor,
                (double)options.width * 0.5, (double)options.height * 0.5)) {
            document.shape_error = SVG_SHAPE_ERROR_VIEWPORT;
            loaded = false;
        }
    }
    if (loaded && options.has_pan) {
        navigation.pan_x += options.pan_x;
        navigation.pan_y += options.pan_y;
        viewer_navigation_clamp(&navigation);
    }

    size_t width = (size_t)options.width;
    size_t height = (size_t)options.height;
    if (width > SIZE_MAX / height) {
        viewer_loaded_document_destroy(&document);
        (void)fputs("render-frame: requested frame size overflows addressable memory\n", stderr);
        return 1;
    }
    size_t pixels = width * height;
    if (pixels > SIZE_MAX / 4U) {
        viewer_loaded_document_destroy(&document);
        (void)fputs("render-frame: requested frame size overflows addressable memory\n", stderr);
        return 1;
    }
    size_t frame_size = pixels * 4U;
    uint8_t *frame = malloc(frame_size);
    if (frame == NULL) {
        viewer_loaded_document_destroy(&document);
        (void)fputs("render-frame: frame allocation failed\n", stderr);
        return 1;
    }
    const char *error_message = document.shape_error != SVG_SHAPE_ERROR_NONE ?
        svg_shape_error_name(document.shape_error) : document.load_error;
    viewer_frame_kind kind = loaded ? VIEWER_FRAME_DOCUMENT : VIEWER_FRAME_ERROR;
    bool composed = viewer_compose_frame(loaded ? &document.tile : NULL,
        loaded ? &navigation : NULL, NULL, kind,
        error_message == NULL || error_message[0] == '\0' ? "Load error" : error_message,
        frame, frame_size, options.width, options.height);
    bool written = composed && write_ppm(options.output_path, frame,
                                         options.width, options.height);
    free(frame);
    if (!written) {
        viewer_loaded_document_destroy(&document);
        (void)fprintf(stderr, "render-frame: could not compose or write '%s'\n", options.output_path);
        return 1;
    }
    if (!loaded) report_load_failure(options.input_path, &document);
    viewer_loaded_document_destroy(&document);
    return loaded ? 0 : 1;
}
