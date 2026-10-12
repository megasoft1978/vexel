#include "viewer_editor_internal.h"

#include "viewer_chrome.h"
#include "ui_widgets.h"
#include "platform/macos/platform_accessibility.h"
#include "platform/platform_localization.h"
#include "platform/platform_text_ui.h"

#include <math.h>
#include <string.h>

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

static bool scale_metric(uint32_t value, double scale, uint32_t *out_value)
{
    if (out_value == NULL || !isfinite(scale) || scale <= 0.0) return false;
    if (value == 0U) {
        *out_value = 0U;
        return true;
    }
    double scaled = ceil((double)value * scale);
    if (!isfinite(scaled) || scaled < 1.0 || scaled > (double)UINT32_MAX)
        return false;
    *out_value = (uint32_t)scaled;
    return true;
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
    if (role == UI_ROLE_SLIDER) return ui_widget_add_slider(&editor->tree, &spec);
    if (role == UI_ROLE_ROW) return ui_widget_add_row(&editor->tree, &spec);
    bool added = ui_tree_add(&editor->tree, &(ui_node_spec){.id = id, .parent_id = parent,
        .axis = axis, .preferred_width = width, .preferred_height = height,
        .padding = padding, .gap = gap, .color = color, .role = role,
        .widget = widget, .icon = icon,
        .label = label, .label_len = label_len, .value = value,
        .value_len = value_len, .state = state, .command_id = command_id});
    return added;
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
    if (viewer_editor_native_material_enabled(editor)) {
        size_t row_bytes = (size_t)editor->width * 4U;
        double scale = editor->backing_scale *
            ((double)viewer_editor_ui_scale_percent(editor) / 100.0);
        double raw_height = ceil(40.0 * scale);
        if (row_bytes <= stride && stride > 0U &&
            (size_t)editor->height <= capacity / stride && isfinite(raw_height) &&
            raw_height > 0.0) {
            uint32_t height = raw_height >= (double)editor->height ? editor->height :
                (uint32_t)raw_height;
            for (uint32_t y = 0U; y < height; ++y)
                memset(rgba + (size_t)y * stride, 0, row_bytes);
        }
    }
    if (!editor->tree_ready) editor_build_tree(editor);
    if (!editor->tree_ready) return;
    bool painted = editor->modern_style ?
        ui_tree_paint_backgrounds_into(&editor->tree, rgba, capacity, stride) :
        ui_tree_paint_into(&editor->tree, rgba, capacity, stride);
    if (!painted) return;
    if (editor->modern_style) {
        editor_paint_widgets(editor, rgba, capacity, stride);
        editor_paint_snap_popover(editor, rgba, capacity, stride);
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
    if (action == EDITOR_BUTTON_HISTORY)
        return viewer_editor_open_history(editor) ? VIEWER_EDITOR_HANDLED : 0U;
    if (action == EDITOR_BUTTON_ZOOM_HUD) {
        editor->zoom_menu_open = !editor->zoom_menu_open;
        editor->tree_ready = false;
        editor->redraw_requested = true;
        return VIEWER_EDITOR_HANDLED;
    }
    if (action >= EDITOR_BUTTON_ZOOM_PRESET_BASE &&
        action < EDITOR_BUTTON_ZOOM_PRESET_BASE + 6U) {
        editor->zoom_preset_request = action - EDITOR_BUTTON_ZOOM_PRESET_BASE;
        editor->zoom_preset_pending = true;
        editor->zoom_menu_open = false;
        editor->tree_ready = false;
        editor->redraw_requested = true;
        return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_ZOOM_PRESET_REQUEST;
    }
    if (action == EDITOR_BUTTON_NODE_CORNER)
        return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_NODE_CORNER;
    if (action == EDITOR_BUTTON_NODE_SMOOTH)
        return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_NODE_SMOOTH;
    if (action == EDITOR_BUTTON_DELETE && editor->node_edit_active) {
        if (editor->path_selected_node_segment == SIZE_MAX ||
            editor->path_node_is_initial) {
            editor_status_ok(editor,
                "The initial move-to anchor is protected");
            return VIEWER_EDITOR_HANDLED;
        }
        return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_DELETE_PATH_NODE;
    }
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
    if (action == EDITOR_BUTTON_SNAPPING) {
        editor->snap_popover_open = !editor->snap_popover_open;
        editor->snap_popover_hovered = false;
        editor->snap_popover_hovered_row = UINT8_MAX;
        editor->tree_ready = false;
        editor->redraw_requested = true;
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
        (void)editor_canvas_path_node_mode_enter(editor);
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
    if (action >= EDITOR_BUTTON_WORKSPACE_BASE &&
        action < EDITOR_BUTTON_WORKSPACE_BASE + (uint32_t)viewer_workspace_count()) {
        (void)viewer_editor_set_workspace(editor,
            (viewer_workspace_kind)(action - EDITOR_BUTTON_WORKSPACE_BASE));
        return VIEWER_EDITOR_HANDLED;
    }
    if (action >= EDITOR_BUTTON_INSPECTOR_TAB_OBJECT &&
        action <= EDITOR_BUTTON_INSPECTOR_TAB_DOCUMENT) {
        editor->inspector_tab = (uint8_t)(action - EDITOR_BUTTON_INSPECTOR_TAB_OBJECT);
        editor->tree_ready = false;
        editor->redraw_requested = true;
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
    if (action == EDITOR_BUTTON_TOGGLE_TIMELINE) {
        if (editor->timeline_available)
            (void)viewer_editor_set_timeline_state(editor, true,
                !editor->timeline_visible, editor->timeline_height_points);
        return VIEWER_EDITOR_HANDLED;
    }
    if (action == EDITOR_BUTTON_FOCUS_GROUP) {
        (void)editor_focus_scope_toggle(editor);
        return VIEWER_EDITOR_HANDLED;
    }
    if (action == EDITOR_BUTTON_RESET_FOCUS) {
        editor_focus_scope_reset(editor);
        return VIEWER_EDITOR_HANDLED;
    }
    if (action == EDITOR_BUTTON_INSPECTOR_GEOMETRY ||
        action == EDITOR_BUTTON_INSPECTOR_APPEARANCE ||
        action == EDITOR_BUTTON_INSPECTOR_OTHER_ATTRIBUTES) {
        if (action == EDITOR_BUTTON_INSPECTOR_GEOMETRY)
            editor->inspector_geometry_expanded = !editor->inspector_geometry_expanded;
        else if (action == EDITOR_BUTTON_INSPECTOR_APPEARANCE)
            editor->inspector_appearance_expanded = !editor->inspector_appearance_expanded;
        else editor->inspector_other_attributes_expanded =
            !editor->inspector_other_attributes_expanded;
        editor->tree_ready = false;
        editor->redraw_requested = true;
        return VIEWER_EDITOR_HANDLED;
    }
    if (action == EDITOR_BUTTON_COPY_FULL_ID)
        return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_COPY_FULL_ID;
    if (action == EDITOR_BUTTON_FIT_CANVAS)
        return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_FIT_CANVAS;
    if (action == EDITOR_BUTTON_ACTUAL_SIZE)
        return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_ACTUAL_SIZE;
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

uint32_t viewer_editor_pointer_down_click_count(viewer_editor *editor,
    uint32_t x, uint32_t y, uint8_t button, uint8_t click_count)
{
    if (editor == NULL || button != 0U) return 0U;
    if (!editor->tree_ready) editor_build_tree(editor);
    if (!editor->tree_ready) return 0U;
    if (editor_snap_popover_pointer_down(editor, x, y))
        return VIEWER_EDITOR_HANDLED;
    if (!editor->tree_ready) editor_build_tree(editor);
    if (!editor->tree_ready) return 0U;
    if (editor_splitter_pointer_down(editor, x, y, click_count))
        return VIEWER_EDITOR_HANDLED;
    const ui_node *hit = NULL;
    for (size_t index = editor->tree.count; index > 0U; --index) {
        const ui_node *candidate = &editor->tree.nodes[index - 1U];
        if (editor_node_contains(candidate, x, y)) { hit = candidate; break; }
    }
    if (editor->zoom_menu_open) {
        uint32_t command = hit == NULL ? 0U : hit->spec.command_id;
        bool zoom_target = command == EDITOR_BUTTON_ZOOM_HUD ||
            (command >= EDITOR_BUTTON_ZOOM_PRESET_BASE &&
             command < EDITOR_BUTTON_ZOOM_PRESET_BASE + 6U) ||
            (hit != NULL && hit->spec.id == EDITOR_NODE_CANVAS_ZOOM_MENU);
        if (!zoom_target) {
            editor->zoom_menu_open = false;
            editor->tree_ready = false;
            editor->redraw_requested = true;
        }
    }
    if (hit == NULL || hit->spec.id == EDITOR_NODE_CANVAS ||
        hit->spec.id == EDITOR_NODE_WORKSPACE || hit->spec.id == UI_ROOT_ID) return 0U;
    uint32_t overlay_result = editor_overlay_pointer_down(editor,
        hit->spec.id, hit->spec.command_id);
    if (overlay_result != 0U) return overlay_result;
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
        size_t scale = (size_t)ceil(editor->backing_scale > 0.0 ?
            editor->backing_scale : 1.0);
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

uint32_t viewer_editor_pointer_down(viewer_editor *editor, uint32_t x, uint32_t y,
                                    uint8_t button)
{
    return viewer_editor_pointer_down_click_count(editor, x, y, button, 1U);
}

bool viewer_editor_scroll(viewer_editor *editor, uint32_t x, double delta_y)
{
    return viewer_editor_scroll_at(editor, x, editor == NULL ? 0U : editor->hovered_y,
        delta_y);
}

bool viewer_editor_scroll_at(viewer_editor *editor, uint32_t x, uint32_t y,
                             double delta_y)
{
    if (editor == NULL || delta_y == 0.0) return false;
    if (editor_overlay_scroll(editor, x, y, delta_y)) return true;
    if (editor_reveal_scroll(editor, x, y, delta_y)) return true;
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
    editor_overlay_add_nodes(editor);
    editor_overlay_layout_nodes(editor);
    for (size_t index = 0U; index < editor->tree.count; ++index) {
        ui_node *node = &editor->tree.nodes[index];
        if (node->spec.id == editor->hovered_id)
            node->spec.state |= UI_STATE_HOVERED;
        if (node->spec.id == editor->pressed_id)
            node->spec.state |= UI_STATE_PRESSED;
    }
}
