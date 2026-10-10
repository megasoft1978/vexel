#include "viewer_chrome.h"
#include "viewer_editor.h"
#include "platform/platform_window.h"

#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static const uint8_t source[] =
    "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='64' "
    "data-long='0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
    "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789'>"
    "<rect id='shape' x='1' y='2' width='3' height='4' "
    "fill='#336699' stroke='#112233'/></svg>";

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

static const ui_semantics_node *find_role_label(const ui_semantics_snapshot *snapshot,
    ui_role role, const char *label)
{
    size_t length = strlen(label);
    for (size_t index = 0U; index < snapshot->count; ++index) {
        const ui_semantics_node *node = &snapshot->nodes[index];
        const char *full = NULL;
        size_t full_length = 0U;
        if (node->role == role && ui_semantics_text(snapshot, node, false,
                &full, &full_length) && full_length == length &&
            memcmp(full, label, length) == 0) return node;
    }
    return NULL;
}

static bool read_text(const ui_semantics_snapshot *snapshot,
                      const ui_semantics_node *node, bool value,
                      const char **out_text, size_t *out_length)
{
    return node != NULL && ui_semantics_text(snapshot, node, value,
        out_text, out_length);
}

static bool click_node(viewer_editor *editor, const ui_semantics_node *node,
                       uint32_t offset_x)
{
    if (editor == NULL || node == NULL || offset_x >= node->frame.width) return false;
    uint32_t x = node->frame.x + offset_x;
    uint32_t y = node->frame.y + node->frame.height / 2U;
    (void)viewer_editor_pointer_move(editor, x, y);
    return (viewer_editor_pointer_down(editor, x, y, 0U) & VIEWER_EDITOR_HANDLED) != 0U;
}

static bool send_text(viewer_editor *editor, const char *text)
{
    for (size_t index = 0U; text[index] != '\0'; ++index)
        if ((viewer_editor_key(editor, 0U, 0U, text + index, 1U) &
                VIEWER_EDITOR_HANDLED) == 0U) return false;
    return true;
}

static int test_layer_tree(viewer_editor *editor, ui_semantics_snapshot *snapshot)
{
    if (!viewer_editor_semantics(editor, snapshot)) return 30;
    const ui_semantics_node *document = find_label(snapshot, "Document");
    if (document == NULL || !click_node(editor, document, 8U) ||
        !viewer_editor_semantics(editor, snapshot) ||
        find_label(snapshot, "Rectangle #shape") != NULL) return 31;
    document = find_label(snapshot, "Document");
    if (document == NULL || !click_node(editor, document, 8U) ||
        !viewer_editor_semantics(editor, snapshot) ||
        find_role_label(snapshot, UI_ROLE_ROW, "Rectangle #shape") == NULL) return 32;

    const ui_semantics_node *search = find_label(snapshot, "Search layers");
    if (search == NULL) return 33;
    if (!click_node(editor, search, search->frame.width / 2U)) return 34;
    if (!send_text(editor, "shape")) return 35;
    if (!viewer_editor_semantics(editor, snapshot)) return 36;
    if (find_role_label(snapshot, UI_ROLE_ROW, "Rectangle #shape") == NULL) {
        return 37;
    }
    if (find_role_label(snapshot, UI_ROLE_ROW, "Document") != NULL) return 38;
    if ((viewer_editor_key(editor, 36U, 0U, NULL, 0U) & VIEWER_EDITOR_HANDLED) == 0U)
        return 34;
    if (!viewer_editor_semantics(editor, snapshot)) return 35;
    search = find_label(snapshot, "Search layers");
    if (search == NULL || !click_node(editor, search, search->frame.width / 2U) ||
        (viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "a", 1U) &
            VIEWER_EDITOR_HANDLED) == 0U ||
        (viewer_editor_key(editor, 51U, 0U, NULL, 0U) &
            VIEWER_EDITOR_HANDLED) == 0U ||
        (viewer_editor_key(editor, 36U, 0U, NULL, 0U) &
            VIEWER_EDITOR_HANDLED) == 0U ||
        !viewer_editor_semantics(editor, snapshot)) return 36;

    const ui_semantics_node *rectangle = find_role_label(snapshot, UI_ROLE_ROW,
        "Rectangle #shape");
    if (rectangle == NULL) return 37;
    uint32_t row_x = rectangle->frame.x + rectangle->frame.width / 2U;
    uint32_t row_y = rectangle->frame.y + rectangle->frame.height / 2U;
    (void)viewer_editor_pointer_move(editor, row_x, row_y);
    if ((viewer_editor_pointer_down(editor, row_x, row_y, 0U) &
            VIEWER_EDITOR_HANDLED) == 0U || !viewer_editor_semantics(editor, snapshot))
        return 37;
    rectangle = find_role_label(snapshot, UI_ROLE_ROW, "Rectangle #shape");
    const ui_semantics_node *geometry = find_label(snapshot, "Geometry");
    const ui_semantics_node *appearance = find_label(snapshot, "Appearance");
    if (rectangle == NULL || geometry == NULL || appearance == NULL ||
        find_label(snapshot, "Name") == NULL || find_label(snapshot, "Fill") == NULL ||
        find_label(snapshot, "Id") != NULL || find_label(snapshot, "Content") != NULL)
        return 37;
    if ((viewer_editor_key(editor, 120U, 0U, NULL, 0U) &
            VIEWER_EDITOR_HANDLED) == 0U ||
        (viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "a", 1U) &
            VIEWER_EDITOR_HANDLED) == 0U ||
        !send_text(editor, "renamed-shape") ||
        (viewer_editor_key(editor, 36U, 0U, NULL, 0U) &
            VIEWER_EDITOR_MUTATED) == 0U) return 38;
    size_t source_length = viewer_editor_byte_length(editor);
    uint8_t *source_copy = malloc(source_length + 1U);
    if (source_copy == NULL || !viewer_editor_materialize(editor, source_copy,
            source_length + 1U, &source_length)) {
        free(source_copy);
        return 39;
    }
    source_copy[source_length] = '\0';
    if (strstr((const char *)source_copy, "id='renamed-shape'") == NULL) {
        free(source_copy);
        return 39;
    }
    free(source_copy);

    if (!viewer_editor_semantics(editor, snapshot)) return 40;
    rectangle = find_label(snapshot, "Rectangle #renamed-shape");
    if (rectangle == NULL) return 41;
    row_x = rectangle->frame.x + rectangle->frame.width - 27U;
    row_y = rectangle->frame.y + rectangle->frame.height / 2U;
    (void)viewer_editor_pointer_move(editor, row_x, row_y);
    if ((viewer_editor_pointer_down(editor, row_x, row_y, 0U) &
            VIEWER_EDITOR_HANDLED) == 0U) return 42;
    source_length = viewer_editor_byte_length(editor);
    source_copy = malloc(source_length + 1U);
    if (source_copy == NULL || !viewer_editor_materialize(editor, source_copy,
            source_length + 1U, &source_length)) {
        free(source_copy);
        return 43;
    }
    source_copy[source_length] = '\0';
    if (strstr((const char *)source_copy, "display=\"none\"") == NULL) {
        free(source_copy);
        return 43;
    }
    free(source_copy);
    if (!viewer_editor_semantics(editor, snapshot)) return 44;
    rectangle = find_label(snapshot, "Rectangle #renamed-shape");
    const char *value = NULL;
    size_t value_length = 0U;
    if (rectangle == NULL || (rectangle->state & UI_STATE_CHECKED) == 0U ||
        !read_text(snapshot, rectangle, true, &value, &value_length) ||
        value_length != 6U || memcmp(value, "Hidden", 6U) != 0) return 45;

    row_x = rectangle->frame.x + rectangle->frame.width - 11U;
    row_y = rectangle->frame.y + rectangle->frame.height / 2U;
    (void)viewer_editor_pointer_move(editor, row_x, row_y);
    if ((viewer_editor_pointer_down(editor, row_x, row_y, 0U) &
            VIEWER_EDITOR_HANDLED) == 0U || !viewer_editor_semantics(editor, snapshot))
        return 46;
    rectangle = find_label(snapshot, "Rectangle #renamed-shape");
    if (rectangle == NULL || (rectangle->state & UI_STATE_CHECKED) == 0U ||
        !read_text(snapshot, rectangle, true, &value, &value_length) ||
        value_length != sizeof("Hidden · Locked") - 1U ||
        memcmp(value, "Hidden · Locked", sizeof("Hidden · Locked") - 1U) != 0) return 47;

    (void)viewer_editor_key(editor, 120U, 0U, NULL, 0U);
    (void)viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "a", 1U);
    bool text_sent = send_text(editor, "blocked-name");
    uint32_t blocked_commit = viewer_editor_key(editor, 36U, 0U, NULL, 0U);
    if (!text_sent || (blocked_commit & VIEWER_EDITOR_MUTATED) != 0U) {
        return 48;
    }
    (void)viewer_editor_key(editor, 53U, 0U, NULL, 0U);
    return 0;
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
    const ui_semantics_node *attribute = find_label(snapshot, "Data-long");
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
    int tree_result = test_layer_tree(editor, snapshot);
    if (tree_result != 0) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return tree_result;
    }
    (void)viewer_editor_take_redraw_request(editor);
    viewer_editor_set_canvas_status(editor, 1.25, true, 10.5, 20.5,
        64U, 64U, 2U, 4U, true);
    if (!viewer_editor_take_redraw_request(editor) ||
        !viewer_editor_semantics(editor, snapshot) ||
        find_label(snapshot, "125%  ·  10.5, 20.5") == NULL ||
        find_label(snapshot, "64 × 64") == NULL ||
        find_label(snapshot, "Rendering sharp detail · 2 of 4") == NULL) {
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
    const ui_semantics_node *pan_tool = find_label(snapshot, "Pan tool · Space");
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
    pan_tool = find_label(snapshot, "Pan tool · Space");
    if (pan_tool == NULL || (pan_tool->state & UI_STATE_HOVERED) == 0U) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 19;
    }
    uint32_t pressed = viewer_editor_pointer_down(editor, pan_x, pan_y, 0U);
    if ((pressed & VIEWER_EDITOR_HANDLED) == 0U ||
        !viewer_editor_take_redraw_request(editor) ||
        !viewer_editor_semantics(editor, snapshot)) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 20;
    }
    pan_tool = find_label(snapshot, "Pan tool · Space");
    if (pan_tool == NULL || (pan_tool->state & UI_STATE_PRESSED) == 0U) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 21;
    }
    viewer_editor_pointer_up(editor);
    if (!viewer_editor_take_redraw_request(editor) ||
        !viewer_editor_semantics(editor, snapshot)) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 22;
    }
    pan_tool = find_label(snapshot, "Pan tool · Space");
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
