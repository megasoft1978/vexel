#include "viewer_editor_internal.h"

#include <stdlib.h>
#include <string.h>

typedef struct preview_visibility {
    size_t source_offset;
    bool visible;
} preview_visibility;

static int preview_visibility_descending(const void *left, const void *right)
{
    const preview_visibility *a = left;
    const preview_visibility *b = right;
    if (a->source_offset > b->source_offset) return -1;
    if (a->source_offset < b->source_offset) return 1;
    return 0;
}

bool viewer_editor_materialize_preview(viewer_editor *editor, uint8_t *buffer,
                                       size_t capacity, size_t *out_length)
{
    if (editor == NULL || buffer == NULL || out_length == NULL) return false;
    *out_length = 0U;
    size_t source_length = viewer_editor_byte_length(editor);
    if (source_length == 0U || source_length > capacity ||
        !viewer_editor_materialize(editor, buffer, capacity, out_length) ||
        *out_length != source_length) return false;
    if (editor->visibility_count == 0U) return true;

    preview_visibility overrides[EDITOR_VISIBILITY_OVERRIDE_CAP];
    for (size_t index = 0U; index < editor->visibility_count; ++index) {
        document_source_span span;
        if (!document_edit_source_span(editor->edit,
                editor->visibility_handles[index], &span)) return false;
        overrides[index] = (preview_visibility){span.offset,
            editor->visibility_values[index]};
    }
    qsort(overrides, editor->visibility_count, sizeof(overrides[0]),
        preview_visibility_descending);

    uint8_t *source_copy = malloc(source_length);
    if (source_copy == NULL) return false;
    memcpy(source_copy, buffer, source_length);
    document_edit_result created = document_edit_create(
        (base_bytes){source_copy, source_length}, document_edit_default_limits());
    if (created.error != DOCUMENT_EDIT_OK || created.edit == NULL) {
        free(source_copy);
        return false;
    }

    static const uint8_t display_name[] = "display";
    static const uint8_t hidden_value[] = "none";
    bool valid = true;
    for (size_t index = 0U; index < editor->visibility_count && valid; ++index) {
        document_handle element = {0};
        if (!document_edit_element_at_offset(created.edit,
                overrides[index].source_offset, &element)) {
            valid = false;
            break;
        }
        bool has_display = false;
        bool hidden = false;
        size_t attribute_count = document_edit_attribute_count(created.edit, element);
        for (size_t attribute_index = 0U; attribute_index < attribute_count;
             ++attribute_index) {
            document_edit_attribute attribute;
            base_bytes name;
            uint8_t value[16];
            size_t value_length = 0U;
            if (!document_edit_attribute_at(created.edit, element,
                    attribute_index, &attribute) ||
                !document_edit_span_bytes(created.edit, attribute.name, &name)) {
                valid = false;
                break;
            }
            if (name.len != sizeof(display_name) - 1U ||
                memcmp(name.ptr, display_name, sizeof(display_name) - 1U) != 0) continue;
            has_display = true;
            if (!document_edit_decode_span(created.edit, attribute.value,
                    value, sizeof(value), &value_length)) {
                valid = false;
                break;
            }
            hidden = value_length == sizeof(hidden_value) - 1U &&
                memcmp(value, hidden_value, sizeof(hidden_value) - 1U) == 0;
            break;
        }
        if (!valid) break;
        if (overrides[index].visible) {
            if (has_display && hidden &&
                document_edit_remove_attribute(created.edit, element,
                    (base_bytes){display_name, sizeof(display_name) - 1U}) !=
                    DOCUMENT_EDIT_OK) valid = false;
        } else if (!hidden &&
            document_edit_set_attribute(created.edit, element,
                (base_bytes){display_name, sizeof(display_name) - 1U},
                (base_bytes){hidden_value, sizeof(hidden_value) - 1U}) !=
                DOCUMENT_EDIT_OK) {
            valid = false;
        }
    }
    size_t preview_length = 0U;
    if (valid) valid = document_edit_materialize(created.edit, buffer,
        capacity, &preview_length);
    document_edit_destroy(created.edit);
    free(source_copy);
    if (!valid) return false;
    *out_length = preview_length;
    return true;
}
