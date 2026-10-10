#define _DARWIN_C_SOURCE
#define _POSIX_C_SOURCE 200809L

#include "mcp_broker.h"
#include "mcp_json.h"
#include "platform/media/platform_image.h"

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

static uint64_t buffer_hash(const mcp_json_buffer *buffer)
{
    uint64_t value = UINT64_C(1469598103934665603);
    if (buffer == NULL || buffer->data == NULL) return 0U;
    for (size_t index = 0U; index < buffer->length; index++) {
        value ^= (uint8_t)buffer->data[index];
        value *= UINT64_C(1099511628211);
    }
    return value;
}

static bool gif_has_frames_and_delay(const char *path, size_t expected_frames,
                                     uint16_t expected_delay_centiseconds)
{
    FILE *file = fopen(path, "rb");
    if (file == NULL || fseek(file, 0L, SEEK_END) != 0) {
        if (file != NULL) (void)fclose(file);
        return false;
    }
    long raw_length = ftell(file);
    bool okay = raw_length >= 13L && raw_length <= 2L * 1024L * 1024L &&
        fseek(file, 0L, SEEK_SET) == 0;
    size_t length = okay ? (size_t)raw_length : 0U;
    uint8_t *bytes = okay ? malloc(length) : NULL;
    if (bytes == NULL) okay = false;
    if (okay) okay = fread(bytes, 1U, length, file) == length;
    if (fclose(file) != 0) okay = false;
    if (!okay) { free(bytes); return false; }

    bool signature = memcmp(bytes, "GIF89a", 6U) == 0 ||
        memcmp(bytes, "GIF87a", 6U) == 0;
    bool loop_extension = false;
    size_t frame_count = 0U;
    for (size_t index = 0U; index < length; index++) {
        if (index + 11U <= length &&
            memcmp(bytes + index, "NETSCAPE2.0", 11U) == 0) loop_extension = true;
        if (index + 8U >= length) break;
        if (bytes[index] != 0x21U || bytes[index + 1U] != 0xf9U ||
            bytes[index + 2U] != 4U || bytes[index + 7U] != 0U) continue;
        uint16_t delay = (uint16_t)bytes[index + 4U] |
            (uint16_t)((uint16_t)bytes[index + 5U] << 8U);
        if (delay != expected_delay_centiseconds) { okay = false; break; }
        frame_count++;
    }
    platform_image decoded = PLATFORM_IMAGE_INIT;
    platform_media_status decode_status = platform_image_decode(bytes, length, &decoded);
    platform_image_destroy(&decoded);
    free(bytes);
    return signature && loop_extension && okay && frame_count == expected_frames &&
        decode_status == PLATFORM_MEDIA_ANIMATED;
}

int main(void)
{
    char directory_template[] = "/tmp/vexel-mcp-features-XXXXXX";
    char *directory = mkdtemp(directory_template);
    CHECK(directory != NULL);
    CHECK(setenv("VEXEL_MCP_ALLOWED_DIRS", directory, 1) == 0);
    char current_path[1024], other_path[1024], save_path[1024], outside_path[1024];
    char gif_path[1024], frame_zero[1024], frame_one[1024];
    int current_length = snprintf(current_path, sizeof(current_path), "%s/current.svg", directory);
    int other_length = snprintf(other_path, sizeof(other_path), "%s/other.svg", directory);
    int save_length = snprintf(save_path, sizeof(save_path), "%s/new.svg", directory);
    int outside_length = snprintf(outside_path, sizeof(outside_path), "%s-outside.svg", directory);
    int gif_length = snprintf(gif_path, sizeof(gif_path), "%s/animation.gif", directory);
    int frame_zero_length = snprintf(frame_zero, sizeof(frame_zero), "%s/frame-0000.png", directory);
    int frame_one_length = snprintf(frame_one, sizeof(frame_one), "%s/frame-0001.png", directory);
    CHECK(current_length > 0 && (size_t)current_length < sizeof(current_path));
    CHECK(other_length > 0 && (size_t)other_length < sizeof(other_path));
    CHECK(save_length > 0 && (size_t)save_length < sizeof(save_path));
    CHECK(outside_length > 0 && (size_t)outside_length < sizeof(outside_path));
    CHECK(gif_length > 0 && (size_t)gif_length < sizeof(gif_path));
    CHECK(frame_zero_length > 0 && (size_t)frame_zero_length < sizeof(frame_zero));
    CHECK(frame_one_length > 0 && (size_t)frame_one_length < sizeof(frame_one));
    static const char source[] =
        "<svg width='100' height='100'><text id='media' x='1' y='20'>Media</text>"
        "<rect id='animated' x='2' y='3' width='20' height='10' fill='red'>"
        "<animate id='grow' attributeName='width' from='20' to='40' dur='2s'/>"
        "</rect>"
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

    amount = snprintf(arguments, sizeof(arguments), "{\"documentId\":\"%s\"}", document_id);
    CHECK(amount > 0 && (size_t)amount < sizeof(arguments));
    CHECK(call_tool(&backend, "animation_list", arguments, &response));
    CHECK(!response.is_error && strstr(response.structured_json.data,
        "\"animationId\":\"grow\"") != NULL &&
        strstr(response.structured_json.data, "\"targetId\":\"animated\"") != NULL &&
        strstr(response.structured_json.data, "\"sourceSpan\"") != NULL);
    response_destroy(&response);
    amount = snprintf(arguments, sizeof(arguments),
        "{\"documentId\":\"%s\",\"animationId\":\"grow\","
        "\"name\":\"to\",\"value\":\"50\"}", document_id);
    CHECK(amount > 0 && (size_t)amount < sizeof(arguments));
    CHECK(call_tool(&backend, "animation_set", arguments, &response) && !response.is_error);
    response_destroy(&response);
    amount = snprintf(arguments, sizeof(arguments),
        "{\"documentId\":\"%s\",\"targetId\":\"animated\","
        "\"xml\":\"<animate id='fade' attributeName='opacity' from='1' to='0' dur='1s'/>\"}",
        document_id);
    CHECK(amount > 0 && (size_t)amount < sizeof(arguments));
    CHECK(call_tool(&backend, "animation_add", arguments, &response) && !response.is_error);
    response_destroy(&response);
    amount = snprintf(arguments, sizeof(arguments),
        "{\"documentId\":\"%s\",\"animationId\":\"fade\"}", document_id);
    CHECK(amount > 0 && (size_t)amount < sizeof(arguments));
    CHECK(call_tool(&backend, "animation_remove", arguments, &response) && !response.is_error);
    response_destroy(&response);
    amount = snprintf(arguments, sizeof(arguments),
        "{\"documentId\":\"%s\",\"animationId\":\"fade\"}", document_id);
    CHECK(amount > 0 && (size_t)amount < sizeof(arguments));
    CHECK(expect_error(&backend, "animation_remove", arguments, "animation_not_found"));

    amount = snprintf(arguments, sizeof(arguments),
        "{\"documentId\":\"%s\",\"parentId\":\"%s\","
        "\"xml\":\"<g><text></g>\"}", document_id, root_id);
    CHECK(amount > 0 && (size_t)amount < sizeof(arguments));
    CHECK(expect_error(&backend, "element_add_fragment", arguments,
        "malformed_xml"));

    amount = snprintf(arguments, sizeof(arguments),
        "{\"documentId\":\"%s\",\"region\":{\"x\":0,\"y\":0,"
        "\"width\":100,\"height\":100},\"time\":1}", document_id);
    CHECK(amount > 0 && (size_t)amount < sizeof(arguments));
    CHECK(call_tool(&backend, "render_png", arguments, &response));
    CHECK(!response.is_error && response.content_json.data != NULL &&
        strstr(response.content_json.data, "\"type\":\"image\"") != NULL);
    uint64_t timed_render_hash = buffer_hash(&response.content_json);
    response_destroy(&response);
    amount = snprintf(arguments, sizeof(arguments),
        "{\"documentId\":\"%s\",\"region\":{\"x\":0,\"y\":0,"
        "\"width\":100,\"height\":100}}", document_id);
    CHECK(amount > 0 && (size_t)amount < sizeof(arguments));
    CHECK(call_tool(&backend, "render_png", arguments, &response) &&
        !response.is_error && buffer_hash(&response.content_json) != timed_render_hash);
    response_destroy(&response);

    amount = snprintf(arguments, sizeof(arguments),
        "{\"documentId\":\"%s\",\"path\":\"%s\",\"format\":\"frames\","
        "\"fps\":2,\"duration\":1}", document_id, directory);
    CHECK(amount > 0 && (size_t)amount < sizeof(arguments));
    CHECK(call_tool(&backend, "export_animation", arguments, &response) &&
        !response.is_error && strstr(response.structured_json.data, "\"frames\":2") != NULL);
    response_destroy(&response);
    FILE *frame = fopen(frame_zero, "rb");
    CHECK(frame != NULL);
    uint8_t png_signature[8] = {0};
    CHECK(fread(png_signature, 1U, sizeof(png_signature), frame) == sizeof(png_signature));
    CHECK(fclose(frame) == 0);
    static const uint8_t expected_png[8] = {137U, 80U, 78U, 71U, 13U, 10U, 26U, 10U};
    CHECK(memcmp(png_signature, expected_png, sizeof(expected_png)) == 0);
    CHECK(access(frame_one, F_OK) == 0);
    amount = snprintf(arguments, sizeof(arguments),
        "{\"documentId\":\"%s\",\"path\":\"%s\",\"format\":\"frames\","
        "\"fps\":2,\"duration\":1}", document_id, outside_path);
    CHECK(amount > 0 && (size_t)amount < sizeof(arguments));
    CHECK(expect_error(&backend, "export_animation", arguments, "path_not_granted"));
    amount = snprintf(arguments, sizeof(arguments),
        "{\"documentId\":\"%s\",\"path\":\"%s\",\"format\":\"gif\","
        "\"fps\":2,\"duration\":1}", document_id, gif_path);
    CHECK(amount > 0 && (size_t)amount < sizeof(arguments));
    CHECK(call_tool(&backend, "export_animation", arguments, &response) &&
        !response.is_error && strstr(response.structured_json.data, "\"format\":\"gif\"") != NULL);
    response_destroy(&response);
    CHECK(gif_has_frames_and_delay(gif_path, 2U, 50U));

    amount = snprintf(arguments, sizeof(arguments),
        "{\"documentId\":\"%s\",\"path\":\"%s\"}",
        document_id, save_path);
    CHECK(amount > 0 && (size_t)amount < sizeof(arguments));
    CHECK(call_tool(&backend, "save_as", arguments, &response) && !response.is_error);
    response_destroy(&response);
    CHECK(file_contains(save_path, "MCP &lt;&amp;&gt;"));
    amount = snprintf(arguments, sizeof(arguments),
        "{\"documentId\":\"%s\",\"path\":\"%s\",\"format\":\"gif\","
        "\"fps\":2,\"duration\":1}", document_id, save_path);
    CHECK(amount > 0 && (size_t)amount < sizeof(arguments));
    CHECK(expect_error(&backend, "export_animation", arguments, "animation_export_failed"));
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
    CHECK(unlink(save_path) == 0 && unlink(gif_path) == 0 &&
        unlink(frame_zero) == 0 && unlink(frame_one) == 0 &&
        unlink(current_path) == 0 && unlink(other_path) == 0);
    CHECK(rmdir(directory) == 0);
    (void)puts("MCP feature tests passed: animation edits, PNG/GIF export, media render, atomic Save As, and document close");
    return 0;
}
