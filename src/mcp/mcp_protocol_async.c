#include "mcp_protocol_async.h"

#include <stdio.h>
#include <string.h>

#define ASYNC_TASK_ID_LIMIT 1024U
#define ASYNC_PROGRESS_TOKEN_LIMIT 1024U
#define ASYNC_PROGRESS_MESSAGE_LIMIT 127U
#define ASYNC_TASK_TTL_LIMIT UINT64_C(900000)

bool mcp_protocol_async_tasks_ready(const mcp_protocol_async_backend *backend)
{
    return backend != NULL && backend->tasks_available &&
        backend->call_tool != NULL && backend->get_task != NULL &&
        backend->cancel_task != NULL;
}

static bool string_field(const mcp_json_doc *doc, size_t object,
                         const char *key, char *buffer, size_t capacity,
                         size_t *out_length)
{
    size_t token = mcp_json_object_get(doc, object, key);
    if (!mcp_json_string_copy(doc, token, buffer, capacity, out_length)) return false;
    return memchr(buffer, '\0', *out_length) == NULL;
}

static bool valid_string_token(const mcp_json_doc *doc, size_t token,
                               bool require_nonempty)
{
    if (token == MCP_JSON_NO_TOKEN || doc->tokens[token].type != MCP_JSON_STRING)
        return false;
    size_t start = doc->tokens[token].start + 1U;
    size_t end = doc->tokens[token].end - 1U;
    if (require_nonempty && start == end) return false;
    for (size_t cursor = start; cursor < end;) {
        if (doc->source[cursor] != '\\') { cursor++; continue; }
        if (cursor + 1U >= end) return false;
        if (doc->source[cursor + 1U] != 'u') { cursor += 2U; continue; }
        if (end - cursor < 6U) return false;
        unsigned codepoint = 0U;
        for (size_t index = cursor + 2U; index < cursor + 6U; ++index) {
            uint8_t ch = doc->source[index];
            unsigned digit = ch >= '0' && ch <= '9' ? (unsigned)(ch - '0') :
                ch >= 'a' && ch <= 'f' ? (unsigned)(ch - 'a') + 10U :
                ch >= 'A' && ch <= 'F' ? (unsigned)(ch - 'A') + 10U : 16U;
            if (digit > 15U) return false;
            codepoint = (codepoint << 4U) | digit;
        }
        if (codepoint == 0U) return false;
        cursor += 6U;
    }
    return true;
}

static bool optional_string_field(const mcp_json_doc *doc, size_t object,
                                  const char *key, bool require_nonempty)
{
    size_t token = mcp_json_object_get(doc, object, key);
    return token == MCP_JSON_NO_TOKEN ||
        valid_string_token(doc, token, require_nonempty);
}

bool mcp_protocol_async_task_id(const mcp_json_doc *doc, size_t params,
                                char *out_id, size_t capacity)
{
    if (doc == NULL || params == MCP_JSON_NO_TOKEN || out_id == NULL ||
        capacity < 2U) return false;
    size_t length = 0U;
    if (!string_field(doc, params, "taskId", out_id, capacity - 1U, &length) ||
        length == 0U || length > ASYNC_TASK_ID_LIMIT) return false;
    out_id[length] = '\0';
    return true;
}

static bool valid_timestamp(const mcp_json_doc *doc, size_t object,
                            const char *key)
{
    char value[64];
    size_t length = 0U;
    if (!string_field(doc, object, key, value, sizeof(value) - 1U, &length) ||
        length < 20U) return false;
    if (value[4] != '-' || value[7] != '-' || value[10] != 'T' ||
        value[13] != ':' || value[16] != ':') return false;
    for (size_t index = 0U; index < 19U; ++index) {
        if (index == 4U || index == 7U || index == 10U ||
            index == 13U || index == 16U) continue;
        if (value[index] < '0' || value[index] > '9') return false;
    }
    unsigned year = (unsigned)(value[0] - '0') * 1000U +
        (unsigned)(value[1] - '0') * 100U +
        (unsigned)(value[2] - '0') * 10U + (unsigned)(value[3] - '0');
    unsigned month = (unsigned)(value[5] - '0') * 10U + (unsigned)(value[6] - '0');
    unsigned day = (unsigned)(value[8] - '0') * 10U + (unsigned)(value[9] - '0');
    unsigned hour = (unsigned)(value[11] - '0') * 10U + (unsigned)(value[12] - '0');
    unsigned minute = (unsigned)(value[14] - '0') * 10U + (unsigned)(value[15] - '0');
    unsigned second = (unsigned)(value[17] - '0') * 10U + (unsigned)(value[18] - '0');
    static const unsigned month_days[] = {31U, 28U, 31U, 30U, 31U, 30U,
        31U, 31U, 30U, 31U, 30U, 31U};
    if (year == 0U || month == 0U || month > 12U || hour > 23U ||
        minute > 59U || second > 60U) return false;
    unsigned max_day = month_days[month - 1U];
    if (month == 2U && (year % 4U == 0U && (year % 100U != 0U || year % 400U == 0U)))
        max_day++;
    if (day == 0U || day > max_day) return false;
    size_t cursor = 19U;
    if (cursor < length && value[cursor] == '.') {
        cursor++;
        size_t fraction_start = cursor;
        while (cursor < length && value[cursor] >= '0' && value[cursor] <= '9') cursor++;
        if (cursor == fraction_start) return false;
    }
    if (cursor < length && value[cursor] == 'Z') return cursor + 1U == length;
    if (cursor + 6U != length ||
        (value[cursor] != '+' && value[cursor] != '-') ||
        value[cursor + 3U] != ':') return false;
    for (size_t index = cursor + 1U; index < length; ++index) {
        if (index == cursor + 3U) continue;
        if (value[index] < '0' || value[index] > '9') return false;
    }
    unsigned zone_hour = (unsigned)(value[cursor + 1U] - '0') * 10U +
        (unsigned)(value[cursor + 2U] - '0');
    unsigned zone_minute = (unsigned)(value[cursor + 4U] - '0') * 10U +
        (unsigned)(value[cursor + 5U] - '0');
    return zone_hour <= 14U && zone_minute <= 59U &&
        (zone_hour != 14U || zone_minute == 0U);
}

static bool validate_error(const mcp_json_doc *doc, size_t object)
{
    size_t code = mcp_json_object_get(doc, object, "code");
    char message[256];
    size_t length = 0U;
    if (code == MCP_JSON_NO_TOKEN || doc->tokens[code].type != MCP_JSON_NUMBER ||
        !string_field(doc, object, "message", message, sizeof(message) - 1U,
                      &length) || length == 0U) return false;
    for (size_t index = doc->tokens[code].start; index < doc->tokens[code].end; ++index)
        if (doc->source[index] == '.' || doc->source[index] == 'e' ||
            doc->source[index] == 'E') return false;
    return true;
}

static bool validate_annotations(const mcp_json_doc *doc, size_t object)
{
    size_t annotations = mcp_json_object_get(doc, object, "annotations");
    return annotations == MCP_JSON_NO_TOKEN ||
        doc->tokens[annotations].type == MCP_JSON_OBJECT;
}

static bool validate_resource_block(const mcp_json_doc *doc, size_t block)
{
    size_t resource = mcp_json_object_get(doc, block, "resource");
    if (resource == MCP_JSON_NO_TOKEN ||
        doc->tokens[resource].type != MCP_JSON_OBJECT ||
        !optional_string_field(doc, resource, "uri", true) ||
        !optional_string_field(doc, resource, "mimeType", true)) return false;
    size_t uri = mcp_json_object_get(doc, resource, "uri");
    size_t text = mcp_json_object_get(doc, resource, "text");
    size_t blob = mcp_json_object_get(doc, resource, "blob");
    if (!valid_string_token(doc, uri, true) ||
        ((text == MCP_JSON_NO_TOKEN) == (blob == MCP_JSON_NO_TOKEN)) ||
        (text != MCP_JSON_NO_TOKEN && !valid_string_token(doc, text, false)) ||
        (blob != MCP_JSON_NO_TOKEN && !valid_string_token(doc, blob, false)))
        return false;
    return validate_annotations(doc, block);
}

static bool validate_resource_link(const mcp_json_doc *doc, size_t block)
{
    size_t uri = mcp_json_object_get(doc, block, "uri");
    size_t name = mcp_json_object_get(doc, block, "name");
    size_t icons = mcp_json_object_get(doc, block, "icons");
    size_t size = mcp_json_object_get(doc, block, "size");
    uint64_t ignored_size = 0U;
    return valid_string_token(doc, uri, true) &&
        valid_string_token(doc, name, true) &&
        optional_string_field(doc, block, "title", false) &&
        optional_string_field(doc, block, "description", false) &&
        optional_string_field(doc, block, "mimeType", true) &&
        (icons == MCP_JSON_NO_TOKEN || doc->tokens[icons].type == MCP_JSON_ARRAY) &&
        (size == MCP_JSON_NO_TOKEN ||
         mcp_json_uint(doc, size, &ignored_size)) &&
        validate_annotations(doc, block);
}

static bool validate_content_block(const mcp_json_doc *doc, size_t block)
{
    if (doc->tokens[block].type != MCP_JSON_OBJECT ||
        !validate_annotations(doc, block)) return false;
    size_t type = mcp_json_object_get(doc, block, "type");
    if (!valid_string_token(doc, type, true)) return false;
    if (mcp_json_string_equals(doc, type, "text")) {
        size_t text = mcp_json_object_get(doc, block, "text");
        return valid_string_token(doc, text, false);
    }
    if (mcp_json_string_equals(doc, type, "image") ||
        mcp_json_string_equals(doc, type, "audio")) {
        return valid_string_token(doc, mcp_json_object_get(doc, block, "data"), true) &&
            valid_string_token(doc, mcp_json_object_get(doc, block, "mimeType"), true);
    }
    if (mcp_json_string_equals(doc, type, "resource"))
        return validate_resource_block(doc, block);
    if (mcp_json_string_equals(doc, type, "resource_link"))
        return validate_resource_link(doc, block);
    return false;
}

static bool validate_content(const mcp_json_doc *doc, size_t token)
{
    if (token == MCP_JSON_NO_TOKEN || doc->tokens[token].type != MCP_JSON_ARRAY)
        return false;
    for (size_t block = doc->tokens[token].first_child;
         block != MCP_JSON_NO_TOKEN; block = doc->tokens[block].next_sibling)
        if (!validate_content_block(doc, block)) return false;
    return true;
}

static bool validate_call_result(const mcp_json_doc *doc, size_t object)
{
    if (object == MCP_JSON_NO_TOKEN || doc->tokens[object].type != MCP_JSON_OBJECT)
        return false;
    size_t content = mcp_json_object_get(doc, object, "content");
    size_t is_error = mcp_json_object_get(doc, object, "isError");
    size_t structured = mcp_json_object_get(doc, object, "structuredContent");
    size_t kind = mcp_json_object_get(doc, object, "resultType");
    size_t meta = mcp_json_object_get(doc, object, "_meta");
    return validate_content(doc, content) &&
        (is_error == MCP_JSON_NO_TOKEN || doc->tokens[is_error].type == MCP_JSON_TRUE ||
         doc->tokens[is_error].type == MCP_JSON_FALSE) &&
        (structured == MCP_JSON_NO_TOKEN ||
         doc->tokens[structured].type == MCP_JSON_OBJECT) &&
        (kind == MCP_JSON_NO_TOKEN || mcp_json_string_equals(doc, kind, "complete")) &&
        (meta == MCP_JSON_NO_TOKEN || doc->tokens[meta].type == MCP_JSON_OBJECT);
}

static bool task_status(const mcp_json_doc *doc, size_t object, const char *status)
{
    return mcp_json_string_equals(doc, mcp_json_object_get(doc, object, "status"), status);
}

static bool validate_task_metadata(const mcp_json_doc *doc, size_t object,
                                   bool allow_poll)
{
    char task_id[ASYNC_TASK_ID_LIMIT + 1U];
    size_t id_length = 0U;
    uint64_t ttl = 0U, poll = 0U;
    if (!string_field(doc, object, "taskId", task_id, ASYNC_TASK_ID_LIMIT,
                      &id_length) || id_length == 0U ||
        (!task_status(doc, object, "working") &&
         !task_status(doc, object, "completed") &&
         !task_status(doc, object, "failed") &&
         !task_status(doc, object, "cancelled")) ||
        !valid_timestamp(doc, object, "createdAt") ||
        !valid_timestamp(doc, object, "lastUpdatedAt") ||
        !mcp_json_uint(doc, mcp_json_object_get(doc, object, "ttlMs"), &ttl) ||
        ttl == 0U || ttl > ASYNC_TASK_TTL_LIMIT) return false;
    size_t poll_token = mcp_json_object_get(doc, object, "pollIntervalMs");
    if (poll_token != MCP_JSON_NO_TOKEN &&
        (!allow_poll || !mcp_json_uint(doc, poll_token, &poll) || poll == 0U ||
         poll > ASYNC_TASK_TTL_LIMIT)) return false;
    size_t status_message = mcp_json_object_get(doc, object, "statusMessage");
    size_t meta = mcp_json_object_get(doc, object, "_meta");
    if (status_message != MCP_JSON_NO_TOKEN &&
        doc->tokens[status_message].type != MCP_JSON_STRING) return false;
    if (meta != MCP_JSON_NO_TOKEN && doc->tokens[meta].type != MCP_JSON_OBJECT)
        return false;
    return true;
}

static bool validate_terminal_fields(const mcp_json_doc *doc, size_t object)
{
    size_t result = mcp_json_object_get(doc, object, "result");
    size_t error = mcp_json_object_get(doc, object, "error");
    if (task_status(doc, object, "completed"))
        return result != MCP_JSON_NO_TOKEN && error == MCP_JSON_NO_TOKEN &&
            validate_call_result(doc, result);
    if (task_status(doc, object, "failed"))
        return result == MCP_JSON_NO_TOKEN && error != MCP_JSON_NO_TOKEN &&
            validate_error(doc, error);
    return result == MCP_JSON_NO_TOKEN && error == MCP_JSON_NO_TOKEN;
}

static bool validate_create_task(const mcp_json_doc *doc)
{
    return validate_task_metadata(doc, doc->root, true) &&
        mcp_json_object_get(doc, doc->root, "result") == MCP_JSON_NO_TOKEN &&
        mcp_json_object_get(doc, doc->root, "error") == MCP_JSON_NO_TOKEN;
}

static bool validate_get_task(const mcp_json_doc *doc)
{
    return validate_task_metadata(doc, doc->root, false) &&
        validate_terminal_fields(doc, doc->root);
}

static bool validate_cancel_ack(const mcp_json_doc *doc)
{
    size_t root = doc->root;
    if (!mcp_json_string_equals(doc, mcp_json_object_get(doc, root, "resultType"),
                                "complete")) return false;
    size_t fields[] = {
        mcp_json_object_get(doc, root, "taskId"),
        mcp_json_object_get(doc, root, "status"),
        mcp_json_object_get(doc, root, "createdAt"),
        mcp_json_object_get(doc, root, "lastUpdatedAt"),
        mcp_json_object_get(doc, root, "ttlMs"),
        mcp_json_object_get(doc, root, "result"),
        mcp_json_object_get(doc, root, "error")};
    for (size_t index = 0U; index < sizeof(fields) / sizeof(fields[0]); ++index)
        if (fields[index] != MCP_JSON_NO_TOKEN) return false;
    size_t meta = mcp_json_object_get(doc, root, "_meta");
    if (meta != MCP_JSON_NO_TOKEN && doc->tokens[meta].type != MCP_JSON_OBJECT)
        return false;
    for (size_t key = doc->tokens[root].first_child;
         key != MCP_JSON_NO_TOKEN;) {
        size_t value = doc->tokens[key].next_sibling;
        if (value == MCP_JSON_NO_TOKEN) return false;
        if (!mcp_json_string_equals(doc, key, "resultType") &&
            !mcp_json_string_equals(doc, key, "_meta")) return false;
        key = doc->tokens[value].next_sibling;
    }
    return true;
}

bool mcp_protocol_async_validate_payload(const char *data, size_t length,
    mcp_protocol_async_payload_kind kind, bool *out_is_task)
{
    if (out_is_task != NULL) *out_is_task = false;
    if (data == NULL || length == 0U || length > MCP_JSON_MAX_BYTES) return false;
    mcp_json_doc doc = {0};
    if (!mcp_json_parse((const uint8_t *)data, length, &doc)) return false;
    bool okay = doc.tokens[doc.root].type == MCP_JSON_OBJECT;
    if (okay && kind == MCP_PROTOCOL_ASYNC_PAYLOAD_ERROR) {
        okay = validate_error(&doc, doc.root);
    } else if (okay && kind == MCP_PROTOCOL_ASYNC_PAYLOAD_TOOL_RESULT) {
        size_t result_type = mcp_json_object_get(&doc, doc.root, "resultType");
        if (result_type != MCP_JSON_NO_TOKEN &&
            mcp_json_string_equals(&doc, result_type, "task")) {
            okay = validate_create_task(&doc);
            if (okay && out_is_task != NULL) *out_is_task = true;
        } else {
            okay = validate_call_result(&doc, doc.root);
        }
    } else if (okay && kind == MCP_PROTOCOL_ASYNC_PAYLOAD_TASK_GET) {
        okay = mcp_json_string_equals(&doc,
            mcp_json_object_get(&doc, doc.root, "resultType"), "complete") &&
            validate_get_task(&doc);
    } else if (okay && kind == MCP_PROTOCOL_ASYNC_PAYLOAD_TASK_CANCEL) {
        okay = validate_cancel_ack(&doc);
    } else {
        okay = false;
    }
    mcp_json_doc_destroy(&doc);
    return okay;
}

static bool token_is_scalar_integer(const mcp_json_doc *doc, size_t token)
{
    if (token == MCP_JSON_NO_TOKEN) return false;
    if (doc->tokens[token].type == MCP_JSON_STRING) return true;
    if (doc->tokens[token].type != MCP_JSON_NUMBER) return false;
    size_t start = doc->tokens[token].start;
    size_t end = doc->tokens[token].end;
    for (size_t index = start; index < end; ++index)
        if (doc->source[index] == '.' || doc->source[index] == 'e' ||
            doc->source[index] == 'E') return false;
    return true;
}

bool mcp_protocol_async_legacy_progress(const char *raw_token,
    size_t raw_token_length, uint32_t progress_permille,
    int64_t *prior_progress_permille, const char *message,
    mcp_json_buffer *out_notification)
{
    if (raw_token == NULL || raw_token_length == 0U ||
        raw_token_length > ASYNC_PROGRESS_TOKEN_LIMIT ||
        progress_permille > 1000U || prior_progress_permille == NULL ||
        (int64_t)progress_permille <= *prior_progress_permille ||
        message == NULL || out_notification == NULL) return false;
    size_t message_length = 0U;
    while (message_length <= ASYNC_PROGRESS_MESSAGE_LIMIT &&
           message[message_length] != '\0') message_length++;
    if (message_length > ASYNC_PROGRESS_MESSAGE_LIMIT) return false;
    mcp_json_doc token_doc = {0};
    if (!mcp_json_parse((const uint8_t *)raw_token, raw_token_length, &token_doc))
        return false;
    bool valid_token = token_doc.tokens[token_doc.root].type == MCP_JSON_STRING ||
        token_is_scalar_integer(&token_doc, token_doc.root);
    mcp_json_doc_destroy(&token_doc);
    if (!valid_token) return false;
    size_t original_length = out_notification->length;
    bool okay = mcp_json_buffer_literal(out_notification,
        "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/progress\","
        "\"params\":{\"progressToken\":") &&
        mcp_json_buffer_append(out_notification, raw_token, raw_token_length) &&
        mcp_json_buffer_literal(out_notification, ",\"progress\":");
    char number[16];
    int written = snprintf(number, sizeof(number), "%u", progress_permille);
    okay = okay && written > 0 && (size_t)written < sizeof(number) &&
        mcp_json_buffer_append(out_notification, number, (size_t)written) &&
        mcp_json_buffer_literal(out_notification, ",\"total\":1000,\"message\":") &&
        mcp_json_buffer_quoted(out_notification, message, message_length) &&
        mcp_json_buffer_literal(out_notification, "}}");
    if (!okay) {
        out_notification->length = original_length;
        if (out_notification->data != NULL) out_notification->data[original_length] = '\0';
        return false;
    }
    *prior_progress_permille = (int64_t)progress_permille;
    return true;
}
