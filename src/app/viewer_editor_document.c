#include "viewer_editor_internal.h"

#include "viewer_chrome.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

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
