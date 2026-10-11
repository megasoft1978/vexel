#include "ui_widgets.h"

#include <errno.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static bool finite_in_range(double value, double minimum, double maximum)
{
    return isfinite(value) && isfinite(minimum) && isfinite(maximum) && minimum <= maximum &&
        value >= minimum && value <= maximum;
}

static bool utf8_valid(const char *text, size_t length)
{
    if (length > 0U && text == NULL) return false;
    for (size_t index = 0U; index < length;) {
        uint8_t first = (uint8_t)text[index];
        size_t extra = 0U;
        uint8_t minimum = 0x80U, maximum = 0xbfU;
        if (first <= 0x7fU) { index++; continue; }
        if (first >= 0xc2U && first <= 0xdfU) extra = 1U;
        else if (first == 0xe0U) { extra = 2U; minimum = 0xa0U; }
        else if (first >= 0xe1U && first <= 0xecU) extra = 2U;
        else if (first == 0xedU) { extra = 2U; maximum = 0x9fU; }
        else if (first >= 0xeeU && first <= 0xefU) extra = 2U;
        else if (first == 0xf0U) { extra = 3U; minimum = 0x90U; }
        else if (first >= 0xf1U && first <= 0xf3U) extra = 3U;
        else if (first == 0xf4U) { extra = 3U; maximum = 0x8fU; }
        else return false;
        if (extra > length - index - 1U) return false;
        uint8_t second = (uint8_t)text[index + 1U];
        if (second < minimum || second > maximum) return false;
        for (size_t offset = 2U; offset <= extra; ++offset) {
            uint8_t byte = (uint8_t)text[index + offset];
            if (byte < 0x80U || byte > 0xbfU) return false;
        }
        index += extra + 1U;
    }
    return true;
}

static bool copy_label(char *destination, size_t capacity, const char *source,
                       size_t length, bool require_nonempty)
{
    if (source == NULL || capacity == 0U || length >= capacity ||
        (require_nonempty && length == 0U) || !utf8_valid(source, length)) return false;
    if (length > 0U) memcpy(destination, source, length);
    destination[length] = '\0';
    return true;
}

static bool utf8_boundary(const char *text, size_t length, size_t offset)
{
    return offset <= length &&
        (offset == length || (((uint8_t)text[offset] & 0xc0U) != 0x80U));
}

static size_t previous_codepoint(const char *text, size_t offset)
{
    if (offset == 0U) return 0U;
    offset--;
    while (offset > 0U && (((uint8_t)text[offset] & 0xc0U) == 0x80U)) offset--;
    return offset;
}

static size_t next_codepoint(const char *text, size_t length, size_t offset)
{
    if (offset >= length) return length;
    offset++;
    while (offset < length && (((uint8_t)text[offset] & 0xc0U) == 0x80U)) offset++;
    return offset;
}

static void selection_range(const ui_widget_text_edit *edit, size_t *start,
                            size_t *end)
{
    *start = edit->cursor < edit->anchor ? edit->cursor : edit->anchor;
    *end = edit->cursor > edit->anchor ? edit->cursor : edit->anchor;
}

static bool encode_codepoint(uint32_t codepoint, char encoded[4], size_t *length)
{
    if (codepoint <= 0x7fU) { encoded[0] = (char)codepoint; *length = 1U; }
    else if (codepoint <= 0x7ffU) {
        encoded[0] = (char)(0xc0U | (codepoint >> 6U));
        encoded[1] = (char)(0x80U | (codepoint & 0x3fU)); *length = 2U;
    } else if (codepoint >= 0xd800U && codepoint <= 0xdfffU) return false;
    else if (codepoint <= 0xffffU) {
        encoded[0] = (char)(0xe0U | (codepoint >> 12U));
        encoded[1] = (char)(0x80U | ((codepoint >> 6U) & 0x3fU));
        encoded[2] = (char)(0x80U | (codepoint & 0x3fU)); *length = 3U;
    } else if (codepoint <= 0x10ffffU) {
        encoded[0] = (char)(0xf0U | (codepoint >> 18U));
        encoded[1] = (char)(0x80U | ((codepoint >> 12U) & 0x3fU));
        encoded[2] = (char)(0x80U | ((codepoint >> 6U) & 0x3fU));
        encoded[3] = (char)(0x80U | (codepoint & 0x3fU)); *length = 4U;
    } else return false;
    return codepoint >= 0x20U && codepoint != 0x7fU;
}

ui_control_state ui_widget_pointer_visual_state(const ui_widget_pointer_state *state)
{
    if (state == NULL || !state->enabled) return UI_CONTROL_DISABLED;
    if (state->pressed) return UI_CONTROL_PRESSED;
    if (state->focused) return UI_CONTROL_FOCUS;
    if (state->selected) return UI_CONTROL_SELECTED;
    if (state->hovered) return UI_CONTROL_HOVER;
    return UI_CONTROL_REST;
}

bool ui_widget_pointer_transition(ui_widget_pointer_state *state,
                                  ui_widget_pointer_event event)
{
    if (state == NULL || event < UI_WIDGET_POINTER_ENTER ||
        event > UI_WIDGET_POINTER_CANCEL) return false;
    if (!state->enabled) {
        state->hovered = false;
        state->pressed = false;
        return false;
    }
    switch (event) {
        case UI_WIDGET_POINTER_ENTER: state->hovered = true; break;
        case UI_WIDGET_POINTER_LEAVE:
            state->hovered = false;
            if (!state->pressed) state->focused = false;
            break;
        case UI_WIDGET_POINTER_MOVE: break;
        case UI_WIDGET_POINTER_DOWN:
            if (!state->hovered) return false;
            state->pressed = true;
            state->focused = true;
            break;
        case UI_WIDGET_POINTER_UP_INSIDE: {
            bool activated = state->pressed && state->hovered;
            state->pressed = false;
            return activated;
        }
        case UI_WIDGET_POINTER_UP_OUTSIDE:
        case UI_WIDGET_POINTER_CANCEL: state->pressed = false; break;
    }
    return false;
}

bool ui_widget_text_edit_init(ui_widget_text_edit *edit, const char *semantic_label,
    size_t label_length, const char *initial_value, size_t value_length)
{
    if (edit == NULL || initial_value == NULL || value_length > UI_WIDGET_EDIT_MAX_BYTES ||
        !utf8_valid(initial_value, value_length)) return false;
    ui_widget_text_edit fresh = {0};
    if (!copy_label(fresh.semantic_label, sizeof(fresh.semantic_label), semantic_label,
            label_length, true)) return false;
    memcpy(fresh.value, initial_value, value_length);
    memcpy(fresh.original, initial_value, value_length);
    fresh.length = value_length;
    fresh.original_length = value_length;
    fresh.cursor = value_length;
    fresh.anchor = value_length;
    fresh.semantic_label_length = label_length;
    *edit = fresh;
    return true;
}

bool ui_widget_text_edit_begin(ui_widget_text_edit *edit)
{
    if (edit == NULL || edit->length > UI_WIDGET_EDIT_MAX_BYTES) return false;
    memcpy(edit->original, edit->value, edit->length);
    edit->original[edit->length] = '\0';
    edit->original_length = edit->length;
    edit->editing = true;
    edit->invalid = false;
    return true;
}

bool ui_widget_text_edit_commit(ui_widget_text_edit *edit)
{
    if (edit == NULL || !edit->editing || edit->invalid) return false;
    memcpy(edit->original, edit->value, edit->length + 1U);
    edit->original_length = edit->length;
    edit->editing = false;
    return true;
}

bool ui_widget_text_edit_cancel(ui_widget_text_edit *edit)
{
    if (edit == NULL || !edit->editing) return false;
    memcpy(edit->value, edit->original, edit->original_length);
    edit->value[edit->original_length] = '\0';
    edit->length = edit->original_length;
    edit->cursor = edit->length;
    edit->anchor = edit->cursor;
    edit->editing = false;
    edit->invalid = false;
    return true;
}

static bool edit_insert(ui_widget_text_edit *edit, const char *bytes, size_t length)
{
    size_t start = 0U, end = 0U;
    selection_range(edit, &start, &end);
    size_t base = edit->length - (end - start);
    if (length > UI_WIDGET_EDIT_MAX_BYTES - base) return false;
    memmove(edit->value + start + length, edit->value + end, edit->length - end);
    memcpy(edit->value + start, bytes, length);
    edit->length = base + length;
    edit->value[edit->length] = '\0';
    edit->cursor = start + length;
    edit->anchor = edit->cursor;
    edit->invalid = false;
    return true;
}

static void edit_delete_selection(ui_widget_text_edit *edit, size_t start, size_t end)
{
    memmove(edit->value + start, edit->value + end, edit->length - end);
    edit->length -= end - start;
    edit->value[edit->length] = '\0';
    edit->cursor = start;
    edit->anchor = start;
}

ui_widget_input_result ui_widget_text_edit_key(ui_widget_text_edit *edit,
    uint32_t key, uint32_t modifiers)
{
    if (edit == NULL) return UI_WIDGET_INPUT_INVALID;
    if (key == UI_KEY_ESCAPE) {
        return ui_widget_text_edit_cancel(edit) ? UI_WIDGET_INPUT_CANCELLED :
            UI_WIDGET_INPUT_UNHANDLED;
    }
    if (key == UI_KEY_RETURN) {
        if (edit->invalid) return UI_WIDGET_INPUT_INVALID;
        return ui_widget_text_edit_commit(edit) ? UI_WIDGET_INPUT_COMMITTED :
            UI_WIDGET_INPUT_UNHANDLED;
    }
    if ((modifiers & UI_MOD_COMMAND) != 0U) return UI_WIDGET_INPUT_UNHANDLED;
    if (!edit->editing && !ui_widget_text_edit_begin(edit)) return UI_WIDGET_INPUT_INVALID;
    bool extend = (modifiers & UI_MOD_SHIFT) != 0U;
    if (key == UI_KEY_LEFT || key == UI_KEY_RIGHT || key == UI_KEY_HOME || key == UI_KEY_END) {
        size_t destination = edit->cursor;
        if (key == UI_KEY_LEFT) destination = previous_codepoint(edit->value, destination);
        else if (key == UI_KEY_RIGHT) destination = next_codepoint(edit->value, edit->length, destination);
        else if (key == UI_KEY_HOME) destination = 0U;
        else destination = edit->length;
        edit->cursor = destination;
        if (!extend) edit->anchor = destination;
        return UI_WIDGET_INPUT_CHANGED;
    }
    if (key == UI_KEY_BACKSPACE || key == UI_KEY_DELETE) {
        size_t start = 0U, end = 0U;
        selection_range(edit, &start, &end);
        if (start == end) {
            if (key == UI_KEY_BACKSPACE) start = previous_codepoint(edit->value, start);
            else end = next_codepoint(edit->value, edit->length, end);
        }
        if (start == end) return UI_WIDGET_INPUT_CHANGED;
        edit_delete_selection(edit, start, end);
        return UI_WIDGET_INPUT_CHANGED;
    }
    if (key < 0x20U || key >= 0x110000U) return UI_WIDGET_INPUT_UNHANDLED;
    char encoded[4];
    size_t encoded_length = 0U;
    if (!encode_codepoint(key, encoded, &encoded_length) ||
        !edit_insert(edit, encoded, encoded_length)) {
        edit->invalid = true;
        return UI_WIDGET_INPUT_INVALID;
    }
    return UI_WIDGET_INPUT_CHANGED;
}

bool ui_widget_text_edit_pointer(ui_widget_text_edit *edit, size_t byte_offset,
                                 bool extend_selection)
{
    if (edit == NULL || byte_offset > edit->length ||
        !utf8_boundary(edit->value, edit->length, byte_offset)) return false;
    if (!edit->editing && !ui_widget_text_edit_begin(edit)) return false;
    edit->cursor = byte_offset;
    if (!extend_selection) edit->anchor = byte_offset;
    return true;
}

bool ui_widget_number_parse(const char *text, size_t length, double *out_value)
{
    if (text == NULL || out_value == NULL || length == 0U || length > UI_WIDGET_EDIT_MAX_BYTES)
        return false;
    size_t start = 0U, end_offset = length;
    while (start < end_offset && (text[start] == ' ' || text[start] == '\t')) start++;
    while (end_offset > start && (text[end_offset - 1U] == ' ' || text[end_offset - 1U] == '\t')) end_offset--;
    if (start == end_offset) return false;
    size_t offset = start;
    if (text[offset] == '+' || text[offset] == '-') offset++;
    size_t integer_digits = 0U;
    while (offset < end_offset && text[offset] >= '0' && text[offset] <= '9') {
        integer_digits++; offset++;
    }
    size_t fraction_digits = 0U;
    if (offset < end_offset && text[offset] == '.') {
        offset++;
        while (offset < end_offset && text[offset] >= '0' && text[offset] <= '9') {
            fraction_digits++; offset++;
        }
    }
    if (integer_digits + fraction_digits == 0U) return false;
    if (offset < end_offset && (text[offset] == 'e' || text[offset] == 'E')) {
        offset++;
        if (offset < end_offset && (text[offset] == '+' || text[offset] == '-')) offset++;
        size_t exponent_digits = 0U;
        while (offset < end_offset && text[offset] >= '0' && text[offset] <= '9') {
            exponent_digits++; offset++;
        }
        if (exponent_digits == 0U) return false;
    }
    if (offset != end_offset || end_offset - start >= UI_WIDGET_EDIT_MAX_BYTES + 1U) return false;
    char bounded[UI_WIDGET_EDIT_MAX_BYTES + 1U];
    size_t bounded_length = end_offset - start;
    memcpy(bounded, text + start, bounded_length);
    bounded[bounded_length] = '\0';
    char *parse_end = NULL;
    errno = 0;
    double parsed = strtod(bounded, &parse_end);
    if (errno == ERANGE || parse_end != bounded + bounded_length || !isfinite(parsed)) return false;
    *out_value = parsed;
    return true;
}

bool ui_widget_number_format(double value, char *buffer, size_t capacity,
                             size_t *out_length)
{
    if (buffer == NULL || capacity == 0U || !isfinite(value)) return false;
    int written = snprintf(buffer, capacity, "%.17g", value);
    if (written < 0 || (size_t)written >= capacity) return false;
    if (out_length != NULL) *out_length = (size_t)written;
    return true;
}

bool ui_widget_number_field_init(ui_widget_number_field *field,
    const char *semantic_label, size_t label_length, double value,
    double minimum, double maximum, double step,
    const char *unit, size_t unit_length)
{
    if (field == NULL || !finite_in_range(value, minimum, maximum) ||
        !isfinite(step) || step <= 0.0 || unit_length > UI_WIDGET_UNIT_MAX_BYTES ||
        (unit_length > 0U && (unit == NULL || !utf8_valid(unit, unit_length)))) return false;
    ui_widget_number_field fresh = {0};
    char formatted[UI_WIDGET_EDIT_MAX_BYTES + 1U];
    size_t formatted_length = 0U;
    if (!ui_widget_number_format(value, formatted, sizeof(formatted), &formatted_length) ||
        !ui_widget_text_edit_init(&fresh.edit, semantic_label, label_length,
            formatted, formatted_length)) return false;
    if (unit_length > 0U) memcpy(fresh.unit, unit, unit_length);
    fresh.unit[unit_length] = '\0';
    fresh.unit_length = unit_length;
    fresh.value = value; fresh.original = value;
    fresh.minimum = minimum; fresh.maximum = maximum; fresh.step = step;
    *field = fresh;
    return true;
}

bool ui_widget_number_field_set_draft(ui_widget_number_field *field,
                                     const char *text, size_t length)
{
    if (field == NULL || text == NULL || length > UI_WIDGET_EDIT_MAX_BYTES ||
        !utf8_valid(text, length)) return false;
    if (!field->edit.editing && !ui_widget_text_edit_begin(&field->edit)) return false;
    memcpy(field->edit.value, text, length);
    field->edit.value[length] = '\0';
    field->edit.length = length;
    field->edit.cursor = length;
    field->edit.anchor = length;
    double value = 0.0;
    field->edit.invalid = !ui_widget_number_parse(text, length, &value) ||
        value < field->minimum || value > field->maximum;
    if (!field->edit.invalid) field->value = value;
    return true;
}

static bool number_set_value(ui_widget_number_field *field, double value)
{
    if (!isfinite(value)) return false;
    if (value < field->minimum) value = field->minimum;
    if (value > field->maximum) value = field->maximum;
    char formatted[UI_WIDGET_EDIT_MAX_BYTES + 1U];
    size_t length = 0U;
    if (!ui_widget_number_format(value, formatted, sizeof(formatted), &length)) return false;
    memcpy(field->edit.value, formatted, length + 1U);
    field->edit.length = length; field->edit.cursor = length; field->edit.anchor = length;
    field->edit.invalid = false; field->value = value;
    return true;
}

ui_widget_input_result ui_widget_number_field_key(ui_widget_number_field *field,
    uint32_t key, uint32_t modifiers)
{
    if (field == NULL) return UI_WIDGET_INPUT_INVALID;
    if (key == UI_KEY_UP || key == UI_KEY_DOWN) {
        double current = 0.0;
        if (!ui_widget_number_parse(field->edit.value, field->edit.length, &current) ||
            current < field->minimum || current > field->maximum) {
            field->edit.invalid = true;
            return UI_WIDGET_INPUT_INVALID;
        }
        double factor = (modifiers & UI_MOD_SHIFT) != 0U ? 10.0 :
            (modifiers & UI_MOD_OPTION) != 0U ? 0.1 : 1.0;
        double direction = key == UI_KEY_UP ? 1.0 : -1.0;
        if ((!field->edit.editing && !ui_widget_text_edit_begin(&field->edit)) ||
            !number_set_value(field, current + direction * field->step * factor))
            return UI_WIDGET_INPUT_INVALID;
        return UI_WIDGET_INPUT_CHANGED;
    }
    if (key == UI_KEY_RETURN) {
        double value = 0.0;
        if (!ui_widget_number_parse(field->edit.value, field->edit.length, &value) ||
            value < field->minimum || value > field->maximum) {
            field->edit.invalid = true;
            return UI_WIDGET_INPUT_INVALID;
        }
        field->value = value;
        field->edit.invalid = false;
        bool committed = ui_widget_text_edit_commit(&field->edit);
        if (committed) field->original = value;
        return committed ? UI_WIDGET_INPUT_COMMITTED : UI_WIDGET_INPUT_UNHANDLED;
    }
    if (key == UI_KEY_ESCAPE) {
        if (!ui_widget_text_edit_cancel(&field->edit)) return UI_WIDGET_INPUT_UNHANDLED;
        field->scrubbing = false;
        if (!ui_widget_number_parse(field->edit.value, field->edit.length, &field->value))
            return UI_WIDGET_INPUT_INVALID;
        return UI_WIDGET_INPUT_CANCELLED;
    }
    ui_widget_input_result result = ui_widget_text_edit_key(&field->edit, key, modifiers);
    if (result == UI_WIDGET_INPUT_CHANGED) {
        double value = 0.0;
        field->edit.invalid = !ui_widget_number_parse(field->edit.value,
            field->edit.length, &value) || value < field->minimum || value > field->maximum;
        if (!field->edit.invalid) field->value = value;
    }
    return result;
}

ui_widget_input_result ui_widget_number_field_pointer(ui_widget_number_field *field,
    ui_widget_pointer_event event, int32_t x, uint32_t modifiers)
{
    if (field == NULL || event > UI_WIDGET_POINTER_CANCEL) return UI_WIDGET_INPUT_INVALID;
    if (event == UI_WIDGET_POINTER_DOWN) {
        if (!ui_widget_text_edit_begin(&field->edit)) return UI_WIDGET_INPUT_INVALID;
        field->scrub_origin_x = x;
        field->scrub_origin_value = field->value;
        field->scrubbing = true;
        return UI_WIDGET_INPUT_CHANGED;
    }
    if (event == UI_WIDGET_POINTER_ENTER || event == UI_WIDGET_POINTER_LEAVE)
        return UI_WIDGET_INPUT_UNHANDLED;
    if (event == UI_WIDGET_POINTER_MOVE && !field->scrubbing)
        return UI_WIDGET_INPUT_UNHANDLED;
    if (!field->scrubbing) return UI_WIDGET_INPUT_UNHANDLED;
    if (event == UI_WIDGET_POINTER_CANCEL || event == UI_WIDGET_POINTER_UP_OUTSIDE) {
        field->scrubbing = false;
        if (!ui_widget_text_edit_cancel(&field->edit)) return UI_WIDGET_INPUT_INVALID;
        field->value = field->scrub_origin_value;
        return UI_WIDGET_INPUT_CANCELLED;
    }
    if (event == UI_WIDGET_POINTER_UP_INSIDE) {
        field->scrubbing = false;
        field->original = field->value;
        return ui_widget_text_edit_commit(&field->edit) ? UI_WIDGET_INPUT_COMMITTED :
            UI_WIDGET_INPUT_UNHANDLED;
    }
    double multiplier = (modifiers & UI_MOD_SHIFT) != 0U ? 10.0 :
        (modifiers & UI_MOD_OPTION) != 0U ? 0.1 : 1.0;
    double delta = ((double)x - (double)field->scrub_origin_x) * 0.25 * multiplier;
    return number_set_value(field, field->scrub_origin_value + delta) ?
        UI_WIDGET_INPUT_CHANGED : UI_WIDGET_INPUT_INVALID;
}

static int hex_digit(char character)
{
    if (character >= '0' && character <= '9') return character - '0';
    if (character >= 'a' && character <= 'f') return character - 'a' + 10;
    if (character >= 'A' && character <= 'F') return character - 'A' + 10;
    return -1;
}

bool ui_widget_hex_parse(const char *text, size_t length, uint32_t *out_rgba)
{
    if (text == NULL || out_rgba == NULL) return false;
    size_t start = length > 0U && text[0] == '#' ? 1U : 0U;
    size_t digits = length - start;
    if (digits != 3U && digits != 4U && digits != 6U && digits != 8U) return false;
    uint8_t channels[4] = {0U, 0U, 0U, 255U};
    if (digits <= 4U) {
        for (size_t channel = 0U; channel < digits; ++channel) {
            int value = hex_digit(text[start + channel]);
            if (value < 0) return false;
            channels[channel] = (uint8_t)(value * 17);
        }
    } else {
        size_t count = digits / 2U;
        for (size_t channel = 0U; channel < count; ++channel) {
            int high = hex_digit(text[start + channel * 2U]);
            int low = hex_digit(text[start + channel * 2U + 1U]);
            if (high < 0 || low < 0) return false;
            channels[channel] = (uint8_t)((high << 4) | low);
        }
    }
    *out_rgba = ((uint32_t)channels[0] << 24U) | ((uint32_t)channels[1] << 16U) |
        ((uint32_t)channels[2] << 8U) | channels[3];
    return true;
}

bool ui_widget_hex_format(uint32_t rgba, char *buffer, size_t capacity,
                          size_t *out_length)
{
    if (buffer == NULL) return false;
    unsigned red = (rgba >> 24U) & 255U, green = (rgba >> 16U) & 255U;
    unsigned blue = (rgba >> 8U) & 255U, alpha = rgba & 255U;
    int written = alpha == 255U ? snprintf(buffer, capacity, "#%02X%02X%02X", red, green, blue) :
        snprintf(buffer, capacity, "#%02X%02X%02X%02X", red, green, blue, alpha);
    if (written < 0 || (size_t)written >= capacity) return false;
    if (out_length != NULL) *out_length = (size_t)written;
    return true;
}

bool ui_widget_color_field_init(ui_widget_color_field *field,
    const char *semantic_label, size_t label_length, uint32_t rgba)
{
    if (field == NULL) return false;
    char formatted[10];
    size_t length = 0U;
    ui_widget_color_field fresh = {0};
    if (!ui_widget_hex_format(rgba, formatted, sizeof(formatted), &length) ||
        !ui_widget_text_edit_init(&fresh.edit, semantic_label, label_length,
            formatted, length)) return false;
    fresh.rgba = rgba; fresh.original_rgba = rgba;
    *field = fresh;
    return true;
}

bool ui_widget_color_field_set_draft(ui_widget_color_field *field,
                                     const char *text, size_t length)
{
    if (field == NULL || text == NULL || length > UI_WIDGET_EDIT_MAX_BYTES ||
        !utf8_valid(text, length)) return false;
    if (!field->edit.editing && !ui_widget_text_edit_begin(&field->edit)) return false;
    memcpy(field->edit.value, text, length);
    field->edit.value[length] = '\0'; field->edit.length = length;
    field->edit.cursor = length; field->edit.anchor = length;
    uint32_t parsed = 0U;
    field->edit.invalid = !ui_widget_hex_parse(text, length, &parsed);
    if (!field->edit.invalid) field->rgba = parsed;
    return true;
}

ui_widget_input_result ui_widget_color_field_key(ui_widget_color_field *field,
    uint32_t key, uint32_t modifiers)
{
    if (field == NULL) return UI_WIDGET_INPUT_INVALID;
    if (key == UI_KEY_RETURN) {
        uint32_t parsed = 0U;
        if (!ui_widget_hex_parse(field->edit.value, field->edit.length, &parsed)) {
            field->edit.invalid = true;
            return UI_WIDGET_INPUT_INVALID;
        }
        field->rgba = parsed; field->edit.invalid = false;
        bool committed = ui_widget_text_edit_commit(&field->edit);
        if (committed) field->original_rgba = parsed;
        return committed ? UI_WIDGET_INPUT_COMMITTED : UI_WIDGET_INPUT_UNHANDLED;
    }
    if (key == UI_KEY_ESCAPE) {
        if (!ui_widget_text_edit_cancel(&field->edit)) return UI_WIDGET_INPUT_UNHANDLED;
        field->rgba = field->original_rgba;
        return UI_WIDGET_INPUT_CANCELLED;
    }
    ui_widget_input_result result = ui_widget_text_edit_key(&field->edit, key, modifiers);
    if (result == UI_WIDGET_INPUT_CHANGED) {
        uint32_t parsed = 0U;
        field->edit.invalid = !ui_widget_hex_parse(field->edit.value,
            field->edit.length, &parsed);
        if (!field->edit.invalid) field->rgba = parsed;
    }
    return result;
}

bool ui_widget_slider_init(ui_widget_slider *slider, const char *semantic_label,
    size_t label_length, double value, double minimum, double maximum, double step)
{
    if (slider == NULL || !finite_in_range(value, minimum, maximum) ||
        !isfinite(step) || step <= 0.0) return false;
    ui_widget_slider fresh = {0};
    if (!copy_label(fresh.semantic_label, sizeof(fresh.semantic_label), semantic_label,
            label_length, true)) return false;
    fresh.semantic_label_length = label_length; fresh.value = value;
    fresh.minimum = minimum; fresh.maximum = maximum; fresh.step = step;
    *slider = fresh;
    return true;
}

ui_widget_input_result ui_widget_slider_key(ui_widget_slider *slider,
    uint32_t key, uint32_t modifiers)
{
    if (slider == NULL) return UI_WIDGET_INPUT_INVALID;
    if (key != UI_KEY_UP && key != UI_KEY_DOWN && key != UI_KEY_LEFT && key != UI_KEY_RIGHT)
        return UI_WIDGET_INPUT_UNHANDLED;
    double direction = key == UI_KEY_UP || key == UI_KEY_RIGHT ? 1.0 : -1.0;
    double factor = (modifiers & UI_MOD_SHIFT) != 0U ? 10.0 :
        (modifiers & UI_MOD_OPTION) != 0U ? 0.1 : 1.0;
    double value = slider->value + direction * slider->step * factor;
    if (value < slider->minimum) value = slider->minimum;
    if (value > slider->maximum) value = slider->maximum;
    slider->value = value;
    return UI_WIDGET_INPUT_CHANGED;
}

static double slider_at(int32_t x, int32_t track_x, uint32_t track_width,
                        double minimum, double maximum, double step)
{
    double position = (double)x - (double)track_x;
    if (position < 0.0) position = 0.0;
    if (position > (double)track_width) position = (double)track_width;
    double ratio = track_width == 0U ? 0.0 : position / (double)track_width;
    double value = minimum + ratio * (maximum - minimum);
    if (step > 0.0) value = minimum + round((value - minimum) / step) * step;
    if (value < minimum) value = minimum;
    if (value > maximum) value = maximum;
    return value;
}

bool ui_widget_slider_pointer(ui_widget_slider *slider, ui_widget_pointer_event event,
    int32_t x, int32_t track_x, uint32_t track_width)
{
    if (slider == NULL || event > UI_WIDGET_POINTER_CANCEL || track_width == 0U) return false;
    if (event == UI_WIDGET_POINTER_DOWN) slider->dragging = true;
    if ((event == UI_WIDGET_POINTER_UP_INSIDE || event == UI_WIDGET_POINTER_UP_OUTSIDE ||
            event == UI_WIDGET_POINTER_CANCEL) && !slider->dragging) return false;
    if (event == UI_WIDGET_POINTER_ENTER || event == UI_WIDGET_POINTER_LEAVE) return true;
    if (event == UI_WIDGET_POINTER_MOVE && !slider->dragging) return false;
    if (event == UI_WIDGET_POINTER_CANCEL) { slider->dragging = false; return true; }
    slider->value = slider_at(x, track_x, track_width, slider->minimum,
        slider->maximum, slider->step);
    if (event == UI_WIDGET_POINTER_UP_INSIDE || event == UI_WIDGET_POINTER_UP_OUTSIDE)
        slider->dragging = false;
    return true;
}

bool ui_widget_dropdown_init(ui_widget_dropdown *dropdown,
    const char *semantic_label, size_t label_length,
    const ui_widget_option *options, size_t option_count, size_t selected_index)
{
    if (dropdown == NULL || options == NULL || option_count == 0U ||
        option_count > UI_WIDGET_MAX_OPTIONS || selected_index >= option_count) return false;
    ui_widget_dropdown fresh = {0};
    if (!copy_label(fresh.semantic_label, sizeof(fresh.semantic_label), semantic_label,
            label_length, true)) return false;
    for (size_t index = 0U; index < option_count; ++index) {
        if (!copy_label(fresh.options[index], sizeof(fresh.options[index]),
                options[index].text, options[index].length, true)) return false;
        fresh.option_lengths[index] = (uint8_t)options[index].length;
    }
    fresh.semantic_label_length = label_length;
    fresh.option_count = option_count; fresh.selected_index = selected_index;
    fresh.active_index = selected_index;
    *dropdown = fresh;
    return true;
}

ui_widget_input_result ui_widget_dropdown_key(ui_widget_dropdown *dropdown,
    uint32_t key, uint32_t modifiers)
{
    (void)modifiers;
    if (dropdown == NULL || dropdown->option_count == 0U) return UI_WIDGET_INPUT_INVALID;
    if (!dropdown->expanded) {
        if (key != UI_KEY_RETURN && key != UI_KEY_SPACE && key != UI_KEY_DOWN &&
            key != UI_KEY_UP) return UI_WIDGET_INPUT_UNHANDLED;
        dropdown->expanded = true;
        dropdown->active_index = dropdown->selected_index;
        return UI_WIDGET_INPUT_CHANGED;
    }
    if (key == UI_KEY_ESCAPE) {
        dropdown->expanded = false; dropdown->active_index = dropdown->selected_index;
        return UI_WIDGET_INPUT_CANCELLED;
    }
    if (key == UI_KEY_RETURN || key == UI_KEY_SPACE) {
        dropdown->selected_index = dropdown->active_index;
        dropdown->expanded = false;
        return UI_WIDGET_INPUT_COMMITTED;
    }
    if (key == UI_KEY_UP || key == UI_KEY_LEFT) {
        dropdown->active_index = dropdown->active_index == 0U ? dropdown->option_count - 1U :
            dropdown->active_index - 1U;
        return UI_WIDGET_INPUT_CHANGED;
    }
    if (key == UI_KEY_DOWN || key == UI_KEY_RIGHT) {
        dropdown->active_index = (dropdown->active_index + 1U) % dropdown->option_count;
        return UI_WIDGET_INPUT_CHANGED;
    }
    return UI_WIDGET_INPUT_UNHANDLED;
}
