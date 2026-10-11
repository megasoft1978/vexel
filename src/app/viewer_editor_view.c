#include "viewer_editor_internal.h"

#include "viewer_chrome.h"
#include "ui_widgets.h"
#include "platform/macos/platform_accessibility.h"
#include "platform/platform_text_ui.h"

#include <limits.h>
#include <math.h>
#include <string.h>

static bool scale_metric(uint32_t value, double scale, uint32_t *out_value)
{
    if (out_value == NULL || !isfinite(scale) || scale <= 0.0) return false;
    double scaled = ceil((double)value * scale);
    if (!isfinite(scaled) || scaled < 0.0 || scaled > (double)UINT32_MAX) return false;
    *out_value = (uint32_t)scaled;
    return true;
}

size_t editor_copy_slice(char *destination, size_t capacity, base_bytes bytes)
{
    if (destination == NULL || capacity == 0U || bytes.ptr == NULL) return 0U;
    size_t amount = editor_utf8_prefix(bytes.ptr, bytes.len, capacity - 1U);
    if (amount > 0U) memcpy(destination, bytes.ptr, amount);
    destination[amount] = '\0';
    return amount;
}

size_t editor_format_outline(const viewer_editor *editor, size_t index,
                             char out[UI_TEXT_MAX_BYTES])
{
    if (index >= editor->outline_count) return 0U;
    size_t at = 0U;
    size_t spaces = (size_t)editor->outline_depth[index] * 2U;
    if (spaces > 12U) spaces = 12U;
    while (spaces-- > 0U && at + 1U < UI_TEXT_MAX_BYTES) out[at++] = ' ';
    base_bytes tag;
    if (document_edit_tag_name((document_edit *)editor->edit, editor->outline[index], &tag)) {
        if (at + 1U < UI_TEXT_MAX_BYTES) out[at++] = '<';
        size_t amount = editor_utf8_prefix(tag.ptr, tag.len, UI_TEXT_MAX_BYTES - at - 2U);
        if (amount > 0U) { memcpy(out + at, tag.ptr, amount); at += amount; }
        if (at + 1U < UI_TEXT_MAX_BYTES) out[at++] = '>';
    }
    size_t attribute_count = document_edit_attribute_count((document_edit *)editor->edit,
        editor->outline[index]);
    for (size_t i = 0U; i < attribute_count; ++i) {
        document_edit_attribute attribute;
        base_bytes name;
        if (!document_edit_attribute_at((document_edit *)editor->edit,
                editor->outline[index], i, &attribute) ||
            !document_edit_span_bytes((document_edit *)editor->edit, attribute.name, &name) ||
            name.len != 2U || memcmp(name.ptr, "id", 2U) != 0) continue;
        uint8_t id[32];
        size_t length = 0U;
        if (document_edit_decode_span((document_edit *)editor->edit, attribute.value,
                id, sizeof(id), &length) && length > 0U && at + 3U < UI_TEXT_MAX_BYTES) {
            out[at++] = ' ';
            out[at++] = '#';
            size_t take = editor_utf8_prefix(id, length, UI_TEXT_MAX_BYTES - at - 1U);
            memcpy(out + at, id, take);
            at += take;
        }
        break;
    }
    out[at] = '\0';
    return at;
}

bool editor_add_node(viewer_editor *editor, uint32_t id, uint32_t parent,
                            ui_axis axis, uint32_t width, uint32_t height,
                            uint32_t padding, uint32_t gap, uint32_t color,
                            ui_role role, const char *label, size_t label_len,
                            const char *value, size_t value_len, uint32_t state,
                            uint32_t command_id)
{
    return editor_add_widget_node(editor, id, parent, axis, width, height,
        padding, gap, color, role, UI_WIDGET_AUTO, UI_ICON_NONE, label,
        label_len, value, value_len, state, command_id);
}

bool editor_add_widget_node(viewer_editor *editor, uint32_t id, uint32_t parent,
                            ui_axis axis, uint32_t width, uint32_t height,
                            uint32_t padding, uint32_t gap, uint32_t color,
                            ui_role role, ui_widget_kind widget, ui_icon_kind icon,
                            const char *label, size_t label_len,
                            const char *value, size_t value_len, uint32_t state,
                            uint32_t command_id)
{
    double scale = editor_presentation_scale(editor);
    uint32_t scaled_width = 0U, scaled_height = 0U;
    uint32_t scaled_padding = 0U, scaled_gap = 0U;
    if (!scale_metric(width, scale, &scaled_width) ||
        !scale_metric(height, scale, &scaled_height) ||
        !scale_metric(padding, scale, &scaled_padding) ||
        !scale_metric(gap, scale, &scaled_gap)) return false;
    width = scaled_width;
    height = scaled_height;
    padding = scaled_padding;
    gap = scaled_gap;
    char localized_label[PLATFORM_LOCALIZATION_TEXT_CAP];
    size_t localized_length = 0U;
    if (label != NULL && label_len > 0U &&
        editor_localize_slice(label, label_len, localized_label,
            sizeof(localized_label), &localized_length)) {
        label = localized_label;
        label_len = localized_length;
    }
    ui_widget_spec spec = {.id = id, .parent_id = parent, .axis = axis,
        .preferred_width = width, .preferred_height = height,
        .padding = padding, .gap = gap, .color = color,
        .command_id = command_id, .state = state, .widget = widget,
        .icon = icon, .label = label, .label_len = label_len,
        .value = value, .value_len = value_len};
    if (id == UI_ROOT_ID) {
        ui_node_spec root = {.id = id, .parent_id = parent, .axis = axis,
            .preferred_width = width, .preferred_height = height, .padding = padding,
            .gap = gap, .color = color, .role = role, .label = label,
            .label_len = label_len, .value = value, .value_len = value_len,
            .state = state, .command_id = command_id, .widget = widget,
            .icon = icon};
        return ui_tree_add(&editor->tree, &root);
    }
    if (role == UI_ROLE_BUTTON) return ui_widget_add_button(&editor->tree, &spec);
    if (role == UI_ROLE_TEXT_FIELD) return ui_widget_add_text_field(&editor->tree, &spec);
    if (role == UI_ROLE_ROW) return ui_widget_add_row(&editor->tree, &spec);
    bool added = ui_tree_add(&editor->tree, &(ui_node_spec){.id = id, .parent_id = parent,
        .axis = axis, .preferred_width = width, .preferred_height = height,
        .padding = padding, .gap = gap, .color = color, .role = role,
        .widget = widget, .icon = icon,
        .label = label, .label_len = label_len, .value = value,
        .value_len = value_len, .state = state, .command_id = command_id});
    return added;
}

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

static void editor_build_tree_legacy(viewer_editor *editor)
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

static size_t editor_codepoints_before(const char *text, size_t byte_offset)
{
    size_t count = 0U;
    for (size_t index = 0U; index < byte_offset; ++index)
        if (((uint8_t)text[index] & 0xc0U) != 0x80U) ++count;
    return count;
}

static void editor_blend_at(uint8_t *pixel, uint32_t color)
{
    uint32_t alpha = color & 255U;
    uint32_t inverse = 255U - alpha;
    for (size_t channel = 0U; channel < 3U; ++channel) {
        uint32_t source = (color >> (24U - (uint32_t)(channel * 8U))) & 255U;
        pixel[channel] = (uint8_t)((source * alpha +
            (uint32_t)pixel[channel] * inverse + 127U) / 255U);
    }
    pixel[3] = 255U;
}

static void editor_paint_active_selection(viewer_editor *editor, uint8_t *rgba,
                                          size_t stride, uint32_t width,
                                          uint32_t height)
{
    if (editor->field_kind == EDITOR_FIELD_NONE || !editor->tree_ready) return;
    const ui_node *node = ui_tree_find(&editor->tree, editor->focus_id);
    if (node == NULL || node->bounds.width == 0U || node->bounds.height == 0U) return;
    uint32_t scale = (uint32_t)ceil(editor->backing_scale);
    if (scale == 0U) scale = 1U;
    uint32_t start_x = node->bounds.x + node->bounds.width / 2U;
    uint32_t x_limit = node->bounds.x + node->bounds.width;
    uint32_t y_start = node->bounds.y + (4U * scale < node->bounds.height ? 4U * scale : 0U);
    uint32_t y_end = node->bounds.y + node->bounds.height;
    uint32_t bottom_inset = 4U * scale;
    if (y_end > bottom_inset) y_end -= bottom_inset;
    if (y_end > height) y_end = height;
    size_t selection_start, selection_end;
    editor_selection_range(editor, &selection_start, &selection_end);
    size_t first = editor_codepoints_before(editor->field, selection_start);
    size_t last = editor_codepoints_before(editor->field, selection_end);
    uint64_t first_x64 = (uint64_t)start_x + (uint64_t)first * 6U * scale;
    uint64_t last_x64 = (uint64_t)start_x + (uint64_t)last * 6U * scale;
    uint32_t first_x = first_x64 > UINT32_MAX ? UINT32_MAX : (uint32_t)first_x64;
    uint32_t last_x = last_x64 > UINT32_MAX ? UINT32_MAX : (uint32_t)last_x64;
    uint32_t selection_color = (editor->tree.tokens.accent & 0xffffff00U) | 80U;
    if (first_x < x_limit && last_x > first_x) {
        uint32_t end = last_x < x_limit ? last_x : x_limit;
        for (uint32_t y = y_start; y < y_end; ++y)
            for (uint32_t x = first_x; x < end && x < width; ++x)
                editor_blend_at(rgba + (size_t)y * stride + (size_t)x * 4U,
                    selection_color);
    }
    uint64_t caret_x64 = (uint64_t)start_x +
        (uint64_t)editor_codepoints_before(editor->field, editor->cursor) * 6U * scale;
    if (caret_x64 >= x_limit || caret_x64 >= width) return;
    uint32_t caret_x = (uint32_t)caret_x64;
    uint32_t caret_width = scale;
    if (caret_width > x_limit - caret_x) caret_width = x_limit - caret_x;
    for (uint32_t y = y_start; y < y_end; ++y)
        for (uint32_t x = caret_x; x < caret_x + caret_width; ++x)
            editor_blend_at(rgba + (size_t)y * stride + (size_t)x * 4U,
                editor->tree.tokens.accent);
}

void viewer_editor_paint(viewer_editor *editor, uint8_t *rgba, size_t capacity,
                         size_t stride)
{
    if (editor == NULL || rgba == NULL) return;
    if (!editor->tree_ready) editor_build_tree(editor);
    if (!editor->tree_ready) return;
    bool painted = editor->modern_style ?
        ui_tree_paint_backgrounds_into(&editor->tree, rgba, capacity, stride) :
        ui_tree_paint_into(&editor->tree, rgba, capacity, stride);
    if (!painted) return;
    if (editor->modern_style) {
        editor_paint_widgets(editor, rgba, capacity, stride);
        if (platform_text_ui_app_is_active())
            (void)platform_accessibility_update_tree(&editor->tree, editor->width,
                editor->height, editor->backing_scale);
    } else {
        editor_paint_active_selection(editor, rgba, stride, editor->width, editor->height);
    }
}

void viewer_editor_paint_scalar(viewer_editor *editor, uint8_t *rgba,
                                size_t capacity, size_t stride)
{
    if (editor == NULL || rgba == NULL) return;
    if (!editor->tree_ready) editor_build_tree(editor);
    if (!editor->tree_ready || !ui_tree_paint_into(&editor->tree, rgba,
            capacity, stride)) return;
    if (!editor->modern_style)
        editor_paint_active_selection(editor, rgba, stride, editor->width, editor->height);
}

static uint32_t editor_run_action(viewer_editor *editor, uint32_t action)
{
    if (action == EDITOR_BUTTON_TOOL_SELECT) {
        (void)viewer_editor_set_tool(editor, UI_ICON_SELECT);
        return VIEWER_EDITOR_HANDLED;
    }
    if (action == EDITOR_BUTTON_TOOL_PAN) {
        (void)viewer_editor_set_tool(editor, UI_ICON_PAN);
        return VIEWER_EDITOR_HANDLED;
    }
    if (action == EDITOR_BUTTON_TOOL_ZOOM) {
        (void)viewer_editor_set_tool(editor, UI_ICON_ZOOM);
        return VIEWER_EDITOR_HANDLED;
    }
    if (action == EDITOR_BUTTON_TOOL_RECTANGLE) {
        (void)viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_RECT);
        return VIEWER_EDITOR_HANDLED;
    }
    if (action == EDITOR_BUTTON_TOOL_ELLIPSE) {
        (void)viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_ELLIPSE);
        return VIEWER_EDITOR_HANDLED;
    }
    if (action == EDITOR_BUTTON_TOOL_LINE) {
        (void)viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_LINE);
        return VIEWER_EDITOR_HANDLED;
    }
    if (action == EDITOR_BUTTON_TOOL_PEN) {
        (void)viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_PEN);
        return VIEWER_EDITOR_HANDLED;
    }
    if (action == EDITOR_BUTTON_TOOL_NODE) {
        (void)viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_PEN);
        editor->node_edit_active = true;
        editor->tree_ready = false;
        editor->redraw_requested = true;
        return VIEWER_EDITOR_HANDLED;
    }
    if (action == EDITOR_BUTTON_TOOL_POLYGON) {
        (void)viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_POLYGON);
        return VIEWER_EDITOR_HANDLED;
    }
    if (action == EDITOR_BUTTON_TOOL_POLYLINE) {
        (void)viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_POLYLINE);
        return VIEWER_EDITOR_HANDLED;
    }
    if (action == EDITOR_BUTTON_TOOL_TEXT) {
        (void)viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_TEXT);
        return VIEWER_EDITOR_HANDLED;
    }
    if (action == EDITOR_BUTTON_TOGGLE_STRUCTURE || action == EDITOR_BUTTON_HIDE_STRUCTURE) {
        (void)viewer_editor_set_sidebar_visibility(editor,
            !editor->want_structure, editor->want_inspector);
        return VIEWER_EDITOR_HANDLED;
    }
    if (action == EDITOR_BUTTON_TOGGLE_INSPECTOR || action == EDITOR_BUTTON_HIDE_INSPECTOR) {
        (void)viewer_editor_set_sidebar_visibility(editor,
            editor->want_structure, !editor->want_inspector);
        return VIEWER_EDITOR_HANDLED;
    }
    if (action == EDITOR_BUTTON_INSPECTOR_OTHER_ATTRIBUTES) {
        editor->inspector_other_attributes_expanded =
            !editor->inspector_other_attributes_expanded;
        editor->tree_ready = false;
        editor->redraw_requested = true;
        return VIEWER_EDITOR_HANDLED;
    }
    if (action == EDITOR_BUTTON_DELETE || action == EDITOR_BUTTON_DUPLICATE ||
        action == EDITOR_BUTTON_GROUP || action == EDITOR_BUTTON_UNGROUP ||
        action == EDITOR_BUTTON_UNDO || action == EDITOR_BUTTON_REDO ||
        action == EDITOR_BUTTON_SAVE || action == EDITOR_BUTTON_SAVE_AS ||
        action == EDITOR_BUTTON_ADD_ATTRIBUTE || action == EDITOR_BUTTON_REMOVE_ATTRIBUTE)
        return editor_apply_button(editor, action);
    return VIEWER_EDITOR_HANDLED;
}

static bool editor_node_contains(const ui_node *node, uint32_t x, uint32_t y)
{
    return node != NULL && x >= node->bounds.x && y >= node->bounds.y &&
        x - node->bounds.x < node->bounds.width &&
        y - node->bounds.y < node->bounds.height;
}

static void editor_place_cursor(viewer_editor *editor, const ui_node *node, uint32_t x)
{
    if (editor->field_kind == EDITOR_FIELD_NONE || node == NULL) return;
    uint32_t scale = (uint32_t)ceil(editor->backing_scale);
    if (scale == 0U) scale = 1U;
    uint32_t value_start = node->bounds.x + node->bounds.width / 2U;
    size_t wanted = x <= value_start ? 0U : (size_t)((x - value_start) / (6U * scale));
    size_t cursor = 0U;
    size_t characters = 0U;
    while (cursor < editor->field_length && characters < wanted) {
        cursor = editor_utf8_next(editor->field, editor->field_length, cursor);
        ++characters;
    }
    editor->cursor = cursor;
    editor->anchor = cursor;
    editor->tree_ready = false;
}

uint32_t viewer_editor_pointer_down(viewer_editor *editor, uint32_t x, uint32_t y,
                                    uint8_t button)
{
    if (editor == NULL || button != 0U) return 0U;
    if (!editor->tree_ready) editor_build_tree(editor);
    if (!editor->tree_ready) return 0U;
    const ui_node *hit = NULL;
    for (size_t index = editor->tree.count; index > 0U; --index) {
        const ui_node *candidate = &editor->tree.nodes[index - 1U];
        if (editor_node_contains(candidate, x, y)) { hit = candidate; break; }
    }
    if (hit == NULL || hit->spec.id == EDITOR_NODE_CANVAS ||
        hit->spec.id == EDITOR_NODE_WORKSPACE || hit->spec.id == UI_ROOT_ID) return 0U;
    if ((hit->spec.state & UI_STATE_DISABLED) != 0U)
        return VIEWER_EDITOR_HANDLED;
    editor->pressed_id = hit->spec.id;
    editor->tree_ready = false;
    editor->redraw_requested = true;
    uint32_t prior = 0U;
    if (editor->field_kind != EDITOR_FIELD_NONE && hit->spec.id != editor->focus_id) {
        if (editor->field_kind == EDITOR_FIELD_NEW_NAME ||
            editor->field_kind == EDITOR_FIELD_NEW_VALUE) {
            if (!editor_store_pending_field(editor)) return VIEWER_EDITOR_HANDLED;
            editor->field_kind = EDITOR_FIELD_NONE;
        } else {
            prior = editor_commit_field(editor);
            if (editor->field_kind != EDITOR_FIELD_NONE) return VIEWER_EDITOR_HANDLED;
        }
    }
    editor->focus_id = hit->spec.id;
    if (hit->spec.id >= EDITOR_OUTLINE_ROW_BASE &&
        hit->spec.id < EDITOR_OUTLINE_ROW_BASE + EDITOR_OUTLINE_VISIBLE) {
        size_t row = hit->spec.id - EDITOR_OUTLINE_ROW_BASE;
        size_t index = editor->outline_start + row;
        if (editor->modern_style &&
            !editor_outline_visible_model_index(editor,
                editor->outline_visible_start + row, &index)) return prior | VIEWER_EDITOR_HANDLED;
        size_t scale = (size_t)ceil(editor_presentation_scale(editor));
        size_t action_start = hit->bounds.x + hit->bounds.width -
            (hit->bounds.width > EDITOR_OUTLINE_ACTION_WIDTH_POINTS * scale ?
                EDITOR_OUTLINE_ACTION_WIDTH_POINTS * scale : hit->bounds.width);
        if (editor->modern_style && x >= action_start) {
            if ((size_t)x < action_start +
                    EDITOR_OUTLINE_VISIBILITY_WIDTH_POINTS * scale) {
                bool changed = editor_outline_toggle_visibility(editor, index);
                return prior | VIEWER_EDITOR_HANDLED |
                    (changed ? VIEWER_EDITOR_PREVIEW_CHANGED : 0U);
            } else {
                (void)editor_outline_toggle_lock(editor, index);
            }
            return prior | VIEWER_EDITOR_HANDLED;
        }
        if (editor->modern_style) {
            size_t indent = editor->outline_depth[index] > 8U ? 8U :
                editor->outline_depth[index];
            size_t disclosure_end = hit->bounds.x +
                (4U + indent * UI_TREE_ROW_INDENT_POINTS + 18U) * scale;
            if (x < disclosure_end && document_model_first_child(editor_model(editor),
                    editor->outline[index], &(document_handle){0})) {
                (void)editor_outline_toggle_collapsed(editor, index);
                return prior | VIEWER_EDITOR_HANDLED;
            }
        }
        (void)editor_select_outline_index(editor, index);
        editor->tree_ready = false;
        return prior | VIEWER_EDITOR_HANDLED;
    }
    if (hit->spec.id >= EDITOR_ATTRIBUTE_ROW_BASE &&
        hit->spec.id < EDITOR_ATTRIBUTE_ROW_BASE + 2048U) {
        size_t attribute = hit->spec.id - EDITOR_ATTRIBUTE_ROW_BASE;
        editor->selected_attribute = attribute;
        if (attribute < editor->attribute_start)
            editor->attribute_start = attribute;
        else if (attribute >= editor->attribute_start + EDITOR_ATTRIBUTE_VISIBLE)
            editor->attribute_start = attribute - EDITOR_ATTRIBUTE_VISIBLE + 1U;
        if (editor->field_kind != EDITOR_FIELD_ATTRIBUTE || editor->field_attribute != attribute)
            (void)editor_begin_field(editor, EDITOR_FIELD_ATTRIBUTE, attribute);
        editor_place_cursor(editor, hit, x);
        return prior | VIEWER_EDITOR_HANDLED;
    }
    if (hit->spec.id == EDITOR_TEXT_FIELD_ID) {
        if (editor->field_kind != EDITOR_FIELD_TEXT)
            (void)editor_begin_field(editor, EDITOR_FIELD_TEXT, 0U);
        editor_place_cursor(editor, hit, x);
        return prior | VIEWER_EDITOR_HANDLED;
    }
    if (hit->spec.id == EDITOR_OUTLINE_SEARCH_ID) {
        if (editor->field_kind != EDITOR_FIELD_SEARCH)
            (void)editor_begin_field(editor, EDITOR_FIELD_SEARCH, 0U);
        editor->cursor = editor->field_length;
        editor->anchor = editor->cursor;
        editor->tree_ready = false;
        return prior | VIEWER_EDITOR_HANDLED;
    }
    if (hit->spec.id == EDITOR_RENAME_ID) {
        if (editor->field_kind != EDITOR_FIELD_RENAME)
            (void)editor_begin_field(editor, EDITOR_FIELD_RENAME, 0U);
        editor_place_cursor(editor, hit, x);
        return prior | VIEWER_EDITOR_HANDLED;
    }
    if (hit->spec.id == EDITOR_NEW_NAME_ID) {
        if (editor->field_kind != EDITOR_FIELD_NEW_NAME)
            (void)editor_begin_field(editor, EDITOR_FIELD_NEW_NAME, 0U);
        editor_place_cursor(editor, hit, x);
        return prior | VIEWER_EDITOR_HANDLED;
    }
    if (hit->spec.id == EDITOR_NEW_VALUE_ID) {
        if (editor->field_kind != EDITOR_FIELD_NEW_VALUE)
            (void)editor_begin_field(editor, EDITOR_FIELD_NEW_VALUE, 0U);
        editor_place_cursor(editor, hit, x);
        return prior | VIEWER_EDITOR_HANDLED;
    }
    if (hit->spec.role == UI_ROLE_BUTTON)
        return prior | editor_run_action(editor, hit->spec.command_id);
    return prior | VIEWER_EDITOR_HANDLED;
}

bool viewer_editor_scroll(viewer_editor *editor, uint32_t x, double delta_y)
{
    if (editor == NULL || delta_y == 0.0) return false;
    viewer_chrome_layout layout;
    bool modern = editor->modern_style && viewer_editor_get_chrome_layout(editor, &layout);
    uint32_t outline_start = modern ? layout.structure.x : 0U;
    uint32_t outline_end = modern ? layout.structure.x + layout.structure.width :
        (uint32_t)ceil((double)EDITOR_LEFT_WIDTH * editor->backing_scale);
    uint32_t inspector_start = modern && layout.inspector_visible ?
        layout.inspector.x : modern ? editor->width :
        editor->width - ((uint32_t)ceil((double)EDITOR_RIGHT_WIDTH *
            editor->backing_scale) < editor->width ?
            (uint32_t)ceil((double)EDITOR_RIGHT_WIDTH * editor->backing_scale) :
            editor->width);
    if (modern && layout.structure_visible && x >= outline_start && x < outline_end) {
        if (delta_y < 0.0) {
            size_t count = editor->modern_style ? editor->outline_visible_count :
                editor->outline_count;
            size_t *start = editor->modern_style ? &editor->outline_visible_start :
                &editor->outline_start;
            size_t maximum = count > EDITOR_OUTLINE_VISIBLE ?
                count - EDITOR_OUTLINE_VISIBLE : 0U;
            size_t step = delta_y <= -3.0 ? 3U : 1U;
            *start = *start + step > maximum ? maximum : *start + step;
        } else {
            size_t *start = editor->modern_style ? &editor->outline_visible_start :
                &editor->outline_start;
            size_t step = delta_y >= 3.0 ? 3U : 1U;
            *start = *start > step ? *start - step : 0U;
        }
    } else if (modern && layout.inspector_visible && x >= inspector_start) {
        size_t count = document_edit_attribute_count(editor->edit, editor->selected);
        size_t maximum = count > EDITOR_ATTRIBUTE_VISIBLE ? count - EDITOR_ATTRIBUTE_VISIBLE : 0U;
        if (delta_y < 0.0) {
            size_t step = delta_y <= -3.0 ? 3U : 1U;
            editor->attribute_start = editor->attribute_start + step > maximum ?
                maximum : editor->attribute_start + step;
        } else {
            size_t step = delta_y >= 3.0 ? 3U : 1U;
            editor->attribute_start = editor->attribute_start > step ?
                editor->attribute_start - step : 0U;
        }
    } else if (!modern && x < outline_end) {
        if (delta_y < 0.0) {
            size_t maximum = editor->outline_count > EDITOR_OUTLINE_VISIBLE ?
                editor->outline_count - EDITOR_OUTLINE_VISIBLE : 0U;
            size_t step = delta_y <= -3.0 ? 3U : 1U;
            editor->outline_start = editor->outline_start + step > maximum ?
                maximum : editor->outline_start + step;
        } else {
            size_t step = delta_y >= 3.0 ? 3U : 1U;
            editor->outline_start = editor->outline_start > step ?
                editor->outline_start - step : 0U;
        }
    } else if (!modern && x >= inspector_start) {
        size_t count = document_edit_attribute_count(editor->edit, editor->selected);
        size_t maximum = count > EDITOR_ATTRIBUTE_VISIBLE ? count - EDITOR_ATTRIBUTE_VISIBLE : 0U;
        if (delta_y < 0.0) {
            size_t step = delta_y <= -3.0 ? 3U : 1U;
            editor->attribute_start = editor->attribute_start + step > maximum ?
                maximum : editor->attribute_start + step;
        } else {
            size_t step = delta_y >= 3.0 ? 3U : 1U;
            editor->attribute_start = editor->attribute_start > step ?
                editor->attribute_start - step : 0U;
        }
    } else return false;
    editor->tree_ready = false;
    return true;
}

bool viewer_editor_semantics(viewer_editor *editor,
                             ui_semantics_snapshot *out_snapshot)
{
    if (editor == NULL || out_snapshot == NULL) return false;
    if (!editor->tree_ready) editor_build_tree(editor);
    return editor->tree_ready && ui_semantics_snapshot_build(&editor->tree, out_snapshot);
}

void editor_build_tree(viewer_editor *editor)
{
    if (editor == NULL) return;
    if (editor->modern_style) editor_build_tree_modern(editor);
    else editor_build_tree_legacy(editor);
    if (!editor->tree_ready) return;
    for (size_t index = 0U; index < editor->tree.count; ++index) {
        ui_node *node = &editor->tree.nodes[index];
        if (node->spec.id == editor->hovered_id)
            node->spec.state |= UI_STATE_HOVERED;
        if (node->spec.id == editor->pressed_id)
            node->spec.state |= UI_STATE_PRESSED;
    }
}
