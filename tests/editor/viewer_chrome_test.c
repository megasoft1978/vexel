#include "viewer_chrome.h"
#include "viewer_editor.h"

#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static const uint8_t source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
    "data-long='0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
    "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'>"
    "<rect id='shape' x='1' y='2' width='3' height='4'/></svg>";

static const ui_semantics_node *find_label(const ui_semantics_snapshot *snapshot,
                                           const char *label)
{
    size_t length = strlen(label);
    for (size_t index = 0U; index < snapshot->count; ++index) {
        const ui_semantics_node *node = &snapshot->nodes[index];
        const char *full = NULL;
        size_t full_length = 0U;
        if (ui_semantics_text(snapshot, node, false, &full, &full_length) &&
            full_length == length && memcmp(full, label, length) == 0) return node;
    }
    return NULL;
}

int main(void)
{
    viewer_editor *editor = viewer_editor_create_styled(
        (base_bytes){source, sizeof(source) - 1U}, UI_THEME_DARK,
        1280U, 800U, true);
    if (editor == NULL) return 1;
    ui_semantics_snapshot *snapshot = malloc(sizeof(*snapshot));
    size_t pixel_bytes = (size_t)1280U * 800U * 4U;
    uint8_t *pixels = malloc(pixel_bytes);
    if (snapshot == NULL || pixels == NULL) {
        free(snapshot); free(pixels); viewer_editor_destroy(editor); return 1;
    }
    memset(pixels, 23, pixel_bytes);
    for (size_t at = 3U; at < pixel_bytes; at += 4U) pixels[at] = 255U;
    viewer_editor_paint_scalar(editor, pixels, pixel_bytes, (size_t)1280U * 4U);
    uint8_t dark_button[3] = {pixels[(10U * 1280U + 10U) * 4U],
        pixels[(10U * 1280U + 10U) * 4U + 1U],
        pixels[(10U * 1280U + 10U) * 4U + 2U]};
    if (!viewer_editor_semantics(editor, snapshot)) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 2;
    }
    const ui_semantics_node *attribute = find_label(snapshot, "data-long");
    const char *full_value = NULL;
    size_t full_value_length = 0U;
    if (attribute == NULL || attribute->value_len != UI_TEXT_MAX_BYTES - 1U ||
        attribute->value_full_len <= 64U ||
        !ui_semantics_text(snapshot, attribute, true, &full_value, &full_value_length) ||
        full_value_length != attribute->value_full_len || full_value_length <= 64U ||
        memcmp(full_value, "0123456789abcdefghijklmnopqrstuvwxyz", 36U) != 0) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 3;
    }
    bool has_selected_outline_row = false;
    for (size_t index = 0U; index < snapshot->count; ++index) {
        if (snapshot->nodes[index].role == UI_ROLE_ROW &&
            (snapshot->nodes[index].state & UI_STATE_CHECKED) != 0U)
            has_selected_outline_row = true;
    }
    if (!has_selected_outline_row) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 10;
    }
    (void)viewer_editor_take_redraw_request(editor);
    viewer_editor_set_canvas_status(editor, 1.25, true, 10.5, 20.5,
        64U, 64U, 2U, 4U, true);
    if (!viewer_editor_take_redraw_request(editor) ||
        !viewer_editor_semantics(editor, snapshot) ||
        find_label(snapshot, "125% · 10.5, 20.5 · 64 × 64") == NULL ||
        find_label(snapshot, "Tiles 2 / 4 · rendering") == NULL) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 15;
    }
    if (!viewer_editor_set_appearance(editor, UI_THEME_LIGHT, true, true) ||
        !viewer_editor_set_tool(editor, UI_ICON_PAN)) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 4;
    }
    if (!viewer_editor_take_redraw_request(editor) ||
        !viewer_editor_semantics(editor, snapshot)) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 16;
    }
    const ui_semantics_node *pan_tool = find_label(snapshot, "Pan tool");
    if (pan_tool == NULL) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 17;
    }
    uint32_t pan_x = pan_tool->frame.x + pan_tool->frame.width / 2U;
    uint32_t pan_y = pan_tool->frame.y + pan_tool->frame.height / 2U;
    viewer_editor_pointer_move(editor, pan_x, pan_y);
    if (!viewer_editor_take_redraw_request(editor) ||
        !viewer_editor_semantics(editor, snapshot)) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 18;
    }
    pan_tool = find_label(snapshot, "Pan tool");
    if (pan_tool == NULL || (pan_tool->state & UI_STATE_HOVERED) == 0U) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 19;
    }
    uint32_t pressed = viewer_editor_pointer_down(editor, pan_x, pan_y, 0U);
    if ((pressed & VIEWER_EDITOR_HANDLED) == 0U ||
        !viewer_editor_take_redraw_request(editor) ||
        !viewer_editor_semantics(editor, snapshot)) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 20;
    }
    pan_tool = find_label(snapshot, "Pan tool");
    if (pan_tool == NULL || (pan_tool->state & UI_STATE_PRESSED) == 0U) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 21;
    }
    viewer_editor_pointer_up(editor);
    if (!viewer_editor_take_redraw_request(editor) ||
        !viewer_editor_semantics(editor, snapshot)) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 22;
    }
    pan_tool = find_label(snapshot, "Pan tool");
    if (pan_tool == NULL || (pan_tool->state & UI_STATE_PRESSED) != 0U) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 23;
    }
    memset(pixels, 242, pixel_bytes);
    for (size_t at = 3U; at < pixel_bytes; at += 4U) pixels[at] = 255U;
    viewer_editor_paint_scalar(editor, pixels, pixel_bytes, (size_t)1280U * 4U);
    if (dark_button[0] == pixels[(10U * 1280U + 10U) * 4U] &&
        dark_button[1] == pixels[(10U * 1280U + 10U) * 4U + 1U] &&
        dark_button[2] == pixels[(10U * 1280U + 10U) * 4U + 2U]) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 13;
    }
    if (!viewer_editor_semantics(editor, snapshot)) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 14;
    }
    viewer_editor_set_status(editor, "Unsupported SVG content · preview retained");
    if (!viewer_editor_semantics(editor, snapshot)) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 8;
    }
    bool error_status = false;
    for (size_t index = 0U; index < snapshot->count; ++index) {
        if (snapshot->nodes[index].role == UI_ROLE_STATUS &&
            (snapshot->nodes[index].state & UI_STATE_INVALID) != 0U)
            error_status = true;
    }
    if (!error_status) { free(pixels); free(snapshot); viewer_editor_destroy(editor); return 9; }
    viewer_chrome_layout layout;
    if (!viewer_editor_get_chrome_layout(editor, &layout) ||
        !layout.structure_visible || !layout.inspector_visible ||
        !viewer_editor_set_sidebar_visibility(editor, false, true) ||
        !viewer_editor_get_chrome_layout(editor, &layout) ||
        layout.structure_visible || !layout.inspector_visible) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 5;
    }
    viewer_editor_resize(editor, 300U, 500U, 1.0);
    if (!viewer_editor_get_chrome_layout(editor, &layout) ||
        layout.structure_visible || layout.inspector_visible ||
        layout.canvas.width == 0U || layout.canvas.x + layout.canvas.width != 300U) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 6;
    }
    viewer_editor_resize(editor, 700U, 500U, 1.0);
    if (!viewer_editor_set_sidebar_visibility(editor, true, true) ||
        !viewer_editor_get_chrome_layout(editor, &layout) ||
        !layout.structure_visible || layout.inspector_visible) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 11;
    }
    viewer_editor_resize(editor, 500U, 500U, 1.0);
    if (!viewer_editor_get_chrome_layout(editor, &layout) ||
        layout.structure_visible || layout.inspector_visible || layout.canvas.width == 0U) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 12;
    }
    viewer_editor_resize(editor, 2560U, 1600U, 2.0);
    if (!viewer_editor_get_chrome_layout(editor, &layout) ||
        !layout.structure_visible || !layout.inspector_visible ||
        layout.rail.width != 88U || layout.structure.width != 464U ||
        layout.inspector.width != 640U) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 7;
    }
    free(pixels);
    free(snapshot);
    viewer_editor_destroy(editor);
    (void)puts("editor chrome tests passed: full semantics, injected appearance, responsive panels");
    return 0;
}
