#include "render_frame_cli.h"

#include "frame_compose.h"
#include "raster/shape_render.h"
#include "viewer_load.h"

#include <errno.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>

#define VIEWER_CLI_PATH_MAX 4096U
#define VIEWER_CLI_NUMBER_MAX 63U
#define VIEWER_CLI_PAN_MAX 10000000.0
#define ANIMATION_CLI_TOTAL_FRAME_PIXELS 32000000ULL

static void report_load_failure(const char *path,
                                const viewer_loaded_document *document);

typedef struct render_options {
    const char *input_path;
    const char *output_path;
    uint32_t width;
    uint32_t height;
    double zoom;
    double pan_x;
    double pan_y;
    double time_seconds;
    uint32_t region_x;
    uint32_t region_y;
    uint32_t region_width;
    uint32_t region_height;
    bool has_zoom;
    bool has_pan;
    bool has_region;
    bool has_time;
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

static bool parse_region_component(const char *text, size_t length, uint32_t maximum,
                                   bool allow_zero, uint32_t *out)
{
    if (text == NULL || out == NULL || length == 0U || length > 8U) return false;
    char buffer[9] = {0};
    memcpy(buffer, text, length);
    if (allow_zero && strcmp(buffer, "0") == 0) {
        *out = 0U;
        return true;
    }
    return parse_positive_integer(buffer, maximum, out);
}

static bool parse_region(const char *text, uint32_t *x, uint32_t *y,
                         uint32_t *width, uint32_t *height)
{
    size_t length = bounded_length(text, 35U);
    if (length == 0U || length > 35U) return false;
    size_t starts[4] = {0U};
    size_t sizes[4] = {0U};
    size_t component = 0U;
    starts[0] = 0U;
    for (size_t index = 0U; index < length; index++) {
        if (text[index] != ',') continue;
        if (component >= 3U) return false;
        sizes[component] = index - starts[component];
        component++;
        starts[component] = index + 1U;
    }
    if (component != 3U) return false;
    sizes[3] = length - starts[3];
    uint32_t values[4] = {0U};
    for (size_t index = 0U; index < 4U; index++) {
        bool allow_zero = index < 2U;
        if (!parse_region_component(text + starts[index], sizes[index], 4096U,
                                    allow_zero, &values[index])) return false;
    }
    if (values[2] == 0U || values[3] == 0U) return false;
    *x = values[0]; *y = values[1]; *width = values[2]; *height = values[3];
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
        } else if (strcmp(option, "--region") == 0 && !options->has_region) {
            if (!parse_region(value, &options->region_x, &options->region_y,
                              &options->region_width, &options->region_height)) return false;
            options->has_region = true;
        } else if (strcmp(option, "--time") == 0 && !options->has_time) {
            if (!parse_number(value, &options->time_seconds) ||
                options->time_seconds < 0.0 || options->time_seconds > 3600.0)
                return false;
            options->has_time = true;
        } else {
            return false;
        }
    }
    return has_size && has_output;
}

static bool render_region_tile(const viewer_loaded_document *document,
                               const render_options *options, raster_tile *out_tile,
                               svg_shape_diagnostic *out_diagnostic)
{
    if (document == NULL || options == NULL || out_tile == NULL || out_diagnostic == NULL ||
        options->region_width == 0U || options->region_height == 0U ||
        (uint64_t)options->region_x + options->region_width > document->tile.width ||
        (uint64_t)options->region_y + options->region_height > document->tile.height) return false;
    double target_width_value = ceil((double)options->region_width * options->zoom);
    double target_height_value = ceil((double)options->region_height * options->zoom);
    if (!isfinite(target_width_value) || !isfinite(target_height_value) ||
        target_width_value < 1.0 || target_height_value < 1.0 ||
        target_width_value > RASTER_TILE_MAX_AXIS || target_height_value > RASTER_TILE_MAX_AXIS)
        return false;
    uint32_t target_width = (uint32_t)target_width_value;
    uint32_t target_height = (uint32_t)target_height_value;
    const svg_shape_document *source = &document->shape;
    double base_scale = fmin((double)document->tile.width / source->view_width,
                             (double)document->tile.height / source->view_height);
    double offset_x = ((double)document->tile.width - source->view_width * base_scale) * 0.5;
    double offset_y = ((double)document->tile.height - source->view_height * base_scale) * 0.5;
    double render_scale = base_scale * options->zoom;
    if (!isfinite(base_scale) || !(base_scale > 0.0) || !isfinite(render_scale) ||
        !(render_scale > 0.0)) return false;
    svg_shape_document view = *source;
    view.width = target_width;
    view.height = target_height;
    view.view_x = source->view_x + ((double)options->region_x - offset_x) / base_scale;
    view.view_y = source->view_y + ((double)options->region_y - offset_y) / base_scale;
    view.view_width = (double)target_width / render_scale;
    view.view_height = (double)target_height / render_scale;
    if (!isfinite(view.view_x) || !isfinite(view.view_y) ||
        !isfinite(view.view_width) || !isfinite(view.view_height) ||
        !(view.view_width > 0.0) || !(view.view_height > 0.0)) return false;
    return options->has_time ?
        raster_shape_render_at(&view, options->time_seconds, out_tile, out_diagnostic) :
        raster_shape_render(&view, out_tile, out_diagnostic);
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

static bool write_tile_ppm(const char *path, const raster_tile *tile)
{
    if (path == NULL || tile == NULL || tile->rgba == NULL || tile->width == 0U ||
        tile->height == 0U || tile->width > 8192U || tile->height > 8192U) return false;
    FILE *file = fopen(path, "wb");
    if (file == NULL) return false;
    char header[64];
    int header_length = snprintf(header, sizeof(header), "P6\n%u %u\n255\n",
                                 tile->width, tile->height);
    bool okay = header_length > 0 && (size_t)header_length < sizeof(header) &&
        fwrite(header, 1U, (size_t)header_length, file) == (size_t)header_length;
    uint8_t row[8192U * 3U];
    size_t pixel_count = (size_t)tile->width * tile->height;
    if (pixel_count > tile->byte_count / 4U) okay = false;
    for (uint32_t y = 0U; okay && y < tile->height; y++) {
        for (uint32_t x = 0U; x < tile->width; x++) {
            size_t source_offset = ((size_t)y * tile->width + x) * 4U;
            size_t output_offset = (size_t)x * 3U;
            uint32_t alpha = tile->rgba[source_offset + 3U];
            for (size_t channel = 0U; channel < 3U; channel++) {
                uint32_t value = tile->rgba[source_offset + channel] + 255U - alpha;
                row[output_offset + channel] = (uint8_t)(value > 255U ? 255U : value);
            }
        }
        okay = fwrite(row, 1U, (size_t)tile->width * 3U, file) ==
            (size_t)tile->width * 3U;
    }
    if (fclose(file) != 0) okay = false;
    return okay;
}

static bool parse_animation_cli(int argc, char *argv[], const char **input,
                                char output_directory[VIEWER_CLI_PATH_MAX + 1U],
                                uint32_t *fps, double *duration, size_t *frame_count)
{
    if (argc < 9 || argv == NULL || strcmp(argv[1], "--render-animation") != 0)
        return false;
    size_t input_length = bounded_length(argv[2], VIEWER_CLI_PATH_MAX);
    if (input_length == 0U || input_length > VIEWER_CLI_PATH_MAX) return false;
    *input = argv[2];
    bool has_fps = false, has_duration = false, has_output = false;
    uint32_t parsed_fps = 0U;
    double parsed_duration = 0.0;
    for (int index = 3; index < argc; index++) {
        if (index + 1 >= argc) return false;
        const char *option = argv[index++];
        const char *value = argv[index];
        if (strcmp(option, "--fps") == 0 && !has_fps) {
            if (!parse_positive_integer(value, 60U, &parsed_fps)) return false;
            has_fps = true;
        } else if (strcmp(option, "--duration") == 0 && !has_duration) {
            if (!parse_number(value, &parsed_duration) || parsed_duration <= 0.0 ||
                parsed_duration > SVG_ANIM_MAX_TIMELINE_SECONDS) return false;
            has_duration = true;
        } else if (strcmp(option, "--out") == 0 && !has_output) {
            size_t output_length = bounded_length(value, VIEWER_CLI_PATH_MAX);
            if (output_length == 0U || output_length > VIEWER_CLI_PATH_MAX) return false;
            memcpy(output_directory, value, output_length);
            output_directory[output_length] = '\0';
            has_output = true;
        } else return false;
    }
    if (!has_fps || !has_duration || !has_output) return false;
    double requested = ceil((double)parsed_fps * parsed_duration);
    if (!isfinite(requested) || requested < 1.0 || requested > 300.0) return false;
    *fps = parsed_fps;
    *duration = parsed_duration;
    *frame_count = (size_t)requested;
    return true;
}

int app_run_render_animation(int argc, char *argv[])
{
    const char *input = NULL;
    char output_directory[VIEWER_CLI_PATH_MAX + 1U] = {0};
    uint32_t fps = 0U;
    double duration = 0.0;
    size_t frame_count = 0U;
    if (!parse_animation_cli(argc, argv, &input, output_directory, &fps,
                             &duration, &frame_count)) {
        (void)fputs("render-animation: expected <file.svg> --fps 1..60 --duration 0..3600 "
                    "--out <existing-directory> (at most 300 frames)\n", stderr);
        return 2;
    }
    struct stat directory_status;
    if (stat(output_directory, &directory_status) != 0 || !S_ISDIR(directory_status.st_mode)) {
        (void)fprintf(stderr, "render-animation: output directory is missing: %s\n",
                      output_directory);
        return 2;
    }
    viewer_loaded_document document = {0};
    viewer_load_status status = viewer_load_document(input, NULL, &document);
    if (status != VIEWER_LOAD_OK) {
        report_load_failure(input, &document);
        viewer_loaded_document_destroy(&document);
        return 1;
    }
    uint64_t frame_pixels = (uint64_t)document.shape.width * document.shape.height;
    if (frame_pixels == 0U || frame_pixels > ANIMATION_CLI_TOTAL_FRAME_PIXELS / frame_count) {
        viewer_loaded_document_destroy(&document);
        (void)fputs("render-animation: total frame pixel budget exceeded\n", stderr);
        return 2;
    }
    bool okay = true;
    for (size_t frame_index = 0U; okay && frame_index < frame_count; frame_index++) {
        double time_seconds = (double)frame_index / (double)fps;
        raster_tile tile = {0};
        svg_shape_diagnostic diagnostic = {SVG_SHAPE_ERROR_NONE, 0U};
        okay = raster_shape_render_at(&document.shape, time_seconds, &tile, &diagnostic);
        char path[VIEWER_CLI_PATH_MAX + 1U];
        size_t directory_length = strlen(output_directory);
        bool has_separator = directory_length > 0U &&
            output_directory[directory_length - 1U] == '/';
        int path_length = snprintf(path, sizeof(path), "%s%sframe-%04zu.ppm",
            output_directory, has_separator ? "" : "/", frame_index);
        okay = okay && path_length > 0 && (size_t)path_length < sizeof(path) &&
            write_tile_ppm(path, &tile);
        raster_tile_destroy(&tile);
        if (!okay && diagnostic.category != SVG_SHAPE_ERROR_NONE) {
            const char *name = svg_shape_error_name(diagnostic.category);
            (void)fprintf(stderr, "render-animation: frame %zu failed: %s at byte %zu\n",
                frame_index, name == NULL ? "render failed" : name, diagnostic.offset);
        }
    }
    viewer_loaded_document_destroy(&document);
    if (!okay) {
        (void)fputs("render-animation: frame rendering or writing failed\n", stderr);
        return 1;
    }
    (void)printf("render-animation: wrote %zu PPM frames at %u fps over %.6g seconds\n",
                 frame_count, fps, duration);
    return 0;
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
                    "[--zoom N] [--pan X,Y] [--region X,Y,W,H] "
                    "[--time seconds] "
                    "(size 1..8192, zoom 0.001..1000, source region in document pixels)\n",
                    stderr);
        return 2;
    }
    viewer_loaded_document document = {0};
    raster_tile region_tile = {0};
    const raster_tile *display_tile = NULL;
    viewer_load_status load_status = viewer_load_document(options.input_path, NULL, &document);
    bool loaded = load_status == VIEWER_LOAD_OK;
    if (loaded && options.has_time && !options.has_region) {
        raster_tile animated = {0};
        svg_shape_diagnostic diagnostic = {SVG_SHAPE_ERROR_NONE, 0U};
        if (!raster_shape_render_at(&document.shape, options.time_seconds,
                                    &animated, &diagnostic)) {
            document.shape_error = diagnostic.category;
            document.error_offset = diagnostic.offset;
            loaded = false;
        } else {
            raster_tile_destroy(&document.tile);
            document.tile = animated;
        }
    }
    if (loaded && options.has_region) {
        svg_shape_diagnostic diagnostic = {SVG_SHAPE_ERROR_NONE, 0U};
        if (!render_region_tile(&document, &options, &region_tile, &diagnostic)) {
            document.shape_error = diagnostic.category == SVG_SHAPE_ERROR_NONE ?
                SVG_SHAPE_ERROR_LIMIT : diagnostic.category;
            document.error_offset = diagnostic.offset;
            loaded = false;
        } else {
            display_tile = &region_tile;
        }
    }
    if (display_tile == NULL && loaded) display_tile = &document.tile;
    ui_viewer_navigation navigation = {0};
    if (loaded && (!ui_viewer_navigation_init(&navigation, (double)options.width,
            (double)options.height, (double)display_tile->width,
            (double)display_tile->height) ||
            !(options.has_region ? ui_viewer_navigation_actual_size(&navigation) :
                                   ui_viewer_navigation_fit(&navigation)))) {
        document.shape_error = SVG_SHAPE_ERROR_VIEWPORT;
        loaded = false;
    }
    if (loaded && options.has_zoom && !options.has_region) {
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
    bool composed = viewer_compose_frame(loaded ? display_tile : NULL,
        loaded ? &navigation : NULL, NULL, kind,
        error_message == NULL || error_message[0] == '\0' ? "Load error" : error_message,
        frame, frame_size, options.width, options.height);
    bool written = composed && write_ppm(options.output_path, frame,
                                         options.width, options.height);
    free(frame);
    raster_tile_destroy(&region_tile);
    if (!written) {
        viewer_loaded_document_destroy(&document);
        (void)fprintf(stderr, "render-frame: could not compose or write '%s'\n", options.output_path);
        return 1;
    }
    if (!loaded) report_load_failure(options.input_path, &document);
    viewer_loaded_document_destroy(&document);
    return loaded ? 0 : 1;
}
