#include "viewer_editor_internal.h"

#include "viewer_chrome.h"
#include "document_model.h"

#include <math.h>
#include <string.h>

static size_t editor_full_attribute_value(viewer_editor *editor,
                                          document_source_span span,
                                          const char **out_value)
{
    size_t length = 0U;
    static const char too_long[] = "(value exceeds 64 KiB UI limit)";
    if (document_edit_decode_span(editor->edit, span,
            (uint8_t *)editor->display_text, sizeof(editor->display_text) - 1U,
            &length)) {
        editor->display_text[length] = '\0';
        *out_value = editor->display_text;
        return length;
    }
    *out_value = too_long;
    return sizeof(too_long) - 1U;
}

static bool editor_add_icon_node(viewer_editor *editor, uint32_t id,
                                 uint32_t parent, const char *label,
                                 size_t label_length, ui_icon_kind icon,
                                 uint32_t state)
{
    if (!editor_add_node(editor, id, parent, UI_AXIS_VERTICAL, 40U, 40U,
            0U, 0U, (state & UI_STATE_CHECKED) != 0U ?
                editor->tree.tokens.selection_fill : editor->tree.tokens.panel,
            UI_ROLE_BUTTON, label, label_length, NULL, 0U, state, id)) return false;
    editor->tree.nodes[editor->tree.count - 1U].spec.icon = icon;
    return true;
}

static bool editor_add_modern_attribute_rows(viewer_editor *editor,
                                             const ui_tokens *tokens)
{
    size_t count = document_edit_attribute_count(editor->edit, editor->selected);
    size_t start = editor->attribute_start;
    if (start > count) start = count;
    size_t visible = count - start;
    if (visible > EDITOR_ATTRIBUTE_VISIBLE) visible = EDITOR_ATTRIBUTE_VISIBLE;
    for (size_t row = 0U; row < visible; ++row) {
        size_t attribute_index = start + row;
        document_edit_attribute attribute;
        base_bytes name;
        const char *value = NULL;
        size_t value_length = 0U;
        if (!document_edit_attribute_at(editor->edit, editor->selected,
                attribute_index, &attribute) ||
            !document_edit_span_bytes(editor->edit, attribute.name, &name)) return false;
        if (editor->field_kind == EDITOR_FIELD_ATTRIBUTE &&
            editor->field_attribute == attribute_index) {
            value = editor->field;
            value_length = editor->field_length;
        } else {
            value_length = editor_full_attribute_value(editor, attribute.value, &value);
        }
        uint32_t state = attribute_index == editor->selected_attribute ?
            UI_STATE_CHECKED : 0U;
        if (editor->field_kind == EDITOR_FIELD_ATTRIBUTE &&
            editor->field_attribute == attribute_index) state |= UI_STATE_FOCUSED;
        uint32_t color = attribute_index == editor->selected_attribute ?
            tokens->selection_fill : tokens->panel;
        if (!editor_add_node(editor, EDITOR_ATTRIBUTE_ROW_BASE + (uint32_t)attribute_index,
                EDITOR_NODE_INSPECTOR, UI_AXIS_HORIZONTAL, 0U, 30U, 4U, 0U,
                color, UI_ROLE_TEXT_FIELD, (const char *)name.ptr, name.len,
                value, value_length,
                state, EDITOR_ATTRIBUTE_ROW_BASE + (uint32_t)attribute_index)) return false;
    }
    return true;
}

void editor_build_tree_modern(viewer_editor *editor)
{
    if (editor == NULL || editor->edit == NULL || editor->width == 0U || editor->height == 0U)
        return;
    viewer_chrome_layout layout;
    if (!viewer_chrome_layout_compute(editor->width, editor->height,
            editor->backing_scale, editor->want_structure, editor->want_inspector,
            &layout) || !ui_tree_init_accessible(&editor->tree, editor->theme,
                editor->increased_contrast)) return;
    editor->effective_structure = layout.structure_visible;
    editor->effective_inspector = layout.inspector_visible;
    editor->tree.nodes[0].spec.color = 0U;
    editor->tree.nodes[0].spec.label_len = 0U;
    editor->tree.backing_scale = editor->backing_scale;
    editor->tree.tokens.font_px = (uint32_t)ceil(13.0 * editor->backing_scale);
    editor->tree.tokens.radius = (uint32_t)ceil(6.0 * editor->backing_scale);
    editor->tree.tokens.spacing = (uint32_t)ceil(8.0 * editor->backing_scale);
    const ui_tokens *tokens = &editor->tree.tokens;
    bool okay = editor_add_node(editor, EDITOR_NODE_TOOLBAR, UI_ROOT_ID,
            UI_AXIS_HORIZONTAL, 0U, VIEWER_CHROME_TOOLBAR_POINTS, 6U, 4U,
            tokens->panel, UI_ROLE_GROUP, NULL, 0U, NULL, 0U, 0U, 0U) &&
        editor_add_node(editor, EDITOR_BUTTON_SAVE, EDITOR_NODE_TOOLBAR,
            UI_AXIS_HORIZONTAL, 52U, 28U, 0U, 0U, tokens->accent,
            UI_ROLE_BUTTON, "Save", 4U, NULL, 0U, 0U, EDITOR_BUTTON_SAVE) &&
        editor_add_node(editor, EDITOR_BUTTON_UNDO, EDITOR_NODE_TOOLBAR,
            UI_AXIS_HORIZONTAL, 52U, 28U, 0U, 0U, tokens->panel_raised,
            UI_ROLE_BUTTON, "Undo", 4U, NULL, 0U, 0U, EDITOR_BUTTON_UNDO) &&
        editor_add_node(editor, EDITOR_BUTTON_REDO, EDITOR_NODE_TOOLBAR,
            UI_AXIS_HORIZONTAL, 52U, 28U, 0U, 0U, tokens->panel_raised,
            UI_ROLE_BUTTON, "Redo", 4U, NULL, 0U, 0U, EDITOR_BUTTON_REDO);
    uint32_t point_width = (uint32_t)((double)editor->width / editor->backing_scale);
    if (okay && point_width >= 760U) {
        okay = editor_add_node(editor, EDITOR_BUTTON_DELETE, EDITOR_NODE_TOOLBAR,
                UI_AXIS_HORIZONTAL, 52U, 28U, 0U, 0U, tokens->panel_raised,
                UI_ROLE_BUTTON, "Delete", 6U, NULL, 0U, 0U, EDITOR_BUTTON_DELETE) &&
            editor_add_node(editor, EDITOR_BUTTON_DUPLICATE, EDITOR_NODE_TOOLBAR,
                UI_AXIS_HORIZONTAL, 64U, 28U, 0U, 0U, tokens->panel_raised,
                UI_ROLE_BUTTON, "Duplicate", 9U, NULL, 0U, 0U, EDITOR_BUTTON_DUPLICATE) &&
            editor_add_node(editor, EDITOR_BUTTON_GROUP, EDITOR_NODE_TOOLBAR,
                UI_AXIS_HORIZONTAL, 52U, 28U, 0U, 0U, tokens->panel_raised,
                UI_ROLE_BUTTON, "Group", 5U, NULL, 0U, 0U, EDITOR_BUTTON_GROUP) &&
            editor_add_node(editor, EDITOR_BUTTON_UNGROUP, EDITOR_NODE_TOOLBAR,
                UI_AXIS_HORIZONTAL, 68U, 28U, 0U, 0U, tokens->panel_raised,
                UI_ROLE_BUTTON, "Ungroup", 7U, NULL, 0U, 0U, EDITOR_BUTTON_UNGROUP) &&
            editor_add_node(editor, EDITOR_BUTTON_SAVE_AS, EDITOR_NODE_TOOLBAR,
                UI_AXIS_HORIZONTAL, 72U, 28U, 0U, 0U, tokens->panel_raised,
                UI_ROLE_BUTTON, "Save As", 7U, NULL, 0U, 0U, EDITOR_BUTTON_SAVE_AS);
    }
    okay = okay && editor_add_node(editor, EDITOR_NODE_OPTIONS, UI_ROOT_ID,
        UI_AXIS_HORIZONTAL, 0U, VIEWER_CHROME_OPTIONS_POINTS, 8U, 8U,
        tokens->panel_raised, UI_ROLE_GROUP, NULL, 0U, NULL, 0U, 0U, 0U);
    const char *tool_hint = editor->active_tool == UI_ICON_PAN ?
        "Pan · drag the canvas" : editor->active_tool == UI_ICON_ZOOM ?
        "Zoom · scroll over the canvas" : "Select · choose structure rows or edit properties";
    okay = okay && editor_add_node(editor, EDITOR_NODE_OPTIONS + 100U,
        EDITOR_NODE_OPTIONS, UI_AXIS_HORIZONTAL, 0U, 0U, 0U, 0U, 0U,
        UI_ROLE_TEXT, tool_hint, strlen(tool_hint), NULL, 0U, 0U, 0U) &&
        editor_add_node(editor, EDITOR_BUTTON_TOGGLE_STRUCTURE, EDITOR_NODE_OPTIONS,
            UI_AXIS_HORIZONTAL, 82U, 24U, 0U, 0U, tokens->panel,
            UI_ROLE_BUTTON, "Structure", 9U, NULL, 0U,
            editor->want_structure ? UI_STATE_CHECKED : 0U,
            EDITOR_BUTTON_TOGGLE_STRUCTURE) &&
        editor_add_node(editor, EDITOR_BUTTON_TOGGLE_INSPECTOR, EDITOR_NODE_OPTIONS,
            UI_AXIS_HORIZONTAL, 82U, 24U, 0U, 0U, tokens->panel,
            UI_ROLE_BUTTON, "Inspector", 9U, NULL, 0U,
            editor->want_inspector ? UI_STATE_CHECKED : 0U,
            EDITOR_BUTTON_TOGGLE_INSPECTOR) &&
        editor_add_node(editor, EDITOR_NODE_WORKSPACE, UI_ROOT_ID,
            UI_AXIS_HORIZONTAL, 0U, 0U, 0U, 0U, 0U, UI_ROLE_GROUP,
            NULL, 0U, NULL, 0U, 0U, 0U) &&
        editor_add_node(editor, EDITOR_NODE_TOOL_RAIL, EDITOR_NODE_WORKSPACE,
            UI_AXIS_VERTICAL, EDITOR_TOOL_RAIL_WIDTH, 0U, 2U, 4U,
            tokens->panel, UI_ROLE_GROUP, NULL, 0U, NULL, 0U, 0U, 0U);
    okay = okay && editor_add_icon_node(editor, EDITOR_BUTTON_TOOL_SELECT,
            EDITOR_NODE_TOOL_RAIL, "Select tool", 11U, UI_ICON_SELECT,
            editor->active_tool == UI_ICON_SELECT ? UI_STATE_CHECKED : 0U) &&
        editor_add_icon_node(editor, EDITOR_BUTTON_TOOL_PAN,
            EDITOR_NODE_TOOL_RAIL, "Pan tool", 8U, UI_ICON_PAN,
            editor->active_tool == UI_ICON_PAN ? UI_STATE_CHECKED : 0U) &&
        editor_add_icon_node(editor, EDITOR_BUTTON_TOOL_ZOOM,
            EDITOR_NODE_TOOL_RAIL, "Zoom tool", 9U, UI_ICON_ZOOM,
            editor->active_tool == UI_ICON_ZOOM ? UI_STATE_CHECKED : 0U);
    if (okay && layout.structure_visible) {
        okay = editor_add_node(editor, EDITOR_NODE_OUTLINE, EDITOR_NODE_WORKSPACE,
                UI_AXIS_VERTICAL, EDITOR_LEFT_WIDTH, 0U, 8U, 1U, tokens->panel,
                UI_ROLE_LIST, "Structure", 9U, NULL, 0U, 0U, 0U) &&
            editor_add_node(editor, EDITOR_NODE_OUTLINE_HEADER, EDITOR_NODE_OUTLINE,
                UI_AXIS_HORIZONTAL, 0U, 28U, 0U, 4U, 0U, UI_ROLE_GROUP,
                NULL, 0U, NULL, 0U, 0U, 0U) &&
            editor_add_node(editor, EDITOR_NODE_OUTLINE_TITLE,
                EDITOR_NODE_OUTLINE_HEADER, UI_AXIS_HORIZONTAL, 0U, 28U, 0U, 0U,
                0U, UI_ROLE_TEXT, "Structure", 9U, NULL, 0U, 0U, 0U) &&
            editor_add_node(editor, EDITOR_BUTTON_HIDE_STRUCTURE,
                EDITOR_NODE_OUTLINE_HEADER, UI_AXIS_HORIZONTAL, 48U, 24U, 0U, 0U,
                tokens->panel_raised, UI_ROLE_BUTTON, "Hide", 4U, NULL, 0U, 0U,
                EDITOR_BUTTON_HIDE_STRUCTURE);
    }
    if (okay && layout.structure_visible) {
        for (size_t row = 0U; row < EDITOR_OUTLINE_VISIBLE; ++row) {
            size_t item = editor->outline_start + row;
            if (item >= editor->outline_count) break;
            char label[UI_TEXT_MAX_BYTES] = {0};
            size_t length = editor_format_outline(editor, item, label);
            uint32_t state = item == editor->selected_index ? UI_STATE_CHECKED : 0U;
            uint32_t color = item == editor->selected_index ? tokens->selection_fill : 0U;
            if (!editor_add_node(editor, EDITOR_OUTLINE_ROW_BASE + (uint32_t)row,
                    EDITOR_NODE_OUTLINE, UI_AXIS_HORIZONTAL, 0U, 28U, 4U, 0U,
                    color, UI_ROLE_ROW, label, length, NULL, 0U, state,
                    EDITOR_OUTLINE_ROW_BASE + (uint32_t)row)) return;
        }
    }
    if (okay) {
        okay = editor_add_node(editor, EDITOR_NODE_CANVAS, EDITOR_NODE_WORKSPACE,
                UI_AXIS_VERTICAL, 0U, 0U, 0U, 0U, 0U, UI_ROLE_GROUP,
                NULL, 0U, NULL, 0U, 0U, 0U) &&
            editor_add_node(editor, EDITOR_NODE_CANVAS_HUD, EDITOR_NODE_CANVAS,
                UI_AXIS_HORIZONTAL, 0U, 32U, 8U, 8U, tokens->panel_raised,
                UI_ROLE_GROUP, NULL, 0U, NULL, 0U, 0U, 0U) &&
            editor_add_node(editor, EDITOR_NODE_CANVAS_HUD + 100U,
                EDITOR_NODE_CANVAS_HUD, UI_AXIS_HORIZONTAL, 0U, 0U, 0U, 0U,
                0U, UI_ROLE_TEXT, "Zoom · fit and actual size are in View",
                strlen("Zoom · fit and actual size are in View"), NULL, 0U, 0U, 0U) &&
            editor_add_node(editor, EDITOR_NODE_CANVAS_TILE_STATUS, EDITOR_NODE_CANVAS,
                UI_AXIS_HORIZONTAL, 0U, 28U, 8U, 0U, tokens->panel_raised,
                UI_ROLE_STATUS, "Preview retained · visible tile rendering",
                strlen("Preview retained · visible tile rendering"), NULL, 0U, 0U, 0U);
    }
    if (okay && layout.inspector_visible) {
        okay = editor_add_node(editor, EDITOR_NODE_INSPECTOR, EDITOR_NODE_WORKSPACE,
                UI_AXIS_VERTICAL, EDITOR_RIGHT_WIDTH, 0U, 8U, 1U,
                tokens->panel, UI_ROLE_GROUP, "Inspector", 9U, NULL, 0U, 0U, 0U) &&
            editor_add_node(editor, EDITOR_NODE_INSPECTOR_HEADER,
                EDITOR_NODE_INSPECTOR, UI_AXIS_HORIZONTAL, 0U, 28U, 0U, 4U,
                0U, UI_ROLE_GROUP, NULL, 0U, NULL, 0U, 0U, 0U) &&
            editor_add_node(editor, EDITOR_NODE_INSPECTOR_TITLE,
                EDITOR_NODE_INSPECTOR_HEADER, UI_AXIS_HORIZONTAL, 0U, 28U, 0U,
                0U, 0U, UI_ROLE_TEXT, "Inspector", 9U, NULL, 0U, 0U, 0U) &&
            editor_add_node(editor, EDITOR_BUTTON_HIDE_INSPECTOR,
                EDITOR_NODE_INSPECTOR_HEADER, UI_AXIS_HORIZONTAL, 48U, 24U,
                0U, 0U, tokens->panel_raised, UI_ROLE_BUTTON, "Hide", 4U,
                NULL, 0U, 0U, EDITOR_BUTTON_HIDE_INSPECTOR);
    }
    char tag[UI_TEXT_MAX_BYTES] = "(no selection)";
    base_bytes tag_bytes;
    size_t tag_length = strlen(tag);
    if (okay && layout.inspector_visible &&
        document_edit_tag_name(editor->edit, editor->selected, &tag_bytes)) {
        size_t amount = editor_copy_slice(tag, sizeof(tag), tag_bytes);
        if (amount + 2U < sizeof(tag)) {
            memmove(tag + 1U, tag, amount);
            tag[0] = '<'; tag[amount + 1U] = '>';
            tag[amount + 2U] = '\0';
        }
        tag_length = strlen(tag);
    }
    if (okay && layout.inspector_visible) {
        okay = editor_add_node(editor, EDITOR_NODE_SELECTED_TAG,
            EDITOR_NODE_INSPECTOR, UI_AXIS_VERTICAL, 0U, 28U, 4U, 0U,
            0U, UI_ROLE_TEXT, tag, tag_length, NULL, 0U, 0U, 0U) &&
            editor_add_modern_attribute_rows(editor, tokens);
    }
    uint8_t text_value[EDITOR_FIELD_CAP];
    size_t text_value_length = 0U;
    if (okay && layout.inspector_visible && document_edit_text_content(
            editor->edit, editor->selected, text_value, sizeof(text_value),
            &text_value_length)) {
        const char *value = editor->field_kind == EDITOR_FIELD_TEXT ?
            editor->field : (const char *)text_value;
        size_t value_length = editor->field_kind == EDITOR_FIELD_TEXT ?
            editor->field_length : text_value_length;
        uint32_t state = editor->field_kind == EDITOR_FIELD_TEXT ? UI_STATE_FOCUSED : 0U;
        okay = editor_add_node(editor, EDITOR_TEXT_FIELD_ID, EDITOR_NODE_INSPECTOR,
            UI_AXIS_HORIZONTAL, 0U, 32U, 4U, 0U, tokens->panel_raised,
            UI_ROLE_TEXT_FIELD, "Text", 4U, value, value_length, state,
            EDITOR_TEXT_FIELD_ID);
    }
    if (okay && layout.inspector_visible) {
        const char *new_name = editor->field_kind == EDITOR_FIELD_NEW_NAME ?
            editor->field : editor->new_name;
        size_t new_name_len = editor->field_kind == EDITOR_FIELD_NEW_NAME ?
            editor->field_length : editor->new_name_length;
        const char *new_value = editor->field_kind == EDITOR_FIELD_NEW_VALUE ?
            editor->field : editor->new_value;
        size_t new_value_len = editor->field_kind == EDITOR_FIELD_NEW_VALUE ?
            editor->field_length : editor->new_value_length;
        okay = editor_add_node(editor, EDITOR_NEW_NAME_ID, EDITOR_NODE_INSPECTOR,
                UI_AXIS_HORIZONTAL, 0U, 30U, 4U, 0U, tokens->panel_raised,
                UI_ROLE_TEXT_FIELD, "New name", 8U, new_name, new_name_len,
                editor->field_kind == EDITOR_FIELD_NEW_NAME ? UI_STATE_FOCUSED : 0U,
                EDITOR_NEW_NAME_ID) &&
            editor_add_node(editor, EDITOR_NEW_VALUE_ID, EDITOR_NODE_INSPECTOR,
                UI_AXIS_HORIZONTAL, 0U, 30U, 4U, 0U, tokens->panel_raised,
                UI_ROLE_TEXT_FIELD, "New value", 9U, new_value, new_value_len,
                editor->field_kind == EDITOR_FIELD_NEW_VALUE ? UI_STATE_FOCUSED : 0U,
                EDITOR_NEW_VALUE_ID) &&
            editor_add_node(editor, EDITOR_BUTTON_ADD_ATTRIBUTE, EDITOR_NODE_INSPECTOR,
                UI_AXIS_HORIZONTAL, 0U, 28U, 4U, 0U, tokens->accent,
                UI_ROLE_BUTTON, "Add attribute", 13U, NULL, 0U, 0U,
                EDITOR_BUTTON_ADD_ATTRIBUTE) &&
            editor_add_node(editor, EDITOR_BUTTON_REMOVE_ATTRIBUTE, EDITOR_NODE_INSPECTOR,
                UI_AXIS_HORIZONTAL, 0U, 28U, 4U, 0U, tokens->panel_raised,
                UI_ROLE_BUTTON, "Remove attribute", 16U, NULL, 0U, 0U,
                EDITOR_BUTTON_REMOVE_ATTRIBUTE);
    }
    bool status_error = strncmp(editor->status, "Edit rejected:", 14U) == 0 ||
        strncmp(editor->status, "Save failed:", 12U) == 0 ||
        strncmp(editor->status, "Preview update failed:", 22U) == 0 ||
        strncmp(editor->status, "Unsupported SVG content", 23U) == 0;
    okay = okay && editor_add_node(editor, EDITOR_NODE_STATUS, UI_ROOT_ID,
        UI_AXIS_HORIZONTAL, 0U, VIEWER_CHROME_STATUS_POINTS, 8U, 0U,
        tokens->panel_raised, UI_ROLE_STATUS, editor->status,
        strlen(editor->status), NULL, 0U,
        status_error ? UI_STATE_INVALID : 0U, 0U);
    if (okay && ui_tree_layout(&editor->tree, editor->width, editor->height)) {
        if (editor->focus_id != 0U) {
            for (size_t index = 0U; index < editor->tree.count; ++index) {
                ui_node *node = &editor->tree.nodes[index];
                if (node->spec.id == editor->focus_id &&
                    (node->spec.state & UI_STATE_DISABLED) == 0U) {
                    node->spec.state |= UI_STATE_FOCUSED;
                    editor->tree.focused_id = editor->focus_id;
                    break;
                }
            }
        }
        editor->tree_ready = true;
    }
}

