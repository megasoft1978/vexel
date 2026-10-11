#include "viewer_editor_internal.h"

#include <ctype.h>
#include <string.h>

static size_t utf8_previous(const char *text, size_t cursor)
{
    if (cursor == 0U) return 0U;
    size_t previous = cursor - 1U;
    while (previous > 0U && (((uint8_t)text[previous] & 0xc0U) == 0x80U)) --previous;
    return previous;
}

size_t editor_utf8_next(const char *text, size_t length, size_t cursor)
{
    if (cursor >= length) return length;
    size_t next = cursor + 1U;
    while (next < length && (((uint8_t)text[next] & 0xc0U) == 0x80U)) ++next;
    return next;
}

static void editor_sync_live_search(viewer_editor *editor)
{
    if (editor->field_kind != EDITOR_FIELD_SEARCH) return;
    size_t length = editor->field_length;
    if (length >= EDITOR_OUTLINE_FILTER_CAP)
        length = EDITOR_OUTLINE_FILTER_CAP - 1U;
    memcpy(editor->outline_filter, editor->field, length);
    editor->outline_filter[length] = '\0';
    editor->outline_filter_length = length;
    editor_rebuild_visible_outline(editor);
}

static bool editor_word_byte(uint8_t byte)
{
    return (byte >= 'a' && byte <= 'z') || (byte >= 'A' && byte <= 'Z') ||
        (byte >= '0' && byte <= '9') || byte >= 0x80U || byte == '_';
}

static size_t editor_word_left(const char *text, size_t cursor)
{
    while (cursor > 0U && !editor_word_byte((uint8_t)text[utf8_previous(text, cursor)]))
        cursor = utf8_previous(text, cursor);
    while (cursor > 0U && editor_word_byte((uint8_t)text[utf8_previous(text, cursor)]))
        cursor = utf8_previous(text, cursor);
    return cursor;
}

static size_t editor_word_right(const char *text, size_t length, size_t cursor)
{
    while (cursor < length && editor_word_byte((uint8_t)text[cursor]))
        cursor = editor_utf8_next(text, length, cursor);
    while (cursor < length && !editor_word_byte((uint8_t)text[cursor]))
        cursor = editor_utf8_next(text, length, cursor);
    return cursor;
}

void editor_selection_range(const viewer_editor *editor, size_t *start, size_t *end)
{
    *start = editor->cursor < editor->anchor ? editor->cursor : editor->anchor;
    *end = editor->cursor > editor->anchor ? editor->cursor : editor->anchor;
}

static bool editor_insert_text(viewer_editor *editor, const char *text, size_t length)
{
    if (text == NULL || length == 0U) return false;
    for (size_t i = 0U; i < length; ++i)
        if ((uint8_t)text[i] < 0x20U || text[i] == 0x7f) return false;
    size_t start, end;
    editor_selection_range(editor, &start, &end);
    size_t retained = editor->field_length - (end - start);
    size_t maximum = sizeof(editor->field) - 1U;
    if (editor->field_kind == EDITOR_FIELD_SEARCH)
        maximum = EDITOR_OUTLINE_FILTER_CAP - 1U;
    else if (editor->field_kind == EDITOR_FIELD_RENAME)
        maximum = EDITOR_NAME_CAP - 1U;
    if (retained > maximum || length > maximum - retained) {
        editor_status_ok(editor, editor->field_kind == EDITOR_FIELD_SEARCH ?
            "Search limit reached" : "Field limit reached");
        return false;
    }
    memmove(editor->field + start + length, editor->field + end,
        editor->field_length - end);
    memcpy(editor->field + start, text, length);
    editor->field_length = retained + length;
    editor->field[editor->field_length] = '\0';
    editor->cursor = start + length;
    editor->anchor = editor->cursor;
    editor_sync_live_search(editor);
    editor->tree_ready = false;
    return true;
}

static bool editor_delete_selection(viewer_editor *editor)
{
    size_t start, end;
    editor_selection_range(editor, &start, &end);
    if (start == end) return false;
    memmove(editor->field + start, editor->field + end,
        editor->field_length - end);
    editor->field_length -= end - start;
    editor->field[editor->field_length] = '\0';
    editor->cursor = editor->anchor = start;
    editor_sync_live_search(editor);
    editor->tree_ready = false;
    return true;
}

uint32_t editor_commit_field(viewer_editor *editor)
{
    if (editor->field_kind == EDITOR_FIELD_TEXT && editor->text_draft_active)
        return editor_commit_text_draft(editor);
    if ((editor->field_kind == EDITOR_FIELD_ATTRIBUTE ||
         editor->field_kind == EDITOR_FIELD_TEXT ||
         editor->field_kind == EDITOR_FIELD_RENAME) &&
        editor_outline_is_locked(editor, editor->selected)) {
        editor_status_ok(editor, "Unlock the selected layer to edit it");
        return VIEWER_EDITOR_HANDLED;
    }
    if (editor->field_kind == EDITOR_FIELD_NEW_NAME ||
        editor->field_kind == EDITOR_FIELD_NEW_VALUE) {
        editor_field_kind old_kind = editor->field_kind;
        if (!editor_store_pending_field(editor)) {
            editor_status_ok(editor, "Field limit reached");
            return VIEWER_EDITOR_HANDLED;
        }
        editor->field_kind = EDITOR_FIELD_NONE;
        if (old_kind == EDITOR_FIELD_NEW_NAME) {
            (void)editor_begin_field(editor, EDITOR_FIELD_NEW_VALUE, 0U);
            editor_status_ok(editor, "Enter the new attribute value");
        } else editor_status_ok(editor, "Attribute draft ready");
        return VIEWER_EDITOR_HANDLED;
    }
    if (editor->field_kind == EDITOR_FIELD_SEARCH) {
        editor_sync_live_search(editor);
        editor->field_kind = EDITOR_FIELD_NONE;
        editor->field_length = editor->cursor = editor->anchor = 0U;
        editor->focus_id = EDITOR_OUTLINE_SEARCH_ID;
        editor_status_ok(editor, "Layer search updated");
        return VIEWER_EDITOR_HANDLED;
    }
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ? editor->selected_id : "");
    document_edit_error result;
    if (editor->field_kind == EDITOR_FIELD_RENAME) {
        result = document_edit_set_attribute(editor->edit, editor->selected,
            (base_bytes){(const uint8_t *)"id", 2U},
            (base_bytes){(const uint8_t *)editor->field, editor->field_length});
        if (result == DOCUMENT_EDIT_OK) {
            editor->field_kind = EDITOR_FIELD_NONE;
            editor->field_length = editor->cursor = editor->anchor = 0U;
            return editor_apply_result(editor, result, old_offset, old_id, editor->selected);
        }
    } else if (editor->field_kind == EDITOR_FIELD_ATTRIBUTE) {
        result = document_edit_set_attribute(editor->edit, editor->selected,
            (base_bytes){(const uint8_t *)editor->selected_attribute_name,
                strlen(editor->selected_attribute_name)},
            (base_bytes){(const uint8_t *)editor->field, editor->field_length});
    } else if (editor->field_kind == EDITOR_FIELD_TEXT) {
        result = document_edit_set_text_content(editor->edit, editor->selected,
            (base_bytes){(const uint8_t *)editor->field, editor->field_length});
    } else return VIEWER_EDITOR_HANDLED;
    if (result == DOCUMENT_EDIT_OK)
        return editor_apply_result(editor, result, old_offset, old_id, (document_handle){0});
    editor_status_error(editor, result);
    return VIEWER_EDITOR_HANDLED;
}

void editor_cancel_field(viewer_editor *editor)
{
    editor->text_draft_active = false;
    if (editor->field_kind == EDITOR_FIELD_NEW_NAME ||
        editor->field_kind == EDITOR_FIELD_NEW_VALUE) {
        editor->new_name[0] = '\0'; editor->new_name_length = 0U;
        editor->new_value[0] = '\0'; editor->new_value_length = 0U;
    }
    if (editor->field_kind == EDITOR_FIELD_SEARCH) {
        memcpy(editor->outline_filter, editor->outline_filter_original,
            editor->outline_filter_original_length + 1U);
        editor->outline_filter_length = editor->outline_filter_original_length;
        editor_rebuild_visible_outline(editor);
    }
    editor->field_kind = EDITOR_FIELD_NONE;
    editor->field_length = editor->cursor = editor->anchor = 0U;
    editor_status_ok(editor, "Edit canceled");
}

uint32_t editor_history(viewer_editor *editor, bool redo)
{
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ? editor->selected_id : "");
    document_edit_error result = redo ? document_edit_redo(editor->edit) :
        document_edit_undo(editor->edit);
    return editor_apply_result(editor, result, old_offset, old_id, (document_handle){0});
}

static uint32_t editor_key_field(viewer_editor *editor, uint16_t key_code,
                                 uint32_t modifiers, const char *text, size_t text_len)
{
    bool command = (modifiers & PLATFORM_MODIFIER_COMMAND) != 0U;
    bool shift = (modifiers & PLATFORM_MODIFIER_SHIFT) != 0U;
    if (command && text_len > 0U) {
        int key = tolower((unsigned char)text[0]);
        if (key == 'a') {
            editor->anchor = 0U; editor->cursor = editor->field_length;
            editor->tree_ready = false;
            return VIEWER_EDITOR_HANDLED;
        }
        if (key == 'c' || key == 'x' || key == 'v') {
            editor_status_ok(editor, "Clipboard needs the platform text service");
            return VIEWER_EDITOR_HANDLED;
        }
        if (key == 'z') return editor_history(editor, shift);
        if (key == 'o' || key == 'q' || key == 'r') {
            uint32_t pending = VIEWER_EDITOR_HANDLED;
            if (editor->field_kind == EDITOR_FIELD_ATTRIBUTE ||
                editor->field_kind == EDITOR_FIELD_TEXT) {
                pending = editor_commit_field(editor);
                if (editor->field_kind != EDITOR_FIELD_NONE)
                    return VIEWER_EDITOR_HANDLED;
            }
            if (key == 'o') return pending | VIEWER_EDITOR_OPEN;
            if (key == 'q') return pending | VIEWER_EDITOR_QUIT;
            return pending | VIEWER_EDITOR_RELOAD;
        }
        if (key == 's') {
            editor_field_kind kind = editor->field_kind;
            uint32_t result = editor_commit_field(editor);
            if ((kind == EDITOR_FIELD_ATTRIBUTE || kind == EDITOR_FIELD_TEXT) &&
                (result & VIEWER_EDITOR_MUTATED) == 0U) return VIEWER_EDITOR_HANDLED;
            return result | (shift ? VIEWER_EDITOR_SAVE_AS : VIEWER_EDITOR_SAVE);
        }
    }
    if (key_code == 53U) { editor_cancel_field(editor); return VIEWER_EDITOR_HANDLED; }
    if (key_code == 36U || key_code == 76U) return editor_commit_field(editor);
    if (key_code == 48U) {
        uint32_t result = editor_commit_field(editor);
        if (editor->field_kind != EDITOR_FIELD_NONE)
            return result | VIEWER_EDITOR_HANDLED;
        editor->tree_ready = false;
        editor_build_tree(editor);
        if (ui_focus_next(&editor->tree, shift)) {
            editor->focus_id = editor->tree.focused_id;
            editor->tree_ready = false;
        }
        return result | VIEWER_EDITOR_HANDLED;
    }
    size_t next = editor->cursor;
    if (key_code == 123U) next = (modifiers & PLATFORM_MODIFIER_OPTION) != 0U ?
        editor_word_left(editor->field, editor->cursor) : utf8_previous(editor->field, editor->cursor);
    else if (key_code == 124U) next = (modifiers & PLATFORM_MODIFIER_OPTION) != 0U ?
        editor_word_right(editor->field, editor->field_length, editor->cursor) :
        editor_utf8_next(editor->field, editor->field_length, editor->cursor);
    else if (key_code == 115U) next = 0U;
    else if (key_code == 119U) next = editor->field_length;
    else if (key_code == 51U || key_code == 117U) {
        if (!editor_delete_selection(editor)) {
            if (key_code == 51U && editor->cursor > 0U) {
                editor->anchor = utf8_previous(editor->field, editor->cursor);
                editor_delete_selection(editor);
            } else if (key_code == 117U && editor->cursor < editor->field_length) {
                editor->anchor = editor_utf8_next(editor->field, editor->field_length, editor->cursor);
                editor_delete_selection(editor);
            }
        }
        return VIEWER_EDITOR_HANDLED;
    } else if (text_len > 0U && !command &&
               (modifiers & (PLATFORM_MODIFIER_CONTROL | PLATFORM_MODIFIER_OPTION)) == 0U) {
        (void)editor_insert_text(editor, text, text_len);
        return VIEWER_EDITOR_HANDLED;
    } else return VIEWER_EDITOR_HANDLED;
    if (!shift && editor->cursor != editor->anchor) {
        size_t start, end;
        editor_selection_range(editor, &start, &end);
        if (key_code == 123U) next = start;
        else if (key_code == 124U) next = end;
    }
    editor->cursor = next;
    if (!shift) editor->anchor = next;
    editor->tree_ready = false;
    return VIEWER_EDITOR_HANDLED;
}

uint32_t viewer_editor_key(viewer_editor *editor, uint16_t key_code,
                           uint32_t modifiers, const char *text, size_t text_len)
{
    if (editor == NULL || (text_len > 0U && text == NULL)) return 0U;
    if (key_code == 53U && editor->canvas_gesture != EDITOR_CANVAS_GESTURE_NONE)
        return viewer_editor_cancel_canvas_gesture(editor);
    if (editor->field_kind != EDITOR_FIELD_NONE)
        return editor_key_field(editor, key_code, modifiers, text, text_len);
    uint32_t focused_result = editor_handle_focused_key(editor, key_code,
        modifiers, text, text_len);
    if (focused_result != 0U) return focused_result;
    if (key_code == 120U) {
        (void)editor_begin_field(editor, EDITOR_FIELD_RENAME, 0U);
        return VIEWER_EDITOR_HANDLED;
    }
    if (editor_canvas_path_active(editor) && (key_code == 36U || key_code == 76U))
        return editor_canvas_path_finish(editor,
            editor->draw_tool == VIEWER_EDITOR_TOOL_POLYGON || editor->path_close_preview);
    bool command = (modifiers & PLATFORM_MODIFIER_COMMAND) != 0U;
    bool shift = (modifiers & PLATFORM_MODIFIER_SHIFT) != 0U;
    bool option = (modifiers & PLATFORM_MODIFIER_OPTION) != 0U;
    int character = text_len > 0U ? tolower((unsigned char)text[0]) : -1;
    if (command && (character == '\'' || key_code == 39U)) {
        editor->snap_enabled = !editor->snap_enabled;
        editor->snap_guide_x_visible = false;
        editor->snap_guide_y_visible = false;
        editor_status_ok(editor, editor->snap_enabled ? "Snapping on" : "Snapping off");
        editor->redraw_requested = true;
        return VIEWER_EDITOR_HANDLED;
    }
    if (command) {
        if (shift && character == 'e')
            return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_EXPORT_PNG;
        if (option && character == 'c')
            return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_COPY_PNG;
        if (character == 'a') return VIEWER_EDITOR_HANDLED |
            VIEWER_EDITOR_SELECT_ALL;
        if (character == 'c') return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_COPY;
        if (character == 'x') return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_CUT;
        if (character == 'v') return VIEWER_EDITOR_HANDLED |
            (shift ? VIEWER_EDITOR_PASTE_IN_PLACE : VIEWER_EDITOR_PASTE);
        if (character == ']' || character == '}' || key_code == 30U) {
            bool moved = shift ? editor_reorder_to_edge(editor, true) :
                editor_reorder(editor, true);
            return moved ? VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_MUTATED :
                VIEWER_EDITOR_HANDLED;
        }
        if (character == '[' || character == '{' || key_code == 33U) {
            bool moved = shift ? editor_reorder_to_edge(editor, false) :
                editor_reorder(editor, false);
            return moved ? VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_MUTATED :
                VIEWER_EDITOR_HANDLED;
        }
        if (character == 's') return VIEWER_EDITOR_HANDLED |
            (shift ? VIEWER_EDITOR_SAVE_AS : VIEWER_EDITOR_SAVE);
        if (character == 'z') return editor_history(editor, shift);
        if (character == 'd') {
            editor->focus_id = 0U;
            return editor_apply_button(editor, EDITOR_BUTTON_DUPLICATE);
        }
        if (character == 'g') return editor_apply_button(editor,
            shift ? EDITOR_BUTTON_UNGROUP : EDITOR_BUTTON_GROUP);
        if (character == 'o') return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_OPEN;
        if (character == 'q') return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_QUIT;
        if (character == 'r') return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_RELOAD;
        return 0U;
    }
    if (text_len > 0U && (text[0] == 'v' || text[0] == 'V')) {
        (void)viewer_editor_set_tool(editor, UI_ICON_SELECT);
        return VIEWER_EDITOR_HANDLED;
    }
    if (text_len > 0U && (text[0] == 'h' || text[0] == 'H')) {
        (void)viewer_editor_set_tool(editor, UI_ICON_PAN);
        return VIEWER_EDITOR_HANDLED;
    }
    if (text_len > 0U && (text[0] == 'z' || text[0] == 'Z')) {
        (void)viewer_editor_set_tool(editor, UI_ICON_ZOOM);
        return VIEWER_EDITOR_HANDLED;
    }
    if (text_len > 0U && (text[0] == 'r' || text[0] == 'R')) {
        (void)viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_RECT);
        return VIEWER_EDITOR_HANDLED;
    }
    if (text_len > 0U && (text[0] == 'o' || text[0] == 'O')) {
        (void)viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_ELLIPSE);
        return VIEWER_EDITOR_HANDLED;
    }
    if (text_len > 0U && (text[0] == 'l' || text[0] == 'L')) {
        (void)viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_LINE);
        return VIEWER_EDITOR_HANDLED;
    }
    if (text_len > 0U && (text[0] == 'y' || text[0] == 'Y')) {
        (void)viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_POLYGON);
        return VIEWER_EDITOR_HANDLED;
    }
    if (text_len > 0U && (text[0] == 'u' || text[0] == 'U')) {
        (void)viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_POLYLINE);
        return VIEWER_EDITOR_HANDLED;
    }
    if (text_len > 0U && (text[0] == 'p' || text[0] == 'P')) {
        (void)viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_PEN);
        return VIEWER_EDITOR_HANDLED;
    }
    if (text_len > 0U && (text[0] == 'n' || text[0] == 'N')) {
        (void)viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_PEN);
        editor->node_edit_active = true;
        editor->tree_ready = false;
        editor->redraw_requested = true;
        return VIEWER_EDITOR_HANDLED;
    }
    if (text_len > 0U && (text[0] == 't' || text[0] == 'T')) {
        (void)viewer_editor_set_draw_tool(editor, VIEWER_EDITOR_TOOL_TEXT);
        return VIEWER_EDITOR_HANDLED;
    }
    if (key_code == 48U) {
        if (!editor->tree_ready) editor_build_tree(editor);
        if (ui_focus_next(&editor->tree, shift)) {
            editor->focus_id = editor->tree.focused_id;
            editor->tree_ready = false;
        }
        return VIEWER_EDITOR_HANDLED;
    }
    if (key_code == 36U || key_code == 76U || (text_len > 0U && text[0] == 'e')) {
        size_t count = document_edit_attribute_count(editor->edit, editor->selected);
        if (count > 0U) {
            size_t index = editor->selected_attribute < count ? editor->selected_attribute : 0U;
            (void)editor_begin_field(editor, EDITOR_FIELD_ATTRIBUTE, index);
        } else (void)editor_begin_field(editor, EDITOR_FIELD_TEXT, 0U);
        return VIEWER_EDITOR_HANDLED;
    }
    if (key_code == 53U) {
        if (editor->new_name_length > 0U || editor->new_value_length > 0U) {
            editor->new_name[0] = editor->new_value[0] = '\0';
            editor->new_name_length = editor->new_value_length = 0U;
            editor_status_ok(editor, "Attribute draft cleared");
        }
        return VIEWER_EDITOR_HANDLED;
    }
    if (key_code >= 123U && key_code <= 126U && editor->canvas_selection_valid) {
        double step = shift ? 10.0 : 1.0;
        double dx = key_code == 123U ? -step : key_code == 124U ? step : 0.0;
        double dy = key_code == 126U ? -step : key_code == 125U ? step : 0.0;
        return viewer_editor_nudge_selection(editor, dx, dy);
    }
    if (key_code == 126U) { (void)editor_select_delta(editor, -1); return VIEWER_EDITOR_HANDLED; }
    if (key_code == 125U) { (void)editor_select_delta(editor, 1); return VIEWER_EDITOR_HANDLED; }
    if (key_code == 123U) { (void)editor_select_parent(editor); return VIEWER_EDITOR_HANDLED; }
    if (key_code == 124U) { (void)editor_select_child(editor); return VIEWER_EDITOR_HANDLED; }
    if ((key_code == 51U || key_code == 117U) &&
        editor->draw_tool == VIEWER_EDITOR_TOOL_PEN &&
        editor->path_selected_node_segment != SIZE_MAX)
        return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_DELETE_PATH_NODE;
    if (key_code == 51U || key_code == 117U)
        return editor_apply_button(editor, EDITOR_BUTTON_DELETE);
    if (text_len > 0U && text[0] == '[') {
        return editor_reorder(editor, false) ?
            VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_MUTATED : VIEWER_EDITOR_HANDLED;
    }
    if (text_len > 0U && text[0] == ']') {
        return editor_reorder(editor, true) ?
            VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_MUTATED : VIEWER_EDITOR_HANDLED;
    }
    return 0U;
}
