#include "viewer_editor_internal.h"

#include "ui/ui_core.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define GUIDE_FRAME_WIDTH 640U
#define GUIDE_FRAME_HEIGHT 480U

static void fill(uint8_t *rgba, uint32_t width, uint32_t height, size_t stride,
    ui_rect rect, uint32_t color)
{
    if (rgba == NULL || rect.x > width || rect.y > height ||
        rect.width > width - rect.x || rect.height > height - rect.y) return;
    uint8_t red = (uint8_t)(color >> 24U);
    uint8_t green = (uint8_t)(color >> 16U);
    uint8_t blue = (uint8_t)(color >> 8U);
    for (uint32_t y = rect.y; y < rect.y + rect.height; ++y) {
        uint8_t *row = rgba + (size_t)y * stride + (size_t)rect.x * 4U;
        for (uint32_t x = 0U; x < rect.width; ++x) {
            uint8_t *pixel = row + (size_t)x * 4U;
            pixel[0] = red;
            pixel[1] = green;
            pixel[2] = blue;
            pixel[3] = 255U;
        }
    }
}

int main(int argc, char **argv)
{
    if (argc != 3 || (strcmp(argv[1], "dark") != 0 &&
            strcmp(argv[1], "light") != 0) ||
        (strcmp(argv[2], "1") != 0 && strcmp(argv[2], "2") != 0)) {
        (void)fprintf(stderr, "usage: viewer_guides_render dark|light 1|2\n");
        return 2;
    }
    uint32_t scale = strcmp(argv[2], "2") == 0 ? 2U : 1U;
    uint32_t width = GUIDE_FRAME_WIDTH * scale;
    uint32_t height = GUIDE_FRAME_HEIGHT * scale;
    size_t stride = (size_t)width * 4U;
    size_t bytes = stride * height;
    ui_theme theme = strcmp(argv[1], "light") == 0 ? UI_THEME_LIGHT : UI_THEME_DARK;
    static const uint8_t source[] =
        "<svg xmlns='http://www.w3.org/2000/svg' width='100' height='100'/>";
    viewer_editor *editor = viewer_editor_create_styled(
        (base_bytes){source, sizeof(source) - 1U}, theme, width, height, true);
    uint8_t *rgba = malloc(bytes);
    if (editor == NULL || rgba == NULL) {
        viewer_editor_destroy(editor);
        free(rgba);
        return 1;
    }
    viewer_editor_resize(editor, width, height, (double)scale);
    editor->modern_style = true;
    editor->backing_scale = (double)scale;
    svg_shape_document document = {.width = 100U, .height = 100U,
        .view_x = -10.0, .view_y = -10.0, .view_width = 120.0,
        .view_height = 120.0,
        .preserve_aspect = {0.5, 0.5, false, false, true}};
    ui_rect canvas = {36U * scale, 36U * scale,
        width - 36U * scale, height - 36U * scale};
    ui_viewer_navigation navigation = {0};
    bool okay = ui_viewer_navigation_init(&navigation, canvas.width, canvas.height,
            document.width, document.height) &&
        ui_viewer_navigation_fit(&navigation) &&
        viewer_editor_guides_add(&editor->guides,
            VIEWER_EDITOR_GUIDE_HORIZONTAL, 55.0) &&
        viewer_editor_guides_add(&editor->guides,
            VIEWER_EDITOR_GUIDE_VERTICAL, 35.0);
    if (!okay) {
        viewer_editor_destroy(editor);
        free(rgba);
        return 1;
    }
    editor->guides.selected = false;
    ui_tokens tokens = ui_theme_tokens(theme, false);
    fill(rgba, width, height, stride, (ui_rect){0U, 0U, width, height}, tokens.window);
    fill(rgba, width, height, stride, canvas, tokens.canvas_well);
    uint32_t inset_x = canvas.width / 5U;
    uint32_t inset_y = canvas.height / 5U;
    fill(rgba, width, height, stride,
        (ui_rect){canvas.x + inset_x, canvas.y + inset_y,
            canvas.width - 2U * inset_x, canvas.height - 2U * inset_y},
        theme == UI_THEME_DARK ? UINT32_C(0x303944ff) : UINT32_C(0xf7f8faff));
    viewer_editor_guides_paint_scalar(editor, &document, &navigation, canvas, rgba,
        width, height, stride);
    okay = fwrite(rgba, 1U, bytes, stdout) == bytes;
    viewer_editor_destroy(editor);
    free(rgba);
    return okay ? 0 : 1;
}
