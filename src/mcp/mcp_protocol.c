#define _POSIX_C_SOURCE 200809L

#include "mcp_protocol.h"
#include "mcp_protocol_async.h"

#include <stdio.h>
#include <string.h>
#include <time.h>

#define MCP_PROTOCOL_LEGACY "2025-11-25"
#define MCP_PROTOCOL_CURRENT "2026-07-28"
#define MCP_RESPONSE_LIMIT MCP_JSON_MAX_BYTES

static uint64_t monotonic_ms(void)
{
    struct timespec value;
    if (clock_gettime(CLOCK_MONOTONIC, &value) != 0) return 0U;
    if ((uint64_t)value.tv_sec > UINT64_MAX / 1000U) return UINT64_MAX;
    return (uint64_t)value.tv_sec * 1000U + (uint64_t)value.tv_nsec / 1000000U;
}

void mcp_protocol_session_reset(mcp_protocol_session *session)
{
    if (session != NULL) *session = (mcp_protocol_session){0};
}

static bool append_error(mcp_json_buffer *out, const char *id, size_t id_length,
                         int code, const char *message, const char *data)
{
    if (!mcp_json_buffer_literal(out, "{\"jsonrpc\":\"2.0\",\"id\":")) return false;
    if (id == NULL) {
        if (!mcp_json_buffer_literal(out, "null")) return false;
    } else if (!mcp_json_buffer_append(out, id, id_length)) return false;
    if (!mcp_json_buffer_literal(out, ",\"error\":{\"code\":")) return false;
    char number[32];
    int amount = snprintf(number, sizeof(number), "%d", code);
    if (amount < 0 || (size_t)amount >= sizeof(number) ||
        !mcp_json_buffer_append(out, number, (size_t)amount) ||
        !mcp_json_buffer_literal(out, ",\"message\":" ) ||
        !mcp_json_buffer_quoted(out, message, strlen(message))) return false;
    if (data != NULL && (!mcp_json_buffer_literal(out, ",\"data\":" ) ||
                         !mcp_json_buffer_literal(out, data))) return false;
    return mcp_json_buffer_literal(out, "}}");
}

static bool append_response_prefix(mcp_json_buffer *out, const mcp_json_doc *doc,
                                   size_t id_token)
{
    return mcp_json_buffer_literal(out, "{\"jsonrpc\":\"2.0\",\"id\":" ) &&
        mcp_json_buffer_raw_token(out, doc, id_token) &&
        mcp_json_buffer_literal(out, ",\"result\":{");
}

static bool append_server_meta(mcp_json_buffer *out, bool modern)
{
    if (!modern) return true;
    bool empty_body = out->length > 0U && out->data[out->length - 1U] == '{';
    return mcp_json_buffer_literal(out, empty_body ? "" : ",") && mcp_json_buffer_literal(out,
        "\"_meta\":{\"io.modelcontextprotocol/serverInfo\":{\"name\":\"Vexel\",\"version\":\"0.1.0\"}}" );
}

static bool append_response_suffix(mcp_json_buffer *out, bool modern)
{
    return append_server_meta(out, modern) && mcp_json_buffer_literal(out, "}}");
}

static bool request_is_modern(const mcp_json_doc *doc, size_t params)
{
    size_t meta = mcp_json_object_get(doc, params, "_meta");
    if (meta == MCP_JSON_NO_TOKEN) return false;
    size_t version = mcp_json_object_get(doc, meta, "io.modelcontextprotocol/protocolVersion");
    return mcp_json_string_equals(doc, version, MCP_PROTOCOL_CURRENT);
}

static bool validate_modern_envelope(const mcp_json_doc *doc, size_t params)
{
    size_t meta = mcp_json_object_get(doc, params, "_meta");
    size_t version = mcp_json_object_get(doc, meta, "io.modelcontextprotocol/protocolVersion");
    return mcp_json_string_equals(doc, version, MCP_PROTOCOL_CURRENT);
}

static bool append_capabilities(mcp_json_buffer *out, bool tasks_available)
{
    return mcp_json_buffer_literal(out, "\"capabilities\":{") &&
        (!tasks_available || mcp_json_buffer_literal(out,
            "\"io.modelcontextprotocol/tasks\":{},")) &&
        mcp_json_buffer_literal(out,
        "\"tools\":{\"listChanged\":false},"
        "\"resources\":{\"subscribe\":false,\"listChanged\":false},"
        "\"prompts\":{\"listChanged\":false}}," );
}

static bool append_initialize(mcp_protocol_session *session, const mcp_json_doc *doc,
                              size_t params, size_t id, mcp_json_buffer *out)
{
    if (session->initialized || session->modern) return append_error(out,
        (const char *)doc->source + doc->tokens[id].start,
        doc->tokens[id].end - doc->tokens[id].start,
        -32600, "initialize is not valid in this protocol session", NULL);
    size_t version = mcp_json_object_get(doc, params, "protocolVersion");
    if (!mcp_json_string_equals(doc, version, MCP_PROTOCOL_LEGACY)) {
        return append_error(out, (const char *)doc->source + doc->tokens[id].start,
            doc->tokens[id].end - doc->tokens[id].start, -32004,
            "unsupported protocol version", "{\"supported\":[\"2025-11-25\"]}");
    }
    memcpy(session->protocol_version, MCP_PROTOCOL_LEGACY, sizeof(MCP_PROTOCOL_LEGACY));
    if (!append_response_prefix(out, doc, id) ||
        !mcp_json_buffer_literal(out, "\"protocolVersion\":\"2025-11-25\"," ) ||
        !append_capabilities(out, false) ||
        !mcp_json_buffer_literal(out,
            "\"serverInfo\":{\"name\":\"Vexel\",\"version\":\"0.1.0\"},"
            "\"instructions\":\"Vexel MCP controls documents open in the Vexel app."
            " File opens and Save As are restricted to user-granted directories.\"" ) ||
        !mcp_json_buffer_literal(out, "}}")) return false;
    return true;
}

static bool append_discover(mcp_protocol_session *session, const mcp_json_doc *doc,
                            size_t id, bool tasks_available, mcp_json_buffer *out)
{
    session->modern = true;
    memcpy(session->protocol_version, MCP_PROTOCOL_CURRENT, sizeof(MCP_PROTOCOL_CURRENT));
    return append_response_prefix(out, doc, id) &&
        mcp_json_buffer_literal(out,
            "\"supportedVersions\":[\"2026-07-28\"],") &&
        append_capabilities(out, tasks_available) &&
        mcp_json_buffer_literal(out,
            "\"instructions\":\"Vexel MCP controls documents open in the Vexel app."
            " File opens and Save As are restricted to user-granted directories.\","
            "\"ttlMs\":10000,\"cacheScope\":\"connection\"" ) &&
        append_response_suffix(out, true);
}

static bool append_resources_list(mcp_json_buffer *out)
{
    return mcp_json_buffer_literal(out,
        "\"resources\":["
        "{\"uri\":\"vexel://documents/current/source\",\"name\":\"Current SVG source\",\"description\":\"Source-preserving SVG text for the current document.\",\"mimeType\":\"image/svg+xml\"},"
        "{\"uri\":\"vexel://documents/current/tree\",\"name\":\"Current SVG tree\",\"description\":\"Bounded element-tree summary for the current document.\",\"mimeType\":\"application/json\"},"
        "{\"uri\":\"vexel://documents/current/render\",\"name\":\"Last document render\",\"description\":\"Last bounded PNG render.\",\"mimeType\":\"image/png\"}]" );
}

static bool append_prompts_list(mcp_json_buffer *out)
{
    return mcp_json_buffer_literal(out,
        "\"prompts\":[{\"name\":\"inspect-document\","
        "\"description\":\"Inspect the open SVG structure and identify unsupported content.\","
        "\"arguments\":[]}]" );
}

static bool append_prompt_get(const mcp_json_doc *doc, size_t params,
                              mcp_json_buffer *out)
{
    size_t name = mcp_json_object_get(doc, params, "name");
    if (!mcp_json_string_equals(doc, name, "inspect-document")) return false;
    return mcp_json_buffer_literal(out,
        "\"description\":\"Inspect the open SVG document.\","
        "\"messages\":[{\"role\":\"user\",\"content\":{\"type\":\"text\","
        "\"text\":\"Use document_list and document_info, then inspect a bounded tree page."
        " Report unsupported features before suggesting edits.\"}}]" );
}

static bool append_text_tool_error(mcp_json_buffer *out, const mcp_json_doc *doc,
                                   size_t id, bool modern,
                                   const char *message, const char *code)
{
    return append_response_prefix(out, doc, id) &&
        mcp_json_buffer_literal(out, "\"content\":[{\"type\":\"text\",\"text\":" ) &&
        mcp_json_buffer_quoted(out, message, strlen(message)) &&
        mcp_json_buffer_literal(out, "}],\"structuredContent\":{\"error\":{\"code\":" ) &&
        mcp_json_buffer_quoted(out, code, strlen(code)) &&
        mcp_json_buffer_literal(out, ",\"message\":" ) &&
        mcp_json_buffer_quoted(out, message, strlen(message)) &&
        mcp_json_buffer_literal(out, "}},\"isError\":true") &&
        append_response_suffix(out, modern);
}

static bool request_supports_tasks(const mcp_json_doc *doc, size_t params)
{
    size_t meta = mcp_json_object_get(doc, params, "_meta");
    size_t caps = mcp_json_object_get(doc, meta,
        "io.modelcontextprotocol/clientCapabilities");
    size_t extensions = mcp_json_object_get(doc, caps, "extensions");
    size_t tasks = mcp_json_object_get(doc, extensions,
        "io.modelcontextprotocol/tasks");
    return tasks != MCP_JSON_NO_TOKEN && doc->tokens[tasks].type == MCP_JSON_OBJECT;
}

static bool append_async_payload(mcp_json_buffer *out, const mcp_json_doc *doc,
    size_t id, const char *payload, size_t payload_length, bool modern,
    bool is_error)
{
    if (payload_length == 0U || payload_length > MCP_RESPONSE_LIMIT) return false;
    if (is_error) {
        return mcp_json_buffer_literal(out, "{\"jsonrpc\":\"2.0\",\"id\":") &&
            mcp_json_buffer_raw_token(out, doc, id) &&
            mcp_json_buffer_literal(out, ",\"error\":") &&
            mcp_json_buffer_append(out, payload, payload_length) &&
            mcp_json_buffer_literal(out, "}");
    }
    if (!mcp_json_buffer_literal(out, "{\"jsonrpc\":\"2.0\",\"id\":") ||
        !mcp_json_buffer_raw_token(out, doc, id) ||
        !mcp_json_buffer_literal(out, ",\"result\":")) return false;
    if (!modern) return mcp_json_buffer_append(out, payload, payload_length) &&
        mcp_json_buffer_literal(out, "}");

    mcp_json_doc result = {0};
    if (!mcp_json_parse((const uint8_t *)payload, payload_length, &result)) return false;
    size_t root = result.root;
    size_t meta = mcp_json_object_get(&result, root, "_meta");
    bool okay = mcp_json_buffer_literal(out, "{");
    bool first = true;
    for (size_t key = result.tokens[root].first_child;
         okay && key != MCP_JSON_NO_TOKEN;) {
        size_t value = result.tokens[key].next_sibling;
        if (value == MCP_JSON_NO_TOKEN) { okay = false; break; }
        size_t next = result.tokens[value].next_sibling;
        if (!mcp_json_string_equals(&result, key, "_meta")) {
            okay = (first || mcp_json_buffer_literal(out, ",")) &&
                mcp_json_buffer_raw_token(out, &result, key) &&
                mcp_json_buffer_literal(out, ":") &&
                mcp_json_buffer_raw_token(out, &result, value);
            first = false;
        }
        key = next;
    }
    if (okay) okay = (first || mcp_json_buffer_literal(out, ",")) &&
        mcp_json_buffer_literal(out, "\"_meta\":{");
    bool meta_first = true;
    if (okay && meta != MCP_JSON_NO_TOKEN) {
        for (size_t key = result.tokens[meta].first_child;
             okay && key != MCP_JSON_NO_TOKEN;) {
            size_t value = result.tokens[key].next_sibling;
            if (value == MCP_JSON_NO_TOKEN) { okay = false; break; }
            size_t next = result.tokens[value].next_sibling;
            if (!mcp_json_string_equals(&result, key,
                    "io.modelcontextprotocol/serverInfo")) {
                okay = (meta_first || mcp_json_buffer_literal(out, ",")) &&
                    mcp_json_buffer_raw_token(out, &result, key) &&
                    mcp_json_buffer_literal(out, ":") &&
                    mcp_json_buffer_raw_token(out, &result, value);
                meta_first = false;
            }
            key = next;
        }
    }
    if (okay) okay = (meta_first || mcp_json_buffer_literal(out, ",")) &&
        mcp_json_buffer_literal(out,
            "\"io.modelcontextprotocol/serverInfo\":{\"name\":\"Vexel\","
            "\"version\":\"0.1.0\"}}}");
    mcp_json_doc_destroy(&result);
    return okay && mcp_json_buffer_literal(out, "}");
}

static bool async_callback_response(mcp_protocol_session *session,
    const mcp_json_doc *doc, size_t id, bool modern, bool supports_tasks,
    bool tasks_ready, mcp_protocol_async_payload_kind payload_kind,
    mcp_protocol_async_disposition disposition, mcp_json_buffer *payload,
    mcp_json_buffer *out)
{
    if (disposition == MCP_PROTOCOL_ASYNC_DEFERRED) {
        if (modern) return append_error(out,
            (const char *)doc->source + doc->tokens[id].start,
            doc->tokens[id].end - doc->tokens[id].start, -32603,
            "deferred response is unavailable for the current protocol", NULL);
        return true;
    }
    if (disposition == MCP_PROTOCOL_ASYNC_FAILED) return append_error(out,
        (const char *)doc->source + doc->tokens[id].start,
        doc->tokens[id].end - doc->tokens[id].start, -32603,
        "asynchronous protocol operation failed", NULL);
    bool is_error = disposition == MCP_PROTOCOL_ASYNC_ERROR;
    if (disposition != MCP_PROTOCOL_ASYNC_RESULT && !is_error)
        return append_error(out,
            (const char *)doc->source + doc->tokens[id].start,
            doc->tokens[id].end - doc->tokens[id].start, -32603,
            "invalid asynchronous protocol disposition", NULL);
    bool is_task = false;
    mcp_protocol_async_payload_kind validation_kind = is_error ?
        MCP_PROTOCOL_ASYNC_PAYLOAD_ERROR : payload_kind;
    if (!mcp_protocol_async_validate_payload(payload->data, payload->length,
                                            validation_kind, &is_task))
        return append_error(out,
            (const char *)doc->source + doc->tokens[id].start,
            doc->tokens[id].end - doc->tokens[id].start, -32603,
            "asynchronous backend returned invalid JSON", NULL);
    size_t id_length = doc->tokens[id].end - doc->tokens[id].start;
    size_t overhead = 512U + id_length;
    if (overhead >= MCP_RESPONSE_LIMIT ||
        payload->length > MCP_RESPONSE_LIMIT - overhead)
        return append_error(out,
            (const char *)doc->source + doc->tokens[id].start,
            id_length, -32603, "asynchronous result exceeds the response limit", NULL);
    if (is_task && (!supports_tasks || !tasks_ready)) return append_error(out,
        (const char *)doc->source + doc->tokens[id].start,
        doc->tokens[id].end - doc->tokens[id].start, -32021,
        "task support was not negotiated for this request",
        "{\"requiredCapabilities\":{\"io.modelcontextprotocol/tasks\":{}}}");
    (void)session;
    return append_async_payload(out, doc, id, payload->data, payload->length,
                                modern, is_error);
}

static bool append_tool_call(mcp_protocol_session *session, const mcp_json_doc *doc,
                             size_t params, size_t id, const mcp_protocol_backend *backend,
                             const mcp_protocol_async_backend *async_backend,
                             mcp_json_buffer *out)
{
    size_t name_token = mcp_json_object_get(doc, params, "name");
    size_t args = mcp_json_object_get(doc, params, "arguments");
    char name[64];
    size_t name_length = 0U;
    if (!mcp_json_string_copy(doc, name_token, name, sizeof(name) - 1U, &name_length) ||
        name_length == 0U || args == MCP_JSON_NO_TOKEN) {
        return append_error(out, (const char *)doc->source + doc->tokens[id].start,
            doc->tokens[id].end - doc->tokens[id].start, -32602,
            "tools/call requires a tool name and arguments", NULL);
    }
    name[name_length] = '\0';
    const char *validation_error = NULL;
    if (!mcp_tool_validate(doc, args, name, &validation_error)) {
        return append_text_tool_error(out, doc, id, session->modern,
            validation_error == NULL ? "invalid tool arguments" : validation_error,
            strcmp(validation_error == NULL ? "" : validation_error, "unknown tool") == 0 ?
            "unknown_tool" : "invalid_arguments");
    }
    if (async_backend != NULL && async_backend->call_tool != NULL) {
        mcp_json_buffer payload = {0};
        bool tasks_ready = mcp_protocol_async_tasks_ready(async_backend);
        bool supports_tasks = session->modern && request_supports_tasks(doc, params) &&
            tasks_ready;
        if (!mcp_json_buffer_init(&payload, 128U, MCP_RESPONSE_LIMIT)) return false;
        mcp_protocol_async_disposition disposition = async_backend->call_tool(
            async_backend->context, doc, args, name, session->modern,
            supports_tasks, &payload);
        if (disposition != MCP_PROTOCOL_ASYNC_UNHANDLED) {
            bool emitted = async_callback_response(session, doc, id,
                session->modern, supports_tasks, tasks_ready,
                MCP_PROTOCOL_ASYNC_PAYLOAD_TOOL_RESULT, disposition, &payload, out);
            mcp_json_buffer_destroy(&payload);
            return emitted;
        }
        mcp_json_buffer_destroy(&payload);
    }
    if (backend == NULL || backend->call_tool == NULL) return append_text_tool_error(out,
        doc, id, session->modern, "Vexel broker is unavailable", "broker_unavailable");
    mcp_tool_response result = {0};
    if (!mcp_json_buffer_init(&result.structured_json, 128U, MCP_RESPONSE_LIMIT) ||
        !mcp_json_buffer_init(&result.content_json, 128U, MCP_RESPONSE_LIMIT)) {
        mcp_json_buffer_destroy(&result.structured_json);
        mcp_json_buffer_destroy(&result.content_json);
        return false;
    }
    bool okay = backend->call_tool(backend->context, doc, args, name, &result);
    if (!okay || result.structured_json.length == 0U) {
        mcp_json_buffer_destroy(&result.structured_json);
        mcp_json_buffer_destroy(&result.content_json);
        return append_text_tool_error(out, doc, id, session->modern,
                                      "The tool could not complete its request", "tool_failed");
    }
    bool response_okay = append_response_prefix(out, doc, id) &&
        mcp_json_buffer_literal(out, "\"content\":");
    if (response_okay && result.content_json.length > 0U) {
        response_okay = mcp_json_buffer_append(out, result.content_json.data,
                                               result.content_json.length);
    } else if (response_okay) {
        const char *fallback = result.is_error ? "Tool request failed" : "Tool completed.";
        response_okay = mcp_json_buffer_literal(out, "[{\"type\":\"text\",\"text\":" ) &&
            mcp_json_buffer_quoted(out, fallback, strlen(fallback)) &&
            mcp_json_buffer_literal(out, "}]" );
    }
    if (response_okay) response_okay = mcp_json_buffer_literal(out, ",\"structuredContent\":") &&
        mcp_json_buffer_append(out, result.structured_json.data, result.structured_json.length) &&
        mcp_json_buffer_literal(out, result.is_error ? ",\"isError\":true" : ",\"isError\":false") &&
        append_response_suffix(out, session->modern);
    mcp_json_buffer_destroy(&result.structured_json);
    mcp_json_buffer_destroy(&result.content_json);
    return response_okay;
}

static bool append_resource_read(mcp_protocol_session *session, const mcp_json_doc *doc,
                                 size_t params, size_t id,
                                 const mcp_protocol_backend *backend,
                                 mcp_json_buffer *out)
{
    size_t uri_token = mcp_json_object_get(doc, params, "uri");
    char uri[256];
    size_t length = 0U;
    if (!mcp_json_string_copy(doc, uri_token, uri, sizeof(uri) - 1U, &length))
        return append_error(out, (const char *)doc->source + doc->tokens[id].start,
            doc->tokens[id].end - doc->tokens[id].start, -32602, "resources/read requires a URI", NULL);
    uri[length] = '\0';
    mcp_json_buffer contents = {0};
    if (!mcp_json_buffer_init(&contents, 128U, MCP_RESPONSE_LIMIT)) return false;
    bool okay = backend != NULL && backend->read_resource != NULL &&
        backend->read_resource(backend->context, doc, uri, &contents);
    bool response_okay = okay ? append_response_prefix(out, doc, id) &&
        mcp_json_buffer_literal(out, "\"contents\":" ) &&
        mcp_json_buffer_append(out, contents.data, contents.length) &&
        append_response_suffix(out, session->modern) :
        append_error(out, (const char *)doc->source + doc->tokens[id].start,
            doc->tokens[id].end - doc->tokens[id].start, -32002, "resource not found", NULL);
    mcp_json_buffer_destroy(&contents);
    return response_okay;
}

static bool append_task_method(mcp_protocol_session *session,
    const mcp_json_doc *doc, size_t params, size_t id, const char *method,
    const mcp_protocol_async_backend *async_backend, mcp_json_buffer *out)
{
    if (!session->modern || !mcp_protocol_async_tasks_ready(async_backend))
        return append_error(out, (const char *)doc->source + doc->tokens[id].start,
            doc->tokens[id].end - doc->tokens[id].start, -32601,
            "method not found", NULL);
    if (!request_supports_tasks(doc, params)) return append_error(out,
        (const char *)doc->source + doc->tokens[id].start,
        doc->tokens[id].end - doc->tokens[id].start, -32021,
        "task support was not negotiated for this request",
        "{\"requiredCapabilities\":{\"io.modelcontextprotocol/tasks\":{}}}");
    char task_id[1025];
    if (!mcp_protocol_async_task_id(doc, params, task_id, sizeof(task_id)))
        return append_error(out, (const char *)doc->source + doc->tokens[id].start,
            doc->tokens[id].end - doc->tokens[id].start, -32602,
            "task method requires a valid taskId", NULL);
    mcp_json_buffer payload = {0};
    if (!mcp_json_buffer_init(&payload, 128U, MCP_RESPONSE_LIMIT)) return false;
    mcp_protocol_async_disposition disposition = strcmp(method, "tasks/get") == 0 ?
        async_backend->get_task(async_backend->context, doc, task_id, &payload) :
        async_backend->cancel_task(async_backend->context, doc, task_id, &payload);
    bool is_cancel = strcmp(method, "tasks/cancel") == 0;
    bool emitted = async_callback_response(session, doc, id, true, true, true,
        is_cancel ? MCP_PROTOCOL_ASYNC_PAYLOAD_TASK_CANCEL :
            MCP_PROTOCOL_ASYNC_PAYLOAD_TASK_GET,
        disposition, &payload, out);
    mcp_json_buffer_destroy(&payload);
    return emitted;
}

bool mcp_protocol_handle_async(mcp_protocol_session *session, const uint8_t *line,
                         size_t length, const mcp_protocol_backend *backend,
                         const mcp_protocol_async_backend *async_backend,
                         mcp_json_buffer *response)
{
    if (session == NULL || response == NULL || line == NULL || length == 0U ||
        length > MCP_JSON_MAX_BYTES) return false;
    response->length = 0U;
    mcp_json_doc doc = {0};
    if (!mcp_json_parse(line, length, &doc)) {
        return append_error(response, NULL, 0U, -32700, "parse error", NULL);
    }
    if (doc.tokens[doc.root].type != MCP_JSON_OBJECT) {
        mcp_json_doc_destroy(&doc);
        return append_error(response, NULL, 0U, -32600, "invalid request", NULL);
    }
    size_t id = mcp_json_object_get(&doc, doc.root, "id");
    size_t method_token = mcp_json_object_get(&doc, doc.root, "method");
    size_t jsonrpc = mcp_json_object_get(&doc, doc.root, "jsonrpc");
    bool has_id = id != MCP_JSON_NO_TOKEN;
    char method[96];
    size_t method_length = 0U;
    if (!mcp_json_string_equals(&doc, jsonrpc, "2.0") ||
        !mcp_json_string_copy(&doc, method_token, method, sizeof(method) - 1U, &method_length) ||
        method_length == 0U || (has_id && doc.tokens[id].type != MCP_JSON_STRING &&
                                doc.tokens[id].type != MCP_JSON_NUMBER)) {
        bool emitted = has_id ? append_error(response,
            (const char *)doc.source + doc.tokens[id].start,
            doc.tokens[id].end - doc.tokens[id].start, -32600, "invalid request", NULL) : true;
        mcp_json_doc_destroy(&doc);
        return emitted;
    }
    method[method_length] = '\0';
    if (!has_id && strncmp(method, "notifications/", 14U) != 0) {
        mcp_json_doc_destroy(&doc);
        return true;
    }
    uint64_t now = monotonic_ms();
    if (session->window_started_ms == 0U || now < session->window_started_ms ||
        now - session->window_started_ms >= 60000U) {
        session->window_started_ms = now;
        session->requests_in_window = 0U;
    }
    if (++session->requests_in_window > 120U) {
        bool emitted = has_id ? append_error(response,
            (const char *)doc.source + doc.tokens[id].start,
            doc.tokens[id].end - doc.tokens[id].start, -32000, "rate limit exceeded", NULL) : true;
        mcp_json_doc_destroy(&doc);
        return emitted;
    }
    size_t params = mcp_json_object_get(&doc, doc.root, "params");
    bool modern_probe = strcmp(method, "server/discover") == 0;
    bool modern_request = request_is_modern(&doc, params);
    if (modern_probe) {
        bool emitted = has_id ? (validate_modern_envelope(&doc, params) ?
            append_discover(session, &doc, id,
                mcp_protocol_async_tasks_ready(async_backend), response) : append_error(response,
                (const char *)doc.source + doc.tokens[id].start,
                doc.tokens[id].end - doc.tokens[id].start, -32022,
                "server/discover requires current protocol version metadata",
                "{\"supported\":[\"2026-07-28\"]}")) : true;
        mcp_json_doc_destroy(&doc);
        return emitted;
    }
    if (session->modern || modern_request) {
        if (!validate_modern_envelope(&doc, params)) {
            bool emitted = has_id ? append_error(response,
                (const char *)doc.source + doc.tokens[id].start,
                doc.tokens[id].end - doc.tokens[id].start, -32022,
                "unsupported or missing protocol version metadata",
                "{\"supported\":[\"2026-07-28\"]}") : true;
            mcp_json_doc_destroy(&doc);
            return emitted;
        }
        session->modern = true;
        memcpy(session->protocol_version, MCP_PROTOCOL_CURRENT, sizeof(MCP_PROTOCOL_CURRENT));
    } else if (strcmp(method, "initialize") != 0 &&
               strcmp(method, "notifications/initialized") != 0 &&
               !session->initialized && strcmp(method, "ping") != 0) {
        bool emitted = has_id ? append_error(response,
            (const char *)doc.source + doc.tokens[id].start,
            doc.tokens[id].end - doc.tokens[id].start, -32002,
            "initialize must complete before requests", NULL) : true;
        mcp_json_doc_destroy(&doc);
        return emitted;
    }
    bool okay = true;
    if (strcmp(method, "initialize") == 0) {
        okay = has_id ? append_initialize(session, &doc, params, id, response) : true;
    } else if (strcmp(method, "notifications/initialized") == 0 ||
               strcmp(method, "notifications/cancelled") == 0 ||
               strcmp(method, "notifications/progress") == 0) {
        if (strcmp(method, "notifications/initialized") == 0) session->initialized = true;
        else if (strcmp(method, "notifications/cancelled") == 0 &&
                 !has_id && session->initialized && !session->modern &&
                 async_backend != NULL && async_backend->cancel_request != NULL &&
                 params != MCP_JSON_NO_TOKEN &&
                 doc.tokens[params].type == MCP_JSON_OBJECT) {
            size_t request_id = mcp_json_object_get(&doc, params, "requestId");
            if (request_id != MCP_JSON_NO_TOKEN &&
                (doc.tokens[request_id].type == MCP_JSON_STRING ||
                 doc.tokens[request_id].type == MCP_JSON_NUMBER))
                async_backend->cancel_request(async_backend->context, &doc,
                                               request_id);
        }
    } else if (strcmp(method, "ping") == 0) {
        okay = !has_id || (append_response_prefix(response, &doc, id) &&
            append_response_suffix(response, session->modern));
    } else if (strcmp(method, "tools/list") == 0) {
        okay = !has_id || (append_response_prefix(response, &doc, id) &&
            mcp_json_buffer_literal(response, "\"tools\":" ) &&
            mcp_tools_list_json(response) && append_response_suffix(response, session->modern));
    } else if (strcmp(method, "tools/call") == 0) {
        okay = !has_id || append_tool_call(session, &doc, params, id, backend,
                                            async_backend, response);
    } else if (strcmp(method, "tasks/get") == 0 ||
               strcmp(method, "tasks/cancel") == 0) {
        okay = !has_id || append_task_method(session, &doc, params, id, method,
                                              async_backend, response);
    } else if (strcmp(method, "resources/list") == 0) {
        okay = !has_id || (append_response_prefix(response, &doc, id) &&
            append_resources_list(response) && append_response_suffix(response, session->modern));
    } else if (strcmp(method, "resources/read") == 0) {
        okay = !has_id || append_resource_read(session, &doc, params, id, backend, response);
    } else if (strcmp(method, "prompts/list") == 0) {
        okay = !has_id || (append_response_prefix(response, &doc, id) &&
            append_prompts_list(response) && append_response_suffix(response, session->modern));
    } else if (strcmp(method, "prompts/get") == 0) {
        okay = !has_id || (append_response_prefix(response, &doc, id) &&
            (append_prompt_get(&doc, params, response) ||
             mcp_json_buffer_literal(response, "\"messages\":[]" )) &&
            append_response_suffix(response, session->modern));
    } else if (has_id) {
        okay = append_error(response, (const char *)doc.source + doc.tokens[id].start,
            doc.tokens[id].end - doc.tokens[id].start, -32601, "method not found", NULL);
    }
    mcp_json_doc_destroy(&doc);
    return okay;
}

bool mcp_protocol_handle(mcp_protocol_session *session, const uint8_t *line,
                         size_t length, const mcp_protocol_backend *backend,
                         mcp_json_buffer *response)
{
    return mcp_protocol_handle_async(session, line, length, backend, NULL,
                                     response);
}

bool mcp_protocol_request_expects_response(const uint8_t *line, size_t length)
{
    mcp_json_doc doc = {0};
    if (!mcp_json_parse(line, length, &doc)) return true;
    if (doc.tokens[doc.root].type != MCP_JSON_OBJECT) {
        mcp_json_doc_destroy(&doc);
        return true;
    }
    size_t method = mcp_json_object_get(&doc, doc.root, "method");
    bool expects = mcp_json_object_get(&doc, doc.root, "id") != MCP_JSON_NO_TOKEN;
    if (mcp_json_string_equals(&doc, method, "notifications/initialized") ||
        mcp_json_string_equals(&doc, method, "notifications/cancelled") ||
        mcp_json_string_equals(&doc, method, "notifications/progress")) expects = false;
    if (!expects && !mcp_json_string_equals(&doc, method, "notifications/initialized") &&
        !mcp_json_string_equals(&doc, method, "notifications/cancelled") &&
        !mcp_json_string_equals(&doc, method, "notifications/progress")) expects = true;
    mcp_json_doc_destroy(&doc);
    return expects;
}
