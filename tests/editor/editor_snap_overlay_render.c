#include "viewer_editor_internal.h"
#include "viewer_chrome.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define SNAPSHOT_WIDTH 1280U
#define SNAPSHOT_HEIGHT 800U

static const uint8_t source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='100' height='100' "
    "viewBox='0 0 100 100'><rect id='selected' x='20' y='20' width='10' "
    "height='10' fill='#6b9eff'/></svg>";

static void fill_shape(uint8_t *rgba, uint32_t width, uint32_t height,
    size_t stride, ui_rect canvas, const ui_viewer_navigation *navigation,
    svg_shape_rect bounds)
{
    double x0 = canvas.x + navigation->pan_x + bounds.x * navigation->scale;
    double y0 = canvas.y + navigation->pan_y + bounds.y * navigation->scale;
    double x1 = x0 + bounds.width * navigation->scale;
    double y1 = y0 + bounds.height * navigation->scale;
    uint32_t left = (uint32_t)fmax((double)canvas.x, floor(x0));
    uint32_t top = (uint32_t)fmax((double)canvas.y, floor(y0));
    uint32_t right = (uint32_t)fmin((double)width, ceil(x1));
    uint32_t bottom = (uint32_t)fmin((double)height, ceil(y1));
    uint64_t canvas_right = (uint64_t)canvas.x + canvas.width;
    uint64_t canvas_bottom = (uint64_t)canvas.y + canvas.height;
    if ((uint64_t)right > canvas_right) right = (uint32_t)canvas_right;
    if ((uint64_t)bottom > canvas_bottom) bottom = (uint32_t)canvas_bottom;
    for (uint32_t y = top; y < bottom; ++y)
        for (uint32_t x = left; x < right; ++x) {
            uint8_t *pixel = rgba + (size_t)y * stride + (size_t)x * 4U;
            pixel[0] = 107U;
            pixel[1] = 158U;
            pixel[2] = 255U;
            pixel[3] = 255U;
        }
}

int main(int argc, char **argv)
{
    if (argc != 2 || (strcmp(argv[1], "dark") != 0 &&
            strcmp(argv[1], "light") != 0)) {
        (void)fprintf(stderr, "usage: editor_snap_overlay_render dark|light\n");
        return 2;
    }
    ui_theme theme = strcmp(argv[1], "light") == 0 ? UI_THEME_LIGHT : UI_THEME_DARK;
    viewer_editor *editor = viewer_editor_create_styled(
        (base_bytes){source, sizeof(source) - 1U}, theme,
        SNAPSHOT_WIDTH, SNAPSHOT_HEIGHT, false);
    svg_shape_document document = {0};
    svg_shape_diagnostic diagnostic = {0};
    uint8_t *rgba = malloc((size_t)SNAPSHOT_WIDTH * SNAPSHOT_HEIGHT * 4U);
    if (editor == NULL || rgba == NULL || svg_shape_decode(
            (base_bytes){source, sizeof(source) - 1U}, &document,
            &diagnostic) != SVG_SHAPE_ERROR_NONE) goto fail;
    size_t stride = (size_t)SNAPSHOT_WIDTH * 4U;
    size_t bytes = stride * SNAPSHOT_HEIGHT;
    viewer_editor_resize(editor, SNAPSHOT_WIDTH, SNAPSHOT_HEIGHT, 1.0);
    viewer_chrome_layout layout;
    ui_viewer_navigation navigation = {0};
    if (!viewer_editor_get_chrome_layout(editor, &layout) ||
        !ui_viewer_navigation_init(&navigation, layout.canvas.width,
            layout.canvas.height, document.width, document.height) ||
        !ui_viewer_navigation_fit(&navigation)) goto fail;
    ui_tokens tokens = ui_theme_tokens(theme, false);
    for (size_t at = 0U; at < bytes; at += 4U) {
        rgba[at] = (uint8_t)(tokens.window >> 24U);
        rgba[at + 1U] = (uint8_t)(tokens.window >> 16U);
        rgba[at + 2U] = (uint8_t)(tokens.window >> 8U);
        rgba[at + 3U] = 255U;
    }
    const char *selected = strstr((const char *)source, "<rect");
    if (selected == NULL || !viewer_editor_select_at_source_offset(editor,
            (size_t)(selected - (const char *)source))) goto fail;
    viewer_editor_paint_scalar(editor, rgba, bytes, stride);
    svg_shape_selection selection;
    if (!svg_shape_element_at_offset(&document,
            (size_t)(selected - (const char *)source), &selection)) goto fail;
    fill_shape(rgba, SNAPSHOT_WIDTH, SNAPSHOT_HEIGHT, stride, layout.canvas,
        &navigation, selection.device_bounds);
    editor->snap_grid_enabled = false;
    editor->snap_objects_enabled = false;
    editor->snap_guides_enabled = true;
    if (!editor_canvas_snap_guide_add(editor, true, 35.0) ||
        (viewer_editor_canvas_pointer_down(editor, &document, &navigation,
            navigation.pan_x + 25.0 * navigation.scale,
            navigation.pan_y + 25.0 * navigation.scale, 0U) &
            VIEWER_EDITOR_HANDLED) == 0U ||
        (viewer_editor_canvas_pointer_move(editor, &document, &navigation,
            navigation.pan_x + 39.5 * navigation.scale,
            navigation.pan_y + 25.0 * navigation.scale) &
            VIEWER_EDITOR_HANDLED) == 0U ||
        !editor->snap_guide_x_visible || editor->snap_guide_y_visible ||
        !editor->snap_label_visible) goto fail;
    viewer_editor_paint_canvas_overlay(editor, &document, &navigation,
        layout.canvas, rgba, SNAPSHOT_WIDTH, SNAPSHOT_HEIGHT, stride);
    bool okay = fwrite(rgba, 1U, bytes, stdout) == bytes;
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    free(rgba);
    return okay ? 0 : 1;

fail:
    svg_shape_document_destroy(&document);
    viewer_editor_destroy(editor);
    free(rgba);
    (void)fputs("snapping overlay renderer failed\n", stderr);
    return 1;
}
