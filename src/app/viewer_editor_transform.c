#include "viewer_editor_internal.h"

#include "document_edit_internal.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static bool transform_space(char value)
{
    return value == ' ' || value == '\t' || value == '\r' || value == '\n';
}

static bool parse_matrix_prefix(char *text, size_t length,
    geom_affine *out_matrix, size_t *out_suffix)
{
    if (text == NULL || out_matrix == NULL || out_suffix == NULL) return false;
    size_t at = 0U;
    while (at < length && transform_space(text[at])) ++at;
    static const char prefix[] = "matrix(";
    if (length - at < sizeof(prefix) - 1U ||
        memcmp(text + at, prefix, sizeof(prefix) - 1U) != 0) return false;
    at += sizeof(prefix) - 1U;
    double values[6];
    for (size_t index = 0U; index < 6U; ++index) {
        size_t before = at;
        while (at < length && transform_space(text[at])) ++at;
        bool had_space = at > before;
        if (index > 0U && at < length && text[at] == ',') {
            ++at;
            while (at < length && transform_space(text[at])) ++at;
            had_space = true;
        }
        if (index > 0U && !had_space && at < length &&
            text[at] != '+' && text[at] != '-') return false;
        char *end = NULL;
        values[index] = strtod(text + at, &end);
        if (end == text + at || end == NULL || !isfinite(values[index])) return false;
        at = (size_t)(end - text);
        if (at > length) return false;
    }
    while (at < length && transform_space(text[at])) ++at;
    if (at >= length || text[at] != ')') return false;
    ++at;
    *out_matrix = (geom_affine){values[0], values[1], values[2], values[3],
        values[4], values[5]};
    *out_suffix = at;
    return geom_affine_valid(*out_matrix);
}

static bool format_matrix(char output[256], geom_affine matrix, size_t *out_length)
{
    if (output == NULL || out_length == NULL || !geom_affine_valid(matrix)) return false;
    double values[6] = {matrix.a, matrix.b, matrix.c, matrix.d, matrix.e, matrix.f};
    for (size_t index = 0U; index < 6U; ++index)
        if (values[index] == 0.0) values[index] = 0.0;
    int length = snprintf(output, 256U, "matrix(%.9g %.9g %.9g %.9g %.9g %.9g)",
        values[0], values[1], values[2], values[3], values[4], values[5]);
    if (length <= 0 || length >= 256) return false;
    *out_length = (size_t)length;
    return true;
}

uint32_t editor_commit_transform(viewer_editor *editor,
    double translate_x, double translate_y, double scale_x, double scale_y,
    double rotate_radians, double pivot_x, double pivot_y)
{
    if (editor == NULL || editor->edit == NULL ||
        !isfinite(translate_x) || !isfinite(translate_y) ||
        !isfinite(scale_x) || !isfinite(scale_y) ||
        !isfinite(rotate_radians) || !isfinite(pivot_x) || !isfinite(pivot_y) ||
        !(scale_x > 0.0) || !(scale_y > 0.0)) return VIEWER_EDITOR_HANDLED;
    if (editor_canvas_selection_is_locked(editor)) {
        editor_status_ok(editor, "Unlock the selected layer to transform it");
        return VIEWER_EDITOR_HANDLED;
    }
    double cosine = cos(rotate_radians), sine = sin(rotate_radians);
    geom_affine next = {
        cosine * scale_x, sine * scale_x,
        -sine * scale_y, cosine * scale_y,
        pivot_x + translate_x - cosine * scale_x * pivot_x +
            sine * scale_y * pivot_y,
        pivot_y + translate_y - sine * scale_x * pivot_x -
            cosine * scale_y * pivot_y
    };
    char matrix_text[256];
    size_t matrix_length = 0U;
    if (!format_matrix(matrix_text, next, &matrix_length)) {
        editor_status_ok(editor, "Transform exceeds the editor limit");
        return VIEWER_EDITOR_HANDLED;
    }

    document_edit_attribute transform_attribute = {0};
    bool has_transform = false;
    size_t attribute_count = document_edit_attribute_count(editor->edit,
        editor->selected);
    for (size_t index = 0U; index < attribute_count; ++index) {
        document_edit_attribute attribute;
        base_bytes name;
        if (!document_edit_attribute_at(editor->edit, editor->selected, index,
                &attribute) || !document_edit_span_bytes(editor->edit,
                attribute.name, &name)) continue;
        if (name.len == 9U && memcmp(name.ptr, "transform", 9U) == 0) {
            transform_attribute = attribute;
            has_transform = true;
            break;
        }
    }
    size_t old_length = 0U;
    uint8_t *old_raw = NULL;
    if (has_transform) {
        if (transform_attribute.value.length >= EDITOR_FIELD_CAP) {
            editor_status_ok(editor, "Existing transform exceeds the editor limit");
            return VIEWER_EDITOR_HANDLED;
        }
        old_raw = malloc(transform_attribute.value.length + 1U);
        if (old_raw == NULL) {
            editor_status_ok(editor, "Transform allocation failed");
            return VIEWER_EDITOR_HANDLED;
        }
        base_bytes raw;
        if (!document_edit_span_bytes(editor->edit, transform_attribute.value, &raw) ||
            raw.len != transform_attribute.value.length) {
            free(old_raw);
            editor_status_ok(editor, "Existing transform is unavailable");
            return VIEWER_EDITOR_HANDLED;
        }
        old_length = raw.len;
        if (old_length > 0U) memcpy(old_raw, raw.ptr, old_length);
        old_raw[old_length] = '\0';
    }

    size_t suffix_offset = 0U;
    geom_affine previous;
    bool combine = has_transform && parse_matrix_prefix((char *)old_raw,
        old_length, &previous, &suffix_offset);
    geom_affine combined = next;
    if (combine && !geom_affine_compose(next, previous, &combined)) {
        free(old_raw);
        editor_status_ok(editor, "Transform composition failed");
        return VIEWER_EDITOR_HANDLED;
    }
    if (!format_matrix(matrix_text, combined, &matrix_length)) {
        free(old_raw);
        editor_status_ok(editor, "Transform exceeds the editor limit");
        return VIEWER_EDITOR_HANDLED;
    }
    size_t suffix_length = has_transform ?
        old_length - (combine ? suffix_offset : 0U) : 0U;
    size_t separator = has_transform && !combine && old_length > 0U ? 1U : 0U;
    size_t value_length = matrix_length;
    if (!base_size_add(value_length, separator, &value_length) ||
        !base_size_add(value_length, suffix_length, &value_length) ||
        value_length >= EDITOR_FIELD_CAP) {
        free(old_raw);
        editor_status_ok(editor, "Transform exceeds the editor limit");
        return VIEWER_EDITOR_HANDLED;
    }
    uint8_t *value = malloc(value_length == 0U ? 1U : value_length);
    if (value == NULL) {
        free(old_raw);
        editor_status_ok(editor, "Transform allocation failed");
        return VIEWER_EDITOR_HANDLED;
    }
    memcpy(value, matrix_text, matrix_length);
    size_t used = matrix_length;
    if (separator != 0U) value[used++] = ' ';
    if (suffix_length > 0U) {
        size_t offset = combine ? suffix_offset : 0U;
        memcpy(value + used, old_raw + offset, suffix_length);
        used += suffix_length;
    }
    free(old_raw);
    size_t old_offset = editor->selected_offset;
    char old_id[EDITOR_NAME_CAP];
    editor_set_text(old_id, sizeof(old_id), editor->selected_has_id ?
        editor->selected_id : "");
    document_edit_error error = has_transform ?
        document_edit_replace_source_span(editor->edit, transform_attribute.value,
            (base_bytes){value, used}) :
        document_edit_add_attribute(editor->edit, editor->selected,
            (base_bytes){(const uint8_t *)"transform", 9U},
            (base_bytes){value, used});
    free(value);
    if (error != DOCUMENT_EDIT_OK)
        return editor_apply_result(editor, error, old_offset, old_id,
            (document_handle){0});
    document_edit_history_relabel_latest(editor->edit,
        rotate_radians != 0.0 ? DOCUMENT_EDIT_HISTORY_ACTION_ROTATE :
        (scale_x != 1.0 || scale_y != 1.0 ?
            DOCUMENT_EDIT_HISTORY_ACTION_RESIZE :
            DOCUMENT_EDIT_HISTORY_ACTION_MOVE));
    editor_status_ok(editor, "Transform committed · preview updating");
    return editor_apply_result(editor, DOCUMENT_EDIT_OK, old_offset, old_id,
        (document_handle){0});
}
