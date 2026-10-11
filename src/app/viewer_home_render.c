#include "viewer_home.h"

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MAX_AXIS 8192U
#define MAX_BYTES (128U * 1024U * 1024U)

static bool parse_axis(const char *text, uint32_t *out_value)
{
    if (text == NULL || out_value == NULL || text[0] == '\0') return false;
    errno = 0;
    char *end = NULL;
    unsigned long value = strtoul(text, &end, 10);
    if (errno != 0 || end == text || *end != '\0' || value == 0UL || value > MAX_AXIS)
        return false;
    *out_value = (uint32_t)value;
    return true;
}

static void fill(uint8_t *rgba, uint32_t width, uint32_t height, uint32_t color)
{
    uint8_t red = (uint8_t)(color >> 24U);
    uint8_t green = (uint8_t)(color >> 16U);
    uint8_t blue = (uint8_t)(color >> 8U);
    for (size_t offset = 0U; offset < (size_t)width * height * 4U; offset += 4U) {
        rgba[offset] = red;
        rgba[offset + 1U] = green;
        rgba[offset + 2U] = blue;
        rgba[offset + 3U] = 255U;
    }
}

int main(int argc, char **argv)
{
    if (argc != 5 || (strcmp(argv[1], "dark") != 0 && strcmp(argv[1], "light") != 0)) {
        (void)fprintf(stderr,
            "usage: viewer_home_render dark|light empty|loading|error|document-loading|document-error WIDTH HEIGHT\n");
        return 2;
    }
    uint32_t width = 0U, height = 0U;
    if (!parse_axis(argv[3], &width) || !parse_axis(argv[4], &height) ||
        (size_t)width > SIZE_MAX / 4U ||
        (size_t)height > SIZE_MAX / ((size_t)width * 4U)) return 2;
    size_t bytes = (size_t)width * height * 4U;
    if (bytes > MAX_BYTES) return 2;
    bool dark = strcmp(argv[1], "dark") == 0;
    const char *kind = argv[2];
    bool known = strcmp(kind, "empty") == 0 || strcmp(kind, "loading") == 0 ||
        strcmp(kind, "error") == 0 || strcmp(kind, "document-loading") == 0 ||
        strcmp(kind, "document-error") == 0;
    if (!known) return 2;
    uint8_t *rgba = malloc(bytes);
    if (rgba == NULL) return 1;
    ui_theme theme = dark ? UI_THEME_DARK : UI_THEME_LIGHT;
    ui_tokens tokens = ui_theme_tokens(theme, false);
    fill(rgba, width, height, tokens.window);

    viewer_home_state state = {0};
    state.loading = strcmp(kind, "loading") == 0 ||
        strcmp(kind, "document-loading") == 0;
    state.load_failed = strcmp(kind, "error") == 0 ||
        strcmp(kind, "document-error") == 0;
    state.load_error = "UNSUPPORTED SVG CONTENT";
    if (kind[0] == 'd') {
        state.has_document = true;
        state.canvas = (ui_rect){80U, 40U, width - 160U, height - 120U};
        uint8_t red = (uint8_t)(tokens.canvas_well >> 24U);
        uint8_t green = (uint8_t)(tokens.canvas_well >> 16U);
        uint8_t blue = (uint8_t)(tokens.canvas_well >> 8U);
        for (uint32_t y = state.canvas.y; y < state.canvas.y + state.canvas.height; ++y) {
            for (uint32_t x = state.canvas.x; x < state.canvas.x + state.canvas.width; ++x) {
                size_t at = ((size_t)y * width + x) * 4U;
                rgba[at] = red;
                rgba[at + 1U] = green;
                rgba[at + 2U] = blue;
            }
        }
        if (state.canvas.width > 240U && state.canvas.height > 160U) {
            for (uint32_t y = state.canvas.y + 120U; y < state.canvas.y + 160U; ++y) {
                for (uint32_t x = state.canvas.x + 120U; x < state.canvas.x + 360U; ++x) {
                    size_t at = ((size_t)y * width + x) * 4U;
                    rgba[at] = (uint8_t)(tokens.accent >> 24U);
                    rgba[at + 1U] = (uint8_t)(tokens.accent >> 16U);
                    rgba[at + 2U] = (uint8_t)(tokens.accent >> 8U);
                }
            }
        }
    }
    double scale = width >= 2500U ? 2.0 : 1.0;
    viewer_home_paint(&state, theme, rgba, bytes, (size_t)width * 4U,
        width, height, scale);
    bool okay = fwrite(rgba, 1U, bytes, stdout) == bytes;
    free(rgba);
    return okay ? 0 : 1;
}
