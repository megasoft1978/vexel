#define _DARWIN_C_SOURCE
#define _POSIX_C_SOURCE 200809L

#include "mcp_broker_internal.h"
#include "project/project_animation_render.h"
#include "project/project_timeline_integration.h"
#include "project/project_bundle_asset_overlay.h"
#include "platform/media/platform_image.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dirent.h>
#include <errno.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    return EXIT_FAILURE; } } while (0)

typedef struct fixture {
    document_edit *edit;
    char path[MCP_BROKER_PATH_CAP];
    project_bundle_view bundle;
    project_bundle_asset_overlay overlay;
    project_timeline_session *session;
    unsigned changed;
    unsigned action_count;
    char last_action[64];
    bool last_action_succeeded;
    unsigned saves;
    uint8_t *saved_timeline;
    size_t saved_timeline_length;
} fixture;

static void host_action_completed(void *context, const char *tool_name,
                                  bool succeeded)
{
    fixture *state = context;
    size_t length = strnlen(tool_name, sizeof(state->last_action) - 1U);
    memcpy(state->last_action, tool_name, length);
    state->last_action[length] = '\0';
    state->last_action_succeeded = succeeded;
    state->action_count++;
}

static document_edit *host_edit(void *context)
{
    return ((fixture *)context)->edit;
}

static const char *host_path(void *context)
{
    return ((fixture *)context)->path;
}

static project_timeline_session *host_session(void *context)
{
    fixture *state = context;
    size_t length = strlen(state->path);
    return length >= 6U && strcmp(state->path + length - 6U, ".vexel") == 0 ?
        state->session : NULL;
}

static const project_bundle_view *host_bundle(void *context)
{
    return &((fixture *)context)->bundle;
}

static project_bundle_result host_add_asset(void *context, base_bytes name,
    base_bytes svg)
{
    return project_bundle_asset_overlay_add_svg(
        &((fixture *)context)->overlay, name, svg);
}

static void host_changed(void *context)
{
    ((fixture *)context)->changed++;
}

static bool host_save(void *context, const char *path, size_t path_length,
                     bool save_as)
{
    fixture *state = context;
    if (path == NULL || path_length == 0U ||
        (save_as && path_length >= sizeof(state->path))) return false;
    base_bytes timeline = project_timeline_session_source(state->session);
    uint8_t *copy = malloc(timeline.len);
    if (copy == NULL) return false;
    memcpy(copy, timeline.ptr, timeline.len);
    free(state->saved_timeline);
    state->saved_timeline = copy;
    state->saved_timeline_length = timeline.len;
    state->saves++;
    if (!project_timeline_session_mark_saved(state->session)) return false;
    if (save_as) {
        memcpy(state->path, path, path_length);
        state->path[path_length] = '\0';
    }
    return true;
}

static bool call_tool(const mcp_protocol_backend *backend, const char *name,
                      const char *args, mcp_tool_response *response)
{
    memset(response, 0, sizeof(*response));
    if (!mcp_json_buffer_init(&response->structured_json, 128U,
            MCP_JSON_MAX_BYTES) ||
        !mcp_json_buffer_init(&response->content_json, 128U,
            MCP_JSON_MAX_BYTES)) return false;
    mcp_json_doc request = {0};
    bool okay = mcp_json_parse((const uint8_t *)args, strlen(args), &request);
    if (okay) okay = backend->call_tool(backend->context, &request,
        request.root, name, response);
    mcp_json_doc_destroy(&request);
    return okay;
}

static bool call_tool_with_structured_limit(
    const mcp_protocol_backend *backend, const char *name, const char *args,
    mcp_tool_response *response, size_t structured_limit)
{
    memset(response, 0, sizeof(*response));
    size_t initial_capacity = structured_limit < 16U ? structured_limit : 16U;
    if (!mcp_json_buffer_init(&response->structured_json, initial_capacity,
            structured_limit) ||
        !mcp_json_buffer_init(&response->content_json, 128U, MCP_JSON_MAX_BYTES)) return false;
    mcp_json_doc request = {0};
    bool okay = mcp_json_parse((const uint8_t *)args, strlen(args), &request);
    if (okay) okay = backend->call_tool(backend->context, &request,
        request.root, name, response);
    mcp_json_doc_destroy(&request);
    return okay;
}

static void response_destroy(mcp_tool_response *response)
{
    mcp_json_buffer_destroy(&response->structured_json);
    mcp_json_buffer_destroy(&response->content_json);
}

static bool read_png(const char *path, platform_image *image)
{
    FILE *file = fopen(path, "rb");
    if (file == NULL || fseek(file, 0L, SEEK_END) != 0) {
        if (file != NULL) (void)fclose(file);
        return false;
    }
    long length = ftell(file);
    if (length <= 0L || fseek(file, 0L, SEEK_SET) != 0) {
        (void)fclose(file);
        return false;
    }
    uint8_t *bytes = malloc((size_t)length);
    bool okay = bytes != NULL && fread(bytes, 1U, (size_t)length, file) ==
        (size_t)length;
    if (fclose(file) != 0) okay = false;
    if (okay) okay = platform_image_decode(bytes, (size_t)length, image) ==
        PLATFORM_MEDIA_OK;
    free(bytes);
    return okay;
}

static bool file_has_prefix(const char *path, const uint8_t *prefix,
                            size_t prefix_length)
{
    FILE *file = fopen(path, "rb");
    if (file == NULL) return false;
    uint8_t actual[8];
    bool okay = prefix_length <= sizeof(actual) &&
        fread(actual, 1U, prefix_length, file) == prefix_length &&
        memcmp(actual, prefix, prefix_length) == 0;
    if (fclose(file) != 0) okay = false;
    return okay;
}

static bool file_matches(const char *path, const uint8_t *expected, size_t length)
{
    int fd = open(path, O_RDONLY | O_CLOEXEC);
    if (fd < 0) return false;
    uint8_t bytes[32];
    ssize_t amount = read(fd, bytes, sizeof(bytes));
    bool okay = amount >= 0 && (size_t)amount == length &&
        length <= sizeof(bytes) && memcmp(bytes, expected, length) == 0;
    return close(fd) == 0 && okay;
}

static size_t new_file_temp_count(const char *directory)
{
    DIR *dir = opendir(directory);
    if (dir == NULL) return SIZE_MAX;
    size_t count = 0U;
    struct dirent *entry;
    while ((entry = readdir(dir)) != NULL)
        if (strncmp(entry->d_name, ".vexel-new-", 11U) == 0) count++;
    (void)closedir(dir);
    return count;
}

int main(void)
{
    static const uint8_t scene[] = "<svg width='2' height='2' viewBox='0 0 2 2'><g id='layer'><rect width='2' height='2' fill='blue'/></g></svg>";
    static const uint8_t manifest[] = "{\"title\":\"MCP timeline\"}";
    static const uint8_t timeline[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"draw\",\"kind\":\"drawing\","
        "\"target\":\"layer\",\"visible\":true,"
        "\"cels\":[{\"tick\":0,\"asset\":\"cel0\"}]}]}";
    static const uint8_t asset_name[] = "cel0";
    static const uint8_t media_type[] = "image/svg+xml";
    static const uint8_t asset_data[] =
        "<svg width='2' height='2' viewBox='0 0 2 2'><path d='M0 0L2 2' stroke='red'/></svg>";
    project_bundle_asset asset = {{asset_name, sizeof(asset_name) - 1U},
        {media_type, sizeof(media_type) - 1U},
        {asset_data, sizeof(asset_data) - 1U}};
    project_bundle_input input = {
        .manifest_json = {manifest, sizeof(manifest) - 1U},
        .scene_svg = {scene, sizeof(scene) - 1U},
        .timeline_json = {timeline, sizeof(timeline) - 1U},
        .assets = &asset,
        .asset_count = 1U
    };
    project_bundle_result encoded_size = project_bundle_encoded_size(&input);
    CHECK(encoded_size.error == PROJECT_BUNDLE_OK);
    uint8_t *bundle_bytes = malloc(encoded_size.encoded_size);
    CHECK(bundle_bytes != NULL && project_bundle_encode(&input,
        (base_mut_bytes){bundle_bytes, encoded_size.encoded_size}).error ==
        PROJECT_BUNDLE_OK);
    char cwd[MCP_BROKER_PATH_CAP], allowed_directory[MCP_BROKER_PATH_CAP];
    CHECK(getcwd(cwd, sizeof(cwd)) != NULL);
    CHECK(mkdir(".cache", 0700) == 0 || errno == EEXIST);
    int temporary_length = snprintf(allowed_directory, sizeof(allowed_directory),
        "%s/.cache/vexel-mcp-native-XXXXXX", cwd);
    CHECK(temporary_length > 0 && (size_t)temporary_length < sizeof(allowed_directory));
    CHECK(mkdtemp(allowed_directory) != NULL);
    CHECK(setenv("VEXEL_MCP_ALLOWED_DIRS", allowed_directory, 1) == 0);
    fixture state = {.path = "/tmp/active.vexel"};
    CHECK(project_bundle_decode((base_bytes){bundle_bytes, encoded_size.encoded_size},
        &state.bundle).error == PROJECT_BUNDLE_OK);
    CHECK(project_bundle_asset_overlay_init(&state.overlay, &state.bundle));
    document_edit_result edit = document_edit_create(
        (base_bytes){scene, sizeof(scene) - 1U}, document_edit_default_limits());
    CHECK(edit.error == DOCUMENT_EDIT_OK);
    state.edit = edit.edit;
    project_timeline_context_owner context = {0};
    project_timeline_view timeline_view = {0};
    project_timeline_integration_result validation =
        project_timeline_validate_bundle_owned(&state.bundle, state.bundle.scene_svg,
            NULL, NULL, &timeline_view, &context);
    CHECK(validation.validation.error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_session_create(state.bundle.timeline_json, &context,
        project_timeline_edit_default_limits(), &state.session).error ==
        PROJECT_TIMELINE_OK);
    project_timeline_session *session = state.session;

    mcp_broker_host host = {.context = &state, .current_edit = host_edit,
        .current_path = host_path,
        .current_timeline_session = host_session,
        .current_project_bundle = host_bundle,
        .native_timeline_changed = host_changed,
        .add_native_drawing_asset = host_add_asset,
        .save_current_project = host_save,
        .agent_action_completed = host_action_completed};
    mcp_broker *broker = NULL;
    CHECK(mcp_broker_create(&host, &broker));
    mcp_broker_bind_current(broker);
    mcp_broker_document *active = mcp_broker_current(broker);
    CHECK(active != NULL);
    mcp_protocol_backend backend = mcp_broker_protocol_backend(broker);
    char args[512];
    int amount = snprintf(args, sizeof(args), "{\"documentId\":\"%s\"}", active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    mcp_tool_response response = {0};
    CHECK(call_tool(&backend, "native_timeline_get", args, &response));
    CHECK(!response.is_error && strstr(response.structured_json.data,
        "\"id\":\"draw\"") != NULL && strstr(response.structured_json.data,
        "\"tick\":0") != NULL);
    CHECK(state.action_count == 1U &&
        strcmp(state.last_action, "native_timeline_get") == 0 &&
        state.last_action_succeeded);
    response_destroy(&response);

    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"trackId\":\"draw\","
        "\"tick\":12,\"assetName\":\"cel0\",\"label\":\"pose\"}", active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_set_cel", args, &response));
    CHECK(!response.is_error && state.changed == 1U &&
        project_timeline_session_is_dirty(state.session));
    response_destroy(&response);
    amount = snprintf(args, sizeof(args), "{\"documentId\":\"%s\"}", active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_get", args, &response));
    CHECK(!response.is_error && strstr(response.structured_json.data,
        "\"tick\":12") != NULL);
    response_destroy(&response);

    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"trackId\":\"draw\",\"fromTick\":12,\"toTick\":10}",
        active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_move_item", args, &response));
    CHECK(!response.is_error && state.changed == 2U &&
        project_timeline_session_is_dirty(state.session));
    response_destroy(&response);
    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"trackId\":\"draw\",\"fromTick\":10,\"toTick\":0}",
        active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_move_item", args, &response));
    CHECK(response.is_error && state.changed == 2U);
    CHECK(state.action_count > 4U &&
        strcmp(state.last_action, "native_timeline_move_item") == 0 &&
        !state.last_action_succeeded);
    response_destroy(&response);
    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"trackId\":\"draw\",\"fromTick\":10,\"toTick\":10}",
        active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_move_item", args, &response));
    CHECK(!response.is_error && state.changed == 2U &&
        strstr(response.structured_json.data, "\"changed\":false") != NULL);
    response_destroy(&response);
    amount = snprintf(args, sizeof(args), "{\"documentId\":\"%s\"}", active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_get", args, &response));
    CHECK(!response.is_error && strstr(response.structured_json.data,
        "\"tick\":10") != NULL);
    response_destroy(&response);

    CHECK(call_tool(&backend, "native_timeline_undo", args, &response));
    CHECK(!response.is_error && state.changed == 3U);
    response_destroy(&response);
    CHECK(call_tool(&backend, "native_timeline_redo", args, &response));
    CHECK(!response.is_error && state.changed == 4U);
    response_destroy(&response);

    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"trackId\":\"draw\",\"tick\":10}",
        active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_remove_cel", args, &response));
    CHECK(!response.is_error && state.changed == 5U);
    response_destroy(&response);
    amount = snprintf(args, sizeof(args), "{\"documentId\":\"%s\"}", active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_undo", args, &response));
    CHECK(!response.is_error && state.changed == 6U);
    response_destroy(&response);

    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"trackId\":\"new-track\","
        "\"targetId\":\"layer\",\"assetName\":\"cel0\",\"name\":\"Ink\"}",
        active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_add_track", args, &response));
    CHECK(!response.is_error && state.changed == 7U);
    response_destroy(&response);

    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"trackId\":\"fade\","
        "\"targetId\":\"layer\",\"property\":\"fill\","
        "\"valueType\":\"rgba\",\"interpolation\":\"linear\","
        "\"value\":[1,0,0,1],\"name\":\"Fill color\"}", active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_add_property_track", args, &response));
    CHECK(!response.is_error && state.changed == 8U);
    response_destroy(&response);

    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"trackId\":\"fade\","
        "\"tick\":18,\"value\":[0,1,0,0.5]}", active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_set_property_key", args, &response));
    CHECK(!response.is_error && state.changed == 9U);
    response_destroy(&response);
    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"trackId\":\"fade\",\"fromTick\":18,\"toTick\":20}",
        active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_move_item", args, &response));
    CHECK(!response.is_error && state.changed == 10U);
    response_destroy(&response);
    amount = snprintf(args, sizeof(args), "{\"documentId\":\"%s\"}", active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_undo", args, &response));
    CHECK(!response.is_error && state.changed == 11U);
    response_destroy(&response);
    CHECK(call_tool(&backend, "native_timeline_redo", args, &response));
    CHECK(!response.is_error && state.changed == 12U);
    response_destroy(&response);
    CHECK(call_tool(&backend, "native_timeline_get", args, &response));
    CHECK(!response.is_error && strstr(response.structured_json.data,
        "\"value_type\":\"rgba\"") != NULL &&
        strstr(response.structured_json.data, "\"tick\":20") != NULL);
    response_destroy(&response);

    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"trackId\":\"visibility\","
        "\"targetId\":\"layer\",\"property\":\"visibility\","
        "\"valueType\":\"boolean\",\"interpolation\":\"step\","
        "\"value\":true}", active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_add_property_track", args, &response));
    CHECK(!response.is_error && state.changed == 13U);
    response_destroy(&response);
    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"trackId\":\"visibility\","
        "\"tick\":12,\"value\":false}", active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_set_property_key", args, &response));
    CHECK(!response.is_error && state.changed == 14U);
    response_destroy(&response);

    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"trackId\":\"fade-opacity\","
        "\"targetId\":\"layer\",\"property\":\"opacity\","
        "\"valueType\":\"scalar\",\"interpolation\":\"cubic\","
        "\"value\":1}", active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_add_property_track", args, &response));
    CHECK(!response.is_error && state.changed == 15U);
    response_destroy(&response);
    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"trackId\":\"fade-opacity\","
        "\"tick\":8,\"value\":0.4}", active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_set_property_key", args, &response));
    CHECK(!response.is_error && state.changed == 16U);
    response_destroy(&response);

    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"trackId\":\"draw\","
        "\"tick\":14,\"assetName\":\"cel0\",\"label\":7}", active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_set_cel", args, &response));
    CHECK(response.is_error && state.changed == 16U);
    response_destroy(&response);

    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"trackId\":\"draw\","
        "\"tick\":13,\"assetName\":\"missing\"}", active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_set_cel", args, &response));
    CHECK(response.is_error && state.changed == 16U);
    response_destroy(&response);

    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"mode\":\"ping_pong\"}",
        active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_set_loop", args, &response));
    CHECK(!response.is_error && state.changed == 17U &&
        project_timeline_session_is_dirty(state.session));
    response_destroy(&response);
    amount = snprintf(args, sizeof(args), "{\"documentId\":\"%s\"}", active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_get", args, &response));
    CHECK(!response.is_error && strstr(response.structured_json.data,
        "\"loop\":\"ping_pong\"") != NULL);
    response_destroy(&response);
    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"mode\":\"bounce\"}",
        active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_set_loop", args, &response));
    CHECK(response.is_error && state.changed == 17U);
    response_destroy(&response);
    amount = snprintf(args, sizeof(args), "{\"documentId\":\"%s\"}", active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_undo", args, &response));
    CHECK(!response.is_error && state.changed == 18U);
    response_destroy(&response);
    CHECK(call_tool(&backend, "native_timeline_redo", args, &response));
    CHECK(!response.is_error && state.changed == 19U);
    response_destroy(&response);

    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"parentId\":\"layer\",\"template\":\"g\"}",
        active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "element_add", args, &response));
    CHECK(!response.is_error && strstr(response.structured_json.data,
        "\"elementId\":\"vexel-object-1\"") != NULL);
    response_destroy(&response);
    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"parentId\":\"layer\","
        "\"xml\":\"<g><path d='M0 0L1 1'/></g>\"}", active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "element_add_fragment", args, &response));
    CHECK(!response.is_error && strstr(response.structured_json.data,
        "\"elementId\":\"vexel-object-2\"") != NULL);
    response_destroy(&response);

    amount = snprintf(args, sizeof(args), "{\"documentId\":\"%s\"}", active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "save", args, &response));
    CHECK(!response.is_error && state.saves == 1U);
    response_destroy(&response);
    base_bytes current_session = project_timeline_session_source(state.session);
    CHECK(state.saved_timeline_length == current_session.len &&
        memcmp(state.saved_timeline, current_session.ptr,
            current_session.len) == 0);

    char svg_path[MCP_BROKER_PATH_CAP];
    amount = snprintf(svg_path, sizeof(svg_path), "%s/active.svg", allowed_directory);
    CHECK(amount > 0 && (size_t)amount < sizeof(svg_path));
    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"path\":\"%s\"}", active->id,
        svg_path);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "save_as", args, &response));
    CHECK(!response.is_error && strstr(response.structured_json.data,
        "\"format\":\"svg\"") != NULL);
    response_destroy(&response);
    char limited_response_path[MCP_BROKER_PATH_CAP];
    amount = snprintf(limited_response_path, sizeof(limited_response_path),
        "%s/response-limited.gif", allowed_directory);
    CHECK(amount > 0 && (size_t)amount < sizeof(limited_response_path));
    CHECK(access(limited_response_path, F_OK) != 0);
    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"path\":\"%s\","
        "\"format\":\"gif\",\"fps\":2,\"duration\":1}",
        active->id, limited_response_path);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    bool limited_response_encoded = call_tool_with_structured_limit(
        &backend, "export_animation", args, &response, 1U);
    CHECK(!limited_response_encoded && response.is_error &&
        access(limited_response_path, F_OK) != 0);
    response_destroy(&response);
    amount = snprintf(args, sizeof(args), "{\"documentId\":\"%s\"}", active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_get", args, &response));
    CHECK(response.is_error && strstr(response.structured_json.data,
        "native_timeline_unavailable") != NULL);
    response_destroy(&response);
    char denied_path[MCP_BROKER_PATH_CAP];
    amount = snprintf(denied_path, sizeof(denied_path),
        "%s/.cache/vexel-mcp-denied-%ld.gif", cwd, (long)getpid());
    CHECK(amount > 0 && (size_t)amount < sizeof(denied_path));
    CHECK(access(denied_path, F_OK) != 0);
    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"path\":\"%s\","
        "\"format\":\"gif\",\"fps\":2,\"duration\":1}",
        active->id, denied_path);
    CHECK(amount > 0 && (size_t)amount < sizeof(args) &&
        call_tool(&backend, "export_animation", args, &response));
    CHECK(response.is_error && strstr(response.structured_json.data,
        "path_not_granted") != NULL && access(denied_path, F_OK) != 0);
    response_destroy(&response);

    char vexel_path[MCP_BROKER_PATH_CAP];
    amount = snprintf(vexel_path, sizeof(vexel_path), "%s/active.vexel",
        allowed_directory);
    CHECK(amount > 0 && (size_t)amount < sizeof(vexel_path));
    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"path\":\"%s\"}", active->id,
        vexel_path);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "save_as", args, &response));
    CHECK(!response.is_error && strstr(response.structured_json.data,
        "\"format\":\"vexel\"") != NULL);
    response_destroy(&response);

    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"path\":\"%s\\u0000suffix\"}",
        active->id, svg_path);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "save_as", args, &response));
    CHECK(response.is_error && state.saves == 3U &&
        strstr(response.structured_json.data, "path_not_granted") != NULL);
    response_destroy(&response);

    state.session = NULL;
    amount = snprintf(args, sizeof(args), "{\"documentId\":\"%s\"}", active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_get", args, &response));
    CHECK(response.is_error && strstr(response.structured_json.data,
        "native_timeline_unavailable") != NULL);
    response_destroy(&response);
    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"path\":\"%s/without-session.gif\","
        "\"format\":\"gif\",\"fps\":2,\"duration\":1}",
        active->id, allowed_directory);
    CHECK(amount > 0 && (size_t)amount < sizeof(args) &&
        call_tool(&backend, "export_animation", args, &response));
    CHECK(response.is_error && strstr(response.structured_json.data,
        "native_timeline_unavailable") != NULL);
    response_destroy(&response);
    state.session = session;

    static const char new_asset_svg[] =
        "<svg width='2' height='2' viewBox='0 0 2 2'><path d='M0 0L2 2' stroke='red'/></svg>";
    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"assetName\":\"newcel\",\"svg\":\"%s\"}",
        active->id, new_asset_svg);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_add_drawing_asset", args, &response));
    CHECK(!response.is_error && strstr(response.structured_json.data,
        "\"assetAdded\":true") != NULL && state.bundle.asset_count == 2U &&
        project_bundle_asset_overlay_is_dirty(&state.overlay));
    response_destroy(&response);
    project_bundle_asset added_asset;
    CHECK(project_bundle_asset_at(&state.bundle, 1U, &added_asset) &&
        added_asset.name.len == 6U &&
        memcmp(added_asset.name.ptr, "newcel", 6U) == 0);
    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"trackId\":\"overlaydraw\","
        "\"targetId\":\"layer\",\"assetName\":\"newcel\"}", active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_add_track", args, &response));
    CHECK(!response.is_error);
    response_destroy(&response);
    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"trackId\":\"overlaydraw\","
        "\"tick\":6,\"assetName\":\"newcel\"}", active->id);
    CHECK(amount > 0 && (size_t)amount < sizeof(args));
    CHECK(call_tool(&backend, "native_timeline_set_cel", args, &response));
    CHECK(!response.is_error);
    response_destroy(&response);

    CHECK(project_timeline_session_refresh_context(state.session,
        &state.bundle, state.bundle.scene_svg, NULL, NULL).error ==
        PROJECT_TIMELINE_OK);

    project_timeline_view live_timeline = {0};
    CHECK(project_timeline_session_decode(state.session, &live_timeline).error ==
        PROJECT_TIMELINE_OK);
    svg_shape_document live_scene = {0};
    svg_shape_diagnostic live_diagnostic = {SVG_SHAPE_ERROR_NONE, 0U};
    base_bytes live_svg = state.bundle.scene_svg;
    CHECK(svg_shape_decode(live_svg, &live_scene, &live_diagnostic) ==
        SVG_SHAPE_ERROR_NONE);
    raster_tile live_frame = {0};
    project_animation_render_result live_render = project_animation_render_frame(
        &live_scene, &state.bundle, &live_timeline, 6U, &live_frame);
    CHECK(live_render.error == PROJECT_ANIMATION_RENDER_OK &&
        live_frame.rgba != NULL);
    raster_tile_destroy(&live_frame);
    live_render = project_animation_render_frame(&live_scene, &state.bundle,
        &live_timeline, 0U, &live_frame);
    if (live_render.error != PROJECT_ANIMATION_RENDER_OK)
        (void)fprintf(stderr, "direct native tick zero: %s\n",
            project_animation_render_error_name(live_render.error));
    CHECK(live_render.error == PROJECT_ANIMATION_RENDER_OK &&
        live_frame.rgba != NULL);
    raster_tile_destroy(&live_frame);
    svg_shape_document_destroy(&live_scene);

    char frames_directory[MCP_BROKER_PATH_CAP];
    char frame_zero_path[MCP_BROKER_PATH_CAP];
    char frame_one_path[MCP_BROKER_PATH_CAP];
    char gif_path[MCP_BROKER_PATH_CAP];
    amount = snprintf(frames_directory, sizeof(frames_directory),
        "%s/native-frames", allowed_directory);
    CHECK(amount > 0 && (size_t)amount < sizeof(frames_directory) &&
        mkdir(frames_directory, 0700) == 0);
    amount = snprintf(frame_zero_path, sizeof(frame_zero_path),
        "%s/frame-0000.png", frames_directory);
    CHECK(amount > 0 && (size_t)amount < sizeof(frame_zero_path));
    amount = snprintf(frame_one_path, sizeof(frame_one_path),
        "%s/frame-0001.png", frames_directory);
    CHECK(amount > 0 && (size_t)amount < sizeof(frame_one_path));
    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"path\":\"%s\","
        "\"format\":\"frames\",\"fps\":2,\"duration\":1}",
        active->id, frames_directory);
    CHECK(amount > 0 && (size_t)amount < sizeof(args) &&
        call_tool(&backend, "export_animation", args, &response));
    if (response.is_error || strstr(response.structured_json.data,
            "\"frames\":2") == NULL)
        (void)fprintf(stderr, "native export response: %s\n",
            response.structured_json.data == NULL ? "(null)" :
            response.structured_json.data);
    CHECK(!response.is_error &&
        strstr(response.structured_json.data, "\"frames\":2") != NULL);
    response_destroy(&response);
    platform_image first_exported = PLATFORM_IMAGE_INIT;
    platform_image second_exported = PLATFORM_IMAGE_INIT;
    bool pngs_read = read_png(frame_zero_path, &first_exported) &&
        read_png(frame_one_path, &second_exported);
    if (!pngs_read || first_exported.width != 2U ||
        first_exported.rgba[0] <= first_exported.rgba[2] ||
        memcmp(first_exported.rgba, second_exported.rgba,
            first_exported.byte_count) == 0)
        (void)fprintf(stderr, "native frame pixels: read=%d size=%ux%u first=%u,%u,%u second=%u,%u,%u\n",
            pngs_read, first_exported.width, first_exported.height,
            first_exported.rgba == NULL ? 0U : first_exported.rgba[0],
            first_exported.rgba == NULL ? 0U : first_exported.rgba[1],
            first_exported.rgba == NULL ? 0U : first_exported.rgba[2],
            second_exported.rgba == NULL ? 0U : second_exported.rgba[0],
            second_exported.rgba == NULL ? 0U : second_exported.rgba[1],
            second_exported.rgba == NULL ? 0U : second_exported.rgba[2]);
    CHECK(pngs_read && first_exported.width == 2U && first_exported.height == 2U &&
        first_exported.rgba[0] > first_exported.rgba[2] &&
        memcmp(first_exported.rgba, second_exported.rgba,
            first_exported.byte_count) != 0);
    platform_image_destroy(&first_exported);
    platform_image_destroy(&second_exported);
    amount = snprintf(gif_path, sizeof(gif_path), "%s/native.gif",
        allowed_directory);
    CHECK(amount > 0 && (size_t)amount < sizeof(gif_path));
    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"path\":\"%s\","
        "\"format\":\"gif\",\"fps\":2,\"duration\":1}",
        active->id, gif_path);
    CHECK(amount > 0 && (size_t)amount < sizeof(args) &&
        call_tool(&backend, "export_animation", args, &response));
    CHECK(!response.is_error && strstr(response.structured_json.data,
        "\"format\":\"gif\"") != NULL &&
        strstr(response.structured_json.data,
            "\"published\":true,\"durable\":true") != NULL);
    mcp_json_doc parsed_export = {0};
    CHECK(mcp_json_parse((const uint8_t *)response.structured_json.data,
        response.structured_json.length, &parsed_export));
    mcp_json_doc_destroy(&parsed_export);
    response_destroy(&response);
    static const uint8_t gif_header[] = {'G', 'I', 'F', '8', '9', 'a'};
    CHECK(file_has_prefix(gif_path, gif_header, sizeof(gif_header)));
    static const uint8_t sentinel[] = "preserve-existing-gif";
    int sentinel_fd = open(gif_path, O_WRONLY | O_TRUNC | O_CLOEXEC);
    CHECK(sentinel_fd >= 0 && write(sentinel_fd, sentinel,
        sizeof(sentinel) - 1U) == (ssize_t)(sizeof(sentinel) - 1U) &&
        close(sentinel_fd) == 0);
    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"path\":\"%s\","
        "\"format\":\"gif\",\"fps\":2,\"duration\":1}",
        active->id, gif_path);
    CHECK(amount > 0 && (size_t)amount < sizeof(args) &&
        call_tool(&backend, "export_animation", args, &response));
    CHECK(response.is_error && strstr(response.structured_json.data,
        "animation_export_failed") != NULL);
    response_destroy(&response);
    CHECK(file_matches(gif_path, sentinel, sizeof(sentinel) - 1U) &&
        new_file_temp_count(allowed_directory) == 0U);
    amount = snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"path\":\"%s/too-long.gif\","
        "\"format\":\"gif\",\"fps\":2,\"duration\":2}",
        active->id, allowed_directory);
    CHECK(amount > 0 && (size_t)amount < sizeof(args) &&
        call_tool(&backend, "export_animation", args, &response));
    CHECK(response.is_error && strstr(response.structured_json.data,
        "native_duration_exceeded") != NULL);
    response_destroy(&response);

    base_bytes current = project_timeline_session_source(state.session);
    input.timeline_json = current;
    project_bundle_asset saved_assets[2];
    CHECK(project_bundle_asset_at(&state.bundle, 0U, &saved_assets[0]) &&
        project_bundle_asset_at(&state.bundle, 1U, &saved_assets[1]));
    input.assets = saved_assets;
    input.asset_count = 2U;
    encoded_size = project_bundle_encoded_size(&input);
    CHECK(encoded_size.error == PROJECT_BUNDLE_OK);
    uint8_t *saved = malloc(encoded_size.encoded_size);
    CHECK(saved != NULL && project_bundle_encode(&input,
        (base_mut_bytes){saved, encoded_size.encoded_size}).error == PROJECT_BUNDLE_OK);
    project_bundle_view decoded = {0};
    CHECK(project_bundle_decode((base_bytes){saved, encoded_size.encoded_size},
        &decoded).error == PROJECT_BUNDLE_OK);
    CHECK(decoded.timeline_json.len == current.len &&
        memcmp(decoded.timeline_json.ptr, current.ptr, current.len) == 0);
    free(saved);

    mcp_broker_destroy(broker);
    project_timeline_session_destroy(state.session);
    project_bundle_asset_overlay_destroy(&state.overlay);
    document_edit_destroy(state.edit);
    free(state.saved_timeline);
    free(bundle_bytes);
    CHECK(unlink(frame_zero_path) == 0 && unlink(frame_one_path) == 0 &&
        rmdir(frames_directory) == 0 && unlink(gif_path) == 0);
    CHECK(rmdir(allowed_directory) == 0);
    return EXIT_SUCCESS;
}
