#include "viewer_editor.h"
#include "viewer_chrome.h"
#include "ui/ui_core.h"
#include "svg/shape/shape.h"

#include <errno.h>
#include <math.h>
#include <stdint.h>
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
    if (errno != 0 || end == text || *end != '\0' || value == 0UL ||
        value > MAX_AXIS) return false;
    *out_value = (uint32_t)value;
    return true;
}

static void fill_window(uint8_t *rgba, size_t length, uint32_t packed)
{
    uint8_t red = (uint8_t)(packed >> 24U);
    uint8_t green = (uint8_t)(packed >> 16U);
    uint8_t blue = (uint8_t)(packed >> 8U);
    for (size_t at = 0U; at < length; at += 4U) {
        rgba[at] = red;
        rgba[at + 1U] = green;
        rgba[at + 2U] = blue;
        rgba[at + 3U] = 255U;
    }
}

static void fill_rect(uint8_t *rgba, uint32_t width, uint32_t height,
                      size_t stride, ui_rect rect, uint32_t packed)
{
    if (rect.x > width || rect.y > height || rect.width > width - rect.x ||
        rect.height > height - rect.y) return;
    uint8_t red = (uint8_t)(packed >> 24U);
    uint8_t green = (uint8_t)(packed >> 16U);
    uint8_t blue = (uint8_t)(packed >> 8U);
    for (uint32_t y = rect.y; y < rect.y + rect.height; ++y) {
        for (uint32_t x = rect.x; x < rect.x + rect.width; ++x) {
            uint8_t *pixel = rgba + (size_t)y * stride + (size_t)x * 4U;
            pixel[0] = red;
            pixel[1] = green;
            pixel[2] = blue;
            pixel[3] = 255U;
        }
    }
}

static geom_point path_screen_point(ui_rect canvas,
    const ui_viewer_navigation *navigation, geom_point point)
{
    return (geom_point){(double)canvas.x + navigation->pan_x +
            point.x * navigation->scale,
        (double)canvas.y + navigation->pan_y + point.y * navigation->scale};
}

static void paint_path_line(uint8_t *rgba, uint32_t width, uint32_t height,
    size_t stride, ui_rect clip, geom_point from, geom_point to,
    uint32_t packed)
{
    int64_t x0 = (int64_t)llround(from.x), y0 = (int64_t)llround(from.y);
    int64_t x1 = (int64_t)llround(to.x), y1 = (int64_t)llround(to.y);
    int64_t dx = llabs(x1 - x0), sx = x0 < x1 ? 1 : -1;
    int64_t dy = -llabs(y1 - y0), sy = y0 < y1 ? 1 : -1;
    int64_t error = dx + dy;
    for (;;) {
        if (x0 >= (int64_t)clip.x && y0 >= (int64_t)clip.y &&
            x0 < (int64_t)clip.x + clip.width &&
            y0 < (int64_t)clip.y + clip.height && x0 >= 0 && y0 >= 0 &&
            (uint64_t)x0 < width && (uint64_t)y0 < height) {
            uint8_t *pixel = rgba + (size_t)y0 * stride + (size_t)x0 * 4U;
            pixel[0] = (uint8_t)(packed >> 24U);
            pixel[1] = (uint8_t)(packed >> 16U);
            pixel[2] = (uint8_t)(packed >> 8U);
            pixel[3] = 255U;
        }
        if (x0 == x1 && y0 == y1) break;
        int64_t twice = 2 * error;
        if (twice >= dy) { error += dy; x0 += sx; }
        if (twice <= dx) { error += dx; y0 += sy; }
    }
}

static void paint_node_path(uint8_t *rgba, uint32_t width, uint32_t height,
    size_t stride, ui_rect canvas, const ui_viewer_navigation *navigation,
    uint32_t color)
{
    geom_point start = {8.0, 8.0}, line_end = {16.0, 8.0};
    paint_path_line(rgba, width, height, stride, canvas,
        path_screen_point(canvas, navigation, start),
        path_screen_point(canvas, navigation, line_end), color);
    geom_point previous = line_end;
    for (size_t step = 1U; step <= 32U; ++step) {
        double t = (double)step / 32.0, inverse = 1.0 - t;
        geom_point current = {
            inverse * inverse * inverse * line_end.x +
                3.0 * inverse * inverse * t * 20.0 +
                3.0 * inverse * t * t * 24.0 + t * t * t * 28.0,
            inverse * inverse * inverse * line_end.y +
                3.0 * inverse * inverse * t * 4.0 +
                3.0 * inverse * t * t * 12.0 + t * t * t * 8.0};
        paint_path_line(rgba, width, height, stride, canvas,
            path_screen_point(canvas, navigation, previous),
            path_screen_point(canvas, navigation, current), color);
        previous = current;
    }
    geom_point smooth_start = {28.0, 8.0}, smooth_control = {32.0, 4.0};
    previous = smooth_start;
    for (size_t step = 1U; step <= 32U; ++step) {
        double t = (double)step / 32.0, inverse = 1.0 - t;
        geom_point current = {
            inverse * inverse * inverse * smooth_start.x +
                3.0 * inverse * inverse * t * smooth_control.x +
                3.0 * inverse * t * t * smooth_control.x + t * t * t * 36.0,
            inverse * inverse * inverse * smooth_start.y +
                3.0 * inverse * inverse * t * smooth_control.y +
                3.0 * inverse * t * t * smooth_control.y + t * t * t * 8.0};
        paint_path_line(rgba, width, height, stride, canvas,
            path_screen_point(canvas, navigation, previous),
            path_screen_point(canvas, navigation, current), color);
        previous = current;
    }
}

int main(int argc, char **argv)
{
    if ((argc != 5 && argc != 6) ||
        (strcmp(argv[1], "dark") != 0 && strcmp(argv[1], "light") != 0)) {
        (void)fprintf(stderr,
            "usage: viewer_chrome_render dark|light WIDTH HEIGHT STATE [LAYOUT_WIDTH]\n");
        return 2;
    }
    uint32_t width = 0U, height = 0U, layout_width = 0U;
    if (!parse_axis(argv[2], &width) || !parse_axis(argv[3], &height)) return 2;
    const char *state = argv[4];
    const char *states[] = {"empty", "selection", "editing-field", "error",
        "narrow", "node-edit", "scale-110", "scale-120"};
    bool known_state = false;
    for (size_t index = 0U; index < sizeof(states) / sizeof(states[0]); ++index)
        if (strcmp(state, states[index]) == 0) known_state = true;
    if (!known_state) return 2;
    bool narrow = strcmp(state, "narrow") == 0;
    if (argc == 6) {
        if (!parse_axis(argv[5], &layout_width) || layout_width > width) return 2;
    } else layout_width = narrow ? width / 2U : width;
    if (layout_width == 0U) return 2;
    if ((size_t)width > SIZE_MAX / 4U ||
        (size_t)height > SIZE_MAX / ((size_t)width * 4U)) return 2;
    size_t length = (size_t)width * height * 4U;
    if (length > MAX_BYTES) return 2;
    ui_theme theme = strcmp(argv[1], "light") == 0 ? UI_THEME_LIGHT : UI_THEME_DARK;
    static const uint8_t selection_source[] =
        "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64'>"
        "<g id='layer'><rect id='selected' x='8' y='8' width='40' height='24' fill='#6b9eff'/>"
        "<circle id='dot' cx='32' cy='48' r='8' fill='#ef8354'/></g></svg>";
    static const uint8_t empty_source[] =
        "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64'></svg>";
    static const uint8_t node_source[] =
        "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64'>"
        "<path d='M 8 8 L 16 8 C 20 4 24 12 28 8 S 32 4 36 8' "
        "fill='none' stroke='#246bda'/></svg>";
    bool node_edit = strcmp(state, "node-edit") == 0;
    const uint8_t *source = narrow || strcmp(state, "selection") == 0 ||
        strcmp(state, "editing-field") == 0 || strcmp(state, "error") == 0 ?
        selection_source : node_edit ? node_source : empty_source;
    size_t source_length = source == selection_source ?
        sizeof(selection_source) - 1U : source == node_source ?
        sizeof(node_source) - 1U : sizeof(empty_source) - 1U;
    viewer_editor *editor = viewer_editor_create_styled((base_bytes){source, source_length},
        theme, layout_width, height, true);
    uint8_t *rgba = malloc(length);
    if (editor == NULL || rgba == NULL) {
        viewer_editor_destroy(editor);
        free(rgba);
        return 1;
    }
    double scale = width >= 2500U ? 2.0 : 1.0;
    viewer_editor_resize(editor, layout_width, height, scale);
    if (strcmp(state, "scale-110") == 0)
        (void)viewer_editor_set_ui_scale(editor, 110U);
    else if (strcmp(state, "scale-120") == 0)
        (void)viewer_editor_set_ui_scale(editor, 120U);
    ui_tokens tokens = ui_theme_tokens(theme, false);
    fill_window(rgba, length, tokens.window);
    viewer_chrome_layout layout;
    if (viewer_editor_get_chrome_layout(editor, &layout))
        fill_rect(rgba, width, height, (size_t)width * 4U,
            layout.canvas, tokens.canvas_well);
    svg_shape_document node_document = {0};
    ui_viewer_navigation navigation = {0};
    if (node_edit) {
        svg_shape_diagnostic diagnostic = {0};
        const char *path = strstr((const char *)node_source, "<path");
        if (path == NULL || svg_shape_decode((base_bytes){node_source,
                sizeof(node_source) - 1U}, &node_document, &diagnostic) !=
                SVG_SHAPE_ERROR_NONE ||
            !viewer_editor_select_at_source_offset(editor,
                (size_t)(path - (const char *)node_source)) ||
            (viewer_editor_key(editor, 0U, 0U, "n", 1U) &
                VIEWER_EDITOR_HANDLED) == 0U) {
            svg_shape_document_destroy(&node_document);
            viewer_editor_destroy(editor);
            free(rgba);
            return 1;
        }
        for (size_t index = 0U; index < 3U; ++index) {
            if ((viewer_editor_node_edit_key(editor, &node_document,
                    48U, 0U) & VIEWER_EDITOR_HANDLED) == 0U) {
                svg_shape_document_destroy(&node_document);
                viewer_editor_destroy(editor);
                free(rgba);
                return 1;
            }
        }
        (void)viewer_editor_node_edit_hover(editor, &node_document,
            (geom_point){12.0, 8.0}, 0.25);
        if (!viewer_editor_get_chrome_layout(editor, &layout) ||
            !ui_viewer_navigation_init(&navigation, layout.canvas.width,
                layout.canvas.height, node_document.width,
                node_document.height)) {
            svg_shape_document_destroy(&node_document);
            viewer_editor_destroy(editor);
            free(rgba);
            return 1;
        }
        navigation.scale = 6.0 * scale;
        navigation.pan_x = ((double)layout.canvas.width - 28.0 *
            navigation.scale) * 0.5 - 8.0 * navigation.scale;
        navigation.pan_y = ((double)layout.canvas.height - 8.0 *
            navigation.scale) * 0.5 - 4.0 * navigation.scale;
    }
    if (strcmp(state, "empty") == 0)
        viewer_editor_set_status(editor, "Open an SVG to begin");
    else if (strcmp(state, "error") == 0)
        viewer_editor_set_status(editor, "Unsupported SVG content · preview retained");
    else if (strcmp(state, "selection") == 0) {
        const char *selected = strstr((const char *)selection_source, "<rect");
        if (selected != NULL) (void)viewer_editor_select_at_source_offset(editor,
            (size_t)(selected - (const char *)selection_source));
    } else if (strcmp(state, "editing-field") == 0) {
        (void)viewer_editor_key(editor, 76U, 0U, NULL, 0U);
    }
    viewer_editor_paint_scalar(editor, rgba, length, (size_t)width * 4U);
    if (node_edit) {
        paint_node_path(rgba, width, height, (size_t)width * 4U,
            layout.canvas, &navigation, tokens.accent);
        viewer_editor_paint_canvas_overlay(editor, &node_document,
            &navigation, layout.canvas, rgba, width, height,
            (size_t)width * 4U);
    }
    bool okay = fwrite(rgba, 1U, length, stdout) == length;
    svg_shape_document_destroy(&node_document);
    viewer_editor_destroy(editor);
    free(rgba);
    return okay ? 0 : 1;
}
