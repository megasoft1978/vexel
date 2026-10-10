#include "viewer_editor_internal.h"

#include <stdlib.h>
#include <string.h>

#define EDITOR_CLIPBOARD_FRAGMENT_LIMIT (64U * 1024U)

static void sort_spans(document_source_span *spans, size_t count)
{
    for (size_t index = 1U; index < count; ++index) {
        document_source_span value = spans[index];
        size_t cursor = index;
        while (cursor > 0U && (spans[cursor - 1U].offset > value.offset ||
            (spans[cursor - 1U].offset == value.offset &&
             spans[cursor - 1U].length < value.length))) {
            spans[cursor] = spans[cursor - 1U];
            --cursor;
        }
        spans[cursor] = value;
    }
}

static bool append_bytes(uint8_t *buffer, size_t capacity, size_t *used,
    const uint8_t *bytes, size_t length)
{
    if (buffer == NULL || used == NULL || (length > 0U && bytes == NULL) ||
        *used > capacity || length > capacity - *used) return false;
    if (length > 0U) memcpy(buffer + *used, bytes, length);
    *used += length;
    return true;
}

bool viewer_editor_copy_selection(viewer_editor *editor, uint8_t *buffer,
    size_t capacity, size_t *out_length)
{
    if (editor == NULL || editor->edit == NULL || buffer == NULL ||
        out_length == NULL || capacity == 0U) return false;
    *out_length = 0U;
    size_t requested = editor->canvas_selection_count;
    if (requested == 0U) requested = 1U;
    if (requested > EDITOR_CANVAS_SELECTION_CAP) return false;
    document_source_span spans[EDITOR_CANVAS_SELECTION_CAP];
    size_t count = 0U;
    for (size_t index = 0U; index < requested; ++index) {
        size_t offset = editor->canvas_selection_count == 0U ?
            editor->selected_offset : editor->canvas_selection_offsets[index];
        document_handle handle = {0};
        document_source_span span;
        if (!document_edit_element_at_offset(editor->edit, offset, &handle) ||
            !document_edit_source_span(editor->edit, handle, &span) ||
            span.length == 0U) return false;
        spans[count++] = span;
    }
    sort_spans(spans, count);
    size_t visible_count = 0U;
    size_t covered_end = 0U;
    for (size_t index = 0U; index < count; ++index) {
        if (visible_count > 0U && spans[index].offset < covered_end) continue;
        spans[visible_count++] = spans[index];
        if (spans[index].length > SIZE_MAX - spans[index].offset) return false;
        covered_end = spans[index].offset + spans[index].length;
    }
    if (visible_count == 0U) return false;
    if (visible_count == 1U) {
        base_bytes source;
        if (!document_edit_span_bytes(editor->edit, spans[0], &source) ||
            source.len > capacity || source.len > EDITOR_CLIPBOARD_FRAGMENT_LIMIT)
            return false;
        memcpy(buffer, source.ptr, source.len);
        *out_length = source.len;
        return true;
    }
    static const uint8_t prefix[] =
        "<g xmlns='http://www.w3.org/2000/svg'>";
    static const uint8_t suffix[] = "</g>";
    size_t used = 0U;
    if (!append_bytes(buffer, capacity, &used, prefix, sizeof(prefix) - 1U))
        return false;
    for (size_t index = 0U; index < visible_count; ++index) {
        base_bytes source;
        if (!document_edit_span_bytes(editor->edit, spans[index], &source) ||
            source.len > EDITOR_CLIPBOARD_FRAGMENT_LIMIT - used ||
            !append_bytes(buffer, capacity, &used, source.ptr, source.len)) return false;
    }
    if (!append_bytes(buffer, capacity, &used, suffix, sizeof(suffix) - 1U))
        return false;
    *out_length = used;
    return true;
}

uint32_t viewer_editor_cut_selection(viewer_editor *editor)
{
    if (editor == NULL) return 0U;
    if (editor->canvas_selection_count > 1U)
        return editor_canvas_multi_delete(editor);
    return editor_apply_button(editor, EDITOR_BUTTON_DELETE);
}

uint32_t viewer_editor_paste_svg(viewer_editor *editor,
    const uint8_t *bytes, size_t length, bool paste_in_place)
{
    if (editor == NULL || editor->edit == NULL || bytes == NULL || length == 0U ||
        length > EDITOR_CLIPBOARD_FRAGMENT_LIMIT || memchr(bytes, 0, length) != NULL) {
        if (editor != NULL) editor_status_ok(editor,
            "Clipboard SVG is empty or exceeds the 64 KiB limit");
        return VIEWER_EDITOR_HANDLED;
    }
    document_handle parent = {0}, root = {0}, inserted = {0};
    if (!document_edit_root(editor->edit, &root)) return VIEWER_EDITOR_HANDLED;
    parent = root;
    base_bytes tag = {0};
    if (document_edit_tag_name(editor->edit, editor->selected, &tag) &&
        tag.len == 1U && tag.ptr[0] == 'g') parent = editor->selected;
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ?
        editor->selected_id : "");
    document_edit_error error = document_edit_insert_xml_fragment(editor->edit,
        parent, (base_bytes){bytes, length}, &inserted);
    if (error != DOCUMENT_EDIT_OK)
        return editor_apply_result(editor, error, old_offset, old_id,
            (document_handle){0});
    editor->canvas_selection_count = 0U;
    editor->canvas_selection_valid = false;
    editor_status_ok(editor, paste_in_place ?
        "SVG pasted in place · preview updating" : "SVG pasted · preview updating");
    return editor_apply_result(editor, DOCUMENT_EDIT_OK, old_offset, old_id,
        inserted);
}
