#include "viewer_chrome.h"
#include "viewer_editor_internal.h"
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
    "<defs><linearGradient id='gradient'><stop offset='0' stop-color='red'/></linearGradient></defs>"
    "<g id='group'><rect id='shape' x='1' y='2' width='3' height='4' "
    "fill='#336699' stroke='#112233'/><circle id='accent' cx='8' cy='8' r='3'/>"
    "<path id='curve' d='M1 1 L8 8'/><text id='title' x='1' y='12'>Vexel</text>"
    "<image id='thumb' href='data:image/png;base64,AA==' width='2' height='2'/>"
    "</g></svg>";

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

static bool row_has_icon(viewer_editor *editor,
    const ui_semantics_snapshot *snapshot, const char *label, ui_icon_kind icon)
{
    const ui_semantics_node *row = find_role_label(snapshot, UI_ROLE_ROW, label);
    const ui_node *node = row == NULL ? NULL : ui_tree_find(&editor->tree, row->id);
    return node != NULL && node->spec.icon == icon;
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
    bool handled = (viewer_editor_pointer_down(editor, x, y, 0U) &
        VIEWER_EDITOR_HANDLED) != 0U;
    viewer_editor_pointer_up(editor);
    return handled;
}

static size_t selected_row_count(const ui_semantics_snapshot *snapshot)
{
    size_t count = 0U;
    for (size_t index = 0U; index < snapshot->count; ++index)
        if (snapshot->nodes[index].role == UI_ROLE_ROW &&
            (snapshot->nodes[index].state & UI_STATE_CHECKED) != 0U) ++count;
    return count;
}

static uint8_t *copy_editor_source(viewer_editor *editor, size_t *out_length)
{
    if (editor == NULL || out_length == NULL) return NULL;
    size_t length = viewer_editor_byte_length(editor);
    if (length == SIZE_MAX) return NULL;
    uint8_t *copy = malloc(length + 1U);
    if (copy == NULL || !viewer_editor_materialize(editor, copy, length + 1U, &length)) {
        free(copy);
        return NULL;
    }
    copy[length] = '\0';
    *out_length = length;
    return copy;
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
    if (!viewer_editor_semantics(editor, snapshot) || selected_row_count(snapshot) != 0U ||
        find_label(snapshot, "Name") != NULL) return 30;
    const ui_semantics_node *undo = find_label(snapshot, "Undo · Command-Z");
    const ui_semantics_node *redo = find_label(snapshot, "Redo · Shift-Command-Z");
    const ui_semantics_node *duplicate = find_label(snapshot, "Duplicate · Command-D");
    const ui_semantics_node *delete_button = find_label(snapshot, "Delete · Delete");
    const ui_node *layers_toggle = ui_tree_find(&editor->tree,
        EDITOR_BUTTON_TOGGLE_STRUCTURE);
    const ui_node *inspector_toggle = ui_tree_find(&editor->tree,
        EDITOR_BUTTON_TOGGLE_INSPECTOR);
    if (undo == NULL || redo == NULL || duplicate == NULL || delete_button == NULL ||
        (undo->state & UI_STATE_DISABLED) == 0U ||
        (redo->state & UI_STATE_DISABLED) == 0U ||
        (duplicate->state & UI_STATE_DISABLED) == 0U ||
        (delete_button->state & UI_STATE_DISABLED) == 0U ||
        layers_toggle == NULL || layers_toggle->spec.icon != UI_ICON_LAYERS ||
        inspector_toggle == NULL || inspector_toggle->spec.icon != UI_ICON_SIDEBAR_RIGHT)
        return 30;
    const ui_node *empty_widget = ui_tree_find(&editor->tree, 23000U);
    const ui_node *hud_widget = ui_tree_find(&editor->tree, EDITOR_NODE_CANVAS_HUD + 100U);
    const ui_node *toolbar_divider = ui_tree_find(&editor->tree, 30U);
    const ui_node *rail_divider_one = ui_tree_find(&editor->tree, 33U);
    const ui_node *rail_divider_two = ui_tree_find(&editor->tree, 34U);
    const ui_node *line_tool = ui_tree_find(&editor->tree, EDITOR_BUTTON_TOOL_LINE);
    const ui_node *polyline_tool = ui_tree_find(&editor->tree, EDITOR_BUTTON_TOOL_POLYLINE);
    const ui_node *pen_tool = ui_tree_find(&editor->tree, EDITOR_BUTTON_TOOL_PEN);
    const ui_node *node_tool = ui_tree_find(&editor->tree, EDITOR_BUTTON_TOOL_NODE);
    if (empty_widget == NULL || empty_widget->spec.widget != UI_WIDGET_EMPTY_STATE ||
        empty_widget->bounds.width < 240U || empty_widget->bounds.height < 112U ||
        hud_widget == NULL || hud_widget->spec.widget != UI_WIDGET_HUD_PILL ||
        toolbar_divider == NULL || toolbar_divider->spec.widget != UI_WIDGET_DIVIDER ||
        rail_divider_one == NULL || rail_divider_one->spec.widget != UI_WIDGET_DIVIDER ||
        rail_divider_two == NULL || rail_divider_two->spec.widget != UI_WIDGET_DIVIDER ||
        line_tool == NULL || polyline_tool == NULL || pen_tool == NULL || node_tool == NULL ||
        line_tool->spec.icon == polyline_tool->spec.icon ||
        pen_tool->spec.icon == node_tool->spec.icon)
        return 30;
    const ui_semantics_node *empty = find_label(snapshot, "No layer selected");
    const char *empty_message = NULL;
    size_t empty_message_length = 0U;
    if (empty == NULL || !read_text(snapshot, empty, true,
            &empty_message, &empty_message_length) ||
        empty_message_length != sizeof("Select a layer to edit its properties") - 1U ||
        memcmp(empty_message, "Select a layer to edit its properties",
            empty_message_length) != 0) return 30;

    const ui_semantics_node *document = find_role_label(snapshot, UI_ROLE_ROW, "Document");
    if (document == NULL || !click_node(editor, document, document->frame.width / 2U) ||
        !viewer_editor_semantics(editor, snapshot) || selected_row_count(snapshot) != 1U ||
        find_label(snapshot, "Name") == NULL || find_label(snapshot, "Other attributes") == NULL ||
        find_label(snapshot, "Xmlns") != NULL || find_label(snapshot, "Data-long") != NULL)
        return 31;

    const ui_semantics_node *other = find_label(snapshot, "Other attributes");
    if (other == NULL || !click_node(editor, other, other->frame.width / 2U) ||
        !viewer_editor_semantics(editor, snapshot)) return 32;
    const ui_semantics_node *attribute = find_label(snapshot, "Data-long");
    const char *full_value = NULL;
    size_t full_value_length = 0U;
    if (attribute == NULL || find_label(snapshot, "Xmlns") == NULL ||
        attribute->value_len != UI_TEXT_MAX_BYTES - 1U ||
        attribute->value_full_len <= 64U ||
        !ui_semantics_text(snapshot, attribute, true, &full_value, &full_value_length) ||
        full_value_length != attribute->value_full_len || full_value_length <= 64U ||
        memcmp(full_value, "0123456789abcdefghijklmnopqrstuvwxyz", 36U) != 0) return 33;

    document = find_role_label(snapshot, UI_ROLE_ROW, "Document");
    if (document == NULL || !click_node(editor, document, 8U) ||
        !viewer_editor_semantics(editor, snapshot) ||
        find_role_label(snapshot, UI_ROLE_ROW, "Rectangle #shape") != NULL) return 34;
    document = find_role_label(snapshot, UI_ROLE_ROW, "Document");
    if (document == NULL || !click_node(editor, document, 8U) ||
        !viewer_editor_semantics(editor, snapshot) ||
        find_role_label(snapshot, UI_ROLE_ROW, "Linear Gradient #gradient") == NULL ||
        find_role_label(snapshot, UI_ROLE_ROW, "Rectangle #shape") == NULL) return 35;
    if (!row_has_icon(editor, snapshot, "Document", UI_ICON_DOCUMENT) ||
        !row_has_icon(editor, snapshot, "Linear Gradient #gradient", UI_ICON_GRADIENT) ||
        !row_has_icon(editor, snapshot, "Stop", UI_ICON_STOP) ||
        !row_has_icon(editor, snapshot, "Group #group", UI_ICON_GROUP) ||
        !row_has_icon(editor, snapshot, "Rectangle #shape", UI_ICON_RECTANGLE) ||
        !row_has_icon(editor, snapshot, "Circle #accent", UI_ICON_CIRCLE) ||
        !row_has_icon(editor, snapshot, "Path #curve", UI_ICON_PATH) ||
        !row_has_icon(editor, snapshot, "Text #title", UI_ICON_TEXT) ||
        !row_has_icon(editor, snapshot, "Image #thumb", UI_ICON_IMAGE)) return 35;

    const ui_semantics_node *definitions = find_role_label(snapshot,
        UI_ROLE_ROW, "Definitions");
    if (definitions == NULL) return 36;
    uint32_t toggle_x = definitions->frame.x + definitions->frame.width - 27U;
    uint32_t toggle_y = definitions->frame.y + definitions->frame.height / 2U;
    if ((viewer_editor_pointer_down(editor, toggle_x, toggle_y, 0U) &
            VIEWER_EDITOR_HANDLED) == 0U) return 37;
    viewer_editor_pointer_up(editor);
    size_t source_length = 0U;
    uint8_t *source_copy = copy_editor_source(editor, &source_length);
    uint8_t preview[sizeof(source) + 4096U];
    size_t preview_length = 0U;
    if (source_copy == NULL || source_length != sizeof(source) - 1U ||
        memcmp(source_copy, source, source_length) != 0 ||
        viewer_editor_is_dirty(editor) ||
        !viewer_editor_materialize_preview(editor, preview, sizeof(preview),
            &preview_length) || preview_length >= sizeof(preview))
        { free(source_copy); return 38; }
    preview[preview_length] = '\0';
    if (strstr((const char *)preview, "display=\"none\"") == NULL)
        { free(source_copy); return 38; }
    free(source_copy);

    if (!viewer_editor_semantics(editor, snapshot)) return 39;
    definitions = find_role_label(snapshot, UI_ROLE_ROW, "Definitions");
    if (definitions == NULL || !click_node(editor, definitions,
            definitions->frame.width - 27U)) return 39;
    viewer_editor_pointer_up(editor);
    source_copy = copy_editor_source(editor, &source_length);
    if (source_copy == NULL || source_length != sizeof(source) - 1U ||
        memcmp(source_copy, source, source_length) != 0 ||
        viewer_editor_is_dirty(editor) ||
        !viewer_editor_materialize_preview(editor, preview, sizeof(preview),
            &preview_length) || preview_length >= sizeof(preview))
        { free(source_copy); return 40; }
    preview[preview_length] = '\0';
    if (strstr((const char *)preview, "display=\"none\"") != NULL)
        { free(source_copy); return 40; }
    free(source_copy);

    const ui_semantics_node *search = find_label(snapshot, "Search layers");
    if (!viewer_editor_semantics(editor, snapshot) || search == NULL ||
        !click_node(editor, search, search->frame.width / 2U) ||
        !send_text(editor, "shape") || !viewer_editor_semantics(editor, snapshot) ||
        find_role_label(snapshot, UI_ROLE_ROW, "Rectangle #shape") == NULL ||
        find_role_label(snapshot, UI_ROLE_ROW, "Document") != NULL) return 41;
    if ((viewer_editor_key(editor, 36U, 0U, NULL, 0U) & VIEWER_EDITOR_HANDLED) == 0U ||
        !viewer_editor_semantics(editor, snapshot)) return 42;
    search = find_label(snapshot, "Search layers");
    if (search == NULL || !click_node(editor, search, search->frame.width / 2U) ||
        (viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "a", 1U) &
            VIEWER_EDITOR_HANDLED) == 0U ||
        (viewer_editor_key(editor, 51U, 0U, NULL, 0U) & VIEWER_EDITOR_HANDLED) == 0U ||
        (viewer_editor_key(editor, 36U, 0U, NULL, 0U) & VIEWER_EDITOR_HANDLED) == 0U ||
        !viewer_editor_semantics(editor, snapshot)) return 43;

    const ui_semantics_node *rectangle = find_role_label(snapshot,
        UI_ROLE_ROW, "Rectangle #shape");
    if (rectangle == NULL || !click_node(editor, rectangle,
            rectangle->frame.width / 2U) ||
        !viewer_editor_semantics(editor, snapshot) || selected_row_count(snapshot) != 1U)
        return 44;
    rectangle = find_role_label(snapshot, UI_ROLE_ROW, "Rectangle #shape");
    const ui_semantics_node *geometry = find_label(snapshot, "Geometry");
    const ui_semantics_node *appearance = find_label(snapshot, "Appearance");
    if (rectangle == NULL || geometry == NULL || appearance == NULL ||
        find_label(snapshot, "Name") == NULL || find_label(snapshot, "Fill") == NULL ||
        find_label(snapshot, "Stroke") == NULL || find_label(snapshot, "Id") != NULL ||
        find_label(snapshot, "Content") != NULL) return 45;

    if ((viewer_editor_key(editor, 120U, 0U, NULL, 0U) & VIEWER_EDITOR_HANDLED) == 0U ||
        (viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "a", 1U) &
            VIEWER_EDITOR_HANDLED) == 0U ||
        !send_text(editor, "renamed-shape") ||
        (viewer_editor_key(editor, 36U, 0U, NULL, 0U) & VIEWER_EDITOR_MUTATED) == 0U)
        return 46;
    source_copy = copy_editor_source(editor, &source_length);
    if (source_copy == NULL || strstr((const char *)source_copy,
            "id='renamed-shape'") == NULL) { free(source_copy); return 47; }
    free(source_copy);
    if (!viewer_editor_semantics(editor, snapshot)) return 48;
    rectangle = find_role_label(snapshot, UI_ROLE_ROW, "Rectangle #renamed-shape");
    if (rectangle == NULL) return 49;

    viewer_editor_pointer_move(editor, 0U, 0U);
    if (!viewer_editor_semantics(editor, snapshot)) return 50;
    rectangle = find_role_label(snapshot, UI_ROLE_ROW, "Rectangle #renamed-shape");
    toggle_x = rectangle->frame.x + rectangle->frame.width - 27U;
    toggle_y = rectangle->frame.y + rectangle->frame.height / 2U;
    if ((viewer_editor_pointer_down(editor, toggle_x, toggle_y, 0U) &
            VIEWER_EDITOR_HANDLED) == 0U) return 51;
    viewer_editor_pointer_up(editor);
    source_copy = copy_editor_source(editor, &source_length);
    if (source_copy == NULL || strstr((const char *)source_copy,
            "id='renamed-shape'") == NULL ||
        strstr((const char *)source_copy, "display=\"none\"") != NULL ||
        viewer_editor_is_dirty(editor) == false)
        { free(source_copy); return 52; }
    free(source_copy);

    if (!viewer_editor_semantics(editor, snapshot)) return 53;
    rectangle = find_role_label(snapshot, UI_ROLE_ROW, "Rectangle #renamed-shape");
    if (rectangle == NULL || !click_node(editor, rectangle,
            rectangle->frame.width - 27U)) return 53;
    viewer_editor_pointer_up(editor);
    source_copy = copy_editor_source(editor, &source_length);
    if (source_copy == NULL || strstr((const char *)source_copy,
            "id='renamed-shape'") == NULL ||
        strstr((const char *)source_copy, "display=\"none\"") != NULL)
        { free(source_copy); return 54; }
    free(source_copy);
    if (!viewer_editor_semantics(editor, snapshot)) return 55;
    rectangle = find_role_label(snapshot, UI_ROLE_ROW, "Rectangle #renamed-shape");
    if (rectangle == NULL) return 56;
    toggle_x = rectangle->frame.x + rectangle->frame.width - 27U;
    toggle_y = rectangle->frame.y + rectangle->frame.height / 2U;
    if ((viewer_editor_pointer_down(editor, toggle_x, toggle_y, 0U) &
            VIEWER_EDITOR_HANDLED) == 0U) return 57;
    viewer_editor_pointer_up(editor);
    if (!viewer_editor_semantics(editor, snapshot)) return 58;
    rectangle = find_role_label(snapshot, UI_ROLE_ROW, "Rectangle #renamed-shape");
    const char *value = NULL;
    size_t value_length = 0U;
    if (rectangle == NULL || (rectangle->state & UI_STATE_CHECKED) == 0U ||
        !read_text(snapshot, rectangle, true, &value, &value_length) ||
        value_length != 6U || memcmp(value, "Hidden", 6U) != 0) return 59;

    size_t before_lock_length = 0U;
    uint8_t *before_lock = copy_editor_source(editor, &before_lock_length);
    if (before_lock == NULL) return 60;
    toggle_x = rectangle->frame.x + rectangle->frame.width - 11U;
    toggle_y = rectangle->frame.y + rectangle->frame.height / 2U;
    if ((viewer_editor_pointer_down(editor, toggle_x, toggle_y, 0U) &
            VIEWER_EDITOR_HANDLED) == 0U) { free(before_lock); return 61; }
    viewer_editor_pointer_up(editor);
    size_t after_lock_length = 0U;
    source_copy = copy_editor_source(editor, &after_lock_length);
    if (source_copy == NULL || after_lock_length != before_lock_length ||
        memcmp(source_copy, before_lock, before_lock_length) != 0) {
        free(before_lock); free(source_copy); return 62;
    }
    free(before_lock);
    free(source_copy);
    if (!viewer_editor_semantics(editor, snapshot)) return 63;
    rectangle = find_role_label(snapshot, UI_ROLE_ROW, "Rectangle #renamed-shape");
    if (rectangle == NULL || !read_text(snapshot, rectangle, true, &value, &value_length) ||
        value_length != sizeof("Hidden · Locked") - 1U ||
        memcmp(value, "Hidden · Locked", sizeof("Hidden · Locked") - 1U) != 0)
        return 64;

    (void)viewer_editor_key(editor, 120U, 0U, NULL, 0U);
    (void)viewer_editor_key(editor, 0U, PLATFORM_MODIFIER_COMMAND, "a", 1U);
    bool text_sent = send_text(editor, "blocked-name");
    uint32_t blocked_commit = viewer_editor_key(editor, 36U, 0U, NULL, 0U);
    if (!text_sent || (blocked_commit & VIEWER_EDITOR_MUTATED) != 0U) return 65;
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
    const ui_semantics_node *status_text = find_label(snapshot, "Ready · Saved");
    const ui_semantics_node *selection_summary = find_label(snapshot, "No selection");
    if (selected_row_count(snapshot) != 0U || status_text == NULL ||
        status_text->frame.height < 16U ||
        status_text->frame.y + status_text->frame.height > 800U ||
        selection_summary == NULL || status_text->frame.x != 12U ||
        selection_summary->frame.x + selection_summary->frame.width != 1268U) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 3;
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
        find_label(snapshot,
            "125% · 10.5, 20.5 · 64 × 64 · Rendering 2/4") == NULL) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 15;
    }
    const ui_semantics_node *tool = find_label(snapshot, "Polygon tool · Y");
    if (tool == NULL || tool->frame.width != 32U || tool->frame.height != 32U ||
        !click_node(editor, tool, tool->frame.width / 2U) ||
        viewer_editor_get_draw_tool(editor) != VIEWER_EDITOR_TOOL_POLYGON ||
        !viewer_editor_semantics(editor, snapshot)) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 25;
    }
    tool = find_label(snapshot, "Polygon tool · Y");
    if (tool == NULL || (tool->state & UI_STATE_CHECKED) == 0U) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 26;
    }
    tool = find_label(snapshot, "Polyline tool · U");
    if (tool == NULL || !click_node(editor, tool, tool->frame.width / 2U) ||
        viewer_editor_get_draw_tool(editor) != VIEWER_EDITOR_TOOL_POLYLINE ||
        !viewer_editor_semantics(editor, snapshot)) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 27;
    }
    tool = find_label(snapshot, "Node edit tool · N");
    if (tool == NULL || !click_node(editor, tool, tool->frame.width / 2U) ||
        viewer_editor_get_draw_tool(editor) != VIEWER_EDITOR_TOOL_PEN ||
        !viewer_editor_semantics(editor, snapshot)) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 28;
    }
    tool = find_label(snapshot, "Node edit tool · N");
    if (tool == NULL || (tool->state & UI_STATE_CHECKED) == 0U) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 32;
    }
    if ((viewer_editor_key(editor, 0U, 0U, "n", 1U) & VIEWER_EDITOR_HANDLED) == 0U) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 33;
    }
    if (!viewer_editor_semantics(editor, snapshot)) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 34;
    }
    tool = find_label(snapshot, "Node edit tool · N");
    if (tool == NULL || (tool->state & UI_STATE_CHECKED) == 0U) {
        (void)fprintf(stderr, "node shortcut state: tool=%p state=%u active=%d\n",
            (const void *)tool, tool == NULL ? 0U : tool->state, editor->node_edit_active);
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 35;
    }
    tool = find_label(snapshot, "Pen tool · P");
    if (tool == NULL || !click_node(editor, tool, tool->frame.width / 2U) ||
        !viewer_editor_semantics(editor, snapshot)) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 30;
    }
    tool = find_label(snapshot, "Pen tool · P");
    if (tool == NULL || (tool->state & UI_STATE_CHECKED) == 0U) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 31;
    }
    if (!viewer_editor_set_appearance(editor, UI_THEME_LIGHT, true, true) ||
        !editor->increased_contrast || !editor->reduced_motion ||
        !viewer_editor_set_tool(editor, UI_ICON_PAN)) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 4;
    }
    if (!viewer_editor_take_redraw_request(editor) ||
        !viewer_editor_semantics(editor, snapshot)) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 16;
    }
    const ui_semantics_node *pan_tool = find_label(snapshot, "Pan tool · H / Space");
    if (pan_tool == NULL) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 17;
    }
    if (pan_tool->frame.width != 32U || pan_tool->frame.height != 32U) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 24;
    }
    uint32_t pan_x = pan_tool->frame.x + pan_tool->frame.width / 2U;
    uint32_t pan_y = pan_tool->frame.y + pan_tool->frame.height / 2U;
    viewer_editor_pointer_move(editor, pan_x, pan_y);
    if (!viewer_editor_take_redraw_request(editor) ||
        !viewer_editor_semantics(editor, snapshot)) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 18;
    }
    pan_tool = find_label(snapshot, "Pan tool · H / Space");
    if (pan_tool == NULL || (pan_tool->state & UI_STATE_HOVERED) == 0U) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 19;
    }
    uint32_t tooltip_x = EDITOR_TOOL_RAIL_WIDTH + EDITOR_LEFT_WIDTH + 8U;
    uint32_t tooltip_y = pan_tool->frame.y + pan_tool->frame.height + 4U;
    size_t tooltip_sample = ((size_t)(tooltip_y + 14U) * 1280U +
        tooltip_x + 8U) * 4U;
    memset(pixels, 242, pixel_bytes);
    for (size_t at = 3U; at < pixel_bytes; at += 4U) pixels[at] = 255U;
    viewer_editor_paint(editor, pixels, pixel_bytes, (size_t)1280U * 4U);
    uint8_t tooltip_rgb[3] = {pixels[tooltip_sample], pixels[tooltip_sample + 1U],
        pixels[tooltip_sample + 2U]};
    viewer_editor_pointer_move(editor, 0U, 0U);
    memset(pixels, 242, pixel_bytes);
    for (size_t at = 3U; at < pixel_bytes; at += 4U) pixels[at] = 255U;
    viewer_editor_paint(editor, pixels, pixel_bytes, (size_t)1280U * 4U);
    if (tooltip_rgb[0] == pixels[tooltip_sample] &&
        tooltip_rgb[1] == pixels[tooltip_sample + 1U] &&
        tooltip_rgb[2] == pixels[tooltip_sample + 2U]) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 67;
    }
    viewer_editor_pointer_move(editor, pan_x, pan_y);
    uint32_t pressed = viewer_editor_pointer_down(editor, pan_x, pan_y, 0U);
    if ((pressed & VIEWER_EDITOR_HANDLED) == 0U ||
        !viewer_editor_take_redraw_request(editor) ||
        !viewer_editor_semantics(editor, snapshot)) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 20;
    }
    pan_tool = find_label(snapshot, "Pan tool · H / Space");
    if (pan_tool == NULL || (pan_tool->state & UI_STATE_PRESSED) == 0U) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 21;
    }
    viewer_editor_pointer_up(editor);
    if (!viewer_editor_take_redraw_request(editor) ||
        !viewer_editor_semantics(editor, snapshot)) {
        free(pixels); free(snapshot); viewer_editor_destroy(editor); return 22;
    }
    pan_tool = find_label(snapshot, "Pan tool · H / Space");
    if (pan_tool == NULL || (pan_tool->state & UI_STATE_PRESSED) != 0U) {
        (void)fprintf(stderr, "pan release state: node=%p state=%u\n",
            (const void *)pan_tool, pan_tool == NULL ? 0U : pan_tool->state);
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
