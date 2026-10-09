#include "frame_compose.h"

#include <math.h>
#include <string.h>

#define VIEWER_FRAME_MAX_AXIS 8192U
#define VIEWER_FRAME_CHECKER_SIZE 16U
#define VIEWER_FRAME_FONT_SCALE 2U

static bool frame_size(uint32_t width, uint32_t height, size_t *out_bytes)
{
    if (out_bytes == NULL || width == 0U || height == 0U ||
        width > VIEWER_FRAME_MAX_AXIS || height > VIEWER_FRAME_MAX_AXIS) {
        return false;
    }
    size_t w = (size_t)width;
    size_t h = (size_t)height;
    if (w > SIZE_MAX / h) {
        return false;
    }
    size_t pixels = w * h;
    if (pixels > SIZE_MAX / 4U) {
        return false;
    }
    *out_bytes = pixels * 4U;
    return true;
}

static bool tile_valid(const raster_tile *tile)
{
    if (tile == NULL || tile->rgba == NULL || tile->width == 0U || tile->height == 0U ||
        tile->width > RASTER_TILE_MAX_AXIS || tile->height > RASTER_TILE_MAX_AXIS) {
        return false;
    }
    size_t width = (size_t)tile->width;
    size_t height = (size_t)tile->height;
    if (width > SIZE_MAX / height) {
        return false;
    }
    size_t pixels = width * height;
    return pixels <= SIZE_MAX / 4U && tile->byte_count == pixels * 4U;
}

static uint8_t clamp_byte(double value)
{
    if (!(value > 0.0)) {
        return 0U;
    }
    if (value >= 255.0) {
        return 255U;
    }
    return (uint8_t)(value + 0.5);
}

static double clamp_double(double value, double low, double high)
{
    return value < low ? low : (value > high ? high : value);
}

static void sample_tile(const raster_tile *tile, double source_x, double source_y,
                        bool nearest, double out_premultiplied[4])
{
    double max_x = (double)tile->width - 1.0;
    double max_y = (double)tile->height - 1.0;
    source_x = clamp_double(source_x, 0.0, max_x);
    source_y = clamp_double(source_y, 0.0, max_y);
    if (nearest) {
        uint32_t x = (uint32_t)floor(source_x + 0.5);
        uint32_t y = (uint32_t)floor(source_y + 0.5);
        size_t offset = ((size_t)y * tile->width + x) * 4U;
        double alpha = (double)tile->rgba[offset + 3U] / 255.0;
        out_premultiplied[0] = (double)tile->rgba[offset] * alpha;
        out_premultiplied[1] = (double)tile->rgba[offset + 1U] * alpha;
        out_premultiplied[2] = (double)tile->rgba[offset + 2U] * alpha;
        out_premultiplied[3] = (double)tile->rgba[offset + 3U];
        return;
    }

    uint32_t x0 = (uint32_t)floor(source_x);
    uint32_t y0 = (uint32_t)floor(source_y);
    uint32_t x1 = x0 + 1U < tile->width ? x0 + 1U : x0;
    uint32_t y1 = y0 + 1U < tile->height ? y0 + 1U : y0;
    double fx = source_x - (double)x0;
    double fy = source_y - (double)y0;
    double weights[4] = {(1.0 - fx) * (1.0 - fy), fx * (1.0 - fy),
                         (1.0 - fx) * fy, fx * fy};
    uint32_t xs[4] = {x0, x1, x0, x1};
    uint32_t ys[4] = {y0, y0, y1, y1};
    out_premultiplied[0] = 0.0;
    out_premultiplied[1] = 0.0;
    out_premultiplied[2] = 0.0;
    out_premultiplied[3] = 0.0;
    for (size_t sample = 0U; sample < 4U; sample++) {
        size_t offset = ((size_t)ys[sample] * tile->width + xs[sample]) * 4U;
        double alpha = (double)tile->rgba[offset + 3U] / 255.0;
        double weight = weights[sample];
        out_premultiplied[0] += (double)tile->rgba[offset] * alpha * weight;
        out_premultiplied[1] += (double)tile->rgba[offset + 1U] * alpha * weight;
        out_premultiplied[2] += (double)tile->rgba[offset + 2U] * alpha * weight;
        out_premultiplied[3] += (double)tile->rgba[offset + 3U] * weight;
    }
}

static void composite_pixel(uint8_t *pixel, uint8_t red, uint8_t green, uint8_t blue,
                            double sample[4])
{
    double alpha = sample[3] / 255.0;
    double inverse = 1.0 - alpha;
    pixel[0] = clamp_byte(sample[0] + (double)red * inverse);
    pixel[1] = clamp_byte(sample[1] + (double)green * inverse);
    pixel[2] = clamp_byte(sample[2] + (double)blue * inverse);
    pixel[3] = 255U;
}

static const uint8_t font_glyphs[36][7] = {
    {14, 17, 17, 31, 17, 17, 17}, {30, 17, 17, 30, 17, 17, 30},
    {14, 17, 16, 16, 16, 17, 14}, {30, 17, 17, 17, 17, 17, 30},
    {31, 16, 16, 30, 16, 16, 31}, {31, 16, 16, 30, 16, 16, 16},
    {14, 17, 16, 23, 17, 17, 15}, {17, 17, 17, 31, 17, 17, 17},
    {14, 4, 4, 4, 4, 4, 14}, {7, 2, 2, 2, 18, 18, 12},
    {17, 18, 20, 24, 20, 18, 17}, {16, 16, 16, 16, 16, 16, 31},
    {17, 27, 21, 21, 17, 17, 17}, {17, 25, 21, 19, 17, 17, 17},
    {14, 17, 17, 17, 17, 17, 14}, {30, 17, 17, 30, 16, 16, 16},
    {14, 17, 17, 17, 21, 18, 13}, {30, 17, 17, 30, 20, 18, 17},
    {15, 16, 16, 14, 1, 1, 30}, {31, 4, 4, 4, 4, 4, 4},
    {17, 17, 17, 17, 17, 17, 14}, {17, 17, 17, 17, 17, 10, 4},
    {17, 17, 17, 21, 21, 21, 10}, {17, 17, 10, 4, 10, 17, 17},
    {17, 17, 10, 4, 4, 4, 4}, {31, 1, 2, 4, 8, 16, 31},
    {14, 17, 19, 21, 25, 17, 14}, {4, 12, 4, 4, 4, 4, 14},
    {14, 17, 1, 2, 4, 8, 31}, {30, 1, 1, 14, 1, 1, 30},
    {2, 6, 10, 18, 31, 2, 2}, {31, 16, 16, 30, 1, 1, 30},
    {14, 16, 16, 30, 17, 17, 14}, {31, 1, 2, 4, 8, 8, 8},
    {14, 17, 17, 14, 17, 17, 14}, {14, 17, 17, 15, 1, 1, 14}
};

static const uint8_t *glyph_for(char value)
{
    if (value >= 'a' && value <= 'z') {
        value = (char)(value - 'a' + 'A');
    }
    if (value >= 'A' && value <= 'Z') {
        return font_glyphs[(size_t)(value - 'A')];
    }
    if (value >= '0' && value <= '9') {
        return font_glyphs[26U + (size_t)(value - '0')];
    }
    return NULL;
}

static void draw_text(uint8_t *rgba, uint32_t width, uint32_t height, int64_t x, int64_t y,
                      const char *text, uint8_t red, uint8_t green, uint8_t blue)
{
    if (text == NULL) {
        return;
    }
    size_t length = 0U;
    while (length < 96U && text[length] != '\0') {
        length++;
    }
    int64_t cursor_x = x;
    for (size_t index = 0U; index < length; index++) {
        const uint8_t *glyph = glyph_for(text[index]);
        if (text[index] == '-') {
            static const uint8_t hyphen[7] = {0, 0, 0, 31, 0, 0, 0};
            glyph = hyphen;
        } else if (text[index] == ':') {
            static const uint8_t colon[7] = {0, 4, 4, 0, 4, 4, 0};
            glyph = colon;
        } else if (text[index] == '.') {
            static const uint8_t period[7] = {0, 0, 0, 0, 0, 6, 6};
            glyph = period;
        } else if (text[index] == '_') {
            static const uint8_t underscore[7] = {0, 0, 0, 0, 0, 0, 31};
            glyph = underscore;
        }
        if (glyph != NULL) {
            for (uint32_t row = 0U; row < 7U; row++) {
                for (uint32_t column = 0U; column < 5U; column++) {
                    if ((glyph[row] & (uint8_t)(1U << (4U - column))) == 0U) {
                        continue;
                    }
                    for (uint32_t scale_y = 0U; scale_y < VIEWER_FRAME_FONT_SCALE; scale_y++) {
                        for (uint32_t scale_x = 0U; scale_x < VIEWER_FRAME_FONT_SCALE; scale_x++) {
                            int64_t px = cursor_x + (int64_t)(column * VIEWER_FRAME_FONT_SCALE + scale_x);
                            int64_t py = y + (int64_t)(row * VIEWER_FRAME_FONT_SCALE + scale_y);
                            if (px >= 0 && py >= 0 && (uint64_t)px < width && (uint64_t)py < height) {
                                size_t offset = ((size_t)py * width + (size_t)px) * 4U;
                                rgba[offset] = red;
                                rgba[offset + 1U] = green;
                                rgba[offset + 2U] = blue;
                                rgba[offset + 3U] = 255U;
                            }
                        }
                    }
                }
            }
        }
        cursor_x += 6 * (int64_t)VIEWER_FRAME_FONT_SCALE;
        if (cursor_x >= (int64_t)width) {
            break;
        }
    }
}

static void fill_background(uint8_t *rgba, uint32_t width, uint32_t height,
                            viewer_frame_kind kind)
{
    for (uint32_t y = 0U; y < height; y++) {
        for (uint32_t x = 0U; x < width; x++) {
            size_t offset = ((size_t)y * width + x) * 4U;
            uint8_t red = 0U, green = 0U, blue = 0U;
            if (kind == VIEWER_FRAME_DOCUMENT) {
                bool light = ((x / VIEWER_FRAME_CHECKER_SIZE) +
                              (y / VIEWER_FRAME_CHECKER_SIZE)) % 2U == 0U;
                red = light ? 224U : 204U;
                green = light ? 226U : 206U;
                blue = light ? 230U : 210U;
            } else if (kind == VIEWER_FRAME_LOADING) {
                red = 25U;
                green = 29U;
                blue = 38U;
            } else {
                red = 67U;
                green = 21U;
                blue = 36U;
            }
            rgba[offset] = red;
            rgba[offset + 1U] = green;
            rgba[offset + 2U] = blue;
            rgba[offset + 3U] = 255U;
        }
    }
}

void viewer_navigation_clamp(ui_viewer_navigation *navigation)
{
    if (navigation == NULL || !isfinite(navigation->scale) || !(navigation->scale > 0.0) ||
        !isfinite(navigation->viewport_width) || !(navigation->viewport_width > 0.0) ||
        !isfinite(navigation->viewport_height) || !(navigation->viewport_height > 0.0) ||
        !isfinite(navigation->document_width) || !(navigation->document_width > 0.0) ||
        !isfinite(navigation->document_height) || !(navigation->document_height > 0.0) ||
        !isfinite(navigation->pan_x) || !isfinite(navigation->pan_y)) {
        return;
    }
    double document_width = navigation->document_width * navigation->scale;
    double document_height = navigation->document_height * navigation->scale;
    if (!isfinite(document_width) || !isfinite(document_height)) {
        return;
    }
    if (document_width <= navigation->viewport_width) {
        navigation->pan_x = (navigation->viewport_width - document_width) * 0.5;
    } else {
        double margin = fmin(32.0, navigation->viewport_width * 0.1);
        navigation->pan_x = clamp_double(navigation->pan_x,
            margin - document_width, navigation->viewport_width - margin);
    }
    if (document_height <= navigation->viewport_height) {
        navigation->pan_y = (navigation->viewport_height - document_height) * 0.5;
    } else {
        double margin = fmin(32.0, navigation->viewport_height * 0.1);
        navigation->pan_y = clamp_double(navigation->pan_y,
            margin - document_height, navigation->viewport_height - margin);
    }
}

bool viewer_compose_frame(const raster_tile *document,
                          const ui_viewer_navigation *navigation,
                          const ui_viewer_progress *progress,
                          viewer_frame_kind kind, const char *message,
                          uint8_t *out_rgba, size_t output_capacity,
                          uint32_t width, uint32_t height)
{
    size_t required = 0U;
    if (!frame_size(width, height, &required) || out_rgba == NULL ||
        output_capacity < required || kind < VIEWER_FRAME_LOADING || kind > VIEWER_FRAME_ERROR) {
        return false;
    }
    if (kind == VIEWER_FRAME_DOCUMENT &&
        (!tile_valid(document) || navigation == NULL ||
         !isfinite(navigation->viewport_width) || !isfinite(navigation->viewport_height) ||
         !isfinite(navigation->document_width) || !isfinite(navigation->document_height) ||
         !isfinite(navigation->scale) || !(navigation->scale > 0.0) ||
         !isfinite(navigation->pan_x) || !isfinite(navigation->pan_y) ||
         navigation->viewport_width <= 0.0 || navigation->viewport_height <= 0.0)) {
        return false;
    }

    fill_background(out_rgba, width, height, kind);
    if (kind == VIEWER_FRAME_DOCUMENT) {
        double right = navigation->pan_x + navigation->document_width * navigation->scale;
        double bottom = navigation->pan_y + navigation->document_height * navigation->scale;
        if (!isfinite(right) || !isfinite(bottom)) {
            return false;
        }
        bool nearest = navigation->scale > 4.0;
        for (uint32_t y = 0U; y < height; y++) {
            double screen_y = (double)y + 0.5;
            if (screen_y < navigation->pan_y || screen_y >= bottom) {
                continue;
            }
            double source_y = (screen_y - navigation->pan_y) / navigation->scale - 0.5;
            for (uint32_t x = 0U; x < width; x++) {
                double screen_x = (double)x + 0.5;
                if (screen_x < navigation->pan_x || screen_x >= right) {
                    continue;
                }
                double source_x = (screen_x - navigation->pan_x) / navigation->scale - 0.5;
                double sample[4];
                sample_tile(document, source_x, source_y, nearest, sample);
                size_t offset = ((size_t)y * width + x) * 4U;
                composite_pixel(out_rgba + offset, out_rgba[offset], out_rgba[offset + 1U],
                               out_rgba[offset + 2U], sample);
            }
        }
    } else if (kind == VIEWER_FRAME_LOADING) {
        static const char loading[] = "LOADING SVG";
        size_t text_width = (sizeof(loading) - 1U) * 12U;
        int64_t x = width > text_width ? (int64_t)((width - text_width) / 2U) : 4;
        int64_t y = height > 20U ? (int64_t)(height / 2U) - 7 : 0;
        draw_text(out_rgba, width, height, x, y, loading, 210U, 218U, 232U);
        if (progress == NULL) {
            ui_viewer_progress initial = {0};
            initial.phase = UI_PROGRESS_ACTIVE;
            initial.label[0] = 'L';
            initial.label[1] = 'o';
            initial.label[2] = 'a';
            initial.label[3] = 'd';
            initial.label[4] = 'i';
            initial.label[5] = 'n';
            initial.label[6] = 'g';
            initial.label[7] = '\0';
            (void)ui_viewer_overlay_rgba(&initial, out_rgba, required, width, height,
                                         (size_t)width * 4U);
        } else {
            (void)ui_viewer_overlay_rgba(progress, out_rgba, required, width, height,
                                         (size_t)width * 4U);
        }
    } else {
        static const char heading[] = "SVG ERROR";
        size_t heading_width = (sizeof(heading) - 1U) * 12U;
        int64_t x = width > heading_width ? (int64_t)((width - heading_width) / 2U) : 4;
        int64_t y = height > 40U ? (int64_t)(height / 2U) - 18 : 0;
        draw_text(out_rgba, width, height, x, y, heading, 255U, 224U, 228U);
        if (message != NULL) {
            size_t message_length = 0U;
            while (message_length < 48U && message[message_length] != '\0') {
                message_length++;
            }
            if (message_length > 0U && height > 12U) {
                char line[49];
                memcpy(line, message, message_length);
                line[message_length] = '\0';
                size_t line_width = message_length * 12U;
                int64_t line_x = width > line_width ? (int64_t)((width - line_width) / 2U) : 4;
                draw_text(out_rgba, width, height, line_x, y + 24, line,
                          255U, 183U, 194U);
            }
        }
    }
    return true;
}
