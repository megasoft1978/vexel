#include "viewer_editor_internal.h"

#include "document_model.h"

typedef struct editor_group_selection {
    document_handle handle;
    document_source_span span;
} editor_group_selection;

static int group_selection_compare(const void *left, const void *right)
{
    const editor_group_selection *a = left;
    const editor_group_selection *b = right;
    if (a->span.offset < b->span.offset) return -1;
    if (a->span.offset > b->span.offset) return 1;
    return 0;
}

static bool collect_groups(viewer_editor *editor,
    editor_group_selection nodes[EDITOR_CANVAS_SELECTION_CAP],
    size_t *out_count, size_t *out_start)
{
    if (editor == NULL || nodes == NULL || out_count == NULL || out_start == NULL ||
        editor->canvas_selection_count < 2U ||
        editor->canvas_selection_count > EDITOR_CANVAS_SELECTION_CAP) return false;
    size_t count = 0U;
    for (size_t index = 0U; index < editor->canvas_selection_count; ++index) {
        document_handle handle = {0U};
        document_source_span span = {0U, 0U};
        if (!document_edit_element_at_offset(editor->edit,
                editor->canvas_selection_offsets[index], &handle) ||
            !document_edit_source_span(editor->edit, handle, &span)) return false;
        for (size_t prior = 0U; prior < count; ++prior)
            if (nodes[prior].span.offset == span.offset) return false;
        nodes[count++] = (editor_group_selection){handle, span};
    }
    qsort(nodes, count, sizeof(nodes[0]), group_selection_compare);
    *out_count = count;
    *out_start = nodes[0].span.offset;
    return true;
}

uint32_t editor_canvas_multi_group(viewer_editor *editor)
{
    editor_group_selection nodes[EDITOR_CANVAS_SELECTION_CAP];
    document_handle handles[EDITOR_CANVAS_SELECTION_CAP];
    size_t count = 0U, start = 0U;
    if (!collect_groups(editor, nodes, &count, &start)) {
        if (editor != NULL)
            editor_status_ok(editor,
                "Group needs sibling elements within the 64 KiB edit limit");
        return VIEWER_EDITOR_HANDLED;
    }
    (void)start;
    for (size_t index = 0U; index < count; ++index)
        handles[index] = nodes[index].handle;
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ?
        editor->selected_id : "");
    document_handle group = {0U};
    document_edit_error error = document_edit_group_siblings(editor->edit,
        handles, count, &group);
    uint32_t result = editor_apply_result(editor, error, old_offset, old_id,
        group);
    if ((result & VIEWER_EDITOR_MUTATED) != 0U)
        editor_status_ok(editor, "Elements grouped · preview updating");
    return result;
}

uint32_t editor_canvas_multi_ungroup(viewer_editor *editor)
{
    editor_group_selection nodes[EDITOR_CANVAS_SELECTION_CAP];
    document_handle handles[EDITOR_CANVAS_SELECTION_CAP];
    size_t count = 0U, start = 0U;
    if (!collect_groups(editor, nodes, &count, &start)) {
        if (editor != NULL)
            editor_status_ok(editor,
                "Ungroup needs sibling groups within the 64 KiB edit limit");
        return VIEWER_EDITOR_HANDLED;
    }
    for (size_t index = 0U; index < count; ++index)
        handles[index] = nodes[index].handle;
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ?
        editor->selected_id : "");
    document_edit_error error = document_edit_ungroup_siblings(editor->edit,
        handles, count);
    if (error != DOCUMENT_EDIT_OK)
        return editor_apply_result(editor, error, old_offset, old_id,
            (document_handle){0U});
    editor_rebind_after_change(editor, start, "", (document_handle){0U});
    editor_status_ok(editor, "Elements ungrouped · preview updating");
    return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_MUTATED;
}
