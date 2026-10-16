#ifndef VEXEL_PROJECT_PROJECT_TIMELINE_JSON_H
#define VEXEL_PROJECT_PROJECT_TIMELINE_JSON_H

#include "project_timeline.h"

typedef struct json_span { size_t start; size_t end; } json_span;
typedef struct json_field {
    base_bytes key;
    char key_storage[128];
    json_span value;
} json_field;
typedef struct json_reader {
    base_bytes source;
    size_t cursor;
    size_t error;
    project_timeline_error error_kind;
    bool (*cancel_check)(void *context);
    void *cancel_context;
    bool canceled;
} json_reader;
typedef struct object_fields {
    json_field fields[16];
    size_t count;
} object_fields;

bool is_space(uint8_t byte);
void skip_space(json_reader *reader);
bool fail(json_reader *reader, project_timeline_error error);
bool fail_at(json_reader *reader, project_timeline_error error, size_t offset);
bool json_reader_poll_cancel(json_reader *reader);
bool fail_at(json_reader *reader, project_timeline_error error, size_t offset);
bool scan_value(json_reader *reader, unsigned depth);
project_timeline_error project_timeline_json_classify(base_bytes source,
    bool *out_native_v1, size_t *out_error_offset,
    bool (*cancel_check)(void *), void *cancel_context, bool *out_canceled);
bool scan_number(json_reader *reader);
bool bytes_equal(base_bytes a, base_bytes b);
int bytes_compare(base_bytes a, base_bytes b);
bool field_equal(base_bytes key, const char *literal);
bool json_string_equal(base_bytes encoded, const char *literal);
bool json_strings_equal(base_bytes left, base_bytes right);
bool parse_object_fields(json_reader *reader, json_span span, object_fields *out);
const json_field *get_field(const object_fields *fields, const char *name);
bool only_fields(const object_fields *fields, const char *const *names, size_t name_count);
bool span_string(json_reader *reader, json_span span, base_bytes *out);
bool span_uint(json_reader *reader, json_span span, uint32_t *out);
bool span_bool(json_reader *reader, json_span span, bool *out);
bool span_number_value(json_reader *reader, json_span span, double *out);
bool span_array(json_reader *reader, json_span span, json_span *items, size_t capacity, size_t *out_count);
bool array_count(json_reader *reader, json_span span, size_t capacity, size_t *out_count);
bool array_item_at(json_reader *reader, json_span span, size_t wanted, json_span *out_item);
bool array_item_at_from(json_reader *reader, json_span span, size_t wanted,
                        size_t *in_out_cursor, size_t *in_out_index,
                        json_span *out_item);

#endif
