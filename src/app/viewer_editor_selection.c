#include "viewer_editor_internal.h"

#include "document_model.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static bool append_selection_offset(size_t offsets[EDITOR_CANVAS_SELECTION_CAP],
                                    size_t *count, size_t offset)
{
    if (offsets == NULL || count == NULL) return false;
    for (size_t index = 0U; index < *count; ++index)
        if (offsets[index] == offset) return true;
    if (*count >= EDITOR_CANVAS_SELECTION_CAP) return false;
    offsets[(*count)++] = offset;
    return true;
}

bool editor_selection_restore(viewer_editor *editor, const size_t *offsets,
                              size_t count)
{
    if (editor == NULL || (count > 0U && offsets == NULL)) return false;
    size_t resolved[EDITOR_CANVAS_SELECTION_CAP];
    size_t resolved_count = 0U;
    document_handle primary = {0U};
    for (size_t index = 0U; index < count && index < EDITOR_CANVAS_SELECTION_CAP;
         ++index) {
        document_handle handle = {0U};
        document_source_span span = {0U, 0U};
        if (!document_edit_element_at_offset(editor->edit, offsets[index], &handle) ||
            !document_edit_source_span(editor->edit, handle, &span) ||
            !append_selection_offset(resolved, &resolved_count, span.offset)) continue;
        if (primary.document_id == 0U) primary = handle;
    }
    if (resolved_count == 0U) {
        editor->canvas_selection_count = 0U;
        editor->canvas_selection_valid = false;
        editor->redraw_requested = true;
        return false;
    }
    editor_select_handle(editor, primary);
    memcpy(editor->canvas_selection_offsets, resolved,
        resolved_count * sizeof(resolved[0]));
    editor->canvas_selection_count = resolved_count;
    editor->canvas_selection_valid = true;
    editor->redraw_requested = true;
    return true;
}

bool editor_canvas_selection_contains(const viewer_editor *editor, size_t offset)
{
    if (editor == NULL || editor->edit == NULL) return false;
    document_handle handle = {0U};
    document_source_span span = {0U, 0U};
    if (!document_edit_element_at_offset(editor->edit, offset, &handle) ||
        !document_edit_source_span(editor->edit, handle, &span)) return false;
    for (size_t index = 0U; index < editor->canvas_selection_count; ++index)
        if (editor->canvas_selection_offsets[index] == span.offset) return true;
    return false;
}

bool editor_canvas_selection_bounds(viewer_editor *editor,
    const svg_shape_document *document, svg_shape_selection *out_selection)
{
    if (editor == NULL || document == NULL || out_selection == NULL ||
        editor->canvas_selection_count == 0U) return false;
    bool have_bounds = false;
    svg_shape_selection combined = {0};
    for (size_t index = 0U; index < editor->canvas_selection_count; ++index) {
        svg_shape_selection next;
        if (!svg_shape_element_at_offset(document,
                editor->canvas_selection_offsets[index], &next)) continue;
        if (!have_bounds) {
            combined = next;
            have_bounds = true;
            continue;
        }
        double user_left = fmin(combined.user_bounds.x, next.user_bounds.x);
        double user_top = fmin(combined.user_bounds.y, next.user_bounds.y);
        double user_right = fmax(combined.user_bounds.x + combined.user_bounds.width,
            next.user_bounds.x + next.user_bounds.width);
        double user_bottom = fmax(combined.user_bounds.y + combined.user_bounds.height,
            next.user_bounds.y + next.user_bounds.height);
        double device_left = fmin(combined.device_bounds.x, next.device_bounds.x);
        double device_top = fmin(combined.device_bounds.y, next.device_bounds.y);
        double device_right = fmax(combined.device_bounds.x + combined.device_bounds.width,
            next.device_bounds.x + next.device_bounds.width);
        double device_bottom = fmax(combined.device_bounds.y + combined.device_bounds.height,
            next.device_bounds.y + next.device_bounds.height);
        combined.user_bounds = (svg_shape_rect){user_left, user_top,
            user_right - user_left, user_bottom - user_top};
        combined.device_bounds = (svg_shape_rect){device_left, device_top,
            device_right - device_left, device_bottom - device_top};
    }
    if (!have_bounds) return false;
    *out_selection = combined;
    return true;
}

void editor_canvas_selection_clear(viewer_editor *editor)
{
    if (editor == NULL) return;
    editor->canvas_selection_count = 0U;
    editor->canvas_selection_valid = false;
    editor->redraw_requested = true;
}

bool editor_canvas_selection_toggle(viewer_editor *editor, size_t source_offset)
{
    if (editor == NULL || editor->edit == NULL) return false;
    document_handle clicked = {0U};
    document_source_span clicked_span = {0U, 0U};
    if (!document_edit_element_at_offset(editor->edit, source_offset, &clicked) ||
        !document_edit_source_span(editor->edit, clicked, &clicked_span)) return false;
    size_t updated[EDITOR_CANVAS_SELECTION_CAP];
    size_t count = editor->canvas_selection_count;
    if (count > EDITOR_CANVAS_SELECTION_CAP) return false;
    if (count > 0U)
        memcpy(updated, editor->canvas_selection_offsets, count * sizeof(updated[0]));
    size_t found = count;
    for (size_t index = 0U; index < count; ++index)
        if (updated[index] == clicked_span.offset) { found = index; break; }
    if (found < count) {
        for (size_t index = found + 1U; index < count; ++index)
            updated[index - 1U] = updated[index];
        --count;
        if (count > 0U && found == 0U) {
            size_t primary = updated[0];
            updated[0] = updated[count - 1U];
            updated[count - 1U] = primary;
        }
        if (count == 0U) {
            editor->canvas_selection_count = 0U;
            editor->canvas_selection_valid = false;
            editor->redraw_requested = true;
            return true;
        }
        return editor_selection_restore(editor, updated, count);
    }
    if (count >= EDITOR_CANVAS_SELECTION_CAP) {
        editor_status_ok(editor, "Selection limit reached (128 elements)");
        return false;
    }
    size_t added[EDITOR_CANVAS_SELECTION_CAP];
    added[0] = clicked_span.offset;
    if (count > 0U) memcpy(added + 1U, updated, count * sizeof(updated[0]));
    return editor_selection_restore(editor, added, count + 1U);
}

static bool rectangles_intersect(svg_shape_rect left, svg_shape_rect right)
{
    return left.x <= right.x + right.width && right.x <= left.x + left.width &&
        left.y <= right.y + right.height && right.y <= left.y + left.height;
}

bool editor_canvas_selection_marquee(viewer_editor *editor,
    const svg_shape_document *document, svg_shape_rect bounds, bool preserve)
{
    if (editor == NULL || document == NULL || !isfinite(bounds.x) ||
        !isfinite(bounds.y) || !isfinite(bounds.width) ||
        !isfinite(bounds.height) || bounds.width < 0.0 || bounds.height < 0.0)
        return false;
    size_t offsets[EDITOR_CANVAS_SELECTION_CAP];
    size_t count = 0U;
    if (preserve) {
        for (size_t index = 0U; index < editor->canvas_selection_count; ++index)
            (void)append_selection_offset(offsets, &count,
                editor->canvas_selection_offsets[index]);
    }
    for (size_t shape = 0U; shape < document->shape_count; ++shape) {
        svg_shape_selection selection;
        if (!svg_shape_bounds_query(document, shape, &selection) ||
            !rectangles_intersect(bounds, selection.device_bounds)) continue;
        if (!append_selection_offset(offsets, &count, selection.source_offset)) {
            editor_status_ok(editor, "Selection limit reached (128 elements)");
            break;
        }
    }
    if (!editor_selection_restore(editor, offsets, count)) {
        editor_status_ok(editor, "No rendered elements selected");
        return false;
    }
    editor_status_ok(editor, count == 1U ? "Element selected" : "Elements selected");
    return true;
}

bool viewer_editor_select_all(viewer_editor *editor,
    const svg_shape_document *document)
{
    if (editor == NULL || document == NULL) return false;
    size_t offsets[EDITOR_CANVAS_SELECTION_CAP];
    size_t count = 0U;
    for (size_t shape = 0U; shape < document->shape_count; ++shape) {
        svg_shape_selection selection;
        if (!svg_shape_bounds_query(document, shape, &selection)) continue;
        if (!append_selection_offset(offsets, &count, selection.source_offset)) {
            editor_status_ok(editor, "Select All is limited to 128 rendered elements");
            break;
        }
    }
    if (!editor_selection_restore(editor, offsets, count)) {
        editor_status_ok(editor, "No rendered elements to select");
        return false;
    }
    editor_status_ok(editor, count == 1U ? "Element selected" : "All rendered elements selected");
    return true;
}
