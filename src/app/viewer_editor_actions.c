#include "viewer_editor_internal.h"

#include <string.h>

uint32_t editor_apply_button(viewer_editor *editor, uint32_t id)
{
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ? editor->selected_id : "");
    if (id == EDITOR_BUTTON_SAVE) return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_SAVE;
    if (id == EDITOR_BUTTON_SAVE_AS) return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_SAVE_AS;
    if (id == EDITOR_BUTTON_UNDO || id == EDITOR_BUTTON_REDO) {
        bool redo = id == EDITOR_BUTTON_REDO;
        if (editor->history_callback != NULL &&
            editor->history_callback(editor->history_context, redo, true))
            return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_MUTATED;
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

