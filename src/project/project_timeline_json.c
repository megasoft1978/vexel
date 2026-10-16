#define _DARWIN_C_SOURCE 1
#define _USE_EXTENDED_LOCALES_ 1
#include "project_timeline_json.h"

#include <math.h>
#include <locale.h>
#include <pthread.h>
#include <stdlib.h>
#include <string.h>

#define TIMELINE_MAX_FIELDS 16U

static locale_t timeline_c_locale;
static pthread_once_t timeline_locale_once = PTHREAD_ONCE_INIT;

static void initialize_timeline_locale(void)
{
    timeline_c_locale = newlocale(LC_NUMERIC_MASK, "C", (locale_t)0);
}

bool is_space(uint8_t byte)
{
    return byte == ' ' || byte == '\t' || byte == '\r' || byte == '\n';
}

void skip_space(json_reader *reader)
{
    if (!json_reader_poll_cancel(reader)) return;
    while (reader->cursor < reader->source.len &&
           is_space(reader->source.ptr[reader->cursor])) {
        reader->cursor++;
        if (!json_reader_poll_cancel(reader)) return;
    }
}

bool json_reader_poll_cancel(json_reader *reader)
{
    if (reader->canceled || reader->cancel_check == NULL || reader->cursor == 0U ||
        (reader->cursor & 4095U) != 0U) return !reader->canceled;
    if (reader->cancel_check(reader->cancel_context)) {
        reader->canceled = true;
        reader->error = reader->cursor;
        reader->error_kind = PROJECT_TIMELINE_INVALID_ARGUMENT;
        return false;
    }
    return true;
}

bool fail(json_reader *reader, project_timeline_error error)
{
    reader->error = reader->cursor;
    reader->error_kind = error;
    return false;
}

bool fail_at(json_reader *reader, project_timeline_error error, size_t offset)
{
    reader->error = offset;
    reader->error_kind = error;
    return false;
}

static bool utf8_sequence(const uint8_t *bytes, size_t length, size_t *cursor)
{
    uint8_t first = bytes[*cursor];
    size_t width = first >= 0xc2U && first <= 0xdfU ? 2U :
        first >= 0xe0U && first <= 0xefU ? 3U :
        first >= 0xf0U && first <= 0xf4U ? 4U : 0U;
    if (width == 0U || width > length - *cursor) return false;
    uint8_t second = bytes[*cursor + 1U];
    if ((second & 0xc0U) != 0x80U ||
        (first == 0xe0U && second < 0xa0U) ||
        (first == 0xedU && second >= 0xa0U) ||
        (first == 0xf0U && second < 0x90U) ||
        (first == 0xf4U && second >= 0x90U)) return false;
    for (size_t i = 2U; i < width; i++)
        if ((bytes[*cursor + i] & 0xc0U) != 0x80U) return false;
    *cursor += width;
    return true;
}

static int hex_value(uint8_t byte)
{
    if (byte >= '0' && byte <= '9') return (int)(byte - '0');
    if (byte >= 'a' && byte <= 'f') return (int)(byte - 'a') + 10;
    if (byte >= 'A' && byte <= 'F') return (int)(byte - 'A') + 10;
    return -1;
}

static bool append_utf8(uint32_t codepoint, base_mut_bytes output, size_t *length)
{
    uint8_t bytes[4];
    size_t count;
    if (codepoint <= 0x7fU) { bytes[0] = (uint8_t)codepoint; count = 1U; }
    else if (codepoint <= 0x7ffU) {
        bytes[0] = (uint8_t)(0xc0U | (codepoint >> 6U));
        bytes[1] = (uint8_t)(0x80U | (codepoint & 0x3fU)); count = 2U;
    } else if (codepoint <= 0xffffU) {
        bytes[0] = (uint8_t)(0xe0U | (codepoint >> 12U));
        bytes[1] = (uint8_t)(0x80U | ((codepoint >> 6U) & 0x3fU));
        bytes[2] = (uint8_t)(0x80U | (codepoint & 0x3fU)); count = 3U;
    } else if (codepoint <= 0x10ffffU) {
        bytes[0] = (uint8_t)(0xf0U | (codepoint >> 18U));
        bytes[1] = (uint8_t)(0x80U | ((codepoint >> 12U) & 0x3fU));
        bytes[2] = (uint8_t)(0x80U | ((codepoint >> 6U) & 0x3fU));
        bytes[3] = (uint8_t)(0x80U | (codepoint & 0x3fU)); count = 4U;
    } else return false;
    if (*length > output.len || count > output.len - *length) return false;
    memcpy(output.ptr + *length, bytes, count);
    *length += count;
    return true;
}

static bool decode_string_content(base_bytes encoded, base_mut_bytes output,
                                  size_t *out_length, bool *out_limit)
{
    if (!base_bytes_valid(encoded) || (output.ptr == NULL && output.len != 0U) || out_length == NULL)
        return false;
    size_t input = 0U, length = 0U;
    if (out_limit != NULL) *out_limit = false;
    while (input < encoded.len) {
        uint8_t byte = encoded.ptr[input++];
        uint32_t codepoint = byte;
        if (byte == '\\') {
            if (input >= encoded.len) return false;
            uint8_t escaped = encoded.ptr[input++];
            if (escaped == '"' || escaped == '\\' || escaped == '/') codepoint = escaped;
            else if (escaped == 'b') codepoint = 8U;
            else if (escaped == 'f') codepoint = 12U;
            else if (escaped == 'n') codepoint = 10U;
            else if (escaped == 'r') codepoint = 13U;
            else if (escaped == 't') codepoint = 9U;
            else if (escaped == 'u') {
                if (encoded.len - input < 4U) return false;
                codepoint = 0U;
                for (size_t i = 0U; i < 4U; i++) {
                    int digit = hex_value(encoded.ptr[input++]);
                    if (digit < 0) return false;
                    codepoint = (codepoint << 4U) | (uint32_t)digit;
                }
                if (codepoint >= 0xd800U && codepoint <= 0xdbffU) {
                    if (encoded.len - input < 6U || encoded.ptr[input++] != '\\' ||
                        encoded.ptr[input++] != 'u') return false;
                    uint32_t low = 0U;
                    for (size_t i = 0U; i < 4U; i++) {
                        int digit = hex_value(encoded.ptr[input++]);
                        if (digit < 0) return false;
                        low = (low << 4U) | (uint32_t)digit;
                    }
                    if (low < 0xdc00U || low > 0xdfffU) return false;
                    codepoint = 0x10000U + ((codepoint - 0xd800U) << 10U) + (low - 0xdc00U);
                } else if (codepoint >= 0xdc00U && codepoint <= 0xdfffU) return false;
            } else return false;
        } else if (byte >= 0x80U) {
            input--;
            size_t start = input;
            if (!utf8_sequence(encoded.ptr, encoded.len, &input)) return false;
            size_t width = input - start;
            if (width > output.len - length) {
                if (out_limit != NULL) *out_limit = true;
                return false;
            }
            memcpy(output.ptr + length, encoded.ptr + start, width);
            length += width;
            continue;
        } else if (byte < 0x20U) return false;
        if (!append_utf8(codepoint, output, &length)) {
            if (out_limit != NULL) *out_limit = true;
            return false;
        }
        if (length > PROJECT_TIMELINE_MAX_STRING_BYTES) {
            if (out_limit != NULL) *out_limit = true;
            return false;
        }
    }
    *out_length = length;
    return true;
}

static bool scan_string(json_reader *reader, base_bytes *out)
{
    skip_space(reader);
    if (reader->cursor >= reader->source.len ||
        reader->source.ptr[reader->cursor] != '"') return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
    size_t start = ++reader->cursor;
    while (reader->cursor < reader->source.len) {
        if (!json_reader_poll_cancel(reader)) return false;
        uint8_t byte = reader->source.ptr[reader->cursor];
        if (byte == '"') {
            size_t end = reader->cursor++;
            base_bytes raw = {reader->source.ptr + start, end - start};
            char decoded[PROJECT_TIMELINE_MAX_STRING_BYTES];
            size_t decoded_length = 0U;
            bool limited = false;
            if (!decode_string_content(raw,
                    (base_mut_bytes){(uint8_t *)decoded, sizeof(decoded)},
                    &decoded_length, &limited))
                return fail(reader, limited ? PROJECT_TIMELINE_LIMIT : PROJECT_TIMELINE_MALFORMED_JSON);
            if (out != NULL) *out = raw;
            return true;
        }
        if (byte < 0x20U) return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
        if (byte == '\\') {
            reader->cursor++;
            if (reader->cursor >= reader->source.len) return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
            uint8_t escaped = reader->source.ptr[reader->cursor++];
            if (escaped == '"' || escaped == '\\' || escaped == '/' || escaped == 'b' ||
                escaped == 'f' || escaped == 'n' || escaped == 'r' || escaped == 't') {
                continue;
            }
            if (escaped != 'u' || reader->source.len - reader->cursor < 4U)
                return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
            unsigned code = 0U;
            for (size_t i = 0U; i < 4U; i++) {
                int digit = hex_value(reader->source.ptr[reader->cursor++]);
                if (digit < 0) return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
                code = (code << 4U) | (unsigned)digit;
            }
            if (code >= 0xd800U && code <= 0xdbffU) {
                if (reader->source.len - reader->cursor < 6U ||
                    reader->source.ptr[reader->cursor] != '\\' ||
                    reader->source.ptr[reader->cursor + 1U] != 'u')
                    return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
                reader->cursor += 2U;
                uint32_t low = 0U;
                for (size_t i = 0U; i < 4U; i++) {
                    int digit = hex_value(reader->source.ptr[reader->cursor++]);
                    if (digit < 0) return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
                    low = (low << 4U) | (uint32_t)digit;
                }
                if (low < 0xdc00U || low > 0xdfffU)
                    return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
            } else if (code >= 0xdc00U && code <= 0xdfffU) {
                return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
            }
            continue;
        }
        if (byte >= 0x80U) {
            if (!utf8_sequence(reader->source.ptr, reader->source.len, &reader->cursor))
                return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
        } else reader->cursor++;
    }
    return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
}

bool scan_number(json_reader *reader)
{
    size_t start = reader->cursor;
    if (reader->source.ptr[reader->cursor] == '-') reader->cursor++;
    if (reader->cursor >= reader->source.len) return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
    if (reader->source.ptr[reader->cursor] == '0') reader->cursor++;
    else {
        if (reader->source.ptr[reader->cursor] < '1' || reader->source.ptr[reader->cursor] > '9')
            return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
        while (reader->cursor < reader->source.len && reader->source.ptr[reader->cursor] >= '0' &&
               reader->source.ptr[reader->cursor] <= '9') reader->cursor++;
    }
    if (reader->cursor < reader->source.len && reader->source.ptr[reader->cursor] == '.') {
        reader->cursor++;
        size_t digits = reader->cursor;
        while (reader->cursor < reader->source.len && reader->source.ptr[reader->cursor] >= '0' &&
               reader->source.ptr[reader->cursor] <= '9') reader->cursor++;
        if (reader->cursor == digits) return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
    }
    if (reader->cursor < reader->source.len &&
        (reader->source.ptr[reader->cursor] == 'e' || reader->source.ptr[reader->cursor] == 'E')) {
        reader->cursor++;
        if (reader->cursor < reader->source.len &&
            (reader->source.ptr[reader->cursor] == '+' || reader->source.ptr[reader->cursor] == '-'))
            reader->cursor++;
        size_t digits = reader->cursor;
        while (reader->cursor < reader->source.len && reader->source.ptr[reader->cursor] >= '0' &&
               reader->source.ptr[reader->cursor] <= '9') reader->cursor++;
        if (reader->cursor == digits) return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
    }
    if (reader->cursor - start > 96U) return fail(reader, PROJECT_TIMELINE_LIMIT);
    char buffer[97];
    size_t length = reader->cursor - start;
    memcpy(buffer, reader->source.ptr + start, length);
    buffer[length] = '\0';
    char *end = NULL;
    (void)pthread_once(&timeline_locale_once, initialize_timeline_locale);
    if (timeline_c_locale == (locale_t)0) return fail(reader, PROJECT_TIMELINE_INVALID_VALUE);
    double value = strtod_l(buffer, &end, timeline_c_locale);
    if (end != buffer + length || !isfinite(value)) return fail(reader, PROJECT_TIMELINE_INVALID_VALUE);
    return true;
}

bool scan_value(json_reader *reader, unsigned depth);

static bool scan_array(json_reader *reader, unsigned depth)
{
    reader->cursor++;
    skip_space(reader);
    if (reader->cursor < reader->source.len && reader->source.ptr[reader->cursor] == ']') {
        reader->cursor++;
        return true;
    }
    for (;;) {
        if (!scan_value(reader, depth + 1U)) return false;
        skip_space(reader);
        if (reader->cursor >= reader->source.len) return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
        uint8_t byte = reader->source.ptr[reader->cursor++];
        if (byte == ']') return true;
        if (byte != ',') return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
        skip_space(reader);
    }
}

static bool scan_object(json_reader *reader, unsigned depth)
{
    reader->cursor++;
    base_bytes keys[TIMELINE_MAX_FIELDS];
    size_t key_count = 0U;
    skip_space(reader);
    if (reader->cursor < reader->source.len && reader->source.ptr[reader->cursor] == '}') {
        reader->cursor++;
        return true;
    }
    for (;;) {
        size_t key_offset = reader->cursor;
        base_bytes key;
        if (!scan_string(reader, &key)) return false;
        for (size_t i = 0U; i < key_count; i++) {
            if (json_strings_equal(keys[i], key)) {
                reader->cursor = key_offset;
                return fail(reader, PROJECT_TIMELINE_DUPLICATE);
            }
        }
        if (key_count >= TIMELINE_MAX_FIELDS) return fail(reader, PROJECT_TIMELINE_LIMIT);
        keys[key_count++] = key;
        skip_space(reader);
        if (reader->cursor >= reader->source.len || reader->source.ptr[reader->cursor++] != ':')
            return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
        if (!scan_value(reader, depth + 1U)) return false;
        skip_space(reader);
        if (reader->cursor >= reader->source.len) return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
        uint8_t byte = reader->source.ptr[reader->cursor++];
        if (byte == '}') return true;
        if (byte != ',') return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
        skip_space(reader);
    }
}

bool scan_value(json_reader *reader, unsigned depth)
{
    if (depth > 32U) return fail(reader, PROJECT_TIMELINE_LIMIT);
    skip_space(reader);
    if (reader->cursor >= reader->source.len) return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
    uint8_t byte = reader->source.ptr[reader->cursor];
    if (byte == '"') return scan_string(reader, NULL);
    if (byte == '{') return scan_object(reader, depth);
    if (byte == '[') return scan_array(reader, depth);
    if (byte == '-' || (byte >= '0' && byte <= '9')) return scan_number(reader);
    static const char *const literals[] = {"true", "false", "null"};
    for (size_t i = 0U; i < sizeof(literals) / sizeof(literals[0]); i++) {
        size_t n = strlen(literals[i]);
        if (reader->source.len - reader->cursor >= n &&
            memcmp(reader->source.ptr + reader->cursor, literals[i], n) == 0) {
            reader->cursor += n;
            return true;
        }
    }
    return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
}

bool bytes_equal(base_bytes a, base_bytes b)
{
    return a.len == b.len && (a.len == 0U || memcmp(a.ptr, b.ptr, a.len) == 0);
}

int bytes_compare(base_bytes a, base_bytes b)
{
    size_t common = a.len < b.len ? a.len : b.len;
    int order = common == 0U ? 0 : memcmp(a.ptr, b.ptr, common);
    if (order != 0) return order;
    return a.len < b.len ? -1 : a.len > b.len ? 1 : 0;
}

bool field_equal(base_bytes key, const char *literal)
{
    size_t expected = strlen(literal);
    return key.len == expected && memcmp(key.ptr, literal, expected) == 0;
}

bool json_string_equal(base_bytes encoded, const char *literal)
{
    char decoded[PROJECT_TIMELINE_MAX_STRING_BYTES];
    size_t length = 0U;
    size_t expected = strlen(literal);
    if (expected > sizeof(decoded) || !project_timeline_string_copy(encoded,
            (base_mut_bytes){(uint8_t *)decoded, sizeof(decoded)}, &length)) return false;
    return length == expected && memcmp(decoded, literal, length) == 0;
}

bool json_strings_equal(base_bytes left, base_bytes right)
{
    char left_decoded[PROJECT_TIMELINE_MAX_STRING_BYTES];
    char right_decoded[PROJECT_TIMELINE_MAX_STRING_BYTES];
    size_t left_length = 0U, right_length = 0U;
    if (!project_timeline_string_copy(left,
            (base_mut_bytes){(uint8_t *)left_decoded, sizeof(left_decoded)}, &left_length) ||
        !project_timeline_string_copy(right,
            (base_mut_bytes){(uint8_t *)right_decoded, sizeof(right_decoded)}, &right_length)) return false;
    return left_length == right_length && memcmp(left_decoded, right_decoded, left_length) == 0;
}

bool parse_object_fields(json_reader *reader, json_span span,
                                object_fields *out)
{
    json_reader child = *reader;
    child.cursor = span.start;
    skip_space(&child);
    if (child.cursor >= child.source.len || child.source.ptr[child.cursor++] != '{') {
        *reader = child;
        return fail_at(reader, PROJECT_TIMELINE_INVALID_SCHEMA, span.start);
    }
    out->count = 0U;
    skip_space(&child);
    if (child.cursor < child.source.len && child.source.ptr[child.cursor] == '}') {
        child.cursor++;
        *reader = child;
        return true;
    }
    for (;;) {
        base_bytes encoded_key;
        if (!scan_string(&child, &encoded_key)) { *reader = child; return false; }
        char decoded_key[128];
        size_t decoded_key_length = 0U;
        if (!project_timeline_string_copy(encoded_key,
                (base_mut_bytes){(uint8_t *)decoded_key, sizeof(decoded_key) - 1U},
                &decoded_key_length)) {
            *reader = child; return fail(reader, PROJECT_TIMELINE_LIMIT);
        }
        decoded_key[decoded_key_length] = '\0';
        base_bytes key = {(const uint8_t *)decoded_key, decoded_key_length};
        for (size_t i = 0U; i < out->count; i++)
            if (bytes_equal(out->fields[i].key, key)) {
                *reader = child; return fail(reader, PROJECT_TIMELINE_DUPLICATE);
            }
        skip_space(&child);
        if (child.cursor >= child.source.len || child.source.ptr[child.cursor++] != ':') {
            *reader = child; return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON);
        }
        skip_space(&child);
        if (out->count >= TIMELINE_MAX_FIELDS) { *reader = child; return fail(reader, PROJECT_TIMELINE_LIMIT); }
        size_t start = child.cursor;
        if (!scan_value(&child, 1U)) { *reader = child; return false; }
        json_field *field = &out->fields[out->count++];
        memcpy(field->key_storage, decoded_key, decoded_key_length + 1U);
        field->key = (base_bytes){(const uint8_t *)field->key_storage, decoded_key_length};
        field->value = (json_span){start, child.cursor};
        skip_space(&child);
        if (child.cursor >= child.source.len) { *reader = child; return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON); }
        uint8_t byte = child.source.ptr[child.cursor++];
        if (byte == '}') { *reader = child; return true; }
        if (byte != ',') { *reader = child; return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON); }
        skip_space(&child);
    }
}

const json_field *get_field(const object_fields *fields, const char *name)
{
    for (size_t i = 0U; i < fields->count; i++)
        if (field_equal(fields->fields[i].key, name)) return &fields->fields[i];
    return NULL;
}

bool only_fields(const object_fields *fields, const char *const *names,
                        size_t name_count)
{
    if (fields->count != name_count) return false;
    for (size_t i = 0U; i < fields->count; i++) {
        bool found = false;
        for (size_t n = 0U; n < name_count; n++)
            if (field_equal(fields->fields[i].key, names[n])) found = true;
        if (!found) return false;
    }
    return true;
}

bool span_string(json_reader *reader, json_span span, base_bytes *out)
{
    json_reader child = *reader;
    child.cursor = span.start;
    if (!scan_string(&child, out)) { *reader = child; return false; }
    if (child.cursor != span.end) { *reader = child; return fail_at(reader, PROJECT_TIMELINE_INVALID_VALUE, span.start); }
    return true;
}

bool span_uint(json_reader *reader, json_span span, uint32_t *out)
{
    json_reader child = *reader;
    child.cursor = span.start;
    skip_space(&child);
    size_t start = child.cursor;
    if (start >= child.source.len || child.source.ptr[start] < '0' || child.source.ptr[start] > '9') {
        *reader = child;
        return fail_at(reader, PROJECT_TIMELINE_INVALID_VALUE, span.start);
    }
    uint64_t value = 0U;
    while (child.cursor < child.source.len && child.source.ptr[child.cursor] >= '0' &&
           child.source.ptr[child.cursor] <= '9') {
        uint64_t digit = (uint64_t)(child.source.ptr[child.cursor++] - '0');
        if (value > ((uint64_t)UINT32_MAX - digit) / 10U) {
            *reader = child;
            return fail_at(reader, PROJECT_TIMELINE_LIMIT, span.start);
        }
        value = value * 10U + digit;
    }
    if (child.cursor != span.end) { *reader = child; return fail_at(reader, PROJECT_TIMELINE_INVALID_VALUE, span.start); }
    *out = (uint32_t)value;
    return true;
}

bool span_bool(json_reader *reader, json_span span, bool *out)
{
    size_t length = span.end - span.start;
    if (length == 4U && memcmp(reader->source.ptr + span.start, "true", 4U) == 0) { *out = true; return true; }
    if (length == 5U && memcmp(reader->source.ptr + span.start, "false", 5U) == 0) { *out = false; return true; }
    return fail_at(reader, PROJECT_TIMELINE_INVALID_VALUE, span.start);
}

bool span_number_value(json_reader *reader, json_span span, double *out)
{
    size_t length = span.end - span.start;
    if (length == 0U || length > 96U) return fail_at(reader, PROJECT_TIMELINE_INVALID_VALUE, span.start);
    char buffer[97];
    memcpy(buffer, reader->source.ptr + span.start, length);
    buffer[length] = '\0';
    (void)pthread_once(&timeline_locale_once, initialize_timeline_locale);
    if (timeline_c_locale == (locale_t)0) return fail_at(reader, PROJECT_TIMELINE_INVALID_VALUE, span.start);
    char *end = NULL;
    double value = strtod_l(buffer, &end, timeline_c_locale);
    if (end != buffer + length || !isfinite(value)) return fail_at(reader, PROJECT_TIMELINE_INVALID_VALUE, span.start);
    *out = value;
    return true;
}

bool span_array(json_reader *reader, json_span span, json_span *items,
                       size_t capacity, size_t *out_count)
{
    json_reader child = *reader;
    child.cursor = span.start;
    skip_space(&child);
    if (child.cursor >= child.source.len || child.source.ptr[child.cursor++] != '[') {
        *reader = child; return fail(reader, PROJECT_TIMELINE_INVALID_SCHEMA);
    }
    size_t count = 0U;
    skip_space(&child);
    if (child.cursor < child.source.len && child.source.ptr[child.cursor] == ']') {
        child.cursor++;
    } else for (;;) {
        if (count >= capacity) { *reader = child; return fail_at(reader, PROJECT_TIMELINE_LIMIT, child.cursor); }
        size_t start = child.cursor;
        if (!scan_value(&child, 1U)) { *reader = child; return false; }
        items[count++] = (json_span){start, child.cursor};
        skip_space(&child);
        if (child.cursor >= child.source.len) { *reader = child; return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON); }
        uint8_t byte = child.source.ptr[child.cursor++];
        if (byte == ']') break;
        if (byte != ',') { *reader = child; return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON); }
        skip_space(&child);
    }
    if (child.cursor != span.end) { *reader = child; return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON); }
    *out_count = count;
    return true;
}

bool array_count(json_reader *reader, json_span span, size_t capacity,
                        size_t *out_count)
{
    json_reader child = *reader;
    child.cursor = span.start;
    skip_space(&child);
    if (child.cursor >= child.source.len || child.source.ptr[child.cursor++] != '[') {
        *reader = child; return fail(reader, PROJECT_TIMELINE_INVALID_SCHEMA);
    }
    size_t count = 0U;
    skip_space(&child);
    if (child.cursor < child.source.len && child.source.ptr[child.cursor] == ']') {
        child.cursor++;
    } else for (;;) {
        if (count >= capacity) { *reader = child; return fail(reader, PROJECT_TIMELINE_LIMIT); }
        if (!scan_value(&child, 1U)) { *reader = child; return false; }
        count++;
        skip_space(&child);
        if (child.cursor >= child.source.len) { *reader = child; return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON); }
        uint8_t byte = child.source.ptr[child.cursor++];
        if (byte == ']') break;
        if (byte != ',') { *reader = child; return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON); }
        skip_space(&child);
    }
    if (child.cursor != span.end) { *reader = child; return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON); }
    *out_count = count;
    return true;
}

bool array_item_at(json_reader *reader, json_span span, size_t wanted,
                          json_span *out_item)
{
    json_reader child = *reader;
    child.cursor = span.start;
    skip_space(&child);
    if (child.cursor >= child.source.len || child.source.ptr[child.cursor++] != '[') return false;
    size_t index = 0U;
    skip_space(&child);
    if (child.cursor < child.source.len && child.source.ptr[child.cursor] == ']') return false;
    for (;;) {
        size_t start = child.cursor;
        if (!scan_value(&child, 1U)) { *reader = child; return false; }
        size_t end = child.cursor;
        if (index == wanted) { *out_item = (json_span){start, end}; return true; }
        index++;
        skip_space(&child);
        if (child.cursor >= child.source.len || child.source.ptr[child.cursor++] != ',') return false;
        skip_space(&child);
    }
}

bool array_item_at_from(json_reader *reader, json_span span, size_t wanted,
                        size_t *in_out_cursor, size_t *in_out_index,
                        json_span *out_item)
{
    if (reader == NULL || in_out_cursor == NULL || in_out_index == NULL ||
        out_item == NULL || wanted < *in_out_index || *in_out_cursor > span.end)
        return false;
    json_reader child = *reader;
    child.cursor = *in_out_cursor;
    size_t index = *in_out_index;
    if (index == 0U && child.cursor == 0U) {
        child.cursor = span.start;
        skip_space(&child);
        if (child.cursor >= child.source.len || child.source.ptr[child.cursor++] != '[')
            return false;
        skip_space(&child);
    }
    while (index <= wanted) {
        if (child.cursor >= span.end || child.source.ptr[child.cursor] == ']') return false;
        size_t start = child.cursor;
        if (!scan_value(&child, 1U)) { *reader = child; return false; }
        size_t end = child.cursor;
        skip_space(&child);
        uint8_t delimiter = child.cursor < span.end ? child.source.ptr[child.cursor++] : 0U;
        if (index == wanted) {
            *out_item = (json_span){start, end};
            if (delimiter == ',') skip_space(&child);
            *in_out_cursor = child.cursor;
            *in_out_index = index + 1U;
            *reader = child;
            return delimiter == ',' || delimiter == ']';
        }
        if (delimiter != ',') return false;
        skip_space(&child);
        index++;
    }
    return false;
}


bool project_timeline_string_copy(base_bytes encoded_string, base_mut_bytes output,
                                  size_t *out_length)
{
    return decode_string_content(encoded_string, output, out_length, NULL);
}

