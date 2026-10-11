#include "viewer_editor_internal.h"

#include "document_edit_internal.h"
#include "document_model.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define EDITOR_MULTI_EDIT_BYTES (64U * 1024U)

typedef struct editor_selected_node {
    document_handle handle;
    document_handle parent;
    document_source_span span;
} editor_selected_node;

typedef struct editor_source_patch {
    size_t offset;
    size_t remove_length;
    uint8_t *insert;
    size_t insert_length;
} editor_source_patch;

static int patch_compare(const void *left, const void *right)
{
    const editor_source_patch *a = left;
    const editor_source_patch *b = right;
    if (a->offset < b->offset) return -1;
    if (a->offset > b->offset) return 1;
    return 0;
}

static int selected_node_compare(const void *left, const void *right)
{
    const editor_selected_node *a = left;
    const editor_selected_node *b = right;
    if (a->span.offset < b->span.offset) return -1;
    if (a->span.offset > b->span.offset) return 1;
    return 0;
}

static void patches_destroy(editor_source_patch *patches, size_t count)
{
    if (patches == NULL) return;
    for (size_t index = 0U; index < count; ++index) free(patches[index].insert);
}

static bool nodes_collect(viewer_editor *editor,
    editor_selected_node nodes[EDITOR_CANVAS_SELECTION_CAP], size_t *out_count)
{
    if (editor == NULL || nodes == NULL || out_count == NULL ||
        editor->canvas_selection_count < 2U ||
        editor->canvas_selection_count > EDITOR_CANVAS_SELECTION_CAP) return false;
    const document_model *model = editor_model(editor);
    document_handle common_parent = {0U};
    size_t count = 0U;
    for (size_t index = 0U; index < editor->canvas_selection_count; ++index) {
        document_handle handle = {0U}, parent = {0U};
        document_source_span span = {0U, 0U};
        if (!document_edit_element_at_offset(editor->edit,
                editor->canvas_selection_offsets[index], &handle) ||
            !document_edit_source_span(editor->edit, handle, &span) ||
            !document_model_parent(model, handle, &parent)) return false;
        if (common_parent.document_id == 0U) common_parent = parent;
        else if (!editor_same_handle(common_parent, parent)) return false;
        bool duplicate = false;
        for (size_t seen = 0U; seen < count; ++seen)
            if (nodes[seen].span.offset == span.offset) { duplicate = true; break; }
        if (duplicate) continue;
        nodes[count++] = (editor_selected_node){handle, parent, span};
    }
    if (count < 2U) return false;
    qsort(nodes, count, sizeof(nodes[0]), selected_node_compare);
    *out_count = count;
    return true;
}

static bool range_for_nodes(const editor_selected_node *nodes, size_t count,
                            size_t *out_start, size_t *out_end)
{
    if (nodes == NULL || count == 0U || out_start == NULL || out_end == NULL)
        return false;
    size_t start = nodes[0].span.offset;
    size_t end = start;
    for (size_t index = 0U; index < count; ++index) {
        if (nodes[index].span.offset < start) start = nodes[index].span.offset;
        size_t node_end;
        if (!base_size_add(nodes[index].span.offset, nodes[index].span.length,
                &node_end)) return false;
        if (node_end > end) end = node_end;
        if (index > 0U) {
            size_t previous_end;
            if (!base_size_add(nodes[index - 1U].span.offset,
                    nodes[index - 1U].span.length, &previous_end) ||
                previous_end > nodes[index].span.offset) return false;
        }
    }
    if (end - start > EDITOR_MULTI_EDIT_BYTES) return false;
    *out_start = start;
    *out_end = end;
    return true;
}

static bool commit_region(viewer_editor *editor, document_handle parent,
                          size_t start, size_t end,
                          const uint8_t *bytes, size_t length,
                          document_edit_history_action action,
                          document_edit_error *out_error)
{
    if (out_error != NULL) *out_error = DOCUMENT_EDIT_LIMIT;
    if (editor == NULL || (length > 0U && bytes == NULL) || end < start ||
        end - start > EDITOR_MULTI_EDIT_BYTES ||
        length > EDITOR_MULTI_EDIT_BYTES) return false;
    size_t original_length = end - start;
    document_edit_error error = document_edit_replace_xml_region_labeled(editor->edit,
        parent, (document_source_span){start, original_length},
        (base_bytes){bytes, length}, action);
    if (out_error != NULL) *out_error = error;
    return error == DOCUMENT_EDIT_OK;
}

static bool build_patch_result(document_edit *edit, size_t start, size_t end,
    editor_source_patch *patches, size_t patch_count,
    uint8_t **out_bytes, size_t *out_length)
{
    if (edit == NULL || start > end || patches == NULL || patch_count == 0U ||
        out_bytes == NULL || out_length == NULL) return false;
    qsort(patches, patch_count, sizeof(patches[0]), patch_compare);
    base_bytes original = {0};
    if (!document_edit_span_bytes(edit, (document_source_span){start, end - start},
            &original) || original.len > EDITOR_MULTI_EDIT_BYTES) return false;
    size_t new_length = original.len;
    size_t previous_end = start;
    for (size_t index = 0U; index < patch_count; ++index) {
        editor_source_patch *patch = &patches[index];
        if (patch->offset < previous_end || patch->offset < start ||
            patch->offset - start > original.len ||
            patch->remove_length > original.len - (patch->offset - start) ||
            (patch->insert_length > 0U && patch->insert == NULL)) return false;
        previous_end = patch->offset + patch->remove_length;
        if (new_length < patch->remove_length) return false;
        new_length -= patch->remove_length;
        if (!base_size_add(new_length, patch->insert_length, &new_length) ||
            new_length > EDITOR_MULTI_EDIT_BYTES) return false;
    }
    uint8_t *bytes = new_length == 0U ? NULL : malloc(new_length);
    if (new_length > 0U && bytes == NULL) return false;
    size_t source_cursor = 0U, output_cursor = 0U;
    for (size_t index = 0U; index < patch_count; ++index) {
        size_t patch_start = patches[index].offset - start;
        size_t gap = patch_start - source_cursor;
        if (gap > 0U) memcpy(bytes + output_cursor, original.ptr + source_cursor, gap);
        output_cursor += gap;
        if (patches[index].insert_length > 0U)
            memcpy(bytes + output_cursor, patches[index].insert,
                patches[index].insert_length);
        output_cursor += patches[index].insert_length;
        source_cursor = patch_start + patches[index].remove_length;
    }
    size_t tail = original.len - source_cursor;
    if (tail > 0U) memcpy(bytes + output_cursor, original.ptr + source_cursor, tail);
    output_cursor += tail;
    if (output_cursor != new_length) { free(bytes); return false; }
    *out_bytes = bytes;
    *out_length = new_length;
    return true;
}

static size_t map_offset_after_patches(size_t offset,
    const editor_source_patch *patches, size_t patch_count)
{
    size_t added = 0U, removed = 0U;
    for (size_t index = 0U; index < patch_count; ++index) {
        if (patches[index].offset >= offset) continue;
        if (!base_size_add(added, patches[index].insert_length, &added) ||
            !base_size_add(removed, patches[index].remove_length, &removed)) return offset;
    }
    if (added >= removed) {
        size_t mapped;
        return base_size_add(offset, added - removed, &mapped) ? mapped : offset;
    }
    return offset >= removed - added ? offset - (removed - added) : offset;
}

static bool format_transform(char operation[256], size_t *out_length,
    double translate_x, double translate_y, double scale_x, double scale_y,
    double rotate_radians, double pivot_x, double pivot_y)
{
    if (operation == NULL || out_length == NULL || !isfinite(translate_x) ||
        !isfinite(translate_y) || !isfinite(scale_x) || !isfinite(scale_y) ||
        !isfinite(rotate_radians) || !isfinite(pivot_x) || !isfinite(pivot_y) ||
        !(scale_x > 0.0) || !(scale_y > 0.0)) return false;
    int amount;
    if (rotate_radians != 0.0) {
        double degrees = rotate_radians * (180.0 / 3.14159265358979323846);
        amount = snprintf(operation, 256U,
            "translate(%.9g %.9g) rotate(%.9g) translate(%.9g %.9g)",
            pivot_x, pivot_y, degrees, -pivot_x, -pivot_y);
    } else if (scale_x != 1.0 || scale_y != 1.0) {
        amount = snprintf(operation, 256U,
            "translate(%.9g %.9g) scale(%.9g %.9g) translate(%.9g %.9g)",
            pivot_x, pivot_y, scale_x, scale_y, -pivot_x, -pivot_y);
    } else {
        amount = snprintf(operation, 256U, "translate(%.9g %.9g)",
            translate_x, translate_y);
    }
    if (amount <= 0 || amount >= 256) return false;
    *out_length = (size_t)amount;
    return true;
}

static bool encode_attribute_value(base_bytes value, uint8_t quote,
                                   uint8_t **out_bytes, size_t *out_length)
{
    if (!base_bytes_valid(value) || out_bytes == NULL || out_length == NULL ||
        (quote != '\'' && quote != '"')) return false;
    size_t capacity;
    if (!base_size_mul(value.len, 6U, &capacity) ||
        !base_size_add(capacity, 1U, &capacity)) return false;
    uint8_t *bytes = malloc(capacity);
    if (bytes == NULL) return false;
    size_t used = 0U;
    for (size_t index = 0U; index < value.len; ++index) {
        const char *replacement = NULL;
        size_t length = 0U;
        if (value.ptr[index] == '&') { replacement = "&amp;"; length = 5U; }
        else if (value.ptr[index] == '<') { replacement = "&lt;"; length = 4U; }
        else if (value.ptr[index] == quote && quote == '"') {
            replacement = "&quot;"; length = 6U;
        } else if (value.ptr[index] == quote && quote == '\'') {
            replacement = "&apos;"; length = 6U;
        }
        if (replacement != NULL) {
            memcpy(bytes + used, replacement, length);
            used += length;
        } else bytes[used++] = value.ptr[index];
    }
    *out_bytes = bytes;
    *out_length = used;
    return true;
}

static bool transform_patch(viewer_editor *editor, document_handle handle,
    document_source_span element, const char *operation, size_t operation_length,
    editor_source_patch *out_patch)
{
    size_t count = document_edit_attribute_count(editor->edit, handle);
    document_edit_attribute transform = {0};
    bool found = false;
    base_bytes name = {0};
    for (size_t index = 0U; index < count; ++index) {
        document_edit_attribute attribute;
        base_bytes attribute_name;
        if (!document_edit_attribute_at(editor->edit, handle, index, &attribute) ||
            !document_edit_span_bytes(editor->edit, attribute.name, &attribute_name))
            return false;
        if (attribute_name.len == 9U &&
            memcmp(attribute_name.ptr, "transform", 9U) == 0) {
            transform = attribute;
            name = attribute_name;
            found = true;
            break;
        }
    }
    uint8_t *old_value = NULL;
    size_t old_length = 0U;
    uint8_t quote = '"';
    size_t patch_offset = 0U, remove_length = 0U;
    if (found) {
        old_value = malloc(EDITOR_FIELD_CAP);
        if (old_value == NULL || !document_edit_decode_span(editor->edit,
                transform.value, old_value, EDITOR_FIELD_CAP, &old_length)) {
            free(old_value);
            return false;
        }
        quote = transform.quote;
        patch_offset = transform.value.offset;
        remove_length = transform.value.length;
    } else {
        base_bytes raw = {0};
        if (!document_edit_span_bytes(editor->edit, element, &raw)) return false;
        uint8_t active_quote = 0U;
        size_t tag_end = SIZE_MAX;
        for (size_t index = 0U; index < raw.len; ++index) {
            uint8_t value = raw.ptr[index];
            if (active_quote != 0U) {
                if (value == active_quote) active_quote = 0U;
                continue;
            }
            if (value == '\'' || value == '"') active_quote = value;
            else if (value == '>') { tag_end = index; break; }
        }
        if (tag_end == SIZE_MAX) return false;
        size_t insertion = tag_end;
        if (insertion > 0U && raw.ptr[insertion - 1U] == '/') --insertion;
        patch_offset = element.offset + insertion;
    }
    size_t combined_length = operation_length;
    if (old_length > 0U &&
        (!base_size_add(combined_length, 1U, &combined_length) ||
         !base_size_add(combined_length, old_length, &combined_length) ||
         combined_length >= EDITOR_FIELD_CAP)) {
        free(old_value);
        return false;
    }
    uint8_t *combined = malloc(combined_length == 0U ? 1U : combined_length);
    if (combined == NULL) { free(old_value); return false; }
    memcpy(combined, operation, operation_length);
    size_t used = operation_length;
    if (old_length > 0U) {
        combined[used++] = ' ';
        memcpy(combined + used, old_value, old_length);
        used += old_length;
    }
    free(old_value);
    uint8_t *encoded = NULL;
    size_t encoded_length = 0U;
    if (!encode_attribute_value((base_bytes){combined, used}, quote,
            &encoded, &encoded_length)) {
        free(combined);
        return false;
    }
    free(combined);
    if (!found) {
        size_t total;
        if (!base_size_add(encoded_length, 13U, &total)) { free(encoded); return false; }
        uint8_t *attribute = malloc(total);
        if (attribute == NULL) { free(encoded); return false; }
        memcpy(attribute, " transform=\"", 12U);
        if (encoded_length > 0U) memcpy(attribute + 12U, encoded, encoded_length);
        attribute[12U + encoded_length] = '"';
        free(encoded);
        *out_patch = (editor_source_patch){patch_offset, 0U, attribute,
            13U + encoded_length};
    } else {
        *out_patch = (editor_source_patch){patch_offset, remove_length,
            encoded, encoded_length};
    }
    (void)name;
    return true;
}

static uint32_t apply_region_change(viewer_editor *editor, document_handle parent,
    size_t start, size_t end,
    const uint8_t *replacement, size_t replacement_length,
    size_t old_selection_offset, const char *old_id, const size_t *restore_offsets,
    size_t restore_count, document_edit_history_action action)
{
    document_edit_error error = DOCUMENT_EDIT_OK;
    bool committed = commit_region(editor, parent, start, end, replacement,
        replacement_length, action, &error);
    if (!committed) {
        editor_status_error(editor, error);
        return VIEWER_EDITOR_HANDLED;
    }
    editor_rebind_after_change(editor, old_selection_offset, old_id,
        (document_handle){0U});
    if (restore_count > 0U)
        (void)editor_selection_restore(editor, restore_offsets, restore_count);
    editor_status_ok(editor, "Selection edit committed · preview updating");
    return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_MUTATED;
}

uint32_t editor_canvas_multi_transform(viewer_editor *editor,
    double translate_x, double translate_y, double scale_x, double scale_y,
    double rotate_radians, double pivot_x, double pivot_y)
{
    if (editor_canvas_selection_is_locked(editor)) {
        editor_status_ok(editor, "Unlock every selected layer to transform them");
        return VIEWER_EDITOR_HANDLED;
    }
    editor_selected_node nodes[EDITOR_CANVAS_SELECTION_CAP];
    editor_source_patch patches[EDITOR_CANVAS_SELECTION_CAP] = {{0}};
    size_t count = 0U, start = 0U, end = 0U;
    uint8_t operation[256];
    size_t operation_length = 0U;
    uint8_t *replacement = NULL;
    size_t replacement_length = 0U;
    size_t restored[EDITOR_CANVAS_SELECTION_CAP];
    uint32_t result = VIEWER_EDITOR_HANDLED;
    if (!nodes_collect(editor, nodes, &count) ||
        !range_for_nodes(nodes, count, &start, &end) ||
        !format_transform((char *)operation, &operation_length,
            translate_x, translate_y, scale_x, scale_y, rotate_radians,
            pivot_x, pivot_y)) {
        editor_status_ok(editor, "Group edit needs sibling elements within the 64 KiB edit limit");
        return result;
    }
    for (size_t index = 0U; index < count; ++index) {
        if (!transform_patch(editor, nodes[index].handle, nodes[index].span,
                (const char *)operation, operation_length, &patches[index])) {
            patches_destroy(patches, index);
            editor_status_ok(editor, "Group transform could not be encoded");
            return result;
        }
        restored[index] = map_offset_after_patches(nodes[index].span.offset,
            patches, index + 1U);
    }
    if (!build_patch_result(editor->edit, start, end, patches, count,
            &replacement, &replacement_length)) {
        patches_destroy(patches, count);
        editor_status_ok(editor, "Group edit exceeds the 64 KiB source limit");
        return result;
    }
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ?
        editor->selected_id : "");
    document_edit_history_action action = rotate_radians != 0.0 ?
        DOCUMENT_EDIT_HISTORY_ACTION_ROTATE :
        (scale_x != 1.0 || scale_y != 1.0 ?
            DOCUMENT_EDIT_HISTORY_ACTION_RESIZE :
            DOCUMENT_EDIT_HISTORY_ACTION_MOVE);
    result = apply_region_change(editor, nodes[0].parent, start, end, replacement,
        replacement_length, old_offset, old_id, restored, count, action);
    free(replacement);
    patches_destroy(patches, count);
    return result;
}

uint32_t editor_canvas_multi_translate_each(viewer_editor *editor,
    const double *translate_x, const double *translate_y, size_t delta_count)
{
    editor_selected_node nodes[EDITOR_CANVAS_SELECTION_CAP];
    editor_source_patch patches[EDITOR_CANVAS_SELECTION_CAP] = {{0}};
    size_t node_count = 0U, start = 0U, end = 0U, patch_count = 0U;
    uint8_t operation[256];
    uint8_t *replacement = NULL;
    size_t replacement_length = 0U;
    size_t restored[EDITOR_CANVAS_SELECTION_CAP];
    if (editor == NULL || translate_x == NULL || translate_y == NULL ||
        delta_count != editor->canvas_selection_count ||
        delta_count > EDITOR_CANVAS_SELECTION_CAP ||
        !nodes_collect(editor, nodes, &node_count) || node_count != delta_count ||
        !range_for_nodes(nodes, node_count, &start, &end)) {
        if (editor != NULL)
            editor_status_ok(editor,
                "Arrange needs sibling elements within the 64 KiB edit limit");
        return VIEWER_EDITOR_HANDLED;
    }
    for (size_t index = 0U; index < node_count; ++index) {
        size_t selected_index = 0U;
        while (selected_index < delta_count &&
            editor->canvas_selection_offsets[selected_index] !=
                nodes[index].span.offset) ++selected_index;
        if (selected_index == delta_count ||
            !isfinite(translate_x[selected_index]) ||
            !isfinite(translate_y[selected_index]) ||
            fabs(translate_x[selected_index]) > 1000000.0 ||
            fabs(translate_y[selected_index]) > 1000000.0) {
            patches_destroy(patches, patch_count);
            editor_status_ok(editor, "Arrange translation exceeds the editor limit");
            return VIEWER_EDITOR_HANDLED;
        }
        if (translate_x[selected_index] == 0.0 &&
            translate_y[selected_index] == 0.0) continue;
        size_t operation_length = 0U;
        if (!format_transform((char *)operation, &operation_length,
                translate_x[selected_index], translate_y[selected_index],
                1.0, 1.0, 0.0, 0.0, 0.0) ||
            !transform_patch(editor, nodes[index].handle, nodes[index].span,
                (const char *)operation, operation_length,
                &patches[patch_count])) {
            patches_destroy(patches, patch_count);
            editor_status_ok(editor, "Arrange transform could not be encoded");
            return VIEWER_EDITOR_HANDLED;
        }
        ++patch_count;
    }
    if (patch_count == 0U) {
        editor_status_ok(editor, "Selection already has the requested alignment");
        return VIEWER_EDITOR_HANDLED;
    }
    if (!build_patch_result(editor->edit, start, end, patches, patch_count,
            &replacement, &replacement_length)) {
        patches_destroy(patches, patch_count);
        editor_status_ok(editor, "Arrange exceeds the 64 KiB source limit");
        return VIEWER_EDITOR_HANDLED;
    }
    for (size_t index = 0U; index < node_count; ++index)
        restored[index] = map_offset_after_patches(nodes[index].span.offset,
            patches, patch_count);
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ?
        editor->selected_id : "");
    uint32_t result = apply_region_change(editor, nodes[0].parent, start, end,
        replacement, replacement_length, old_offset, old_id, restored,
        node_count, DOCUMENT_EDIT_HISTORY_ACTION_MOVE);
    free(replacement);
    patches_destroy(patches, patch_count);
    return result;
}

uint32_t editor_canvas_multi_delete(viewer_editor *editor)
{
    if (editor_canvas_selection_is_locked(editor)) {
        editor_status_ok(editor, "Unlock every selected layer to delete them");
        return VIEWER_EDITOR_HANDLED;
    }
    editor_selected_node nodes[EDITOR_CANVAS_SELECTION_CAP];
    size_t count = 0U, start = 0U, end = 0U;
    if (!nodes_collect(editor, nodes, &count) ||
        !range_for_nodes(nodes, count, &start, &end)) {
        editor_status_ok(editor, "Delete needs sibling elements within the 64 KiB edit limit");
        return VIEWER_EDITOR_HANDLED;
    }
    base_bytes original = {0};
    if (!document_edit_span_bytes(editor->edit,
            (document_source_span){start, end - start}, &original)) {
        editor_status_error(editor, DOCUMENT_EDIT_INVALID_OPERATION);
        return VIEWER_EDITOR_HANDLED;
    }
    size_t removed = 0U;
    for (size_t index = 0U; index < count; ++index)
        if (!base_size_add(removed, nodes[index].span.length, &removed)) {
            editor_status_error(editor, DOCUMENT_EDIT_LIMIT);
            return VIEWER_EDITOR_HANDLED;
        }
    if (removed > original.len) return VIEWER_EDITOR_HANDLED;
    size_t length = original.len - removed;
    uint8_t *replacement = length == 0U ? NULL : malloc(length);
    if (length > 0U && replacement == NULL) {
        editor_status_error(editor, DOCUMENT_EDIT_OUT_OF_MEMORY);
        return VIEWER_EDITOR_HANDLED;
    }
    size_t cursor = start, output = 0U;
    for (size_t index = 0U; index < count; ++index) {
        size_t gap = nodes[index].span.offset - cursor;
        if (gap > 0U) memcpy(replacement + output,
            original.ptr + cursor - start, gap);
        output += gap;
        cursor = nodes[index].span.offset + nodes[index].span.length;
    }
    size_t tail = end - cursor;
    if (tail > 0U) memcpy(replacement + output, original.ptr + cursor - start, tail);
    output += tail;
    if (output != length) { free(replacement); return VIEWER_EDITOR_HANDLED; }
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ?
        editor->selected_id : "");
    uint32_t result = apply_region_change(editor, nodes[0].parent, start, end, replacement,
        length, old_offset, old_id, NULL, 0U,
        DOCUMENT_EDIT_HISTORY_ACTION_DELETE);
    editor->canvas_selection_count = 0U;
    editor->canvas_selection_valid = false;
    free(replacement);
    editor_status_ok(editor, "Elements deleted · preview updating");
    return result;
}

uint32_t editor_canvas_multi_duplicate(viewer_editor *editor)
{
    if (editor_canvas_selection_is_locked(editor)) {
        editor_status_ok(editor, "Unlock every selected layer to duplicate them");
        return VIEWER_EDITOR_HANDLED;
    }
    editor_selected_node nodes[EDITOR_CANVAS_SELECTION_CAP];
    size_t count = 0U, start = 0U, end = 0U;
    if (!nodes_collect(editor, nodes, &count) ||
        !range_for_nodes(nodes, count, &start, &end)) {
        editor_status_ok(editor, "Duplicate needs sibling elements within the 64 KiB edit limit");
        return VIEWER_EDITOR_HANDLED;
    }
    base_bytes original = {0};
    if (!document_edit_span_bytes(editor->edit,
            (document_source_span){start, end - start}, &original)) {
        editor_status_error(editor, DOCUMENT_EDIT_INVALID_OPERATION);
        return VIEWER_EDITOR_HANDLED;
    }
    size_t duplicate_length = 0U;
    for (size_t index = 0U; index < count; ++index)
        if (!base_size_add(duplicate_length, nodes[index].span.length,
                &duplicate_length)) {
            editor_status_error(editor, DOCUMENT_EDIT_LIMIT);
            return VIEWER_EDITOR_HANDLED;
        }
    size_t new_length;
    if (!base_size_add(original.len, duplicate_length, &new_length) ||
        new_length > EDITOR_MULTI_EDIT_BYTES) {
        editor_status_ok(editor, "Duplicate exceeds the 64 KiB source limit");
        return VIEWER_EDITOR_HANDLED;
    }
    uint8_t *replacement = malloc(new_length);
    if (replacement == NULL) {
        editor_status_error(editor, DOCUMENT_EDIT_OUT_OF_MEMORY);
        return VIEWER_EDITOR_HANDLED;
    }
    memcpy(replacement, original.ptr, original.len);
    size_t write_at = original.len;
    size_t duplicate_offsets[EDITOR_CANVAS_SELECTION_CAP];
    size_t duplicate_count = 0U;
    for (size_t index = 0U; index < count; ++index) {
        base_bytes raw = {0};
        if (!document_edit_span_bytes(editor->edit, nodes[index].span, &raw)) {
            free(replacement);
            editor_status_error(editor, DOCUMENT_EDIT_INVALID_OPERATION);
            return VIEWER_EDITOR_HANDLED;
        }
        duplicate_offsets[duplicate_count++] = end + write_at - original.len;
        memcpy(replacement + write_at, raw.ptr, raw.len);
        write_at += raw.len;
    }
    if (write_at != new_length) { free(replacement); return VIEWER_EDITOR_HANDLED; }
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ?
        editor->selected_id : "");
    uint32_t result = apply_region_change(editor, nodes[0].parent, start, end, replacement,
        new_length, old_offset, old_id, duplicate_offsets, duplicate_count,
        DOCUMENT_EDIT_HISTORY_ACTION_DUPLICATE);
    free(replacement);
    editor_status_ok(editor, "Elements duplicated · preview updating");
    return result;
}
