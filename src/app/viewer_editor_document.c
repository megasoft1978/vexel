#define _DARWIN_C_SOURCE 1
#define _USE_EXTENDED_LOCALES_ 1
#include "viewer_editor_internal.h"

#include "viewer_chrome.h"

#include <math.h>
#include <locale.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>

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

document_edit_error viewer_editor_snapshot_create(viewer_editor *editor,
    document_edit_snapshot **out_snapshot)
{
    return editor == NULL || editor->edit == NULL ?
        DOCUMENT_EDIT_INVALID_ARGUMENT :
        document_edit_snapshot_create(editor->edit, out_snapshot);
}

void viewer_editor_mark_unsaved(viewer_editor *editor)
{
    if (editor != NULL) document_edit_mark_unsaved(editor->edit);
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

bool viewer_editor_selected_opacity_target(viewer_editor *editor,
    uint8_t *id_buffer, size_t id_capacity, size_t *out_id_length,
    double *out_opacity)
{
    if (out_id_length != NULL) *out_id_length = 0U;
    if (out_opacity != NULL) *out_opacity = 1.0;
    if (editor == NULL || editor->edit == NULL || id_buffer == NULL ||
        id_capacity == 0U || out_id_length == NULL || out_opacity == NULL ||
        editor->canvas_selection_count > 1U) return false;
    size_t count = document_edit_attribute_count(editor->edit, editor->selected);
    bool has_id = false, has_style_opacity = false;
    double opacity = 1.0;
    size_t id_length = 0U;
    uint8_t value[128U];
    for (size_t index = 0U; index < count; ++index) {
        document_edit_attribute attribute;
        base_bytes name = {0};
        if (!document_edit_attribute_at(editor->edit, editor->selected, index,
                &attribute) || !document_edit_span_bytes(editor->edit,
                attribute.name, &name)) return false;
        bool is_id = name.len == 2U && memcmp(name.ptr, "id", 2U) == 0;
        bool is_opacity = name.len == 7U && memcmp(name.ptr, "opacity", 7U) == 0;
        bool is_style = name.len == 5U && memcmp(name.ptr, "style", 5U) == 0;
        if (is_id) {
            if (has_id || !document_edit_decode_span(editor->edit, attribute.value,
                    id_buffer, id_capacity, &id_length) || id_length == 0U ||
                id_length > 256U) return false;
            has_id = true;
        } else if (is_style) {
            uint8_t style[4096U];
            size_t style_length = 0U;
            if (!document_edit_decode_span(editor->edit, attribute.value, style,
                    sizeof(style), &style_length)) return false;
            for (size_t cursor = 0U; cursor + 7U <= style_length; ++cursor)
                if ((cursor == 0U || style[cursor - 1U] == ';' ||
                        style[cursor - 1U] == ' ' || style[cursor - 1U] == '\t') &&
                    memcmp(style + cursor, "opacity", 7U) == 0) {
                    size_t after = cursor + 7U;
                    while (after < style_length && (style[after] == ' ' ||
                            style[after] == '\t')) after++;
                    if (after < style_length && style[after] == ':')
                        has_style_opacity = true;
                }
        } else if (is_opacity) {
            size_t length = 0U;
            if (!document_edit_decode_span(editor->edit, attribute.value, value,
                    sizeof(value) - 1U, &length) || length == 0U) return false;
            value[length] = 0U;
            char *end = NULL;
            errno = 0;
            opacity = strtod((const char *)value, &end);
            if (errno == 0 && end == (char *)value + length - 1U &&
                value[length - 1U] == '%') opacity /= 100.0;
            if (errno != 0 || (end != (char *)value + length &&
                    !(end == (char *)value + length - 1U &&
                      value[length - 1U] == '%')) ||
                !isfinite(opacity) || opacity < 0.0 || opacity > 1.0) return false;
        }
    }
    if (!has_id || has_style_opacity || id_length > id_capacity) return false;
    *out_id_length = id_length;
    *out_opacity = opacity;
    return true;
}

static locale_t transform_c_locale;
static pthread_once_t transform_locale_once = PTHREAD_ONCE_INIT;

static void initialize_transform_locale(void)
{
    transform_c_locale = newlocale(LC_NUMERIC_MASK, "C", (locale_t)0);
}

static bool transform_space(uint8_t value)
{
    return value == ' ' || value == '\t' || value == '\r' || value == '\n';
}

static bool svg_decimal_syntax(const char *text, size_t length)
{
    size_t cursor = 0U;
    if (cursor < length && (text[cursor] == '+' || text[cursor] == '-')) cursor++;
    size_t digits = 0U;
    while (cursor < length && text[cursor] >= '0' && text[cursor] <= '9') {
        cursor++; digits++;
    }
    if (cursor < length && text[cursor] == '.') {
        cursor++;
        while (cursor < length && text[cursor] >= '0' && text[cursor] <= '9') {
            cursor++; digits++;
        }
    }
    if (digits == 0U) return false;
    if (cursor < length && (text[cursor] == 'e' || text[cursor] == 'E')) {
        cursor++;
        if (cursor < length && (text[cursor] == '+' || text[cursor] == '-')) cursor++;
        size_t exponent_digits = 0U;
        while (cursor < length && text[cursor] >= '0' && text[cursor] <= '9') {
            cursor++; exponent_digits++;
        }
        if (exponent_digits == 0U) return false;
    }
    return cursor == length;
}

static bool transform_word_contains(base_bytes style)
{
    static const char word[] = "transform";
    if (!base_bytes_valid(style) || style.len < sizeof(word) - 1U) return false;
    for (size_t offset = 0U; offset <= style.len - (sizeof(word) - 1U); ++offset) {
        size_t index = 0U;
        while (index < sizeof(word) - 1U) {
            uint8_t byte = style.ptr[offset + index];
            if (byte >= 'A' && byte <= 'Z') byte = (uint8_t)(byte - 'A' + 'a');
            if (byte != (uint8_t)word[index]) break;
            index++;
        }
        if (index == sizeof(word) - 1U) return true;
    }
    return false;
}

static bool parse_transform_matrix(base_bytes text, double values[6])
{
    if (!base_bytes_valid(text) || values == NULL || text.len < 8U ||
        memcmp(text.ptr, "matrix(", 7U) != 0 || text.ptr[text.len - 1U] != ')')
        return false;
    size_t cursor = 7U, end = text.len - 1U;
    for (size_t index = 0U; index < 6U; ++index) {
        bool spaced = false;
        while (cursor < end && transform_space(text.ptr[cursor])) {
            spaced = true;
            cursor++;
        }
        if (index > 0U) {
            if (cursor < end && text.ptr[cursor] == ',') {
                cursor++;
                while (cursor < end && transform_space(text.ptr[cursor])) cursor++;
            } else if (!spaced) return false;
        } else if (cursor < end && text.ptr[cursor] == ',') return false;
        if (cursor >= end) return false;
        size_t start = cursor;
        while (cursor < end && !transform_space(text.ptr[cursor]) &&
            text.ptr[cursor] != ',') cursor++;
        size_t length = cursor - start;
        if (length == 0U || length >= 64U) return false;
        char number[64U];
        memcpy(number, text.ptr + start, length);
        number[length] = '\0';
        if (!svg_decimal_syntax(number, length)) return false;
        char *parsed_end = NULL;
        errno = 0;
        (void)pthread_once(&transform_locale_once, initialize_transform_locale);
        if (transform_c_locale == (locale_t)0) return false;
        double value = strtod_l(number, &parsed_end, transform_c_locale);
        if (errno != 0 || parsed_end != number + length || !isfinite(value) ||
            fabs(value) > 1.0e9) return false;
        values[index] = value;
    }
    while (cursor < end && transform_space(text.ptr[cursor])) cursor++;
    return cursor == end;
}

bool viewer_editor_selected_transform_target(viewer_editor *editor,
    uint8_t *id_buffer, size_t id_capacity, size_t *out_id_length,
    double out_matrix[6])
{
    if (out_id_length != NULL) *out_id_length = 0U;
    if (out_matrix != NULL) memset(out_matrix, 0, 6U * sizeof(double));
    if (editor == NULL || editor->edit == NULL || id_buffer == NULL ||
        id_capacity == 0U || out_id_length == NULL || out_matrix == NULL ||
        editor->canvas_selection_count > 1U) return false;
    out_matrix[0] = 1.0;
    out_matrix[3] = 1.0;
    bool has_id = false, has_transform = false;
    size_t id_length = 0U;
    uint8_t value[256U];
    size_t count = document_edit_attribute_count(editor->edit, editor->selected);
    for (size_t index = 0U; index < count; ++index) {
        document_edit_attribute attribute;
        base_bytes name = {0};
        if (!document_edit_attribute_at(editor->edit, editor->selected, index,
                &attribute) || !document_edit_span_bytes(editor->edit,
                attribute.name, &name)) return false;
        if (name.len == 2U && memcmp(name.ptr, "id", 2U) == 0) {
            if (has_id || !document_edit_decode_span(editor->edit, attribute.value,
                    id_buffer, id_capacity, &id_length) || id_length == 0U ||
                id_length > 256U) return false;
            has_id = true;
        } else if (name.len == 9U && memcmp(name.ptr, "transform", 9U) == 0) {
            size_t length = 0U;
            if (has_transform || !document_edit_decode_span(editor->edit,
                    attribute.value, value, sizeof(value), &length) ||
                !parse_transform_matrix((base_bytes){value, length}, out_matrix)) return false;
            has_transform = true;
        } else if (name.len == 5U && memcmp(name.ptr, "style", 5U) == 0) {
            size_t length = 0U;
            if (!document_edit_decode_span(editor->edit, attribute.value,
                    value, sizeof(value), &length) ||
                transform_word_contains((base_bytes){value, length})) return false;
        }
    }
    if (!has_id || id_length > id_capacity) return false;
    size_t id_matches = 0U;
    document_handle first = {0};
    const document_model *model = document_edit_model(editor->edit);
    if (model == NULL) return false;
    size_t node_count = document_model_node_count(model);
    for (size_t slot = 0U; slot < node_count; ++slot) {
        document_handle node = {0};
        if (!document_model_node_at(model, slot, &node)) continue;
        size_t attributes = document_edit_attribute_count(editor->edit, node);
        for (size_t attribute_index = 0U; attribute_index < attributes;
             ++attribute_index) {
            document_edit_attribute attribute;
            base_bytes name = {0};
            if (!document_edit_attribute_at(editor->edit, node, attribute_index,
                    &attribute) || !document_edit_span_bytes(editor->edit,
                    attribute.name, &name)) return false;
            if (name.len != 2U || memcmp(name.ptr, "id", 2U) != 0) continue;
            uint8_t decoded[257U]; size_t decoded_length = 0U;
            if (!document_edit_decode_span(editor->edit, attribute.value,
                    decoded, sizeof(decoded), &decoded_length)) return false;
            if (decoded_length != id_length ||
                memcmp(decoded, id_buffer, id_length) != 0) continue;
            id_matches++;
            first = node;
            if (id_matches > 1U) return false;
        }
    }
    if (id_matches != 1U || first.document_id != editor->selected.document_id ||
        first.slot != editor->selected.slot ||
        first.generation != editor->selected.generation) return false;
    *out_id_length = id_length;
    return true;
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

bool viewer_editor_can_undo(const viewer_editor *editor)
{
    if (editor != NULL && editor->history_callback != NULL &&
        editor->history_callback(editor->history_context, false, false)) return true;
    return editor != NULL && editor->edit != NULL &&
        document_edit_history_cursor(editor->edit) > 0U;
}

bool viewer_editor_can_redo(const viewer_editor *editor)
{
    if (editor != NULL && editor->history_callback != NULL &&
        editor->history_callback(editor->history_context, true, false)) return true;
    return editor != NULL && editor->edit != NULL &&
        document_edit_history_cursor(editor->edit) <
            document_edit_history_count(editor->edit);
}

void viewer_editor_set_history_callback(viewer_editor *editor,
    viewer_editor_history_callback callback, void *context)
{
    if (editor == NULL) return;
    editor->history_callback = callback;
    editor->history_context = context;
}

bool viewer_editor_has_selection(const viewer_editor *editor)
{
    if (editor == NULL || editor->edit == NULL) return false;
    if (editor->canvas_selection_count > 0U) return true;
    document_handle root = {0};
    return document_edit_root(editor->edit, &root) &&
        !editor_same_handle(editor->selected, root);
}

bool viewer_editor_sidebar_visibility(const viewer_editor *editor,
                                      bool *out_structure, bool *out_inspector)
{
    if (editor == NULL || out_structure == NULL || out_inspector == NULL) return false;
    *out_structure = editor->want_structure;
    *out_inspector = editor->want_inspector;
    return true;
}
