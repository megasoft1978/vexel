#include "viewer_editor_internal.h"

#include <ctype.h>
#include <string.h>

static bool handle_in_set(const document_handle *handles, size_t count,
                          document_handle handle)
{
    for (size_t index = 0U; index < count; ++index)
        if (editor_same_handle(handles[index], handle)) return true;
    return false;
}

bool editor_outline_is_locked(const viewer_editor *editor, document_handle handle)
{
    return editor != NULL && handle_in_set(editor->locked_handles,
        editor->locked_count, handle);
}

bool editor_canvas_selection_is_locked(viewer_editor *editor)
{
    if (editor == NULL) return false;
    if (editor->canvas_selection_count == 0U)
        return editor_outline_is_locked(editor, editor->selected);
    for (size_t index = 0U; index < editor->canvas_selection_count; ++index) {
        document_handle handle;
        if (document_edit_element_at_offset(editor->edit,
                editor->canvas_selection_offsets[index], &handle) &&
            editor_outline_is_locked(editor, handle)) return true;
    }
    return false;
}

bool editor_outline_is_collapsed(const viewer_editor *editor, size_t index)
{
    return editor != NULL && index < editor->outline_count &&
        handle_in_set(editor->collapsed_handles, editor->collapsed_count,
            editor->outline[index]);
}

static bool filter_matches(const viewer_editor *editor, size_t index)
{
    if (editor->outline_filter_length == 0U) return true;
    char label[UI_TEXT_MAX_BYTES];
    size_t length = editor_format_outline(editor, index, label);
    for (size_t start = 0U; start + editor->outline_filter_length <= length; ++start) {
        size_t offset = 0U;
        while (offset < editor->outline_filter_length &&
            tolower((unsigned char)label[start + offset]) ==
            tolower((unsigned char)editor->outline_filter[offset])) ++offset;
        if (offset == editor->outline_filter_length) return true;
    }
    return false;
}

void editor_rebuild_visible_outline(viewer_editor *editor)
{
    if (editor == NULL) return;
    editor->outline_visible_count = 0U;
    size_t collapsed_depth = SIZE_MAX;
    for (size_t index = 0U; index < editor->outline_count; ++index) {
        size_t depth = editor->outline_depth[index];
        if (collapsed_depth != SIZE_MAX) {
            if (depth > collapsed_depth) continue;
            collapsed_depth = SIZE_MAX;
        }
        if (!filter_matches(editor, index)) continue;
        editor->outline_visible[editor->outline_visible_count++] = (uint16_t)index;
        if (editor_outline_is_collapsed(editor, index)) collapsed_depth = depth;
    }
    if (editor->outline_visible_count == 0U) editor->outline_visible_start = 0U;
    else {
        size_t maximum = editor->outline_visible_count > EDITOR_OUTLINE_VISIBLE ?
            editor->outline_visible_count - EDITOR_OUTLINE_VISIBLE : 0U;
        if (editor->outline_visible_start > maximum)
            editor->outline_visible_start = maximum;
    }
}

bool editor_outline_visible_model_index(const viewer_editor *editor, size_t row,
                                        size_t *out_index)
{
    if (editor == NULL || out_index == NULL || row >= editor->outline_visible_count)
        return false;
    *out_index = editor->outline_visible[row];
    return *out_index < editor->outline_count;
}

static bool toggle_handle(document_handle *handles, size_t *count,
                          document_handle handle)
{
    for (size_t index = 0U; index < *count; ++index) {
        if (!editor_same_handle(handles[index], handle)) continue;
        if (index + 1U < *count)
            memmove(handles + index, handles + index + 1U,
                (*count - index - 1U) * sizeof(*handles));
        --*count;
        return false;
    }
    if (*count < EDITOR_OUTLINE_CAP) handles[(*count)++] = handle;
    return true;
}

bool editor_outline_toggle_collapsed(viewer_editor *editor, size_t index)
{
    if (editor == NULL || index >= editor->outline_count) return false;
    bool collapsed = toggle_handle(editor->collapsed_handles,
        &editor->collapsed_count, editor->outline[index]);
    editor_rebuild_visible_outline(editor);
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return collapsed;
}

static bool find_display_attribute(viewer_editor *editor, document_handle handle,
                                   document_edit_attribute *out_attribute,
                                   bool *out_hidden)
{
    size_t count = document_edit_attribute_count(editor->edit, handle);
    for (size_t index = 0U; index < count; ++index) {
        document_edit_attribute attribute;
        base_bytes name;
        uint8_t value[16];
        size_t length = 0U;
        if (!document_edit_attribute_at(editor->edit, handle, index, &attribute) ||
            !document_edit_span_bytes(editor->edit, attribute.name, &name) ||
            name.len != 7U || memcmp(name.ptr, "display", 7U) != 0) continue;
        bool decoded = document_edit_decode_span(editor->edit, attribute.value,
            value, sizeof(value), &length);
        *out_attribute = attribute;
        *out_hidden = decoded && length == 4U && memcmp(value, "none", 4U) == 0;
        return true;
    }
    *out_hidden = false;
    return false;
}

bool editor_outline_toggle_visibility(viewer_editor *editor, size_t index)
{
    if (editor == NULL || index >= editor->outline_count) return false;
    document_handle handle = editor->outline[index];
    document_edit_attribute attribute;
    bool hidden = false;
    bool has_display = find_display_attribute(editor, handle, &attribute, &hidden);
    document_edit_error result;
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ?
        editor->selected_id : "");
    if (has_display && hidden) {
        result = document_edit_remove_attribute(editor->edit, handle,
            (base_bytes){(const uint8_t *)"display", 7U});
    } else {
        result = document_edit_set_attribute(editor->edit, handle,
            (base_bytes){(const uint8_t *)"display", 7U},
            (base_bytes){(const uint8_t *)"none", 4U});
    }
    if (result != DOCUMENT_EDIT_OK) {
        editor_status_error(editor, result);
        return false;
    }
    (void)editor_apply_result(editor, result, old_offset, old_id, handle);
    return true;
}

bool editor_outline_toggle_lock(viewer_editor *editor, size_t index)
{
    if (editor == NULL || index >= editor->outline_count) return false;
    bool locked = toggle_handle(editor->locked_handles, &editor->locked_count,
        editor->outline[index]);
    if (locked) editor_status_ok(editor, "Layer locked");
    else editor_status_ok(editor, "Layer unlocked");
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return locked;
}

void editor_refresh_outline(viewer_editor *editor)
{
    editor->outline_count = 0U;
    editor->outline_truncated = false;
    const document_model *model = editor_model(editor);
    document_handle root;
    if (model == NULL || !document_model_root(model, &root)) return;
    editor_outline_frame stack[EDITOR_DEPTH_CAP];
    size_t depth = 0U;
    stack[depth++] = (editor_outline_frame){.node = root, .depth = 0U};
    while (depth > 0U && editor->outline_count < EDITOR_OUTLINE_CAP) {
        editor_outline_frame *frame = &stack[depth - 1U];
        if (!frame->entered) {
            frame->entered = true;
            editor->outline[editor->outline_count] = frame->node;
            editor->outline_depth[editor->outline_count] =
                (uint16_t)(frame->depth > UINT16_MAX ? UINT16_MAX : frame->depth);
            if (editor_same_handle(frame->node, editor->selected))
                editor->selected_index = editor->outline_count;
            editor->outline_count++;
            (void)document_model_first_child(model, frame->node, &frame->child);
        }
        if (frame->child.document_id != 0U) {
            document_handle child = frame->child;
            document_handle sibling = {0};
            (void)document_model_next_sibling(model, child, &sibling);
            frame->child = sibling;
            if (depth >= EDITOR_DEPTH_CAP) {
                editor->outline_truncated = true;
                break;
            }
            stack[depth++] = (editor_outline_frame){.node = child,
                .depth = frame->depth + 1U};
        } else {
            --depth;
        }
    }
    if (depth > 0U || editor->outline_count == EDITOR_OUTLINE_CAP)
        editor->outline_truncated = true;
    if (editor->outline_count == 0U) return;
    if (editor->selected_index >= editor->outline_count ||
        !editor_same_handle(editor->outline[editor->selected_index], editor->selected)) {
        editor->selected_index = 0U;
        editor->selected = editor->outline[0];
    }
    if (editor->selected_index < editor->outline_start)
        editor->outline_start = editor->selected_index;
    if (editor->selected_index >= editor->outline_start + EDITOR_OUTLINE_VISIBLE)
        editor->outline_start = editor->selected_index - EDITOR_OUTLINE_VISIBLE + 1U;
    size_t maximum_start = editor->outline_count > EDITOR_OUTLINE_VISIBLE ?
        editor->outline_count - EDITOR_OUTLINE_VISIBLE : 0U;
    if (editor->outline_start > maximum_start) editor->outline_start = maximum_start;
    editor_rebuild_visible_outline(editor);
    for (size_t row = 0U; row < editor->outline_visible_count; ++row) {
        if (editor->outline_visible[row] != editor->selected_index) continue;
        if (row < editor->outline_visible_start)
            editor->outline_visible_start = row;
        else if (row >= editor->outline_visible_start + EDITOR_OUTLINE_VISIBLE)
            editor->outline_visible_start = row - EDITOR_OUTLINE_VISIBLE + 1U;
        break;
    }
    size_t visible_maximum = editor->outline_visible_count > EDITOR_OUTLINE_VISIBLE ?
        editor->outline_visible_count - EDITOR_OUTLINE_VISIBLE : 0U;
    if (editor->outline_visible_start > visible_maximum)
        editor->outline_visible_start = visible_maximum;
}

void editor_capture_selection_identity(viewer_editor *editor)
{
    editor->selected_has_id = false;
    editor->selected_id[0] = '\0';
    document_source_span span;
    if (!document_edit_source_span(editor->edit, editor->selected, &span)) return;
    editor->selected_offset = span.offset;
    size_t attributes = document_edit_attribute_count(editor->edit, editor->selected);
    for (size_t index = 0U; index < attributes; ++index) {
        document_edit_attribute attribute;
        if (!document_edit_attribute_at(editor->edit, editor->selected, index, &attribute)) continue;
        base_bytes name;
        if (!document_edit_span_bytes(editor->edit, attribute.name, &name) ||
            name.len != 2U || memcmp(name.ptr, "id", 2U) != 0) continue;
        size_t amount = 0U;
        if (!document_edit_decode_span(editor->edit, attribute.value,
                (uint8_t *)editor->selected_id, sizeof(editor->selected_id) - 1U, &amount)) return;
        editor->selected_id[amount] = '\0';
        editor->selected_has_id = amount > 0U;
        return;
    }
}

void editor_select_handle(viewer_editor *editor, document_handle selected)
{
    editor->selected = selected;
    editor->selection_active = true;
    editor->selected_attribute = 0U;
    editor->attribute_start = 0U;
    editor_capture_selection_identity(editor);
    editor->canvas_selection_count = 1U;
    editor->canvas_selection_offsets[0] = editor->selected_offset;
    editor->path_selected_node_segment = SIZE_MAX;
    editor_refresh_outline(editor);
    editor->tree_ready = false;
}

void editor_rebind_after_change(viewer_editor *editor, size_t old_offset,
                                const char *old_id, document_handle preferred)
{
    document_handle next = {0};
    if (preferred.document_id != 0U) {
        document_source_span preferred_span;
        if (document_edit_source_span(editor->edit, preferred, &preferred_span))
            next = preferred;
    }
    if (next.document_id == 0U && old_id != NULL && old_id[0] != '\0') {
        (void)document_edit_find_id(editor->edit,
            (base_bytes){(const uint8_t *)old_id, strlen(old_id)}, &next);
    }
    if (next.document_id == 0U && old_offset < document_edit_byte_length(editor->edit))
        (void)document_edit_element_at_offset(editor->edit, old_offset + 1U, &next);
    if (next.document_id == 0U) (void)document_edit_root(editor->edit, &next);
    editor->selected = next;
    editor->selection_active = true;
    editor->field_kind = EDITOR_FIELD_NONE;
    editor->field_length = editor->cursor = editor->anchor = 0U;
    editor->selected_attribute_name[0] = '\0';
    editor_capture_selection_identity(editor);
    editor->canvas_selection_count = 1U;
    editor->canvas_selection_offsets[0] = editor->selected_offset;
    editor->path_selected_node_segment = SIZE_MAX;
    editor_refresh_outline(editor);
    editor->tree_ready = false;
}

bool editor_select_outline_index(viewer_editor *editor, size_t index)
{
    if (index >= editor->outline_count) return false;
    editor_select_handle(editor, editor->outline[index]);
    editor->selected_index = index;
    editor_status_ok(editor, "Ready");
    return true;
}

bool editor_select_parent(viewer_editor *editor)
{
    document_handle parent;
    if (!document_model_parent(editor_model(editor), editor->selected, &parent)) return false;
    for (size_t index = 0U; index < editor->outline_count; ++index)
        if (editor_same_handle(editor->outline[index], parent))
            return editor_select_outline_index(editor, index);
    return false;
}

bool editor_select_child(viewer_editor *editor)
{
    if (editor->modern_style && editor_outline_is_collapsed(editor,
            editor->selected_index)) {
        (void)editor_outline_toggle_collapsed(editor, editor->selected_index);
        return true;
    }
    document_handle child;
    if (!document_model_first_child(editor_model(editor), editor->selected, &child)) return false;
    for (size_t index = 0U; index < editor->outline_count; ++index)
        if (editor_same_handle(editor->outline[index], child))
            return editor_select_outline_index(editor, index);
    return false;
}

bool editor_select_delta(viewer_editor *editor, int direction)
{
    if (editor->modern_style) {
        size_t position = SIZE_MAX;
        for (size_t row = 0U; row < editor->outline_visible_count; ++row)
            if (editor->outline_visible[row] == editor->selected_index) {
                position = row;
                break;
            }
        if (position == SIZE_MAX) return false;
        if (direction < 0) {
            if (position == 0U) return false;
            return editor_select_outline_index(editor, editor->outline_visible[position - 1U]);
        }
        if (position + 1U >= editor->outline_visible_count) return false;
        return editor_select_outline_index(editor, editor->outline_visible[position + 1U]);
    }
    if (direction < 0) {
        if (editor->selected_index == 0U) return false;
        return editor_select_outline_index(editor, editor->selected_index - 1U);
    }
    if (editor->selected_index + 1U >= editor->outline_count) return false;
    return editor_select_outline_index(editor, editor->selected_index + 1U);
}
