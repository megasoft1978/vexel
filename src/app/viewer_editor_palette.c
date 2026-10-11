#include "viewer_editor_internal.h"

#include "ui_widgets.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

static const char *disabled_reason(viewer_editor *editor, viewer_command_id id)
{
    size_t cursor = document_edit_history_cursor(editor->edit);
    size_t count = document_edit_history_count(editor->edit);
    bool selected = editor->canvas_selection_count > 0U ||
        (editor->selection_active && editor->selected_index > 0U);
    switch (id) {
        case VIEWER_COMMAND_UNDO: return cursor == 0U ? "Nothing to undo" : NULL;
        case VIEWER_COMMAND_REDO: return cursor >= count ? "Nothing to redo" : NULL;
        case VIEWER_COMMAND_DELETE:
        case VIEWER_COMMAND_DUPLICATE:
        case VIEWER_COMMAND_COPY:
        case VIEWER_COMMAND_CUT:
        case VIEWER_COMMAND_BRING_FORWARD:
        case VIEWER_COMMAND_SEND_BACKWARD:
        case VIEWER_COMMAND_BRING_TO_FRONT:
        case VIEWER_COMMAND_SEND_TO_BACK:
            return selected ? NULL : "Select a layer first";
        case VIEWER_COMMAND_GROUP:
            return editor->canvas_selection_count >= 2U ? NULL : "Select 2+ layers";
        case VIEWER_COMMAND_UNGROUP: {
            base_bytes tag = {0};
            bool group = selected && document_edit_tag_name(editor->edit,
                editor->selected, &tag) && tag.len == 1U && tag.ptr[0] == 'g';
            return group ? NULL : "Select a group first";
        }
        case VIEWER_COMMAND_ALIGN_LEFT:
        case VIEWER_COMMAND_ALIGN_CENTER:
        case VIEWER_COMMAND_ALIGN_RIGHT:
        case VIEWER_COMMAND_ALIGN_TOP:
        case VIEWER_COMMAND_ALIGN_MIDDLE:
        case VIEWER_COMMAND_ALIGN_BOTTOM:
        case VIEWER_COMMAND_DISTRIBUTE_HORIZONTAL:
        case VIEWER_COMMAND_DISTRIBUTE_VERTICAL:
            return editor->canvas_selection_count >= 2U ? NULL : "Select 2+ layers";
        default: return NULL;
    }
}

static void palette_refresh(viewer_editor *editor)
{
    editor->palette_result_count = viewer_command_search(editor->palette_query,
        editor->palette_query_length, editor->palette_results,
        UI_MAX_COMMANDS);
    if (editor->palette_result_count == 0U) {
        editor->palette_selected_index = 0U;
        editor->palette_first_visible = 0U;
    } else {
        if (editor->palette_selected_index >= editor->palette_result_count)
            editor->palette_selected_index = editor->palette_result_count - 1U;
        if (editor->palette_first_visible >= editor->palette_result_count)
            editor->palette_first_visible = editor->palette_result_count - 1U;
    }
}

bool viewer_editor_open_command_palette(viewer_editor *editor)
{
    if (editor == NULL) return false;
    editor->zoom_menu_open = false;
    if (!editor->command_palette_open) editor->overlay_return_focus = editor->focus_id;
    editor->history_popover_open = false;
    editor->command_palette_open = true;
    editor->palette_query[0] = '\0';
    editor->palette_query_length = 0U;
    editor->palette_cursor = 0U;
    editor->palette_anchor = 0U;
    editor->palette_selected_index = 0U;
    editor->palette_first_visible = 0U;
    editor->palette_visible_capacity = 0U;
    editor->focus_id = EDITOR_PALETTE_SEARCH;
    palette_refresh(editor);
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return true;
}

bool viewer_editor_open_history(viewer_editor *editor)
{
    if (editor == NULL) return false;
    editor->zoom_menu_open = false;
    if (!editor->history_popover_open) editor->overlay_return_focus = editor->focus_id;
    editor->command_palette_open = false;
    editor->history_popover_open = true;
    editor->history_visible_capacity = 0U;
    editor->history_viewport_initialized = false;
    editor->history_selected_cursor = document_edit_history_cursor(editor->edit);
    editor->history_first_visible = 0U;
    editor->focus_id = EDITOR_HISTORY_ROW_BASE;
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return true;
}

bool viewer_editor_command_palette_is_open(const viewer_editor *editor)
{
    return editor != NULL && editor->command_palette_open;
}

bool viewer_editor_history_is_open(const viewer_editor *editor)
{
    return editor != NULL && editor->history_popover_open;
}

bool viewer_editor_take_pending_command(viewer_editor *editor,
                                        viewer_command_id *out_command)
{
    if (editor == NULL || out_command == NULL || !editor->pending_command_valid)
        return false;
    *out_command = editor->pending_command;
    editor->pending_command = VIEWER_COMMAND_NONE;
    editor->pending_command_valid = false;
    return true;
}

bool viewer_editor_toggle_snapping(viewer_editor *editor)
{
    if (editor == NULL) return false;
    editor->snap_enabled = !editor->snap_enabled;
    editor->snap_guide_x_visible = false;
    editor->snap_guide_y_visible = false;
    editor_status_ok(editor, editor->snap_enabled ? "Snapping on" : "Snapping off");
    editor->redraw_requested = true;
    return true;
}

static void close_overlays(viewer_editor *editor)
{
    if (editor == NULL) return;
    editor->command_palette_open = false;
    editor->history_popover_open = false;
    editor->focus_id = editor->overlay_return_focus;
    editor->tree_ready = false;
    editor->redraw_requested = true;
}

static size_t selected_query_range(const viewer_editor *editor,
                                   size_t *out_start, size_t *out_end)
{
    *out_start = editor->palette_cursor < editor->palette_anchor ?
        editor->palette_cursor : editor->palette_anchor;
    *out_end = editor->palette_cursor > editor->palette_anchor ?
        editor->palette_cursor : editor->palette_anchor;
    return *out_end - *out_start;
}

static uint32_t palette_insert(viewer_editor *editor, const char *text,
                               size_t text_len)
{
    if (text == NULL || text_len == 0U) return VIEWER_EDITOR_HANDLED;
    for (size_t index = 0U; index < text_len; ++index)
        if ((uint8_t)text[index] < 0x20U || (uint8_t)text[index] == 0x7fU)
            return VIEWER_EDITOR_HANDLED;
    size_t start = 0U, end = 0U;
    size_t removed = selected_query_range(editor, &start, &end);
    size_t retained = editor->palette_query_length - removed;
    size_t allowed = (EDITOR_PALETTE_QUERY_CAP - 1U) - retained;
    size_t accepted = editor_utf8_prefix((const uint8_t *)text, text_len, allowed);
    if (accepted == 0U) return VIEWER_EDITOR_HANDLED;
    memmove(editor->palette_query + start + accepted,
        editor->palette_query + end, editor->palette_query_length - end);
    memcpy(editor->palette_query + start, text, accepted);
    editor->palette_query_length = retained + accepted;
    editor->palette_query[editor->palette_query_length] = '\0';
    editor->palette_cursor = start + accepted;
    editor->palette_anchor = editor->palette_cursor;
    editor->palette_selected_index = 0U;
    editor->palette_first_visible = 0U;
    editor->focus_id = EDITOR_PALETTE_SEARCH;
    palette_refresh(editor);
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return VIEWER_EDITOR_HANDLED;
}

static uint32_t palette_backspace(viewer_editor *editor)
{
    size_t start = 0U, end = 0U;
    if (selected_query_range(editor, &start, &end) == 0U) {
        if (start == 0U) return VIEWER_EDITOR_HANDLED;
        end = start;
        --start;
        while (start > 0U &&
            (((uint8_t)editor->palette_query[start] & 0xc0U) == 0x80U)) --start;
    }
    memmove(editor->palette_query + start, editor->palette_query + end,
        editor->palette_query_length - end);
    editor->palette_query_length -= end - start;
    editor->palette_query[editor->palette_query_length] = '\0';
    editor->palette_cursor = editor->palette_anchor = start;
    editor->palette_selected_index = 0U;
    editor->palette_first_visible = 0U;
    palette_refresh(editor);
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return VIEWER_EDITOR_HANDLED;
}

static void palette_move_selection(viewer_editor *editor, bool down)
{
    if (editor->palette_result_count == 0U) return;
    if (down) {
        if (editor->palette_selected_index + 1U < editor->palette_result_count)
            ++editor->palette_selected_index;
    } else if (editor->palette_selected_index > 0U) {
        --editor->palette_selected_index;
    }
    if (editor->palette_selected_index < editor->palette_first_visible)
        editor->palette_first_visible = editor->palette_selected_index;
    size_t visible_capacity = editor->palette_visible_capacity > 0U ?
        editor->palette_visible_capacity : 1U;
    size_t visible_end = editor->palette_first_visible + visible_capacity;
    if (editor->palette_selected_index >= visible_end)
        editor->palette_first_visible = editor->palette_selected_index -
            visible_capacity + 1U;
    editor->focus_id = EDITOR_PALETTE_RESULT_BASE + (uint32_t)(
        editor->palette_selected_index - editor->palette_first_visible);
    editor->tree_ready = false;
    editor->redraw_requested = true;
}

static uint32_t palette_choose(viewer_editor *editor, viewer_command_id id)
{
    const char *reason = disabled_reason(editor, id);
    if (reason != NULL) {
        editor_status_ok(editor, reason);
        return VIEWER_EDITOR_HANDLED;
    }
    editor->pending_command = id;
    editor->pending_command_valid = true;
    close_overlays(editor);
    return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_COMMAND_SELECTED;
}

static uint32_t history_choose(viewer_editor *editor, size_t cursor)
{
    if (cursor == document_edit_history_cursor(editor->edit)) {
        close_overlays(editor);
        return VIEWER_EDITOR_HANDLED;
    }
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id),
        editor->selected_has_id ? editor->selected_id : "");
    document_edit_error error = document_edit_history_jump(editor->edit, cursor);
    uint32_t result = editor_apply_result(editor, error, old_offset, old_id,
        (document_handle){0});
    close_overlays(editor);
    return result;
}

uint32_t editor_overlay_key(viewer_editor *editor, uint16_t key_code,
                            uint32_t modifiers, const char *text,
                            size_t text_len)
{
    if (editor == NULL || (!editor->command_palette_open &&
            !editor->history_popover_open)) return 0U;
    bool command = (modifiers & PLATFORM_MODIFIER_COMMAND) != 0U;
    int character = text_len > 0U ? (unsigned char)text[0] : -1;
    if (key_code == 53U) { close_overlays(editor); return VIEWER_EDITOR_HANDLED; }
    if (editor->history_popover_open) {
        size_t count = document_edit_history_count(editor->edit);
        if (key_code == 126U || key_code == 125U) {
            if (key_code == 126U && editor->history_selected_cursor > 0U)
                --editor->history_selected_cursor;
            if (key_code == 125U && editor->history_selected_cursor < count)
                ++editor->history_selected_cursor;
            size_t index = editor->history_selected_cursor;
            size_t visible_capacity = editor->history_visible_capacity > 0U ?
                editor->history_visible_capacity : EDITOR_HISTORY_VISIBLE_MAX;
            if (index < editor->history_first_visible)
                editor->history_first_visible = index;
            if (index >= editor->history_first_visible + visible_capacity)
                editor->history_first_visible = index - visible_capacity + 1U;
            editor->focus_id = EDITOR_HISTORY_ROW_BASE + (uint32_t)(
                index - editor->history_first_visible);
            editor->tree_ready = false;
            editor->redraw_requested = true;
            return VIEWER_EDITOR_HANDLED;
        }
        if (key_code == 36U || key_code == 76U)
            return history_choose(editor, editor->history_selected_cursor);
        return VIEWER_EDITOR_HANDLED;
    }
    if (command && (character == 'k' || character == 'K')) {
        close_overlays(editor);
        return VIEWER_EDITOR_HANDLED;
    }
    if (key_code == 126U || key_code == 125U) {
        palette_move_selection(editor, key_code == 125U);
        return VIEWER_EDITOR_HANDLED;
    }
    if (key_code == 36U || key_code == 76U) {
        if (editor->palette_result_count == 0U) return VIEWER_EDITOR_HANDLED;
        return palette_choose(editor,
            editor->palette_results[editor->palette_selected_index]);
    }
    if (key_code == 48U) {
        if (editor->palette_result_count > 0U) {
            size_t slot = editor->palette_selected_index - editor->palette_first_visible;
            if (slot < EDITOR_PALETTE_VISIBLE_MAX)
                editor->focus_id = EDITOR_PALETTE_RESULT_BASE + (uint32_t)slot;
        }
        editor->tree_ready = false;
        return VIEWER_EDITOR_HANDLED;
    }
    if (command && (character == 'a' || character == 'A')) {
        editor->palette_anchor = 0U;
        editor->palette_cursor = editor->palette_query_length;
        editor->tree_ready = false;
        return VIEWER_EDITOR_HANDLED;
    }
    if (key_code == 51U || key_code == 117U) return palette_backspace(editor);
    if (text_len > 0U && !command &&
        (modifiers & (PLATFORM_MODIFIER_CONTROL | PLATFORM_MODIFIER_OPTION)) == 0U)
        return palette_insert(editor, text, text_len);
    return VIEWER_EDITOR_HANDLED;
}

static bool add_overlay_node(viewer_editor *editor, uint32_t id, uint32_t parent,
    uint32_t width, uint32_t height, ui_role role, ui_widget_kind widget,
    const char *label, size_t label_length, const char *value, size_t value_length,
    uint32_t state, uint32_t command_id)
{
    return editor_add_widget_node(editor, id, parent, UI_AXIS_OVERLAY,
        width, height, 0U, 0U, 0U, role, widget, UI_ICON_NONE,
        label, label_length, value, value_length, state, command_id);
}

static ui_node *overlay_node(viewer_editor *editor, uint32_t id)
{
    for (size_t index = 1U; index < editor->tree.count; ++index)
        if (editor->tree.nodes[index].spec.id == id) return &editor->tree.nodes[index];
    return NULL;
}

static void set_node_bounds(viewer_editor *editor, uint32_t id, ui_rect bounds)
{
    ui_node *node = overlay_node(editor, id);
    if (node != NULL) node->bounds = bounds;
}

static uint32_t scaled_px(double points, double scale)
{
    double pixels = ceil(points * scale);
    if (!isfinite(pixels) || pixels <= 0.0 || pixels > (double)UINT32_MAX) return 0U;
    return (uint32_t)pixels;
}

static size_t palette_rows_for_height(const viewer_editor *editor)
{
    double scale = editor->backing_scale > 0.0 ? editor->backing_scale : 1.0;
    double height_points = (double)editor->height / scale;
    if (height_points <= 170.0) return 1U;
    size_t rows = (size_t)((height_points - 146.0) / 36.0);
    if (rows == 0U) rows = 1U;
    return rows < EDITOR_PALETTE_VISIBLE_MAX ? rows : EDITOR_PALETTE_VISIBLE_MAX;
}

static size_t history_rows_for_height(const viewer_editor *editor)
{
    double scale = editor->backing_scale > 0.0 ? editor->backing_scale : 1.0;
    double height_points = (double)editor->height / scale;
    if (height_points <= 150.0) return 1U;
    size_t rows = (size_t)((height_points - 132.0) / 30.0);
    if (rows == 0U) rows = 1U;
    return rows < EDITOR_HISTORY_VISIBLE_MAX ? rows : EDITOR_HISTORY_VISIBLE_MAX;
}

static bool add_palette_nodes(viewer_editor *editor)
{
    size_t visible_limit = palette_rows_for_height(editor);
    size_t free_nodes = editor->tree.count < UI_MAX_NODES ?
        UI_MAX_NODES - editor->tree.count : 0U;
    size_t node_limit = free_nodes > 6U ? free_nodes - 6U : 0U;
    if (visible_limit > node_limit) visible_limit = node_limit;
    editor->palette_visible_capacity = visible_limit;
    size_t visible_count = editor->palette_result_count > editor->palette_first_visible ?
        editor->palette_result_count - editor->palette_first_visible : 0U;
    if (visible_count > visible_limit) visible_count = visible_limit;
    if (editor->palette_result_count > 0U && visible_count == 0U) return false;
    size_t nodes_needed = 6U + (visible_count == 0U ? 1U : visible_count);
    if (editor->tree.count > UI_MAX_NODES - nodes_needed) return false;
    bool laid_out = editor->tree.laid_out;
    editor->tree.laid_out = false;
    bool okay = add_overlay_node(editor, EDITOR_PALETTE_BACKDROP, UI_ROOT_ID,
            0U, 0U, UI_ROLE_GROUP, UI_WIDGET_AUTO, NULL, 0U, NULL, 0U, 0U, 0U) &&
        add_overlay_node(editor, EDITOR_PALETTE_PANEL, UI_ROOT_ID,
            560U, 480U, UI_ROLE_GROUP, UI_WIDGET_BUTTON_SECONDARY,
            NULL, 0U, NULL, 0U, 0U, 0U) &&
        add_overlay_node(editor, EDITOR_PALETTE_TITLE, EDITOR_PALETTE_PANEL,
            520U, 24U, UI_ROLE_TEXT, UI_WIDGET_LABEL,
            "Search Commands", 15U, NULL, 0U, 0U, 0U) &&
        add_overlay_node(editor, EDITOR_PALETTE_SEARCH, EDITOR_PALETTE_PANEL,
            520U, 36U, UI_ROLE_TEXT_FIELD, UI_WIDGET_TEXT_FIELD,
            "Search commands", 15U, editor->palette_query,
            editor->palette_query_length, UI_STATE_FOCUSED, 0U) &&
        add_overlay_node(editor, EDITOR_PALETTE_GROUPS, EDITOR_PALETTE_PANEL,
            520U, 20U, UI_ROLE_TEXT, UI_WIDGET_LABEL,
            "Tools · Selection · Canvas · Document",
            strlen("Tools · Selection · Canvas · Document"), NULL, 0U, 0U, 0U);
    if (okay && visible_count == 0U) {
        okay = add_overlay_node(editor, EDITOR_PALETTE_RESULT_BASE,
            EDITOR_PALETTE_PANEL, 520U, 34U, UI_ROLE_TEXT, UI_WIDGET_LABEL,
            "No matching commands", 21U, NULL, 0U, 0U, 0U);
    }
    for (size_t slot = 0U; okay && slot < visible_count; ++slot) {
        size_t result_index = editor->palette_first_visible + slot;
        viewer_command_id id = editor->palette_results[result_index];
        const viewer_command_descriptor *command = viewer_command_find(id);
        const char *reason = disabled_reason(editor, id);
        char row[UI_WIDGET_LABEL_MAX_BYTES + 1U] = {0};
        okay = command != NULL && viewer_command_format_palette_row(command,
            reason, row, sizeof(row)) && add_overlay_node(editor,
            EDITOR_PALETTE_RESULT_BASE + (uint32_t)slot, EDITOR_PALETTE_PANEL,
            520U, 34U, UI_ROLE_BUTTON, UI_WIDGET_BUTTON_SECONDARY,
            row, strlen(row), NULL, 0U,
            (result_index == editor->palette_selected_index ? UI_STATE_CHECKED : 0U) |
                (reason != NULL ? UI_STATE_DISABLED : 0U), (uint32_t)id);
    }
    if (okay) okay = add_overlay_node(editor, EDITOR_PALETTE_FOOTER,
        EDITOR_PALETTE_PANEL, 520U, 20U, UI_ROLE_TEXT, UI_WIDGET_LABEL,
        "↑ ↓ Navigate · Return Run · Esc Close",
        strlen("↑ ↓ Navigate · Return Run · Esc Close"), NULL, 0U, 0U, 0U);
    editor->tree.laid_out = laid_out;
    return okay;
}

static void history_summary(const viewer_editor *editor, bool saved,
                            char output[UI_TEXT_MAX_BYTES])
{
    size_t index = saved ? 0U : document_edit_history_cursor(editor->edit);
    if (saved && !document_edit_history_saved_cursor(editor->edit, &index)) {
        (void)snprintf(output, UI_TEXT_MAX_BYTES, "Saved: Outside retained history");
        return;
    }
    if (index == 0U) {
        (void)snprintf(output, UI_TEXT_MAX_BYTES, "%s: Start",
            saved ? "Saved" : "Current");
        return;
    }
    document_edit_history_entry entry = {{0}, 0U, false, false};
    if (!document_edit_history_entry_at(editor->edit, index - 1U, &entry)) {
        (void)snprintf(output, UI_TEXT_MAX_BYTES, "%s: Start",
            saved ? "Saved" : "Current");
        return;
    }
    (void)snprintf(output, UI_TEXT_MAX_BYTES, "%s: %.48s",
        saved ? "Saved" : "Current", entry.label);
}

static size_t history_state_count(const viewer_editor *editor)
{
    size_t commands = document_edit_history_count(editor->edit);
    return commands == SIZE_MAX ? SIZE_MAX : commands + 1U;
}

static size_t history_row_count(const viewer_editor *editor)
{
    size_t state_count = history_state_count(editor);
    size_t remaining = state_count > editor->history_first_visible ?
        state_count - editor->history_first_visible : 0U;
    size_t limit = editor->history_visible_capacity;
    return remaining < limit ? remaining : limit;
}

static bool add_history_nodes(viewer_editor *editor)
{
    size_t state_count = history_state_count(editor);
    size_t limit = history_rows_for_height(editor);
    size_t free_nodes = editor->tree.count < UI_MAX_NODES ?
        UI_MAX_NODES - editor->tree.count : 0U;
    size_t node_limit = free_nodes > 6U ? free_nodes - 6U : 0U;
    editor->history_visible_capacity = limit < node_limit ? limit : node_limit;
    if (!editor->history_viewport_initialized) {
        editor->history_first_visible = state_count > editor->history_visible_capacity ?
            state_count - editor->history_visible_capacity : 0U;
        size_t current = editor->history_selected_cursor;
        if (current < editor->history_first_visible)
            editor->history_first_visible = current;
        editor->history_viewport_initialized = true;
    }
    size_t row_count = history_row_count(editor);
    if (editor->history_selected_cursor >= editor->history_first_visible &&
        editor->history_selected_cursor - editor->history_first_visible < row_count) {
        editor->focus_id = EDITOR_HISTORY_ROW_BASE + (uint32_t)(
            editor->history_selected_cursor - editor->history_first_visible);
    } else {
        editor->focus_id = 0U;
    }
    size_t nodes_needed = 6U + row_count;
    if (editor->tree.count > UI_MAX_NODES - nodes_needed) return false;
    bool laid_out = editor->tree.laid_out;
    editor->tree.laid_out = false;
    char current[UI_TEXT_MAX_BYTES] = {0}, saved[UI_TEXT_MAX_BYTES] = {0};
    history_summary(editor, false, current);
    history_summary(editor, true, saved);
    bool okay = add_overlay_node(editor, EDITOR_HISTORY_BACKDROP, UI_ROOT_ID,
            0U, 0U, UI_ROLE_GROUP, UI_WIDGET_AUTO, NULL, 0U, NULL, 0U, 0U, 0U) &&
        add_overlay_node(editor, EDITOR_HISTORY_PANEL, UI_ROOT_ID,
            390U, 760U, UI_ROLE_GROUP, UI_WIDGET_BUTTON_SECONDARY,
            NULL, 0U, NULL, 0U, 0U, 0U) &&
        add_overlay_node(editor, EDITOR_HISTORY_TITLE, EDITOR_HISTORY_PANEL,
            350U, 24U, UI_ROLE_TEXT, UI_WIDGET_LABEL,
            "Undo History", 12U, NULL, 0U, 0U, 0U) &&
        add_overlay_node(editor, EDITOR_HISTORY_CURRENT, EDITOR_HISTORY_PANEL,
            350U, 18U, UI_ROLE_TEXT, UI_WIDGET_LABEL,
            current, strlen(current), NULL, 0U, 0U, 0U) &&
        add_overlay_node(editor, EDITOR_HISTORY_SAVED, EDITOR_HISTORY_PANEL,
            350U, 18U, UI_ROLE_TEXT, UI_WIDGET_LABEL,
            saved, strlen(saved), NULL, 0U, 0U, 0U);
    for (size_t slot = 0U; okay && slot < row_count; ++slot) {
        size_t state_cursor = editor->history_first_visible + slot;
        size_t saved_cursor = 0U;
        bool saved_marker = document_edit_history_saved_cursor(editor->edit,
            &saved_cursor) && saved_cursor == state_cursor;
        bool current_marker = editor->history_selected_cursor == state_cursor;
        const char *label = "Start";
        document_edit_history_entry entry = {{0}, 0U, false, false};
        if (state_cursor > 0U) {
            if (!document_edit_history_entry_at(editor->edit,
                    state_cursor - 1U, &entry)) {
                okay = false;
                break;
            }
            label = entry.label;
        }
        char row[UI_WIDGET_LABEL_MAX_BYTES + 1U] = {0};
        int amount = snprintf(row, sizeof(row), "%s%s%s", label,
            current_marker ? " · Current" : "",
            saved_marker ? " · Saved" : "");
        okay = amount >= 0 && (size_t)amount < sizeof(row) &&
            add_overlay_node(editor, EDITOR_HISTORY_ROW_BASE + (uint32_t)slot,
                EDITOR_HISTORY_PANEL, 350U, 30U, UI_ROLE_BUTTON,
                UI_WIDGET_BUTTON_SECONDARY, row, (size_t)amount, NULL, 0U,
                current_marker ? UI_STATE_CHECKED : 0U, (uint32_t)state_cursor);
    }
    const char *footer = state_count > editor->history_visible_capacity ?
        "Scroll for older edits · Esc Close" : "Select a row to restore · Esc Close";
    if (okay) okay = add_overlay_node(editor, EDITOR_HISTORY_FOOTER,
        EDITOR_HISTORY_PANEL, 350U, 20U, UI_ROLE_TEXT, UI_WIDGET_LABEL,
        footer, strlen(footer), NULL, 0U, 0U, 0U);
    editor->tree.laid_out = laid_out;
    return okay;
}

void editor_overlay_add_nodes(viewer_editor *editor)
{
    if (editor == NULL || (!editor->command_palette_open &&
            !editor->history_popover_open)) return;
    palette_refresh(editor);
    bool okay = editor->command_palette_open ? add_palette_nodes(editor) :
        add_history_nodes(editor);
    if (!okay) {
        editor->command_palette_open = false;
        editor->history_popover_open = false;
        editor->focus_id = editor->overlay_return_focus;
        editor_status_ok(editor, "Overlay could not be shown within the UI limit");
        return;
    }
    ui_node *focused = overlay_node(editor, editor->focus_id);
    if (focused != NULL &&
        (focused->spec.state & UI_STATE_DISABLED) == 0U &&
        (focused->spec.role == UI_ROLE_BUTTON ||
         focused->spec.role == UI_ROLE_TEXT_FIELD)) {
        for (size_t index = 0U; index < editor->tree.count; ++index)
            editor->tree.nodes[index].spec.state &= ~(uint32_t)UI_STATE_FOCUSED;
        focused->spec.state |= UI_STATE_FOCUSED;
        editor->tree.focused_id = focused->spec.id;
    }
}

static void layout_palette(viewer_editor *editor)
{
    double scale = editor->backing_scale > 0.0 ? editor->backing_scale : 1.0;
    size_t visible_limit = editor->palette_visible_capacity;
    size_t count = editor->palette_result_count > editor->palette_first_visible ?
        editor->palette_result_count - editor->palette_first_visible : 0U;
    if (count > visible_limit) count = visible_limit;
    uint32_t margin = scaled_px(16.0, scale);
    uint32_t panel_width = scaled_px(560.0, scale);
    uint32_t max_width = editor->width > margin * 2U ? editor->width - margin * 2U : editor->width;
    if (panel_width > max_width) panel_width = max_width;
    uint32_t panel_height = scaled_px(146.0 + (double)(count == 0U ? 1U : count) * 36.0, scale);
    uint32_t max_height = editor->height > margin * 2U ? editor->height - margin * 2U : editor->height;
    if (panel_height > max_height) panel_height = max_height;
    uint32_t x = editor->width > panel_width ? (editor->width - panel_width) / 2U : 0U;
    uint32_t y = editor->height > panel_height ? (editor->height - panel_height) / 2U : 0U;
    ui_rect panel = {x, y, panel_width, panel_height};
    set_node_bounds(editor, EDITOR_PALETTE_BACKDROP,
        (ui_rect){0U, 0U, editor->width, editor->height});
    set_node_bounds(editor, EDITOR_PALETTE_PANEL, panel);
    set_node_bounds(editor, EDITOR_PALETTE_SURFACE, panel);
    set_node_bounds(editor, EDITOR_PALETTE_TITLE, (ui_rect){x + margin,
        y + scaled_px(10.0, scale), panel_width - margin * 2U,
        scaled_px(24.0, scale)});
    set_node_bounds(editor, EDITOR_PALETTE_SEARCH, (ui_rect){x + margin,
        y + scaled_px(38.0, scale), panel_width - margin * 2U,
        scaled_px(36.0, scale)});
    set_node_bounds(editor, EDITOR_PALETTE_GROUPS, (ui_rect){x + margin,
        y + scaled_px(78.0, scale), panel_width - margin * 2U,
        scaled_px(22.0, scale)});
    uint32_t row_y = y + scaled_px(106.0, scale);
    if (count == 0U) {
        set_node_bounds(editor, EDITOR_PALETTE_RESULT_BASE,
            (ui_rect){x + margin, row_y, panel_width - margin * 2U,
                scaled_px(34.0, scale)});
    }
    for (size_t slot = 0U; slot < count; ++slot)
        set_node_bounds(editor, EDITOR_PALETTE_RESULT_BASE + (uint32_t)slot,
            (ui_rect){x + margin, row_y + scaled_px((double)slot * 36.0, scale),
                panel_width - margin * 2U, scaled_px(34.0, scale)});
    set_node_bounds(editor, EDITOR_PALETTE_FOOTER, (ui_rect){x + margin,
        y + panel_height - scaled_px(28.0, scale), panel_width - margin * 2U,
        scaled_px(18.0, scale)});
}

static void layout_history(viewer_editor *editor)
{
    double scale = editor->backing_scale > 0.0 ? editor->backing_scale : 1.0;
    size_t rows = history_row_count(editor);
    uint32_t margin = scaled_px(12.0, scale);
    uint32_t panel_width = scaled_px(390.0, scale);
    uint32_t max_width = editor->width > margin * 2U ? editor->width - margin * 2U : editor->width;
    if (panel_width > max_width) panel_width = max_width;
    uint32_t panel_height = scaled_px(132.0 + (double)(rows == 0U ? 1U : rows) * 30.0, scale);
    uint32_t max_height = editor->height > margin * 2U ? editor->height - margin * 2U : editor->height;
    if (panel_height > max_height) panel_height = max_height;
    uint32_t x = scaled_px(48.0, scale);
    if (x + panel_width > editor->width) x = editor->width > panel_width ? editor->width - panel_width : 0U;
    uint32_t y = scaled_px(44.0, scale);
    if (y + panel_height > editor->height) y = editor->height > panel_height ? editor->height - panel_height : 0U;
    ui_rect panel = {x, y, panel_width, panel_height};
    set_node_bounds(editor, EDITOR_HISTORY_BACKDROP,
        (ui_rect){0U, 0U, editor->width, editor->height});
    set_node_bounds(editor, EDITOR_HISTORY_PANEL, panel);
    set_node_bounds(editor, EDITOR_HISTORY_SURFACE, panel);
    set_node_bounds(editor, EDITOR_HISTORY_TITLE, (ui_rect){x + margin,
        y + scaled_px(8.0, scale), panel_width - margin * 2U,
        scaled_px(24.0, scale)});
    set_node_bounds(editor, EDITOR_HISTORY_CURRENT, (ui_rect){x + margin,
        y + scaled_px(36.0, scale), panel_width - margin * 2U,
        scaled_px(18.0, scale)});
    set_node_bounds(editor, EDITOR_HISTORY_SAVED, (ui_rect){x + margin,
        y + scaled_px(56.0, scale), panel_width - margin * 2U,
        scaled_px(18.0, scale)});
    uint32_t row_y = y + scaled_px(82.0, scale);
    if (rows == 0U) {
        set_node_bounds(editor, EDITOR_HISTORY_ROW_BASE,
            (ui_rect){x + margin, row_y, panel_width - margin * 2U,
                scaled_px(30.0, scale)});
    }
    for (size_t slot = 0U; slot < rows; ++slot)
        set_node_bounds(editor, EDITOR_HISTORY_ROW_BASE + (uint32_t)slot,
            (ui_rect){x + margin, row_y + scaled_px((double)slot * 30.0, scale),
                panel_width - margin * 2U, scaled_px(28.0, scale)});
    set_node_bounds(editor, EDITOR_HISTORY_FOOTER, (ui_rect){x + margin,
        y + panel_height - scaled_px(26.0, scale), panel_width - margin * 2U,
        scaled_px(18.0, scale)});
}

void editor_overlay_layout_nodes(viewer_editor *editor)
{
    if (editor == NULL) return;
    if (editor->command_palette_open) layout_palette(editor);
    else if (editor->history_popover_open) layout_history(editor);
}

uint32_t editor_overlay_pointer_down(viewer_editor *editor, uint32_t id,
                                     uint32_t command_id)
{
    if (editor == NULL) return 0U;
    if (editor->command_palette_open) {
        if (id == EDITOR_PALETTE_BACKDROP) {
            close_overlays(editor);
            return VIEWER_EDITOR_HANDLED;
        }
        if (id == EDITOR_PALETTE_SEARCH) {
            editor->focus_id = EDITOR_PALETTE_SEARCH;
            editor->palette_cursor = editor->palette_query_length;
            editor->palette_anchor = editor->palette_cursor;
            editor->tree_ready = false;
            editor->redraw_requested = true;
            return VIEWER_EDITOR_HANDLED;
        }
        if (id >= EDITOR_PALETTE_RESULT_BASE &&
            id < EDITOR_PALETTE_RESULT_BASE + EDITOR_PALETTE_VISIBLE_MAX) {
            const ui_node *node = ui_tree_find(&editor->tree, id);
            if (node == NULL || (node->spec.state & UI_STATE_DISABLED) != 0U)
                return VIEWER_EDITOR_HANDLED;
            editor->palette_selected_index = editor->palette_first_visible +
                (size_t)(id - EDITOR_PALETTE_RESULT_BASE);
            return palette_choose(editor, (viewer_command_id)command_id);
        }
        if (id == EDITOR_PALETTE_PANEL || id == EDITOR_PALETTE_SURFACE ||
            id == EDITOR_PALETTE_TITLE || id == EDITOR_PALETTE_GROUPS ||
            id == EDITOR_PALETTE_FOOTER) return VIEWER_EDITOR_HANDLED;
    }
    if (editor->history_popover_open) {
        if (id == EDITOR_HISTORY_BACKDROP) {
            close_overlays(editor);
            return VIEWER_EDITOR_HANDLED;
        }
        if (id >= EDITOR_HISTORY_ROW_BASE &&
            id < EDITOR_HISTORY_ROW_BASE + EDITOR_HISTORY_VISIBLE_MAX) {
            editor->history_selected_cursor = command_id;
            return history_choose(editor, command_id);
        }
        if (id == EDITOR_HISTORY_PANEL || id == EDITOR_HISTORY_SURFACE ||
            id == EDITOR_HISTORY_TITLE || id == EDITOR_HISTORY_CURRENT ||
            id == EDITOR_HISTORY_SAVED || id == EDITOR_HISTORY_FOOTER)
            return VIEWER_EDITOR_HANDLED;
    }
    return 0U;
}

bool editor_overlay_scroll(viewer_editor *editor, uint32_t x, uint32_t y,
                           double delta_y)
{
    if (editor == NULL || delta_y == 0.0 ||
        (!editor->command_palette_open && !editor->history_popover_open)) return false;
    if (!editor->tree_ready) editor_build_tree(editor);
    uint32_t backdrop_id = editor->command_palette_open ?
        EDITOR_PALETTE_BACKDROP : EDITOR_HISTORY_BACKDROP;
    const ui_node *backdrop = ui_tree_find(&editor->tree, backdrop_id);
    if (backdrop == NULL || x < backdrop->bounds.x || y < backdrop->bounds.y ||
        x - backdrop->bounds.x >= backdrop->bounds.width ||
        y - backdrop->bounds.y >= backdrop->bounds.height) return false;
    if (editor->command_palette_open) {
        size_t capacity = editor->palette_visible_capacity > 0U ?
            editor->palette_visible_capacity : EDITOR_PALETTE_VISIBLE_MAX;
        size_t max_start = editor->palette_result_count > capacity ?
            editor->palette_result_count - capacity : 0U;
        if (delta_y < 0.0 && editor->palette_first_visible < max_start)
            ++editor->palette_first_visible;
        else if (delta_y > 0.0 && editor->palette_first_visible > 0U)
            --editor->palette_first_visible;
        if (editor->palette_selected_index < editor->palette_first_visible)
            editor->palette_selected_index = editor->palette_first_visible;
        editor->focus_id = EDITOR_PALETTE_RESULT_BASE + (uint32_t)(
            editor->palette_selected_index - editor->palette_first_visible);
        editor->tree_ready = false;
        return true;
    }
    size_t count = history_state_count(editor);
    size_t limit = editor->history_visible_capacity > 0U ?
        editor->history_visible_capacity : history_rows_for_height(editor);
    size_t max_start = count > limit ? count - limit : 0U;
    if (delta_y > 0.0 && editor->history_first_visible > 0U)
        --editor->history_first_visible;
    else if (delta_y < 0.0 && editor->history_first_visible < max_start)
        ++editor->history_first_visible;
    if (editor->history_selected_cursor >= editor->history_first_visible &&
        editor->history_selected_cursor - editor->history_first_visible < limit) {
        editor->focus_id = EDITOR_HISTORY_ROW_BASE + (uint32_t)(
            editor->history_selected_cursor - editor->history_first_visible);
    } else {
        editor->focus_id = 0U;
    }
    editor->tree_ready = false;
    return true;
}
