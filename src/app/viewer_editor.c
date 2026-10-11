#include "viewer_editor_internal.h"

#include "document_model.h"
#include "viewer_chrome.h"
#include "platform/macos/platform_accessibility.h"
#include "platform/platform_text_ui.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static size_t bounded_length(const char *text, size_t capacity)
{
    size_t length = 0U;
    if (text == NULL) return 0U;
    while (length < capacity && text[length] != '\0') ++length;
    return length;
}

void editor_set_text(char *target, size_t capacity, const char *source)
{
    if (target == NULL || capacity == 0U) return;
    size_t length = bounded_length(source, capacity - 1U);
    if (length > 0U) memcpy(target, source, length);
    target[length] = '\0';
}

size_t editor_utf8_prefix(const uint8_t *bytes, size_t length, size_t maximum)
{
    size_t result = length < maximum ? length : maximum;
    while (result > 0U && result < length && (bytes[result] & 0xc0U) == 0x80U) --result;
    return result;
}

bool editor_same_handle(document_handle left, document_handle right)
{
    return left.document_id == right.document_id && left.slot == right.slot &&
        left.generation == right.generation;
}

const document_model *editor_model(const viewer_editor *editor)
{
    return editor == NULL || editor->edit == NULL ? NULL :
        document_edit_model(editor->edit);
}

void editor_status_error(viewer_editor *editor, document_edit_error error)
{
    const char *name = document_edit_error_name(error);
    int amount = snprintf(editor->status, sizeof(editor->status), "Edit rejected: %s",
        name == NULL ? "unknown error" : name);
    if (amount < 0 || (size_t)amount >= sizeof(editor->status))
        editor_set_text(editor->status, sizeof(editor->status), "Edit rejected");
    editor->tree_ready = false;
    editor->redraw_requested = true;
}

void editor_status_ok(viewer_editor *editor, const char *message)
{
    if (message == NULL || message[0] == '\0') message = "Ready";
    editor_set_text(editor->status, sizeof(editor->status), message);
    editor->tree_ready = false;
    editor->redraw_requested = true;
}

bool editor_begin_field(viewer_editor *editor, editor_field_kind kind,
                               size_t attribute_index)
{
    if (kind == EDITOR_FIELD_NEW_NAME || kind == EDITOR_FIELD_NEW_VALUE) {
        const char *source = kind == EDITOR_FIELD_NEW_NAME ? editor->new_name : editor->new_value;
        size_t length = kind == EDITOR_FIELD_NEW_NAME ? editor->new_name_length : editor->new_value_length;
        if (length >= sizeof(editor->field)) return false;
        if (length > 0U) memcpy(editor->field, source, length);
        editor->field[length] = '\0';
        editor->field_length = length;
    } else if (kind == EDITOR_FIELD_ATTRIBUTE) {
        document_edit_attribute attribute;
        base_bytes name;
        uint8_t decoded[EDITOR_FIELD_CAP];
        size_t length = 0U;
        if (!document_edit_attribute_at(editor->edit, editor->selected,
                attribute_index, &attribute) ||
            !document_edit_span_bytes(editor->edit, attribute.name, &name) ||
            name.len >= sizeof(editor->selected_attribute_name) ||
            !document_edit_decode_span(editor->edit, attribute.value,
                decoded, sizeof(decoded), &length) || length >= sizeof(editor->field)) {
            editor_status_ok(editor, "Value exceeds the inline editor limit");
            return false;
        }
        memcpy(editor->selected_attribute_name, name.ptr, name.len);
        editor->selected_attribute_name[name.len] = '\0';
        if (length > 0U) memcpy(editor->field, decoded, length);
        editor->field[length] = '\0';
        editor->field_length = length;
        editor->field_attribute = attribute_index;
        editor->selected_attribute = attribute_index;
    } else if (kind == EDITOR_FIELD_TEXT) {
        size_t length = 0U;
        if (!document_edit_text_content(editor->edit, editor->selected,
                (uint8_t *)editor->field, sizeof(editor->field) - 1U, &length)) {
            editor_status_ok(editor, "Text exceeds the inline editor limit");
            return false;
        }
        editor->field[length] = '\0';
        editor->field_length = length;
    } else if (kind == EDITOR_FIELD_SEARCH) {
        if (editor->outline_filter_length >= EDITOR_OUTLINE_FILTER_CAP) return false;
        memcpy(editor->field, editor->outline_filter, editor->outline_filter_length);
        editor->field[editor->outline_filter_length] = '\0';
        editor->field_length = editor->outline_filter_length;
        memcpy(editor->outline_filter_original, editor->outline_filter,
            editor->outline_filter_length + 1U);
        editor->outline_filter_original_length = editor->outline_filter_length;
    } else if (kind == EDITOR_FIELD_RENAME) {
        size_t length = editor->selected_has_id ? strlen(editor->selected_id) : 0U;
        if (length >= sizeof(editor->field)) return false;
        if (length > 0U) memcpy(editor->field, editor->selected_id, length);
        editor->field[length] = '\0';
        editor->field_length = length;
    } else return false;
    editor->cursor = editor->field_length;
    editor->anchor = editor->cursor;
    editor->field_kind = kind;
    editor->focus_id = kind == EDITOR_FIELD_ATTRIBUTE ?
        EDITOR_ATTRIBUTE_ROW_BASE + (uint32_t)attribute_index :
        kind == EDITOR_FIELD_TEXT ? EDITOR_TEXT_FIELD_ID :
        kind == EDITOR_FIELD_NEW_NAME ? EDITOR_NEW_NAME_ID :
        kind == EDITOR_FIELD_NEW_VALUE ? EDITOR_NEW_VALUE_ID :
        kind == EDITOR_FIELD_SEARCH ? EDITOR_OUTLINE_SEARCH_ID : EDITOR_RENAME_ID;
    editor->tree_ready = false;
    return true;
}

bool editor_store_pending_field(viewer_editor *editor)
{
    if (editor->field_kind == EDITOR_FIELD_NEW_NAME) {
        if (editor->field_length >= sizeof(editor->new_name)) return false;
        memcpy(editor->new_name, editor->field, editor->field_length);
        editor->new_name[editor->field_length] = '\0';
        editor->new_name_length = editor->field_length;
    } else if (editor->field_kind == EDITOR_FIELD_NEW_VALUE) {
        if (editor->field_length >= sizeof(editor->new_value)) return false;
        memcpy(editor->new_value, editor->field, editor->field_length);
        editor->new_value[editor->field_length] = '\0';
        editor->new_value_length = editor->field_length;
    }
    return true;
}

uint32_t editor_apply_result(viewer_editor *editor, document_edit_error result,
                                    size_t old_offset, const char *old_id,
                                    document_handle preferred)
{
    if (result != DOCUMENT_EDIT_OK) {
        editor_status_error(editor, result);
        return VIEWER_EDITOR_HANDLED;
    }
    editor_rebind_after_change(editor, old_offset, old_id, preferred);
    editor_status_ok(editor, "Edited · preview updating");
    return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_MUTATED;
}

uint32_t editor_apply_button(viewer_editor *editor, uint32_t id)
{
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ? editor->selected_id : "");
    if (id == EDITOR_BUTTON_SAVE) return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_SAVE;
    if (id == EDITOR_BUTTON_SAVE_AS) return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_SAVE_AS;
    if (id == EDITOR_BUTTON_UNDO || id == EDITOR_BUTTON_REDO) {
        document_edit_error error = id == EDITOR_BUTTON_UNDO ?
            document_edit_undo(editor->edit) : document_edit_redo(editor->edit);
        return editor_apply_result(editor, error, old_offset, old_id, (document_handle){0});
    }
    if (id == EDITOR_BUTTON_DELETE || id == EDITOR_BUTTON_DUPLICATE ||
        id == EDITOR_BUTTON_GROUP || id == EDITOR_BUTTON_UNGROUP ||
        id == EDITOR_BUTTON_ADD_ATTRIBUTE || id == EDITOR_BUTTON_REMOVE_ATTRIBUTE) {
        if (editor_canvas_selection_is_locked(editor)) {
            editor_status_ok(editor, "Unlock the selected layer to edit it");
            return VIEWER_EDITOR_HANDLED;
        }
    }
    if (id == EDITOR_BUTTON_DELETE) {
        if (editor->canvas_selection_count > 1U)
            return editor_canvas_multi_delete(editor);
        document_handle parent = {0};
        size_t parent_offset = 0U;
        if (document_model_parent(editor_model(editor), editor->selected, &parent)) {
            document_source_span span;
            if (document_edit_source_span(editor->edit, parent, &span)) parent_offset = span.offset;
        }
        document_edit_error error = document_edit_delete_element(editor->edit, editor->selected);
        if (error != DOCUMENT_EDIT_OK) return editor_apply_result(editor, error, old_offset, old_id, (document_handle){0});
        editor_rebind_after_change(editor, parent_offset, "", parent);
        editor_status_ok(editor, "Deleted · preview updating");
        return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_MUTATED;
    }
    if (id == EDITOR_BUTTON_DUPLICATE) {
        if (editor->canvas_selection_count > 1U)
            return editor_canvas_multi_duplicate(editor);
        document_handle duplicate = {0};
        document_edit_error error = document_edit_duplicate_element(editor->edit,
            editor->selected, &duplicate);
        return editor_apply_result(editor, error, old_offset, old_id, duplicate);
    }
    if (id == EDITOR_BUTTON_GROUP) {
        if (editor->canvas_selection_count > 1U)
            return editor_canvas_multi_group(editor);
        document_handle group = {0};
        document_edit_error error = document_edit_wrap_in_group(editor->edit,
            editor->selected, &group);
        return editor_apply_result(editor, error, old_offset, old_id, group);
    }
    if (id == EDITOR_BUTTON_UNGROUP) {
        if (editor->canvas_selection_count > 1U)
            return editor_canvas_multi_ungroup(editor);
        document_handle parent = {0};
        size_t parent_offset = 0U;
        if (document_model_parent(editor_model(editor), editor->selected, &parent)) {
            document_source_span span;
            if (document_edit_source_span(editor->edit, parent, &span)) parent_offset = span.offset;
        }
        document_edit_error error = document_edit_ungroup(editor->edit, editor->selected);
        if (error != DOCUMENT_EDIT_OK) return editor_apply_result(editor, error, old_offset, old_id, (document_handle){0});
        editor_rebind_after_change(editor, parent_offset, "", parent);
        editor_status_ok(editor, "Ungrouped · preview updating");
        return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_MUTATED;
    }
    if (id == EDITOR_BUTTON_ADD_ATTRIBUTE) {
        if (editor->new_name_length == 0U) {
            editor_status_ok(editor, "Enter an attribute name first");
            return VIEWER_EDITOR_HANDLED;
        }
        document_edit_error error = document_edit_add_attribute(editor->edit,
            editor->selected,
            (base_bytes){(const uint8_t *)editor->new_name, editor->new_name_length},
            (base_bytes){(const uint8_t *)editor->new_value, editor->new_value_length});
        if (error == DOCUMENT_EDIT_OK) {
            editor->new_name_length = editor->new_value_length = 0U;
            editor->new_name[0] = editor->new_value[0] = '\0';
        }
        return editor_apply_result(editor, error, old_offset, old_id, (document_handle){0});
    }
    if (id == EDITOR_BUTTON_REMOVE_ATTRIBUTE) {
        if (editor->selected_attribute >= document_edit_attribute_count(editor->edit, editor->selected)) {
            editor_status_ok(editor, "Select an attribute to remove");
            return VIEWER_EDITOR_HANDLED;
        }
        document_edit_attribute attribute;
        base_bytes name;
        if (!document_edit_attribute_at(editor->edit, editor->selected,
                editor->selected_attribute, &attribute) ||
            !document_edit_span_bytes(editor->edit, attribute.name, &name)) {
            editor_status_ok(editor, "Attribute is no longer available");
            return VIEWER_EDITOR_HANDLED;
        }
        document_edit_error error = document_edit_remove_attribute(editor->edit,
            editor->selected, name);
        return editor_apply_result(editor, error, old_offset, old_id, (document_handle){0});
    }
    return VIEWER_EDITOR_HANDLED;
}

bool editor_reorder(viewer_editor *editor, bool forward)
{
    if (editor_canvas_selection_is_locked(editor)) {
        editor_status_ok(editor, "Unlock the selected layer to reorder it");
        return false;
    }
    const document_model *model = editor_model(editor);
    document_handle parent, sibling = {0};
    if (!document_model_parent(model, editor->selected, &parent)) return false;
    if (forward) {
        if (!document_model_next_sibling(model, editor->selected, &sibling)) return false;
        size_t old_offset = editor->selected_offset;
        char old_id[EDITOR_NAME_CAP];
        editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ? editor->selected_id : "");
        document_edit_error result = document_edit_reorder_sibling(editor->edit,
            editor->selected, sibling, false);
        (void)editor_apply_result(editor, result, old_offset, old_id, (document_handle){0});
        return result == DOCUMENT_EDIT_OK;
    }
    document_handle cursor;
    if (!document_model_first_child(model, parent, &cursor)) return false;
    document_handle previous = {0};
    while (!editor_same_handle(cursor, editor->selected)) {
        previous = cursor;
        if (!document_model_next_sibling(model, cursor, &cursor)) return false;
    }
    if (previous.document_id == 0U) return false;
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ? editor->selected_id : "");
    document_edit_error result = document_edit_reorder_sibling(editor->edit,
        editor->selected, previous, true);
    (void)editor_apply_result(editor, result, old_offset, old_id, (document_handle){0});
    return result == DOCUMENT_EDIT_OK;
}

bool editor_reorder_to_edge(viewer_editor *editor, bool front)
{
    if (editor_canvas_selection_is_locked(editor)) {
        editor_status_ok(editor, "Unlock the selected layer to reorder it");
        return false;
    }
    const document_model *model = editor_model(editor);
    document_handle parent = {0U}, target = {0U}, cursor = {0U};
    if (model == NULL || !document_model_parent(model, editor->selected, &parent) ||
        !document_model_first_child(model, parent, &cursor)) return false;
    if (!front) {
        if (editor_same_handle(cursor, editor->selected)) return false;
        target = cursor;
    } else {
        target = cursor;
        while (document_model_next_sibling(model, cursor, &cursor)) target = cursor;
        if (editor_same_handle(target, editor->selected)) return false;
    }
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ?
        editor->selected_id : "");
    document_edit_error result = document_edit_reorder_sibling(editor->edit,
        editor->selected, target, !front);
    (void)editor_apply_result(editor, result, old_offset, old_id,
        (document_handle){0U});
    return result == DOCUMENT_EDIT_OK;
}

static void editor_preferences_changed(ui_theme theme, bool increased_contrast,
                                       bool reduced_motion, void *context)
{
    viewer_editor *editor = context;
    if (editor != NULL)
        (void)viewer_editor_set_appearance(editor, theme, increased_contrast,
                                           reduced_motion);
}

static void editor_view_menu_changed(bool structure_visible,
                                     bool inspector_visible, void *context)
{
    viewer_editor *editor = context;
    (void)viewer_editor_set_sidebar_visibility(editor, structure_visible,
                                               inspector_visible);
}

static bool editor_accessibility_command(uint32_t command_id, void *context)
{
    viewer_editor *editor = context;
    if (editor == NULL) return false;
    if (command_id == EDITOR_BUTTON_TOOL_SELECT)
        return viewer_editor_set_tool(editor, UI_ICON_SELECT) ||
            editor->active_tool == UI_ICON_SELECT;
    if (command_id == EDITOR_BUTTON_TOOL_PAN)
        return viewer_editor_set_tool(editor, UI_ICON_PAN) ||
            editor->active_tool == UI_ICON_PAN;
    if (command_id == EDITOR_BUTTON_TOOL_ZOOM)
        return viewer_editor_set_tool(editor, UI_ICON_ZOOM) ||
            editor->active_tool == UI_ICON_ZOOM;
    if (command_id == EDITOR_BUTTON_SNAPPING) {
        editor->snap_popover_open = !editor->snap_popover_open;
        editor->snap_popover_hovered = false;
        editor->snap_popover_hovered_row = UINT8_MAX;
        editor->tree_ready = false;
        editor->redraw_requested = true;
        return true;
    }
    if (command_id == EDITOR_BUTTON_TOGGLE_STRUCTURE ||
        command_id == EDITOR_BUTTON_HIDE_STRUCTURE) {
        (void)viewer_editor_set_sidebar_visibility(editor,
            !editor->want_structure, editor->want_inspector);
        return true;
    }
    if (command_id == EDITOR_BUTTON_TOGGLE_INSPECTOR ||
        command_id == EDITOR_BUTTON_HIDE_INSPECTOR) {
        (void)viewer_editor_set_sidebar_visibility(editor,
            editor->want_structure, !editor->want_inspector);
        return true;
    }
    if (command_id == EDITOR_BUTTON_INSPECTOR_OTHER_ATTRIBUTES) {
        editor->inspector_other_attributes_expanded =
            !editor->inspector_other_attributes_expanded;
        editor->tree_ready = false;
        editor->redraw_requested = true;
        return true;
    }
    if (command_id >= EDITOR_OUTLINE_ROW_BASE &&
        command_id < EDITOR_OUTLINE_ROW_BASE + EDITOR_OUTLINE_VISIBLE) {
        size_t index = editor->outline_start + command_id - EDITOR_OUTLINE_ROW_BASE;
        if (editor->modern_style && !editor_outline_visible_model_index(editor,
                editor->outline_visible_start + command_id - EDITOR_OUTLINE_ROW_BASE,
                &index)) return false;
        return editor_select_outline_index(editor, index);
    }
    if (command_id >= EDITOR_ATTRIBUTE_ROW_BASE &&
        command_id < EDITOR_ATTRIBUTE_ROW_BASE + 2048U) {
        size_t index = command_id - EDITOR_ATTRIBUTE_ROW_BASE;
        if (index >= document_edit_attribute_count(editor->edit, editor->selected)) return false;
        editor->selected_attribute = index;
        if (!editor_begin_field(editor, EDITOR_FIELD_ATTRIBUTE, index)) return false;
        editor->focus_id = command_id;
        editor->tree_ready = false;
        return true;
    }
    return false;
}

viewer_editor *viewer_editor_create(base_bytes source, ui_theme theme,
                                    uint32_t width, uint32_t height)
{
    return viewer_editor_create_styled(source, theme, width, height,
        platform_text_ui_app_is_active());
}

viewer_editor *viewer_editor_create_styled(base_bytes source, ui_theme theme,
                                           uint32_t width, uint32_t height,
                                           bool modern_style)
{
    if (source.ptr == NULL || source.len == 0U || width == 0U || height == 0U ||
        (theme != UI_THEME_DARK && theme != UI_THEME_LIGHT)) return NULL;
    viewer_editor *editor = calloc(1U, sizeof(*editor));
    if (editor == NULL) return NULL;
    document_edit_result created = document_edit_create(source, document_edit_default_limits());
    if (created.error != DOCUMENT_EDIT_OK) {
        free(editor);
        return NULL;
    }
    editor->edit = created.edit;
    editor->theme = theme;
    editor->modern_style = modern_style;
    editor->active_tool = UI_ICON_SELECT;
    editor->want_structure = true;
    editor->want_inspector = true;
    editor->width = width;
    editor->height = height;
    editor->backing_scale = 1.0;
    editor->path_selected_node_segment = SIZE_MAX;
    editor->snap_enabled = true;
    editor->snap_grid_enabled = true;
    editor->snap_objects_enabled = true;
    editor->snap_guides_enabled = true;
    editor->snap_popover_hovered_row = UINT8_MAX;
    if (modern_style) {
        (void)platform_text_ui_get_preferences(&editor->theme,
            &editor->increased_contrast, &editor->reduced_motion);
    }
    editor_status_ok(editor, "Ready");
    if (!document_edit_root(editor->edit, &editor->selected)) {
        viewer_editor_destroy(editor);
        return NULL;
    }
    editor_capture_selection_identity(editor);
    editor_refresh_outline(editor);
    if (modern_style)
        (void)platform_text_ui_watch_preferences(editor_preferences_changed, editor);
    if (modern_style) {
        platform_accessibility_set_view_callback(editor_view_menu_changed, editor,
            editor->want_structure, editor->want_inspector);
        platform_accessibility_set_command_callback(editor_accessibility_command, editor);
    }
    return editor;
}

void viewer_editor_destroy(viewer_editor *editor)
{
    if (editor == NULL) return;
    platform_text_ui_unwatch_preferences(editor);
    platform_accessibility_clear_view_callback(editor);
    platform_accessibility_set_command_callback(NULL, NULL);
    document_edit_destroy(editor->edit);
    free(editor);
}

bool viewer_editor_set_appearance(viewer_editor *editor, ui_theme theme,
                                   bool increased_contrast, bool reduced_motion)
{
    if (editor == NULL || (theme != UI_THEME_DARK && theme != UI_THEME_LIGHT)) return false;
    if (!editor->modern_style) return true;
    if (editor->theme == theme && editor->increased_contrast == increased_contrast &&
        editor->reduced_motion == reduced_motion) return false;
    editor->theme = theme;
    editor->increased_contrast = increased_contrast;
    editor->reduced_motion = reduced_motion;
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return true;
}

bool viewer_editor_set_sidebar_visibility(viewer_editor *editor,
                                          bool structure_visible,
                                          bool inspector_visible)
{
    if (editor == NULL) return false;
    if (editor->want_structure == structure_visible &&
        editor->want_inspector == inspector_visible) return false;
    editor->want_structure = structure_visible;
    editor->want_inspector = inspector_visible;
    editor->tree_ready = false;
    editor->redraw_requested = true;
    if (editor->modern_style)
        platform_accessibility_set_view_callback(editor_view_menu_changed, editor,
            structure_visible, inspector_visible);
    return true;
}

bool viewer_editor_set_tool(viewer_editor *editor, ui_icon_kind tool)
{
    if (editor == NULL || tool < UI_ICON_SELECT || tool > UI_ICON_ZOOM) return false;
    if (editor->active_tool == tool &&
        editor->draw_tool == VIEWER_EDITOR_TOOL_SELECT) return false;
    editor->active_tool = tool;
    if (editor_canvas_path_active(editor)) editor_canvas_path_cancel(editor);
    editor->draw_tool = VIEWER_EDITOR_TOOL_SELECT;
    editor->node_edit_active = false;
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return true;
}

ui_icon_kind viewer_editor_get_tool(const viewer_editor *editor)
{
    return editor == NULL ? UI_ICON_SELECT : editor->active_tool;
}

bool viewer_editor_text_input_active(const viewer_editor *editor)
{
    return editor != NULL && editor->field_kind != EDITOR_FIELD_NONE;
}

bool viewer_editor_set_draw_tool(viewer_editor *editor,
                                 viewer_editor_draw_tool tool)
{
    if (editor == NULL || tool < VIEWER_EDITOR_TOOL_SELECT ||
        tool > VIEWER_EDITOR_TOOL_TEXT) return false;
    bool node_mode_changed = editor->node_edit_active;
    editor->node_edit_active = false;
    if (editor->draw_tool == tool) {
        if (node_mode_changed) {
            editor->tree_ready = false;
            editor->redraw_requested = true;
        }
        return node_mode_changed;
    }
    if (editor_canvas_path_active(editor)) editor_canvas_path_cancel(editor);
    editor->draw_tool = tool;
    editor->active_tool = UI_ICON_SELECT;
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return true;
}

viewer_editor_draw_tool viewer_editor_get_draw_tool(const viewer_editor *editor)
{
    return editor == NULL ? VIEWER_EDITOR_TOOL_SELECT : editor->draw_tool;
}

bool viewer_editor_get_chrome_layout(viewer_editor *editor,
                                     struct viewer_chrome_layout *out_layout)
{
    if (editor == NULL || out_layout == NULL) return false;
    return viewer_chrome_layout_compute(editor->width, editor->height,
        editor->backing_scale, editor->want_structure, editor->want_inspector,
        (viewer_chrome_layout *)out_layout);
}

bool viewer_editor_is_dirty(const viewer_editor *editor)
{
    if (editor == NULL || editor->edit == NULL) return false;
    if (document_edit_is_dirty(editor->edit) || editor->new_name_length > 0U ||
        editor->new_value_length > 0U) return true;
    uint8_t current[EDITOR_FIELD_CAP];
    size_t length = 0U;
    if (editor->field_kind == EDITOR_FIELD_ATTRIBUTE) {
        document_edit_attribute attribute;
        if (!document_edit_attribute_at(editor->edit, editor->selected,
                editor->field_attribute, &attribute) ||
            !document_edit_decode_span(editor->edit, attribute.value,
                current, sizeof(current), &length)) return true;
    } else if (editor->field_kind == EDITOR_FIELD_TEXT) {
        if (!document_edit_text_content(editor->edit, editor->selected,
                current, sizeof(current), &length)) return true;
    } else if (editor->field_kind == EDITOR_FIELD_NEW_NAME) {
        return editor->field_length != editor->new_name_length ||
            (editor->field_length > 0U && memcmp(editor->field,
                editor->new_name, editor->field_length) != 0);
    } else if (editor->field_kind == EDITOR_FIELD_NEW_VALUE) {
        return editor->field_length != editor->new_value_length ||
            (editor->field_length > 0U && memcmp(editor->field,
                editor->new_value, editor->field_length) != 0);
    } else return false;
    return length != editor->field_length ||
        (length > 0U && memcmp(current, editor->field, length) != 0);
}

const char *viewer_editor_status(const viewer_editor *editor)
{
    return editor == NULL ? "Editing unavailable" : editor->status;
}

void viewer_editor_set_status(viewer_editor *editor, const char *message)
{
    if (editor == NULL) return;
    editor_status_ok(editor, message);
}

void viewer_editor_resize(viewer_editor *editor, uint32_t width, uint32_t height,
                          double backing_scale)
{
    if (editor == NULL || width == 0U || height == 0U ||
        !isfinite(backing_scale) || backing_scale < 0.5 || backing_scale > 8.0) return;
    editor->width = width;
    editor->height = height;
    editor->backing_scale = backing_scale;
    editor->tree_ready = false;
    editor->redraw_requested = true;
}

bool viewer_editor_take_redraw_request(viewer_editor *editor)
{
    if (editor == NULL) return false;
    bool requested = editor->redraw_requested;
    editor->redraw_requested = false;
    return requested;
}

size_t viewer_editor_byte_length(const viewer_editor *editor)
{
    return editor == NULL ? 0U : document_edit_byte_length(editor->edit);
}

bool viewer_editor_materialize(viewer_editor *editor, uint8_t *buffer,
                               size_t capacity, size_t *out_length)
{
    return editor != NULL && document_edit_materialize(editor->edit,
        buffer, capacity, out_length);
}

document_edit_save_result viewer_editor_save(viewer_editor *editor, base_bytes path)
{
    if (editor == NULL || editor->edit == NULL) return (document_edit_save_result){
        .error = DOCUMENT_EDIT_INVALID_ARGUMENT};
    document_edit_save_result result = document_edit_save_as(editor->edit, path, NULL);
    if (result.error == DOCUMENT_EDIT_OK) editor_status_ok(editor, "Saved");
    else {
        const char *name = source_save_error_name(result.source_result.error);
        char message[EDITOR_STATUS_CAP];
        int amount = snprintf(message, sizeof(message), "Save failed: %s",
            name == NULL ? "write error" : name);
        editor_status_ok(editor, amount > 0 && (size_t)amount < sizeof(message) ?
            message : "Save failed");
    }
    return result;
}

bool viewer_editor_select_at_source_offset(viewer_editor *editor, size_t offset)
{
    if (editor == NULL) return false;
    document_handle selected;
    if (!document_edit_element_at_offset(editor->edit, offset, &selected)) return false;
    editor_select_handle(editor, selected);
    editor_status_ok(editor, "Selected from canvas · outline support pending");
    return true;
}

bool viewer_editor_selection_span(viewer_editor *editor,
                                  document_source_span *out_span)
{
    return editor != NULL && out_span != NULL &&
        document_edit_source_span(editor->edit, editor->selected, out_span);
}

bool viewer_editor_get_selection_bounds(viewer_editor *editor,
    const svg_shape_document *document, svg_shape_rect *out_bounds)
{
    if (editor == NULL || document == NULL || out_bounds == NULL) return false;
    svg_shape_selection selection;
    if (editor_canvas_selection_bounds(editor, document, &selection) ||
        svg_shape_element_at_offset(document, editor->selected_offset, &selection)) {
        *out_bounds = selection.device_bounds;
        return isfinite(out_bounds->x) && isfinite(out_bounds->y) &&
            isfinite(out_bounds->width) && isfinite(out_bounds->height) &&
            out_bounds->width >= 0.0 && out_bounds->height >= 0.0;
    }
    return false;
}
