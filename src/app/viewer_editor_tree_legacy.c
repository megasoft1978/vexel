#include "viewer_editor_internal.h"

#include "viewer_chrome.h"
#include "ui_widgets.h"
#include "platform/macos/platform_accessibility.h"
#include "platform/platform_text_ui.h"

#include <math.h>
#include <string.h>

static size_t editor_display_value(viewer_editor *editor, document_source_span span,
                                   char out[UI_TEXT_MAX_BYTES])
{
    uint8_t decoded[EDITOR_FIELD_CAP];
    size_t length = 0U;
    if (!document_edit_decode_span(editor->edit, span, decoded, sizeof(decoded), &length)) {
        editor_set_text(out, UI_TEXT_MAX_BYTES, "(long value)");
        return strlen(out);
    }
    size_t amount = editor_utf8_prefix(decoded, length, UI_TEXT_MAX_BYTES - 1U);
    if (amount > 0U) memcpy(out, decoded, amount);
    if (amount < length && amount + 3U < UI_TEXT_MAX_BYTES) {
        out[amount++] = '.'; out[amount++] = '.'; out[amount++] = '.';
    }
    out[amount] = '\0';
    return amount;
}

static size_t editor_display_field(const char *bytes, size_t length,
                                   char out[UI_TEXT_MAX_BYTES])
{
    size_t amount = editor_utf8_prefix((const uint8_t *)bytes, length, UI_TEXT_MAX_BYTES - 1U);
    if (amount > 0U) memcpy(out, bytes, amount);
    if (amount < length && amount + 3U < UI_TEXT_MAX_BYTES) {
        out[amount++] = '.'; out[amount++] = '.'; out[amount++] = '.';
    }
    out[amount] = '\0';
    return amount;
}

void editor_build_tree_legacy(viewer_editor *editor)
{
    if (editor == NULL || editor->edit == NULL || editor->width == 0U || editor->height == 0U) return;
    if (!ui_tree_init(&editor->tree, editor->theme)) return;
    editor->tree.nodes[0].spec.color = 0U;
    editor->tree.nodes[0].spec.label_len = 0U;
    editor->tree.tokens.font_px = (uint32_t)ceil(13.0 * editor->backing_scale);
    const ui_tokens *tokens = &editor->tree.tokens;
    bool okay = editor_add_node(editor, EDITOR_NODE_TOOLBAR, UI_ROOT_ID,
            UI_AXIS_HORIZONTAL, 0U, EDITOR_TOOLBAR_HEIGHT, 6U, 4U, tokens->panel,
            UI_ROLE_GROUP, NULL, 0U, NULL, 0U, 0U, 0U) &&
        editor_add_node(editor, EDITOR_BUTTON_SAVE, EDITOR_NODE_TOOLBAR,
            UI_AXIS_HORIZONTAL, 52U, 28U, 0U, 0U, tokens->accent, UI_ROLE_BUTTON,
            "Save", 4U, NULL, 0U, 0U, EDITOR_BUTTON_SAVE) &&
        editor_add_node(editor, EDITOR_BUTTON_UNDO, EDITOR_NODE_TOOLBAR,
            UI_AXIS_HORIZONTAL, 52U, 28U, 0U, 0U, tokens->panel_alt, UI_ROLE_BUTTON,
            "Undo", 4U, NULL, 0U, 0U, EDITOR_BUTTON_UNDO) &&
        editor_add_node(editor, EDITOR_BUTTON_REDO, EDITOR_NODE_TOOLBAR,
            UI_AXIS_HORIZONTAL, 52U, 28U, 0U, 0U, tokens->panel_alt, UI_ROLE_BUTTON,
            "Redo", 4U, NULL, 0U, 0U, EDITOR_BUTTON_REDO) &&
        editor_add_node(editor, EDITOR_BUTTON_DELETE, EDITOR_NODE_TOOLBAR,
            UI_AXIS_HORIZONTAL, 52U, 28U, 0U, 0U, tokens->panel_alt, UI_ROLE_BUTTON,
            "Delete", 6U, NULL, 0U, 0U, EDITOR_BUTTON_DELETE) &&
        editor_add_node(editor, EDITOR_BUTTON_DUPLICATE, EDITOR_NODE_TOOLBAR,
            UI_AXIS_HORIZONTAL, 64U, 28U, 0U, 0U, tokens->panel_alt, UI_ROLE_BUTTON,
            "Duplicate", 9U, NULL, 0U, 0U, EDITOR_BUTTON_DUPLICATE) &&
        editor_add_node(editor, EDITOR_BUTTON_GROUP, EDITOR_NODE_TOOLBAR,
            UI_AXIS_HORIZONTAL, 52U, 28U, 0U, 0U, tokens->panel_alt, UI_ROLE_BUTTON,
            "Group", 5U, NULL, 0U, 0U, EDITOR_BUTTON_GROUP) &&
        editor_add_node(editor, EDITOR_BUTTON_UNGROUP, EDITOR_NODE_TOOLBAR,
            UI_AXIS_HORIZONTAL, 68U, 28U, 0U, 0U, tokens->panel_alt, UI_ROLE_BUTTON,
            "Ungroup", 7U, NULL, 0U, 0U, EDITOR_BUTTON_UNGROUP) &&
        editor_add_node(editor, EDITOR_BUTTON_SAVE_AS, EDITOR_NODE_TOOLBAR,
            UI_AXIS_HORIZONTAL, 72U, 28U, 0U, 0U, tokens->panel_alt, UI_ROLE_BUTTON,
            "Save As", 7U, NULL, 0U, 0U, EDITOR_BUTTON_SAVE_AS) &&
        editor_add_node(editor, EDITOR_NODE_WORKSPACE, UI_ROOT_ID,
            UI_AXIS_HORIZONTAL, 0U, 0U, 0U, 1U, 0U, UI_ROLE_GROUP,
            NULL, 0U, NULL, 0U, 0U, 0U) &&
        editor_add_node(editor, EDITOR_NODE_OUTLINE, EDITOR_NODE_WORKSPACE,
            UI_AXIS_VERTICAL, EDITOR_LEFT_WIDTH, 0U, 8U, 1U, tokens->panel,
            UI_ROLE_GROUP, NULL, 0U, NULL, 0U, 0U, 0U) &&
        editor_add_node(editor, EDITOR_NODE_OUTLINE_TITLE, EDITOR_NODE_OUTLINE,
            UI_AXIS_VERTICAL, 0U, 28U, 2U, 0U, tokens->panel_alt,
            UI_ROLE_TEXT, "STRUCTURE", 9U, NULL, 0U, 0U, 0U) &&
        editor_add_node(editor, EDITOR_NODE_CANVAS, EDITOR_NODE_WORKSPACE,
            UI_AXIS_VERTICAL, 0U, 0U, 0U, 0U, 0U, UI_ROLE_GROUP,
            NULL, 0U, NULL, 0U, 0U, 0U) &&
        editor_add_node(editor, EDITOR_NODE_INSPECTOR, EDITOR_NODE_WORKSPACE,
            UI_AXIS_VERTICAL, EDITOR_RIGHT_WIDTH, 0U, 8U, 1U, tokens->panel,
            UI_ROLE_GROUP, NULL, 0U, NULL, 0U, 0U, 0U) &&
        editor_add_node(editor, EDITOR_NODE_INSPECTOR_TITLE, EDITOR_NODE_INSPECTOR,
            UI_AXIS_VERTICAL, 0U, 28U, 2U, 0U, tokens->panel_alt,
            UI_ROLE_TEXT, "INSPECTOR", 9U, NULL, 0U, 0U, 0U);
    if (!okay) return;
    for (size_t row = 0U; row < EDITOR_OUTLINE_VISIBLE; ++row) {
        size_t item = editor->outline_start + row;
        if (item >= editor->outline_count) break;
        char label[UI_TEXT_MAX_BYTES] = {0};
        size_t length = editor_format_outline(editor, item, label);
        uint32_t state = item == editor->selected_index ? UI_STATE_CHECKED : 0U;
        uint32_t color = item == editor->selected_index ? tokens->panel_alt : 0U;
        if (!editor_add_node(editor, EDITOR_OUTLINE_ROW_BASE + (uint32_t)row,
                EDITOR_NODE_OUTLINE, UI_AXIS_HORIZONTAL, 0U, 25U, 2U, 0U,
                color, UI_ROLE_ROW, label, length, NULL, 0U, state,
                EDITOR_OUTLINE_ROW_BASE + (uint32_t)row)) return;
    }
    char tag[UI_TEXT_MAX_BYTES] = "(no selection)";
    base_bytes tag_bytes;
    if (document_edit_tag_name(editor->edit, editor->selected, &tag_bytes)) {
        size_t amount = editor_copy_slice(tag, sizeof(tag), tag_bytes);
        if (amount + 2U < sizeof(tag)) {
            memmove(tag + 1U, tag, amount);
            tag[0] = '<'; tag[amount + 1U] = '>';
            tag[amount + 2U] = '\0';
        }
    }
    okay = editor_add_node(editor, EDITOR_NODE_SELECTED_TAG, EDITOR_NODE_INSPECTOR,
        UI_AXIS_VERTICAL, 0U, 26U, 2U, 0U, 0U, UI_ROLE_TEXT,
        tag, strlen(tag), NULL, 0U, 0U, 0U);
    size_t attribute_count = document_edit_attribute_count(editor->edit, editor->selected);
    size_t first_attribute = editor->attribute_start;
    if (first_attribute > attribute_count) first_attribute = attribute_count;
    size_t visible_attributes = attribute_count - first_attribute;
    if (visible_attributes > EDITOR_ATTRIBUTE_VISIBLE) visible_attributes = EDITOR_ATTRIBUTE_VISIBLE;
    for (size_t row = 0U; okay && row < visible_attributes; ++row) {
        size_t attribute_index = first_attribute + row;
        document_edit_attribute attribute;
        base_bytes name;
        char label[UI_TEXT_MAX_BYTES] = {0}, value[UI_TEXT_MAX_BYTES] = {0};
        if (!document_edit_attribute_at(editor->edit, editor->selected,
                attribute_index, &attribute) ||
            !document_edit_span_bytes(editor->edit, attribute.name, &name)) { okay = false; break; }
        size_t label_length = editor_copy_slice(label, sizeof(label), name);
        size_t value_length = editor->field_kind == EDITOR_FIELD_ATTRIBUTE &&
            editor->field_attribute == attribute_index ?
            editor_display_field(editor->field, editor->field_length, value) :
            editor_display_value(editor, attribute.value, value);
        uint32_t state = attribute_index == editor->selected_attribute ? UI_STATE_CHECKED : 0U;
        if (editor->field_kind == EDITOR_FIELD_ATTRIBUTE &&
            editor->field_attribute == attribute_index) state |= UI_STATE_FOCUSED;
        uint32_t color = attribute_index == editor->selected_attribute ? tokens->panel_alt : tokens->panel;
        okay = editor_add_node(editor, EDITOR_ATTRIBUTE_ROW_BASE + (uint32_t)attribute_index,
            EDITOR_NODE_INSPECTOR, UI_AXIS_HORIZONTAL, 0U, 26U, 2U, 0U,
            color, UI_ROLE_TEXT_FIELD, label, label_length, value, value_length,
            state, EDITOR_ATTRIBUTE_ROW_BASE + (uint32_t)attribute_index);
    }
    char text_value[UI_TEXT_MAX_BYTES] = {0};
    uint8_t decoded_text[EDITOR_FIELD_CAP];
    size_t decoded_text_length = 0U;
    if (okay && document_edit_text_content(editor->edit, editor->selected,
            decoded_text, sizeof(decoded_text), &decoded_text_length)) {
        (void)(editor->field_kind == EDITOR_FIELD_TEXT ?
            editor_display_field(editor->field, editor->field_length, text_value) :
            editor_display_field((const char *)decoded_text, decoded_text_length, text_value));
        uint32_t state = editor->field_kind == EDITOR_FIELD_TEXT ? UI_STATE_FOCUSED : 0U;
        okay = editor_add_node(editor, EDITOR_TEXT_FIELD_ID, EDITOR_NODE_INSPECTOR,
            UI_AXIS_HORIZONTAL, 0U, 30U, 2U, 0U, tokens->panel_alt,
            UI_ROLE_TEXT_FIELD, "Text", 4U, text_value, strlen(text_value),
            state, EDITOR_TEXT_FIELD_ID);
    }
    char new_name_display[UI_TEXT_MAX_BYTES] = {0};
    char new_value_display[UI_TEXT_MAX_BYTES] = {0};
    size_t new_name_display_length = editor->field_kind == EDITOR_FIELD_NEW_NAME ?
        editor_display_field(editor->field, editor->field_length, new_name_display) :
        editor_display_field(editor->new_name, editor->new_name_length, new_name_display);
    size_t new_value_display_length = editor->field_kind == EDITOR_FIELD_NEW_VALUE ?
        editor_display_field(editor->field, editor->field_length, new_value_display) :
        editor_display_field(editor->new_value, editor->new_value_length, new_value_display);
    uint32_t name_state = editor->field_kind == EDITOR_FIELD_NEW_NAME ? UI_STATE_FOCUSED : 0U;
    uint32_t value_state = editor->field_kind == EDITOR_FIELD_NEW_VALUE ? UI_STATE_FOCUSED : 0U;
    okay = okay && editor_add_node(editor, EDITOR_NEW_NAME_ID, EDITOR_NODE_INSPECTOR,
        UI_AXIS_HORIZONTAL, 0U, 26U, 2U, 0U, tokens->panel_alt,
        UI_ROLE_TEXT_FIELD, "New name", 8U, new_name_display,
        new_name_display_length,
        name_state, EDITOR_NEW_NAME_ID) &&
        editor_add_node(editor, EDITOR_NEW_VALUE_ID, EDITOR_NODE_INSPECTOR,
        UI_AXIS_HORIZONTAL, 0U, 26U, 2U, 0U, tokens->panel_alt,
        UI_ROLE_TEXT_FIELD, "New value", 9U, new_value_display,
        new_value_display_length,
        value_state, EDITOR_NEW_VALUE_ID) &&
        editor_add_node(editor, EDITOR_BUTTON_ADD_ATTRIBUTE, EDITOR_NODE_INSPECTOR,
        UI_AXIS_HORIZONTAL, 0U, 28U, 2U, 0U, tokens->accent,
        UI_ROLE_BUTTON, "Add attribute", 13U, NULL, 0U, 0U,
        EDITOR_BUTTON_ADD_ATTRIBUTE) &&
        editor_add_node(editor, EDITOR_BUTTON_REMOVE_ATTRIBUTE, EDITOR_NODE_INSPECTOR,
        UI_AXIS_HORIZONTAL, 0U, 28U, 2U, 0U, tokens->panel_alt,
        UI_ROLE_BUTTON, "Remove attribute", 16U, NULL, 0U, 0U,
        EDITOR_BUTTON_REMOVE_ATTRIBUTE) &&
        editor_add_node(editor, EDITOR_NODE_STATUS, UI_ROOT_ID,
        UI_AXIS_HORIZONTAL, 0U, EDITOR_STATUS_HEIGHT, 8U, 0U,
        tokens->panel_alt, UI_ROLE_STATUS, editor->status,
        editor_utf8_prefix((const uint8_t *)editor->status, strlen(editor->status), UI_TEXT_MAX_BYTES),
        NULL, 0U, 0U, 0U);
    if (okay && ui_tree_layout(&editor->tree, editor->width, editor->height)) {
        if (editor->focus_id != 0U) {
            for (size_t index = 0U; index < editor->tree.count; ++index) {
                if (editor->tree.nodes[index].spec.id == editor->focus_id &&
                    (editor->tree.nodes[index].spec.state & UI_STATE_DISABLED) == 0U) {
                    editor->tree.nodes[index].spec.state |= UI_STATE_FOCUSED;
                    editor->tree.focused_id = editor->focus_id;
                    break;
                }
            }
        }
        editor->tree_ready = true;
    }
}
