#define _DARWIN_C_SOURCE
#define _POSIX_C_SOURCE 200809L

#include "mcp_broker.h"
#include "mcp_json.h"

#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    return 1; } } while (0)

static bool response_init(mcp_tool_response *response)
{
    memset(response, 0, sizeof(*response));
    return mcp_json_buffer_init(&response->structured_json, 128U,
            MCP_JSON_MAX_BYTES) &&
        mcp_json_buffer_init(&response->content_json, 128U, MCP_JSON_MAX_BYTES);
}

static void response_destroy(mcp_tool_response *response)
{
    mcp_json_buffer_destroy(&response->structured_json);
    mcp_json_buffer_destroy(&response->content_json);
    *response = (mcp_tool_response){0};
}

static bool call_tool(const mcp_protocol_backend *backend, const char *name,
                      const char *arguments, mcp_tool_response *out)
{
    mcp_json_doc request = {0};
    size_t length = strlen(arguments);
    bool okay = response_init(out) &&
        mcp_json_parse((const uint8_t *)arguments, length, &request);
    if (okay) okay = backend->call_tool(backend->context, &request,
        request.root, name, out);
    mcp_json_doc_destroy(&request);
    return okay;
}

static bool response_string(const mcp_json_buffer *response, const char *field,
                            char *out, size_t capacity)
{
    mcp_json_doc parsed = {0};
    bool okay = response != NULL && response->data != NULL && capacity > 0U &&
        mcp_json_parse((const uint8_t *)response->data, response->length, &parsed);
    size_t token = okay ? mcp_json_object_get(&parsed, parsed.root, field) :
        MCP_JSON_NO_TOKEN;
    size_t length = 0U;
    okay = okay && mcp_json_string_copy(&parsed, token, out, capacity - 1U,
        &length) && length < capacity;
    if (okay) out[length] = '\0';
    mcp_json_doc_destroy(&parsed);
    return okay;
}

static bool tree_root_id(const mcp_json_buffer *response, char *out,
                         size_t capacity)
{
    mcp_json_doc parsed = {0};
    bool okay = response != NULL && mcp_json_parse(
        (const uint8_t *)response->data, response->length, &parsed);
    size_t elements = okay ? mcp_json_object_get(&parsed, parsed.root, "elements") :
        MCP_JSON_NO_TOKEN;
    size_t root = okay ? mcp_json_array_get(&parsed, elements, 0U) :
        MCP_JSON_NO_TOKEN;
    size_t id = okay ? mcp_json_object_get(&parsed, root, "elementId") :
        MCP_JSON_NO_TOKEN;
    size_t length = 0U;
    okay = okay && capacity > 0U && mcp_json_string_copy(&parsed, id, out,
        capacity - 1U, &length) && length < capacity;
    if (okay) out[length] = '\0';
    mcp_json_doc_destroy(&parsed);
    return okay;
}

static bool expect_error(const mcp_protocol_backend *backend, const char *name,
                         const char *arguments, const char *expected_code)
{
    mcp_tool_response response = {0};
    if (!call_tool(backend, name, arguments, &response)) return false;
    mcp_json_doc parsed = {0};
    bool okay = response.is_error && mcp_json_parse(
        (const uint8_t *)response.structured_json.data,
        response.structured_json.length, &parsed);
    size_t error = okay ? mcp_json_object_get(&parsed, parsed.root, "error") :
        MCP_JSON_NO_TOKEN;
    size_t code = okay ? mcp_json_object_get(&parsed, error, "code") :
        MCP_JSON_NO_TOKEN;
    okay = okay && mcp_json_string_equals(&parsed, code, expected_code);
    mcp_json_doc_destroy(&parsed);
    response_destroy(&response);
    return okay;
}

static bool write_file(const char *path, const char *bytes)
{
    int descriptor = open(path, O_WRONLY | O_CREAT | O_EXCL, 0600);
    if (descriptor < 0) return false;
    size_t length = strlen(bytes), offset = 0U;
    while (offset < length) {
        ssize_t amount = write(descriptor, bytes + offset, length - offset);
        if (amount < 0 && errno == EINTR) continue;
        if (amount <= 0) { (void)close(descriptor); return false; }
        offset += (size_t)amount;
    }
    return close(descriptor) == 0;
}

static bool file_contains(const char *path, const char *needle)
{
    FILE *file = fopen(path, "rb");
    if (file == NULL) return false;
    char content[65536];
    size_t length = fread(content, 1U, sizeof(content) - 1U, file);
    bool okay = !ferror(file);
    if (fclose(file) != 0) okay = false;
    content[length] = '\0';
    return okay && strstr(content, needle) != NULL;
}

int main(void)
{
    char directory_template[] = "/tmp/vexel-mcp-features-XXXXXX";
    char *directory = mkdtemp(directory_template);
    CHECK(directory != NULL);
    CHECK(setenv("VEXEL_MCP_ALLOWED_DIRS", directory, 1) == 0);
    char current_path[1024], other_path[1024], save_path[1024], outside_path[1024];
    int current_length = snprintf(current_path, sizeof(current_path), "%s/current.svg", directory);
    int other_length = snprintf(other_path, sizeof(other_path), "%s/other.svg", directory);
    int save_length = snprintf(save_path, sizeof(save_path), "%s/new.svg", directory);
    int outside_length = snprintf(outside_path, sizeof(outside_path), "%s-outside.svg", directory);
    CHECK(current_length > 0 && (size_t)current_length < sizeof(current_path));
    CHECK(other_length > 0 && (size_t)other_length < sizeof(other_path));
    CHECK(save_length > 0 && (size_t)save_length < sizeof(save_path));
    CHECK(outside_length > 0 && (size_t)outside_length < sizeof(outside_path));
    static const char source[] =
        "<svg width='100' height='100'><text id='media' x='1' y='20'>Media</text>"
        "<image x='50' y='50' width='10' height='10' "
        "href='data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJ"
        "AAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg=='/></svg>";
    static const char other_source[] = "<svg width='10' height='10'><rect/></svg>";
    CHECK(write_file(current_path, source) && write_file(other_path, other_source));

    mcp_broker *broker = NULL;
    CHECK(mcp_broker_create(NULL, &broker));
    mcp_protocol_backend backend = mcp_broker_protocol_backend(broker);
    char document_args[1200];
    int amount = snprintf(document_args, sizeof(document_args),
        "{\"path\":\"%s\"}", current_path);
    CHECK(amount > 0 && (size_t)amount < sizeof(document_args));
    mcp_tool_response response = {0};
    CHECK(call_tool(&backend, "document_open", document_args, &response));
    CHECK(!response.is_error);
    char document_id[64];
    CHECK(response_string(&response.structured_json, "documentId",
        document_id, sizeof(document_id)));
    response_destroy(&response);

    char arguments[2048];
    amount = snprintf(arguments, sizeof(arguments),
        "{\"documentId\":\"%s\",\"limit\":1}", document_id);
    CHECK(amount > 0 && (size_t)amount < sizeof(arguments));
    CHECK(call_tool(&backend, "tree_get", arguments, &response));
    char root_id[64];
    CHECK(tree_root_id(&response.structured_json, root_id, sizeof(root_id)));
    response_destroy(&response);

    amount = snprintf(arguments, sizeof(arguments),
        "{\"documentId\":\"%s\",\"parentId\":\"%s\","
        "\"template\":\"text\",\"attributes\":{\"id\":\"new-text\"}}",
        document_id, root_id);
    CHECK(amount > 0 && (size_t)amount < sizeof(arguments));
    CHECK(call_tool(&backend, "element_add", arguments, &response));
    CHECK(!response.is_error && response_string(&response.structured_json,
        "elementId", arguments, sizeof(arguments)) &&
        strcmp(arguments, "new-text") == 0);
    response_destroy(&response);

    amount = snprintf(arguments, sizeof(arguments),
        "{\"documentId\":\"%s\",\"elementId\":\"new-text\","
        "\"text\":\"MCP <&>\"}", document_id);
    CHECK(amount > 0 && (size_t)amount < sizeof(arguments));
    CHECK(call_tool(&backend, "text_set", arguments, &response) && !response.is_error);
    response_destroy(&response);

    amount = snprintf(arguments, sizeof(arguments),
        "{\"documentId\":\"%s\",\"parentId\":\"%s\","
        "\"xml\":\"<g id='fragment'><text id='fragment-text'>Fragment</text></g>\"}",
        document_id, root_id);
    CHECK(amount > 0 && (size_t)amount < sizeof(arguments));
    CHECK(call_tool(&backend, "element_add_fragment", arguments, &response) &&
        !response.is_error);
    response_destroy(&response);
    amount = snprintf(arguments, sizeof(arguments),
        "{\"documentId\":\"%s\",\"parentId\":\"%s\","
        "\"xml\":\"<g><text></g>\"}", document_id, root_id);
    CHECK(amount > 0 && (size_t)amount < sizeof(arguments));
    CHECK(expect_error(&backend, "element_add_fragment", arguments,
        "malformed_xml"));

    amount = snprintf(arguments, sizeof(arguments),
        "{\"documentId\":\"%s\",\"region\":{\"x\":0,\"y\":0,"
        "\"width\":100,\"height\":100}}", document_id);
    CHECK(amount > 0 && (size_t)amount < sizeof(arguments));
    CHECK(call_tool(&backend, "render_png", arguments, &response));
    CHECK(!response.is_error && response.content_json.data != NULL &&
        strstr(response.content_json.data, "\"type\":\"image\"") != NULL);
    response_destroy(&response);

    amount = snprintf(arguments, sizeof(arguments),
        "{\"documentId\":\"%s\",\"path\":\"%s\"}",
        document_id, save_path);
    CHECK(amount > 0 && (size_t)amount < sizeof(arguments));
    CHECK(call_tool(&backend, "save_as", arguments, &response) && !response.is_error);
    response_destroy(&response);
    CHECK(file_contains(save_path, "MCP &lt;&amp;&gt;"));
    amount = snprintf(arguments, sizeof(arguments),
        "{\"documentId\":\"%s\",\"path\":\"%s\"}",
        document_id, outside_path);
    CHECK(amount > 0 && (size_t)amount < sizeof(arguments));
    CHECK(expect_error(&backend, "save_as", arguments, "path_not_granted"));

    amount = snprintf(document_args, sizeof(document_args),
        "{\"path\":\"%s\"}", other_path);
    CHECK(amount > 0 && (size_t)amount < sizeof(document_args));
    CHECK(call_tool(&backend, "document_open", document_args, &response));
    CHECK(!response.is_error);
    char other_id[64];
    CHECK(response_string(&response.structured_json, "documentId",
        other_id, sizeof(other_id)) && strcmp(other_id, document_id) != 0);
    response_destroy(&response);
    amount = snprintf(arguments, sizeof(arguments),
        "{\"documentId\":\"%s\"}", document_id);
    CHECK(amount > 0 && (size_t)amount < sizeof(arguments));
    CHECK(call_tool(&backend, "document_close", arguments, &response) &&
        !response.is_error && strstr(response.structured_json.data, "\"closed\":true") != NULL);
    response_destroy(&response);
    amount = snprintf(arguments, sizeof(arguments),
        "{\"documentId\":\"%s\"}", other_id);
    CHECK(amount > 0 && (size_t)amount < sizeof(arguments));
    CHECK(call_tool(&backend, "document_info", arguments, &response) &&
        !response.is_error);
    response_destroy(&response);
    CHECK(call_tool(&backend, "document_close", arguments, &response) &&
        !response.is_error);
    response_destroy(&response);

    mcp_broker_destroy(broker);
    CHECK(unlink(save_path) == 0 && unlink(current_path) == 0 && unlink(other_path) == 0);
    CHECK(rmdir(directory) == 0);
    (void)puts("MCP feature tests passed: media render, text/fragment edits, atomic Save As, and non-active close");
    return 0;
}
