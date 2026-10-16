#include "mcp_protocol.h"
#include "mcp_protocol_async.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(value) do { if (!(value)) { \
    (void)fprintf(stderr, "async protocol check failed at line %d: %s\n", \
                  __LINE__, #value); return EXIT_FAILURE; } } while (0)

typedef struct fake_async {
    bool durable;
    bool canceled;
    bool expired;
    bool foreign;
    unsigned cancel_request_count;
    char cancel_target[64];
    mcp_protocol_async_disposition tool_disposition;
    const char *tool_payload;
    const char *get_mode;
    const char *cancel_payload;
    bool last_supports_tasks;
    unsigned supports_tasks_calls;
    unsigned no_tasks_calls;
} fake_async;

static const char *task_working =
    "{\"resultType\":\"task\",\"taskId\":\"task-001\","
    "\"status\":\"working\",\"createdAt\":\"2026-10-14T00:00:00Z\","
    "\"lastUpdatedAt\":\"2026-10-14T00:00:00Z\","
    "\"ttlMs\":900000,\"pollIntervalMs\":1000}";

static const char *complete_working =
    "{\"resultType\":\"complete\",\"taskId\":\"task-001\","
    "\"status\":\"working\",\"createdAt\":\"2026-10-14T00:00:00Z\","
    "\"lastUpdatedAt\":\"2026-10-14T00:00:01Z\",\"ttlMs\":900000}";

static bool copy_payload(mcp_json_buffer *out, const char *payload)
{
    return payload != NULL && mcp_json_buffer_append(out, payload, strlen(payload));
}

static mcp_protocol_async_disposition fake_call(void *context,
    const mcp_json_doc *request, size_t arguments, const char *tool_name,
    bool modern, bool supports_tasks, mcp_json_buffer *payload)
{
    (void)request; (void)arguments; (void)tool_name; (void)modern;
    fake_async *fake = context;
    fake->last_supports_tasks = supports_tasks;
    if (supports_tasks) fake->supports_tasks_calls++;
    else fake->no_tasks_calls++;
    if (fake->tool_disposition == MCP_PROTOCOL_ASYNC_UNHANDLED)
        return MCP_PROTOCOL_ASYNC_UNHANDLED;
    if (!copy_payload(payload, fake->tool_payload)) return MCP_PROTOCOL_ASYNC_FAILED;
    if (strcmp(fake->tool_payload, task_working) == 0) fake->durable = true;
    return fake->tool_disposition;
}

static mcp_protocol_async_disposition fake_get(void *context,
    const mcp_json_doc *request, const char *task_id, mcp_json_buffer *payload)
{
    (void)request;
    fake_async *fake = context;
    if (!fake->durable || strcmp(task_id, "task-001") != 0 || fake->foreign ||
        fake->expired) {
        (void)copy_payload(payload,
            "{\"code\":-32023,\"message\":\"task unavailable\"}");
        return MCP_PROTOCOL_ASYNC_ERROR;
    }
    const char *get_mode = fake->get_mode == NULL ? "working" : fake->get_mode;
    if (strcmp(get_mode, "failed") == 0) {
        (void)copy_payload(payload,
            "{\"resultType\":\"complete\",\"taskId\":\"task-001\","
            "\"status\":\"failed\",\"createdAt\":\"2026-10-14T00:00:00Z\","
            "\"lastUpdatedAt\":\"2026-10-14T00:00:02Z\",\"ttlMs\":900000,"
            "\"error\":{\"code\":-32024,\"message\":\"task failed\"}}");
        return MCP_PROTOCOL_ASYNC_RESULT;
    }
    if (strcmp(get_mode, "completed") == 0) {
        (void)copy_payload(payload,
            "{\"resultType\":\"complete\",\"taskId\":\"task-001\","
            "\"status\":\"completed\",\"createdAt\":\"2026-10-14T00:00:00Z\","
            "\"lastUpdatedAt\":\"2026-10-14T00:00:02Z\",\"ttlMs\":900000,"
            "\"result\":{\"content\":[],\"isError\":false}}");
    } else if (strcmp(get_mode, "isError") == 0) {
        (void)copy_payload(payload,
            "{\"resultType\":\"complete\",\"taskId\":\"task-001\","
            "\"status\":\"completed\",\"createdAt\":\"2026-10-14T00:00:00Z\","
            "\"lastUpdatedAt\":\"2026-10-14T00:00:02Z\",\"ttlMs\":900000,"
            "\"result\":{\"content\":[],\"isError\":true}}");
    } else if (strcmp(get_mode, "cancelled") == 0) {
        (void)copy_payload(payload,
            "{\"resultType\":\"complete\",\"taskId\":\"task-001\","
            "\"status\":\"cancelled\",\"createdAt\":\"2026-10-14T00:00:00Z\","
            "\"lastUpdatedAt\":\"2026-10-14T00:00:03Z\",\"ttlMs\":900000}");
    } else if (strcmp(get_mode, "empty") == 0) {
        (void)copy_payload(payload, "{}");
    } else if (strcmp(get_mode, "wrong") == 0) {
        (void)copy_payload(payload, "{\"resultType\":\"task\"}");
    } else if (strcmp(get_mode, "failed_missing") == 0) {
        (void)copy_payload(payload,
            "{\"resultType\":\"complete\",\"taskId\":\"task-001\","
            "\"status\":\"failed\",\"createdAt\":\"2026-10-14T00:00:00Z\","
            "\"lastUpdatedAt\":\"2026-10-14T00:00:02Z\",\"ttlMs\":900000}");
    } else (void)copy_payload(payload, complete_working);
    return MCP_PROTOCOL_ASYNC_RESULT;
}

static mcp_protocol_async_disposition fake_cancel(void *context,
    const mcp_json_doc *request, const char *task_id, mcp_json_buffer *payload)
{
    (void)request;
    fake_async *fake = context;
    if (!fake->durable || strcmp(task_id, "task-001") != 0 || fake->foreign ||
        fake->expired) {
        (void)copy_payload(payload,
            "{\"code\":-32023,\"message\":\"task unavailable\"}");
        return MCP_PROTOCOL_ASYNC_ERROR;
    }
    fake->canceled = true;
    (void)copy_payload(payload, fake->cancel_payload == NULL ?
        "{\"resultType\":\"complete\"}" : fake->cancel_payload);
    return MCP_PROTOCOL_ASYNC_RESULT;
}

static void fake_cancel_request(void *context, const mcp_json_doc *request,
                                size_t request_id_token)
{
    fake_async *fake = context;
    char token[64];
    size_t length = 0U;
    fake->cancel_request_count++;
    if (mcp_json_token_copy(request, request_id_token, token, sizeof(token) - 1U,
                            &length)) {
        token[length] = '\0';
        (void)snprintf(fake->cancel_target, sizeof(fake->cancel_target), "%s", token);
    }
}

static bool sync_tool(void *context, const mcp_json_doc *request, size_t args,
                      const char *name, mcp_tool_response *out)
{
    (void)context; (void)request; (void)args; (void)name;
    return mcp_json_buffer_literal(&out->structured_json, "{\"sync\":true}");
}

static bool call(mcp_protocol_session *session,
    const mcp_protocol_backend *sync, const mcp_protocol_async_backend *async,
    const char *line, mcp_json_buffer *response)
{
    response->length = 0U;
    mcp_json_doc request = {0};
    if (!mcp_json_parse((const uint8_t *)line, strlen(line), &request)) return false;
    bool handled = mcp_protocol_handle_async(session, (const uint8_t *)line,
        strlen(line), sync, async, response);
    if (!handled) { mcp_json_doc_destroy(&request); return false; }
    if (response->length == 0U) {
        mcp_json_doc_destroy(&request);
        return true;
    }
    size_t request_id = mcp_json_object_get(&request, request.root, "id");
    if (request_id == MCP_JSON_NO_TOKEN) {
        bool no_response = response->length == 0U;
        mcp_json_doc_destroy(&request);
        return no_response;
    }
    mcp_json_doc envelope = {0};
    bool valid = response->length > 0U &&
        mcp_json_parse((const uint8_t *)response->data, response->length, &envelope) &&
        envelope.tokens[envelope.root].type == MCP_JSON_OBJECT &&
        mcp_json_string_equals(&envelope,
            mcp_json_object_get(&envelope, envelope.root, "jsonrpc"), "2.0");
    size_t response_id = valid ? mcp_json_object_get(&envelope, envelope.root, "id") :
        MCP_JSON_NO_TOKEN;
    if (valid && response_id != MCP_JSON_NO_TOKEN) {
        size_t request_length = request.tokens[request_id].end -
            request.tokens[request_id].start;
        size_t response_length = envelope.tokens[response_id].end -
            envelope.tokens[response_id].start;
        valid = request_length == response_length &&
            memcmp(request.source + request.tokens[request_id].start,
                envelope.source + envelope.tokens[response_id].start,
                request_length) == 0;
    } else valid = false;
    size_t result = valid ? mcp_json_object_get(&envelope, envelope.root, "result") :
        MCP_JSON_NO_TOKEN;
    size_t error = valid ? mcp_json_object_get(&envelope, envelope.root, "error") :
        MCP_JSON_NO_TOKEN;
    valid = valid && ((result != MCP_JSON_NO_TOKEN && error == MCP_JSON_NO_TOKEN &&
        envelope.tokens[result].type == MCP_JSON_OBJECT) ||
        (error != MCP_JSON_NO_TOKEN && result == MCP_JSON_NO_TOKEN &&
         envelope.tokens[error].type == MCP_JSON_OBJECT));
    mcp_json_doc_destroy(&envelope);
    mcp_json_doc_destroy(&request);
    return valid;
}

static bool modern_discover(mcp_protocol_session *session,
    const mcp_protocol_backend *sync, const mcp_protocol_async_backend *async,
    mcp_json_buffer *response)
{
    return call(session, sync, async,
        "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"server/discover\","
        "\"params\":{\"_meta\":{\"io.modelcontextprotocol/protocolVersion\":"
        "\"2026-07-28\"}}}", response);
}

#define TASKS_META "\"_meta\":{\"io.modelcontextprotocol/protocolVersion\":\"2026-07-28\",\"io.modelcontextprotocol/clientCapabilities\":{\"extensions\":{\"io.modelcontextprotocol/tasks\":{}}}}"
#define TOOL_CALL(id) "{\"jsonrpc\":\"2.0\",\"id\":" id ",\"method\":\"tools/call\",\"params\":{\"name\":\"document_list\",\"arguments\":{}," TASKS_META "}}"
#define TASK_CALL(method, id) "{\"jsonrpc\":\"2.0\",\"id\":" id ",\"method\":\"" method "\",\"params\":{\"taskId\":\"task-001\"," TASKS_META "}}"

int main(void)
{
    mcp_json_buffer response = {0};
    CHECK(mcp_json_buffer_init(&response, 256U, MCP_JSON_MAX_BYTES));
    fake_async fake = {0};
    mcp_protocol_backend sync = {sync_tool, NULL, NULL};
    mcp_protocol_async_backend async = {
        &fake, true, fake_call, fake_get, fake_cancel, fake_cancel_request
    };
    mcp_protocol_session modern = {0};

    CHECK(modern_discover(&modern, &sync, &async, &response));
    CHECK(strstr(response.data, "io.modelcontextprotocol/tasks") != NULL);
    async.get_task = NULL;
    CHECK(modern_discover(&modern, &sync, &async, &response));
    CHECK(strstr(response.data, "io.modelcontextprotocol/tasks") == NULL);
    async.get_task = fake_get;
    mcp_protocol_session_reset(&modern);
    CHECK(modern_discover(&modern, &sync, &async, &response));

    fake.tool_disposition = MCP_PROTOCOL_ASYNC_RESULT;
    fake.tool_payload = task_working;
    CHECK(call(&modern, &sync, &async, TOOL_CALL("2"), &response));
    CHECK(fake.last_supports_tasks);
    CHECK(strstr(response.data, "\"resultType\":\"task\"") != NULL);
    CHECK(fake.durable);
    CHECK(call(&modern, &sync, &async,
        "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tasks/get\","
        "\"params\":{\"taskId\":\"task-001\","
        "\"_meta\":{\"io.modelcontextprotocol/protocolVersion\":\"2026-07-28\"}}}",
        &response));
    CHECK(strstr(response.data, "-32021") != NULL);
    CHECK(call(&modern, &sync, &async, TASK_CALL("tasks/get", "4"), &response));
    CHECK(strstr(response.data, "\"status\":\"working\"") != NULL);
    fake.get_mode = "completed";
    CHECK(call(&modern, &sync, &async, TASK_CALL("tasks/get", "5"), &response));
    CHECK(strstr(response.data, "\"resultType\":\"complete\"") != NULL &&
          strstr(response.data, "\"isError\":false") != NULL);
    fake.get_mode = "isError";
    CHECK(call(&modern, &sync, &async, TASK_CALL("tasks/get", "6"), &response));
    CHECK(strstr(response.data, "\"status\":\"completed\"") != NULL &&
          strstr(response.data, "\"isError\":true") != NULL);
    fake.get_mode = "failed";
    CHECK(call(&modern, &sync, &async, TASK_CALL("tasks/get", "7"), &response));
    CHECK(strstr(response.data, "\"status\":\"failed\"") != NULL &&
          strstr(response.data, "\"error\":{\"code\":-32024") != NULL);
    fake.get_mode = "failed_missing";
    CHECK(call(&modern, &sync, &async, TASK_CALL("tasks/get", "18"), &response));
    CHECK(strstr(response.data, "invalid JSON") != NULL);
    fake.get_mode = "empty";
    CHECK(call(&modern, &sync, &async, TASK_CALL("tasks/get", "19"), &response));
    CHECK(strstr(response.data, "invalid JSON") != NULL);
    fake.get_mode = "wrong";
    CHECK(call(&modern, &sync, &async, TASK_CALL("tasks/get", "20"), &response));
    CHECK(strstr(response.data, "invalid JSON") != NULL);
    fake.get_mode = "cancelled";
    CHECK(call(&modern, &sync, &async, TASK_CALL("tasks/get", "8"), &response));
    CHECK(strstr(response.data, "\"status\":\"cancelled\"") != NULL);
    CHECK(call(&modern, &sync, &async, TASK_CALL("tasks/cancel", "9"), &response));
    CHECK(fake.canceled && strstr(response.data, "\"resultType\":\"complete\"") != NULL &&
          strstr(response.data, "taskId") == NULL);
    fake.get_mode = "cancelled";
    CHECK(call(&modern, &sync, &async, TASK_CALL("tasks/get", "15"), &response));
    CHECK(strstr(response.data, "\"status\":\"cancelled\"") != NULL);
    const char *unknown_task = "{\"jsonrpc\":\"2.0\",\"id\":16,"
        "\"method\":\"tasks/get\",\"params\":{\"taskId\":\"unknown\"," TASKS_META "}}";
    CHECK(call(&modern, &sync, &async, unknown_task, &response));
    CHECK(strstr(response.data, "task unavailable") != NULL);
    fake.foreign = true;
    CHECK(call(&modern, &sync, &async, TASK_CALL("tasks/get", "10"), &response));
    CHECK(strstr(response.data, "task unavailable") != NULL);
    fake.foreign = false;
    fake.expired = true;
    CHECK(call(&modern, &sync, &async, TASK_CALL("tasks/cancel", "11"), &response));
    CHECK(strstr(response.data, "task unavailable") != NULL);
    fake.expired = false;
    fake.cancel_payload = "{}";
    CHECK(call(&modern, &sync, &async, TASK_CALL("tasks/cancel", "21"), &response));
    CHECK(strstr(response.data, "invalid JSON") != NULL);
    fake.cancel_payload = NULL;

    fake.tool_payload = "{\"resultType\":\"task\",\"taskId\":\"x\"}";
    CHECK(call(&modern, &sync, &async, TOOL_CALL("12"), &response));
    CHECK(strstr(response.data, "invalid JSON") != NULL);
    fake.tool_payload = "{\"resultType\":\"task\",\"taskId\":\"task-002\","
        "\"status\":\"working\",\"createdAt\":\"2026-10-14T00:00:00Z\","
        "\"lastUpdatedAt\":\"2026-10-14T00:00:00Z\","
        "\"ttlMs\":1000,\"pollIntervalMs\":10}";
    const char *without_tasks = "{\"jsonrpc\":\"2.0\",\"id\":13,\"method\":\"tools/call\","
        "\"params\":{\"name\":\"document_list\",\"arguments\":{},"
        "\"_meta\":{\"io.modelcontextprotocol/protocolVersion\":\"2026-07-28\"}}}";
    CHECK(call(&modern, &sync, &async, without_tasks, &response));
    CHECK(strstr(response.data, "-32021") != NULL &&
          strstr(response.data, "requiredCapabilities") != NULL);

    const char *bad_tasks_object = "{\"jsonrpc\":\"2.0\",\"id\":22,"
        "\"method\":\"tools/call\",\"params\":{\"name\":\"document_list\","
        "\"arguments\":{},\"_meta\":{\"io.modelcontextprotocol/protocolVersion\":"
        "\"2026-07-28\",\"io.modelcontextprotocol/clientCapabilities\":{"
        "\"extensions\":{\"io.modelcontextprotocol/tasks\":false}}}}}";
    CHECK(call(&modern, &sync, &async, bad_tasks_object, &response));
    CHECK(strstr(response.data, "-32021") != NULL);
    async.get_task = NULL;
    unsigned no_tasks_before = fake.no_tasks_calls;
    CHECK(call(&modern, &sync, &async, TOOL_CALL("23"), &response));
    CHECK(strstr(response.data, "-32021") != NULL);
    CHECK(!fake.last_supports_tasks && fake.no_tasks_calls == no_tasks_before + 1U);
    async.get_task = fake_get;

    fake.tool_disposition = MCP_PROTOCOL_ASYNC_RESULT;
    fake.tool_payload = "{\"content\":[],\"_meta\":{\"trace\":\"kept\"}}";
    CHECK(call(&modern, &sync, &async, TOOL_CALL("24"), &response));
    CHECK(strstr(response.data, "\"result\":{\"content\":[]") != NULL &&
          strstr(response.data, "\"trace\":\"kept\"") != NULL &&
          strstr(response.data, "io.modelcontextprotocol/serverInfo") != NULL);

    size_t large_length = (size_t)MCP_JSON_MAX_BYTES - 256U;
    char *large_payload = malloc(large_length + 1U);
    CHECK(large_payload != NULL);
    const char large_prefix[] = "{\"content\":[],\"pad\":\"";
    const char large_suffix[] = "\"}";
    size_t large_prefix_length = sizeof(large_prefix) - 1U;
    size_t large_suffix_length = sizeof(large_suffix) - 1U;
    memcpy(large_payload, large_prefix, large_prefix_length);
    memset(large_payload + large_prefix_length, 'x', large_length -
        large_prefix_length - large_suffix_length);
    memcpy(large_payload + large_length - large_suffix_length, large_suffix,
        large_suffix_length);
    large_payload[large_length] = '\0';
    fake.tool_payload = large_payload;
    CHECK(call(&modern, &sync, &async, TOOL_CALL("17"), &response));
    CHECK(strstr(response.data, "exceeds the response limit") != NULL);
    free(large_payload);

    fake.tool_disposition = MCP_PROTOCOL_ASYNC_UNHANDLED;
    CHECK(call(&modern, &sync, &async, without_tasks, &response));
    CHECK(strstr(response.data, "sync") != NULL);
    fake.tool_disposition = MCP_PROTOCOL_ASYNC_DEFERRED;
    CHECK(call(&modern, &sync, &async, TOOL_CALL("14"), &response));
    CHECK(strstr(response.data, "deferred response is unavailable") != NULL);

    mcp_protocol_session legacy = {0};
    async.tasks_available = false;
    fake.tool_disposition = MCP_PROTOCOL_ASYNC_DEFERRED;
    const char *initialize = "{\"jsonrpc\":\"2.0\",\"id\":\"i\","
        "\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-11-25\","
        "\"capabilities\":{},\"clientInfo\":{\"name\":\"test\",\"version\":\"1\"}}}";
    CHECK(call(&legacy, &sync, &async, initialize, &response));
    CHECK(strstr(response.data, "io.modelcontextprotocol/tasks") == NULL);
    CHECK(call(&legacy, &sync, &async,
        "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}",
        &response));
    CHECK(call(&legacy, &sync, &async,
        "{\"jsonrpc\":\"2.0\",\"id\":\"legacy-1\",\"method\":\"tools/call\","
        "\"params\":{\"name\":\"document_list\",\"arguments\":{}}}", &response));
    CHECK(response.length == 0U);
    CHECK(call(&legacy, &sync, &async,
        "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/cancelled\","
        "\"params\":{\"requestId\":\"legacy-1\"}}", &response));
    CHECK(response.length == 0U && fake.cancel_request_count == 1U &&
          strcmp(fake.cancel_target, "\"legacy-1\"") == 0);

    int64_t prior = -1;
    mcp_json_buffer progress = {0};
    CHECK(mcp_json_buffer_init(&progress, 128U, MCP_JSON_MAX_BYTES));
    CHECK(mcp_protocol_async_legacy_progress("\"p\"", 3U, 250U, &prior,
        "quarter \" done", &progress));
    CHECK(prior == 250 && strstr(progress.data, "\"total\":1000") != NULL &&
          strstr(progress.data, "quarter \\\" done") != NULL);
    CHECK(!mcp_protocol_async_legacy_progress("1.5", 3U, 500U, &prior,
        "invalid token", &progress) && prior == 250);
    CHECK(!mcp_protocol_async_legacy_progress("\"p\"", 3U, 250U, &prior,
        "non-increasing", &progress) && prior == 250);
    CHECK(!mcp_protocol_async_legacy_progress("[]", 2U, 300U, &prior,
        "array", &progress) && prior == 250);
    CHECK(!mcp_protocol_async_legacy_progress("1", 1U, 1001U, &prior,
        "range", &progress) && prior == 250);
    char long_message[129];
    memset(long_message, 'm', sizeof(long_message) - 1U);
    long_message[sizeof(long_message) - 1U] = '\0';
    CHECK(!mcp_protocol_async_legacy_progress("\"p\"", 3U, 300U, &prior,
        long_message, &progress) && prior == 250);
    char oversized_token[1025];
    memset(oversized_token, '1', sizeof(oversized_token));
    CHECK(!mcp_protocol_async_legacy_progress(oversized_token,
        sizeof(oversized_token), 300U, &prior, "over limit", &progress) &&
        prior == 250);
    bool is_task = false;
    const char *nul_task_id = "{\"resultType\":\"task\",\"taskId\":\"a\\u0000b\","
        "\"status\":\"working\",\"createdAt\":\"2026-10-14T00:00:00Z\","
        "\"lastUpdatedAt\":\"2026-10-14T00:00:00Z\","
        "\"ttlMs\":100,\"pollIntervalMs\":10}";
    CHECK(!mcp_protocol_async_validate_payload(nul_task_id, strlen(nul_task_id),
        MCP_PROTOCOL_ASYNC_PAYLOAD_TOOL_RESULT, &is_task));
    const char *missing_iso = "{\"resultType\":\"task\",\"taskId\":\"task-x\","
        "\"status\":\"working\",\"createdAt\":\"now\","
        "\"lastUpdatedAt\":\"now\",\"ttlMs\":100}";
    CHECK(!mcp_protocol_async_validate_payload(missing_iso, strlen(missing_iso),
        MCP_PROTOCOL_ASYNC_PAYLOAD_TOOL_RESULT, &is_task));
    const char *optional_poll = "{\"resultType\":\"task\",\"taskId\":\"task-x\","
        "\"status\":\"working\",\"createdAt\":\"2026-10-14T00:00:00Z\","
        "\"lastUpdatedAt\":\"2026-10-14T00:00:00Z\",\"ttlMs\":900000}";
    CHECK(mcp_protocol_async_validate_payload(optional_poll, strlen(optional_poll),
        MCP_PROTOCOL_ASYNC_PAYLOAD_TOOL_RESULT, &is_task) && is_task);
    const char *terminal_creation[] = {
        "{\"resultType\":\"task\",\"taskId\":\"seed-working\",\"status\":\"working\",\"createdAt\":\"2026-10-14T00:00:00Z\",\"lastUpdatedAt\":\"2026-10-14T00:00:00Z\",\"ttlMs\":10}",
        "{\"resultType\":\"task\",\"taskId\":\"seed-completed\",\"status\":\"completed\",\"createdAt\":\"2026-10-14T00:00:00Z\",\"lastUpdatedAt\":\"2026-10-14T00:00:00Z\",\"ttlMs\":10}",
        "{\"resultType\":\"task\",\"taskId\":\"seed-failed\",\"status\":\"failed\",\"createdAt\":\"2026-10-14T00:00:00Z\",\"lastUpdatedAt\":\"2026-10-14T00:00:00Z\",\"ttlMs\":10}",
        "{\"resultType\":\"task\",\"taskId\":\"seed-cancelled\",\"status\":\"cancelled\",\"createdAt\":\"2026-10-14T00:00:00Z\",\"lastUpdatedAt\":\"2026-10-14T00:00:00Z\",\"ttlMs\":10}"
    };
    for (size_t index = 0U;
         index < sizeof(terminal_creation) / sizeof(terminal_creation[0]); ++index) {
        is_task = false;
        CHECK(mcp_protocol_async_validate_payload(terminal_creation[index],
            strlen(terminal_creation[index]), MCP_PROTOCOL_ASYNC_PAYLOAD_TOOL_RESULT,
            &is_task) && is_task);
    }
    const char *malformed_content[] = {
        "{\"content\":[null]}", "{\"content\":[{}]}",
        "{\"content\":[7]}", "{\"content\":[{\"type\":\"video\"}]}",
        "{\"content\":[{\"type\":\"text\"}]}",
        "{\"content\":[{\"type\":\"image\",\"data\":\"x\"}]}",
        "{\"content\":[{\"type\":\"audio\",\"mimeType\":\"audio/wav\"}]}",
        "{\"content\":[{\"type\":\"resource\",\"resource\":{\"text\":\"x\"}}]}",
        "{\"content\":[{\"type\":\"resource_link\",\"uri\":\"vexel://x\"}]}"
    };
    for (size_t index = 0U;
         index < sizeof(malformed_content) / sizeof(malformed_content[0]); ++index)
        CHECK(!mcp_protocol_async_validate_payload(malformed_content[index],
            strlen(malformed_content[index]), MCP_PROTOCOL_ASYNC_PAYLOAD_TOOL_RESULT,
            &is_task));
    fake.tool_disposition = MCP_PROTOCOL_ASYNC_RESULT;
    for (size_t index = 0U;
         index < sizeof(malformed_content) / sizeof(malformed_content[0]); ++index) {
        fake.tool_payload = malformed_content[index];
        CHECK(call(&modern, &sync, &async,
            TOOL_CALL("\"malformed-content\""), &response));
        CHECK(strstr(response.data, "invalid JSON") != NULL);
    }
    const char *valid_content = "{\"content\":[{\"type\":\"text\",\"text\":\"ok\"},"
        "{\"type\":\"image\",\"data\":\"YQ==\",\"mimeType\":\"image/png\"},"
        "{\"type\":\"audio\",\"data\":\"YQ==\",\"mimeType\":\"audio/wav\"},"
        "{\"type\":\"resource\",\"resource\":{\"uri\":\"vexel://x\",\"text\":\"x\"}},"
        "{\"type\":\"resource_link\",\"uri\":\"vexel://x\",\"name\":\"x\"}]}";
    CHECK(mcp_protocol_async_validate_payload(valid_content, strlen(valid_content),
        MCP_PROTOCOL_ASYNC_PAYLOAD_TOOL_RESULT, &is_task) && !is_task);
    const char *oversized_ttl = "{\"resultType\":\"task\",\"taskId\":\"task-x\","
        "\"status\":\"working\",\"createdAt\":\"2026-10-14T00:00:00Z\","
        "\"lastUpdatedAt\":\"2026-10-14T00:00:00Z\",\"ttlMs\":900001}";
    CHECK(!mcp_protocol_async_validate_payload(oversized_ttl, strlen(oversized_ttl),
        MCP_PROTOCOL_ASYNC_PAYLOAD_TOOL_RESULT, &is_task));
    char *oversized_payload = calloc((size_t)MCP_JSON_MAX_BYTES + 1U, 1U);
    CHECK(oversized_payload != NULL);
    oversized_payload[0] = '{';
    oversized_payload[MCP_JSON_MAX_BYTES] = '}';
    CHECK(!mcp_protocol_async_validate_payload(oversized_payload,
        (size_t)MCP_JSON_MAX_BYTES + 1U,
        MCP_PROTOCOL_ASYNC_PAYLOAD_TASK_GET, &is_task));
    free(oversized_payload);
    CHECK(mcp_protocol_async_legacy_progress("17", 2U, 260U, &prior,
        "integer token", &progress) && prior == 260 &&
        strstr(progress.data, "\"progressToken\":17") != NULL);

    mcp_json_buffer_destroy(&progress);
    mcp_json_buffer_destroy(&response);
    return EXIT_SUCCESS;
}
