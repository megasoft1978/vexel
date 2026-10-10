#include "viewer_chrome.h"
#include "viewer_editor.h"

#include "svg/shape/shape.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define SNAPSHOT_POINTS_WIDTH 1280U
#define SNAPSHOT_POINTS_HEIGHT 800U

static const uint8_t source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
    "viewBox='0 0 64 64'><rect id='selected' x='8' y='8' width='40' height='24' "
    "fill='#6b9eff'/></svg>";

static void fill_document_shape(uint8_t *rgba, uint32_t width, uint32_t height,
    size_t stride, ui_rect canvas, const ui_viewer_navigation *navigation,
    svg_shape_rect bounds)
{
    double x0 = (double)canvas.x + navigation->pan_x + bounds.x * navigation->scale;
    double y0 = (double)canvas.y + navigation->pan_y + bounds.y * navigation->scale;
    double x1 = x0 + bounds.width * navigation->scale;
    double y1 = y0 + bounds.height * navigation->scale;
    uint32_t left = x0 < 0.0 ? 0U : (uint32_t)x0;
    uint32_t top = y0 < 0.0 ? 0U : (uint32_t)y0;
    uint32_t right = x1 > (double)width ? width : (uint32_t)x1;
    uint32_t bottom = y1 > (double)height ? height : (uint32_t)y1;
    uint64_t canvas_right = (uint64_t)canvas.x + canvas.width;
    uint64_t canvas_bottom = (uint64_t)canvas.y + canvas.height;
    if (left < canvas.x) left = canvas.x;
    if (top < canvas.y) top = canvas.y;
    if ((uint64_t)right > canvas_right) right = (uint32_t)canvas_right;
    if ((uint64_t)bottom > canvas_bottom) bottom = (uint32_t)canvas_bottom;
    for (uint32_t y = top; y < bottom; ++y) {
        for (uint32_t x = left; x < right; ++x) {
            uint8_t *pixel = rgba + (size_t)y * stride + (size_t)x * 4U;
            pixel[0] = 107U; pixel[1] = 158U; pixel[2] = 255U; pixel[3] = 255U;
        }
    }
}

int main(int argc, char **argv)
{
    if (argc != 3 || (strcmp(argv[1], "dark") != 0 &&
            strcmp(argv[1], "light") != 0) ||
        (strcmp(argv[2], "1") != 0 && strcmp(argv[2], "2") != 0)) {
        (void)fprintf(stderr, "usage: editor_selection_render dark|light 1|2\n");
        return 2;
    }
    uint32_t scale = strcmp(argv[2], "2") == 0 ? 2U : 1U;
    uint32_t width = SNAPSHOT_POINTS_WIDTH * scale;
    uint32_t height = SNAPSHOT_POINTS_HEIGHT * scale;
    size_t stride = (size_t)width * 4U;
    size_t bytes = stride * height;
    ui_theme theme = strcmp(argv[1], "light") == 0 ? UI_THEME_LIGHT : UI_THEME_DARK;
    viewer_editor *editor = viewer_editor_create_styled(
        (base_bytes){source, sizeof(source) - 1U}, theme, width, height, false);
    uint8_t *rgba = malloc(bytes);
    svg_shape_document document = {0};
    svg_shape_diagnostic diagnostic = {0};
    ui_viewer_navigation navigation = {0};
    viewer_chrome_layout layout;
    bool okay = editor != NULL && rgba != NULL &&
        svg_shape_decode((base_bytes){source, sizeof(source) - 1U},
            &document, &diagnostic) == SVG_SHAPE_ERROR_NONE;
    if (!okay) goto done;
    viewer_editor_resize(editor, width, height, (double)scale);
    if (!viewer_editor_get_chrome_layout(editor, &layout) ||
        !ui_viewer_navigation_init(&navigation, layout.canvas.width,
            layout.canvas.height, document.width, document.height) ||
        !ui_viewer_navigation_fit(&navigation)) {
        okay = false;
        goto done;
    }
    for (size_t offset = 0U; offset < bytes; offset += 4U) {
        rgba[offset] = theme == UI_THEME_DARK ? 21U : 250U;
        rgba[offset + 1U] = theme == UI_THEME_DARK ? 24U : 251U;
        rgba[offset + 2U] = theme == UI_THEME_DARK ? 30U : 253U;
        rgba[offset + 3U] = 255U;
    }
    svg_shape_selection selection;
    if (!svg_shape_element_at_offset(&document,
            (size_t)(strstr((const char *)source, "<rect") - (const char *)source),
            &selection)) {
        okay = false;
        goto done;
    }
    fill_document_shape(rgba, width, height, stride, layout.canvas,
        &navigation, selection.device_bounds);
    viewer_editor_paint_scalar(editor, rgba, bytes, stride);
    geom_point hit = {navigation.pan_x + 24.0 * navigation.scale,
        navigation.pan_y + 16.0 * navigation.scale};
    uint32_t selected = viewer_editor_canvas_pointer_down(editor, &document,
        &navigation, hit.x, hit.y, 0U);
    if ((selected & VIEWER_EDITOR_HANDLED) == 0U) {
        okay = false;
        goto done;
    }
    (void)viewer_editor_canvas_pointer_up(editor, &document);
    viewer_editor_paint_canvas_overlay(editor, &document, &navigation,
        layout.canvas, rgba, width, height, stride);
    okay = fwrite(rgba, 1U, bytes, stdout) == bytes;
done:
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    free(rgba);
    if (!okay) (void)fputs("selection overlay renderer failed\n", stderr);
    return okay ? 0 : 1;
}
