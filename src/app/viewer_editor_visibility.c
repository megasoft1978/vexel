#include "viewer_editor_internal.h"

#include <string.h>

static bool bytes_are(base_bytes value, const char *text)
{
    size_t length = strlen(text);
    return value.len == length && memcmp(value.ptr, text, length) == 0;
}

static bool style_mentions_visibility(document_edit *edit,
    document_handle node)
{
    size_t count = document_edit_attribute_count(edit, node);
    for (size_t i = 0U; i < count; ++i) {
        document_edit_attribute attribute; base_bytes name = {0};
        if (!document_edit_attribute_at(edit, node, i, &attribute) ||
            !document_edit_span_bytes(edit, attribute.name, &name)) return true;
        if (!bytes_are(name, "style")) continue;
        base_bytes style = {0};
        if (!document_edit_span_bytes(edit, attribute.value, &style)) return true;
        for (size_t cursor = 0U; cursor + 10U <= style.len; ++cursor) {
            if ((cursor == 0U || style.ptr[cursor - 1U] == ';' ||
                    style.ptr[cursor - 1U] == ' ' || style.ptr[cursor - 1U] == '\t') &&
                memcmp(style.ptr + cursor, "visibility", 10U) == 0) {
                size_t after = cursor + 10U;
                while (after < style.len && (style.ptr[after] == ' ' ||
                        style.ptr[after] == '\t')) after++;
                if (after < style.len && style.ptr[after] == ':') return true;
            }
        }
    }
    return false;
}

static bool visibility_value(document_edit *edit, document_handle node,
    bool *found, bool *visible)
{
    size_t count = document_edit_attribute_count(edit, node);
    for (size_t i = 0U; i < count; ++i) {
        document_edit_attribute attribute; base_bytes name = {0};
        if (!document_edit_attribute_at(edit, node, i, &attribute) ||
            !document_edit_span_bytes(edit, attribute.name, &name)) return false;
        if (!bytes_are(name, "visibility")) continue;
        if (*found) return false;
        base_bytes value = {0};
        if (!document_edit_span_bytes(edit, attribute.value, &value)) return false;
        if (bytes_are(value, "visible")) *visible = true;
        else if (bytes_are(value, "hidden") || bytes_are(value, "collapse"))
            *visible = false;
        else return false;
        *found = true;
    }
    return true;
}

bool viewer_editor_selected_visibility_target(viewer_editor *editor,
    uint8_t *id_buffer, size_t id_capacity, size_t *out_id_length,
    bool *out_visible)
{
    if (out_id_length != NULL) *out_id_length = 0U;
    if (out_visible != NULL) *out_visible = true;
    if (editor == NULL || editor->edit == NULL || id_buffer == NULL ||
        id_capacity == 0U || out_id_length == NULL || out_visible == NULL ||
        editor->canvas_selection_count > 1U) return false;
    document_handle selected = editor->selected;
    bool selected_id = false, selected_has_visibility = false;
    bool effective_found = false, visible = true;
    size_t selected_id_length = 0U;
    const document_model *model = document_edit_model(editor->edit);
    if (model == NULL) return false;
    for (size_t slot = 0U; slot < document_model_node_count(model); ++slot) {
        document_handle node = {0}; base_bytes tag = {0};
        if (!document_model_node_at(model, slot, &node) ||
            !document_model_tag_name(model, node, &tag)) return false;
        if (bytes_are(tag, "style")) return false;
    }
    document_handle current = selected;
    for (size_t depth = 0U; depth < 256U; ++depth) {
        if (style_mentions_visibility(editor->edit, current)) return false;
        bool has_local = false, local_visible = true;
        if (!visibility_value(editor->edit, current, &has_local, &local_visible))
            return false;
        if (has_local && !effective_found) {
            visible = local_visible;
            effective_found = true;
        }
        if (current.document_id == selected.document_id &&
            current.slot == selected.slot && current.generation == selected.generation) {
            size_t count = document_edit_attribute_count(editor->edit, current);
            for (size_t i = 0U; i < count; ++i) {
                document_edit_attribute attribute; base_bytes name = {0};
                if (!document_edit_attribute_at(editor->edit, current, i, &attribute) ||
                    !document_edit_span_bytes(editor->edit, attribute.name, &name))
                    return false;
                if (!bytes_are(name, "id")) continue;
                if (selected_id || !document_edit_decode_span(editor->edit,
                        attribute.value, id_buffer, id_capacity,
                        &selected_id_length) || selected_id_length == 0U ||
                    selected_id_length > 256U) return false;
                selected_id = true;
                if (has_local) selected_has_visibility = true;
            }
        }
        document_handle parent = {0};
        if (!document_model_parent(model, current, &parent)) break;
        current = parent;
        if (depth == 255U) return false;
    }
    if (!selected_id || (!selected_has_visibility && effective_found && !visible))
        return false;
    size_t matches = 0U;
    for (size_t slot = 0U; slot < document_model_node_count(model); ++slot) {
        document_handle node = {0};
        if (!document_model_node_at(model, slot, &node)) continue;
        size_t count = document_edit_attribute_count(editor->edit, node);
        for (size_t i = 0U; i < count; ++i) {
            document_edit_attribute attribute; base_bytes name = {0};
            if (!document_edit_attribute_at(editor->edit, node, i, &attribute) ||
                !document_edit_span_bytes(editor->edit, attribute.name, &name))
                return false;
            if (!bytes_are(name, "id")) continue;
            uint8_t candidate[257U]; size_t length = 0U;
            if (!document_edit_decode_span(editor->edit, attribute.value,
                    candidate, sizeof(candidate), &length)) return false;
            if (length == selected_id_length &&
                memcmp(candidate, id_buffer, length) == 0) {
                matches++;
                if (node.document_id != selected.document_id ||
                    node.slot != selected.slot || node.generation != selected.generation)
                    return false;
            }
        }
    }
    if (matches != 1U) return false;
    *out_id_length = selected_id_length;
    *out_visible = visible;
    return true;
}
