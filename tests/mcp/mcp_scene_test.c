#include "mcp_broker_internal.h"
#include "mcp_ui_host.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    return 1; } } while (0)

typedef struct fake_session {
    document_edit *edit;
    document_handle selection;
    char path[128];
    char selected_id[256];
    char active_tool[64];
    size_t mutation_count;
    char last_document[32];
    char last_mutation[32];
    size_t view_updates;
} fake_session;

static document_edit *current_edit(void *context)
{
    return ((fake_session *)context)->edit;
}

static const char *current_path(void *context)
{
    return ((fake_session *)context)->path;
}

static bool current_selection(void *context, document_handle *out)
{
    if (out == NULL) return false;
    *out = ((fake_session *)context)->selection;
    return true;
}

static bool replace_current_edit(void *context, document_edit *expected,
                                 document_edit *replacement,
                                 document_handle selection)
{
    fake_session *session = context;
    if (session->edit != expected || replacement == NULL) return false;
    session->edit = replacement;
    session->selection = selection;
    return true;
}

static bool destroy_current_edit(void *context, document_edit *edit)
{
    fake_session *session = context;
    if (session->edit != edit) return false;
    document_edit_destroy(session->edit);
    session->edit = NULL;
    return true;
}

static bool set_current_path(void *context, const char *path)
{
    (void)context; (void)path;
    return true;
}

static void agent_edited(void *context, const char *path)
{
    (void)context; (void)path;
}

static void selection_changed(void *context, document_handle selection)
{
    ((fake_session *)context)->selection = selection;
}

static void commit_mutation(void *context, void *identity,
                            mcp_broker_mutation_kind mutation_kind,
                            document_handle selection, bool selection_valid)
{
    fake_session *session = context;
    session->mutation_count++;
    (void)identity;
    session->selection = selection;
    (void)snprintf(session->last_mutation, sizeof(session->last_mutation), "%d:%d",
        (int)mutation_kind, selection_valid ? 1 : 0);
}

static bool response_init(mcp_tool_response *response)
{
    memset(response, 0, sizeof(*response));
    return mcp_json_buffer_init(&response->structured_json, 128U, MCP_JSON_MAX_BYTES) &&
        mcp_json_buffer_init(&response->content_json, 128U, MCP_JSON_MAX_BYTES);
}

static void response_destroy(mcp_tool_response *response)
{
    mcp_json_buffer_destroy(&response->structured_json);
    mcp_json_buffer_destroy(&response->content_json);
}

static bool call_tool(const mcp_protocol_backend *backend, const char *name,
                      const char *arguments, mcp_tool_response *out)
{
    mcp_json_doc request = {0};
    size_t length = strlen(arguments);
    bool okay = response_init(out) && mcp_json_parse(
        (const uint8_t *)arguments, length, &request);
    const char *validation_error = NULL;
    if (okay) okay = mcp_tool_validate(&request, request.root, name,
        &validation_error);
    if (okay) okay = backend->call_tool(backend->context, &request,
        request.root, name, out);
    if (!okay && validation_error != NULL)
        (void)fprintf(stderr, "invalid scene test request for %s: %s\n", name,
            validation_error);
    mcp_json_doc_destroy(&request);
    return okay;
}

static size_t field_token(const mcp_json_buffer *json, const char *field,
                          mcp_json_doc *parsed)
{
    if (!mcp_json_parse((const uint8_t *)json->data, json->length, parsed)) return MCP_JSON_NO_TOKEN;
    return mcp_json_object_get(parsed, parsed->root, field);
}

static bool expect_ok(const mcp_protocol_backend *backend, const char *name,
                      const char *arguments, mcp_tool_response *out)
{
    bool called = call_tool(backend, name, arguments, out);
    if (called && out->is_error)
        (void)fprintf(stderr, "%s returned error: %s\n", name,
            out->structured_json.data == NULL ? "<no details>" : out->structured_json.data);
    return called && !out->is_error;
}

static bool expect_error(const mcp_protocol_backend *backend, const char *name,
                         const char *arguments, const char *code)
{
    mcp_tool_response response = {0}; mcp_json_doc parsed = {0};
    bool okay = call_tool(backend, name, arguments, &response) && response.is_error;
    size_t error = okay ? field_token(&response.structured_json, "error", &parsed) : MCP_JSON_NO_TOKEN;
    size_t code_token = error != MCP_JSON_NO_TOKEN ? mcp_json_object_get(&parsed, error, "code") : MCP_JSON_NO_TOKEN;
    okay = okay && mcp_json_string_equals(&parsed, code_token, code);
    mcp_json_doc_destroy(&parsed); response_destroy(&response);
    return okay;
}

static bool response_field_string(const mcp_tool_response *response,
                                  const char *field, char *out, size_t capacity)
{
    mcp_json_doc parsed = {0};
    bool okay = capacity > 0U && mcp_json_parse(
        (const uint8_t *)response->structured_json.data,
        response->structured_json.length, &parsed);
    size_t token = okay ? mcp_json_object_get(&parsed, parsed.root, field) : MCP_JSON_NO_TOKEN;
    size_t length = 0U;
    okay = okay && mcp_json_string_copy(&parsed, token, out, capacity - 1U, &length) && length < capacity;
    if (okay) out[length] = '\0';
    mcp_json_doc_destroy(&parsed);
    return okay;
}

static bool read_source(document_edit *edit, char *out, size_t capacity,
                        size_t *out_length)
{
    size_t length = document_edit_byte_length(edit);
    if (length >= capacity || !document_edit_materialize(edit,
            (uint8_t *)out, capacity, &length)) return false;
    out[length] = '\0'; *out_length = length;
    return true;
}

static bool render_frame(const mcp_protocol_backend *backend,
    mcp_broker_document *document, const char *document_id,
    char **snapshot, size_t *snapshot_length, bool *out_changed)
{
    char args[256];
    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"region\":{\"x\":0,\"y\":0,\"width\":64,\"height\":64},\"scale\":1}",
        document_id);
    mcp_tool_response response = {0};
    if (!expect_ok(backend, "render_png", args, &response)) {
        response_destroy(&response);
        return false;
    }
    size_t length = document->last_render_length;
    char *copy = malloc(length + 1U);
    if (copy == NULL) { response_destroy(&response); return false; }
    bool changed = *snapshot == NULL || *snapshot_length != length ||
        memcmp(*snapshot, document->last_render_base64, length) != 0;
    memcpy(copy, document->last_render_base64, length);
    copy[length] = '\0';
    free(*snapshot);
    *snapshot = copy;
    *snapshot_length = length;
    if (out_changed != NULL) *out_changed = changed;
    response_destroy(&response);
    return true;
}

int main(void)
{
    mcp_json_buffer schemas = {0}; mcp_json_doc parsed_schemas = {0};
    CHECK(mcp_json_buffer_init(&schemas, 512U, MCP_JSON_MAX_BYTES));
    CHECK(mcp_tools_list_json(&schemas));
    CHECK(mcp_json_parse((const uint8_t *)schemas.data, schemas.length, &parsed_schemas));
    mcp_json_doc_destroy(&parsed_schemas); mcp_json_buffer_destroy(&schemas);
    static uint8_t source[] =
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"64\" height=\"64\" viewBox=\"0 0 64 64\" id=\"root\">"
        "<rect id=\"seed\" x=\"2\" y=\"2\" width=\"8\" height=\"8\" fill=\"#ff0000\"/>"
        "<defs id=\"defs\"/></svg>";
    fake_session session = {0};
    (void)snprintf(session.path, sizeof(session.path), "/tmp/vexel-mcp-scene-test.svg");
    (void)snprintf(session.active_tool, sizeof(session.active_tool), "select");
    document_edit_result edit = document_edit_create(
        (base_bytes){source, sizeof(source) - 1U}, document_edit_default_limits());
    CHECK(edit.error == DOCUMENT_EDIT_OK && edit.edit != NULL);
    session.edit = edit.edit;
    mcp_broker_host host = {
        .context = &session,
        .current_edit = current_edit,
        .current_path = current_path,
        .current_selection = current_selection,
        .replace_current_edit = replace_current_edit,
        .destroy_current_edit = destroy_current_edit,
        .set_current_path = set_current_path,
        .agent_edited = agent_edited,
        .selection_changed = selection_changed,
        .commit_mutation = commit_mutation
    };
    mcp_broker *broker = NULL;
    CHECK(mcp_broker_create(&host, &broker));
    mcp_broker_bind_current(broker);
    mcp_broker_document *document = mcp_broker_current(broker);
    CHECK(document != NULL && document->app_owned && document->edit == session.edit);
    mcp_protocol_backend backend = mcp_broker_protocol_backend(broker);
    char document_id[32];
    (void)snprintf(document_id, sizeof(document_id), "%s", document->id);

    mcp_ui_view_state view = {0};
    CHECK(mcp_ui_host_get_view(&broker->app_document->ui_host, &view) ==
        MCP_UI_HOST_UNAVAILABLE);
    mcp_ui_host headless = {0};
    mcp_ui_host_init_headless(&headless);
    CHECK(mcp_ui_host_get_view(&headless, &view) == MCP_UI_HOST_UNAVAILABLE);

    char args[4096];
    char *frame_snapshot = NULL; size_t frame_snapshot_length = 0U;
    bool frame_changed = false;
    CHECK(render_frame(&backend, document, document_id, &frame_snapshot,
        &frame_snapshot_length, &frame_changed) && frame_changed);
    CHECK(document->last_render_width == 64U);

    size_t history_before = document_edit_history_cursor(document->edit);
    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"tag\":\"rect\",\"parentId\":\"root\",\"index\":0,\"attributes\":{\"id\":\"added\",\"x\":\"20\",\"y\":\"20\",\"width\":\"20\",\"height\":\"12\"},\"style\":{\"fill\":\"#0000ff\"}}",
        document_id);
    mcp_tool_response created = {0};
    CHECK(expect_ok(&backend, "element_create", args, &created));
    CHECK(strstr(created.structured_json.data, "\"mode\":\"headless\"") != NULL);
    CHECK(document_edit_history_cursor(document->edit) == history_before + 1U);
    CHECK(session.mutation_count == 1U && strcmp(session.last_mutation, "0:1") == 0);
    CHECK(document->edit == session.edit);
    document_handle added_handle = {0};
    CHECK(document_edit_find_id(document->edit,
        (base_bytes){(const uint8_t *)"added", 5U}, &added_handle));
    CHECK(session.selection.document_id == added_handle.document_id &&
        session.selection.slot == added_handle.slot &&
        session.selection.generation == added_handle.generation);
    char source_after_create[16384]; size_t source_after_create_length = 0U;
    CHECK(read_source(document->edit, source_after_create, sizeof(source_after_create),
        &source_after_create_length));
    const char *created_span = strstr(source_after_create, "<rect id=\"added\"");
    const char *seed_span = strstr(source_after_create,
        "<rect id=\"seed\" x=\"2\" y=\"2\" width=\"8\" height=\"8\" fill=\"#ff0000\"/>");
    CHECK(created_span != NULL && seed_span != NULL && created_span < seed_span);
    CHECK(source_after_create_length > 0U);
    response_destroy(&created);
    CHECK(render_frame(&backend, document, document_id, &frame_snapshot,
        &frame_snapshot_length, &frame_changed) && frame_changed);

    (void)snprintf(args, sizeof(args), "{\"documentId\":\"%s\",\"elementId\":\"added\"}", document_id);
    mcp_tool_response element = {0};
    CHECK(expect_ok(&backend, "element_get", args, &element));
    CHECK(strstr(element.structured_json.data, "\"tag\":\"rect\"") != NULL);
    CHECK(strstr(element.structured_json.data, "\"name\":\"x\",\"value\":\"20\"") != NULL);
    response_destroy(&element);

    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"id\":\"added\"}", document_id);
    mcp_tool_response query = {0};
    CHECK(expect_ok(&backend, "element_query", args, &query));
    CHECK(strstr(query.structured_json.data, "\"elementId\":\"added\"") != NULL);
    response_destroy(&query);
    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"selector\":\"rect#added[style]\"}",
        document_id);
    CHECK(expect_ok(&backend, "element_query", args, &query));
    CHECK(strstr(query.structured_json.data, "\"elementId\":\"added\"") != NULL);
    response_destroy(&query);
    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"hitTest\":{\"x\":22,\"y\":22}}",
        document_id);
    CHECK(expect_ok(&backend, "element_query", args, &query));
    CHECK(strstr(query.structured_json.data, "\"elementId\":\"added\"") != NULL);
    response_destroy(&query);

    size_t history_style = document_edit_history_cursor(document->edit);
    (void)snprintf(args, sizeof(args), "{\"documentId\":\"%s\",\"elementId\":\"added\",\"value\":\"#00ff00\"}", document_id);
    mcp_tool_response style = {0};
    CHECK(expect_ok(&backend, "style_fill", args, &style));
    CHECK(document_edit_history_cursor(document->edit) == history_style + 1U);
    CHECK(session.mutation_count == 2U && strcmp(session.last_mutation, "0:1") == 0);
    response_destroy(&style);
    CHECK(render_frame(&backend, document, document_id, &frame_snapshot,
        &frame_snapshot_length, &frame_changed) && frame_changed);
    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"parentId\":\"root\",\"tag\":\"image\",\"attributes\":{\"id\":\"mcp-image\",\"x\":\"50\",\"y\":\"50\",\"width\":\"10\",\"height\":\"10\",\"href\":\"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==\"}}",
        document_id);
    mcp_tool_response image = {0};
    CHECK(expect_ok(&backend, "element_create", args, &image));
    response_destroy(&image);
    CHECK(render_frame(&backend, document, document_id, &frame_snapshot,
        &frame_snapshot_length, &frame_changed) && frame_changed);
    size_t history_stroke = document_edit_history_cursor(document->edit);
    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"elementId\":\"added\",\"value\":\"#2020ff\"}",
        document_id);
    CHECK(expect_ok(&backend, "style_stroke", args, &style));
    CHECK(document_edit_history_cursor(document->edit) == history_stroke + 1U);
    response_destroy(&style);
    CHECK(render_frame(&backend, document, document_id, &frame_snapshot,
        &frame_snapshot_length, &frame_changed) && frame_changed);
    size_t history_opacity = document_edit_history_cursor(document->edit);
    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"elementId\":\"added\",\"value\":\"0.75\"}",
        document_id);
    CHECK(expect_ok(&backend, "style_opacity", args, &style));
    CHECK(document_edit_history_cursor(document->edit) == history_opacity + 1U);
    response_destroy(&style);
    CHECK(render_frame(&backend, document, document_id, &frame_snapshot,
        &frame_snapshot_length, &frame_changed) && frame_changed);

    (void)snprintf(args, sizeof(args), "{\"documentId\":\"%s\",\"elementId\":\"added\",\"operation\":\"rotate\",\"angle\":30,\"cx\":30,\"cy\":30}", document_id);
    mcp_tool_response transform = {0};
    CHECK(expect_ok(&backend, "transform_apply", args, &transform));
    CHECK(strcmp(session.last_mutation, "0:1") == 0);
    response_destroy(&transform);
    CHECK(render_frame(&backend, document, document_id, &frame_snapshot,
        &frame_snapshot_length, &frame_changed) && frame_changed);
    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"elementId\":\"added\",\"operation\":\"scale\",\"sx\":1.5,\"cx\":30,\"cy\":26}",
        document_id);
    CHECK(expect_ok(&backend, "transform_apply", args, &transform));
    response_destroy(&transform);
    CHECK(read_source(document->edit, source_after_create, sizeof(source_after_create),
        &source_after_create_length));
    CHECK(strstr(source_after_create,
        "translate(30 26) scale(1.5 1.5) translate(-30 -26)") != NULL);
    CHECK(render_frame(&backend, document, document_id, &frame_snapshot,
        &frame_snapshot_length, &frame_changed) && frame_changed);
    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"elementId\":\"added\",\"value\":\"translate(2 1)\"}",
        document_id);
    CHECK(expect_ok(&backend, "transform_set", args, &transform));
    response_destroy(&transform);
    CHECK(render_frame(&backend, document, document_id, &frame_snapshot,
        &frame_snapshot_length, &frame_changed) && frame_changed);

    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"parentId\":\"defs\",\"kind\":\"linearGradient\",\"index\":0,\"attributes\":{\"x1\":\"0%%\",\"x2\":\"100%%\"}}",
        document_id);
    mcp_tool_response gradient = {0};
    CHECK(expect_ok(&backend, "gradient_create", args, &gradient));
    char gradient_id[256];
    CHECK(response_field_string(&gradient, "elementId", gradient_id,
        sizeof(gradient_id)));
    response_destroy(&gradient);
    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"gradientId\":\"%s\",\"index\":0,\"offset\":\"0%%\",\"color\":\"white\"}",
        document_id, gradient_id);
    CHECK(expect_ok(&backend, "gradient_stop_add", args, &gradient));
    response_destroy(&gradient);
    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"gradientId\":\"%s\",\"index\":1,\"offset\":\"1\",\"color\":\"black\"}",
        document_id, gradient_id);
    CHECK(expect_ok(&backend, "gradient_stop_add", args, &gradient));
    response_destroy(&gradient);
    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"gradientId\":\"%s\",\"index\":2,\"offset\":\"1\",\"color\":\"red\"}",
        document_id, gradient_id);
    CHECK(expect_ok(&backend, "gradient_stop_add", args, &gradient));
    response_destroy(&gradient);
    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"gradientId\":\"%s\",\"index\":1,\"color\":\"blue\"}",
        document_id, gradient_id);
    CHECK(expect_ok(&backend, "gradient_stop_set", args, &gradient));
    CHECK(strstr(gradient.structured_json.data, "\"changed\":true") != NULL);
    response_destroy(&gradient);
    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"gradientId\":\"%s\",\"index\":2}",
        document_id, gradient_id);
    CHECK(expect_ok(&backend, "gradient_stop_remove", args, &gradient));
    response_destroy(&gradient);
    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"elementId\":\"added\",\"value\":\"url(#%s)\"}",
        document_id, gradient_id);
    CHECK(expect_ok(&backend, "style_fill", args, &gradient));
    response_destroy(&gradient);
    CHECK(render_frame(&backend, document, document_id, &frame_snapshot,
        &frame_snapshot_length, &frame_changed) && frame_changed);

    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"parentId\":\"defs\",\"tag\":\"filter\",\"attributes\":{\"id\":\"mcp-filter\"}}",
        document_id);
    mcp_tool_response filter = {0};
    CHECK(expect_ok(&backend, "element_create", args, &filter));
    char filter_id[256];
    CHECK(response_field_string(&filter, "elementId", filter_id,
        sizeof(filter_id)));
    response_destroy(&filter);
    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"parentId\":\"%s\",\"tag\":\"feGaussianBlur\",\"attributes\":{\"stdDeviation\":\"1\"}}",
        document_id, filter_id);
    CHECK(expect_ok(&backend, "element_create", args, &filter));
    response_destroy(&filter);
    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"elementId\":\"added\",\"attributes\":{\"filter\":\"url(#%s)\"}}",
        document_id, filter_id);
    CHECK(expect_ok(&backend, "element_set", args, &filter));
    response_destroy(&filter);
    CHECK(render_frame(&backend, document, document_id, &frame_snapshot,
        &frame_snapshot_length, &frame_changed) && frame_changed);

    (void)snprintf(args, sizeof(args), "{\"documentId\":\"%s\",\"parentId\":\"root\",\"commands\":\"M 0 0 L 10 0 L 10 10 Z\"}", document_id);
    mcp_tool_response path = {0};
    CHECK(expect_ok(&backend, "path_from_commands", args, &path));
    CHECK(render_frame(&backend, document, document_id, &frame_snapshot,
        &frame_snapshot_length, &frame_changed) && frame_changed);
    char path_id[256]; CHECK(response_field_string(&path, "elementId", path_id, sizeof(path_id)));
    response_destroy(&path);
    (void)snprintf(args, sizeof(args), "{\"documentId\":\"%s\",\"elementId\":\"%s\",\"nodeIndex\":1,\"values\":\"20 30\"}", document_id, path_id);
    mcp_tool_response path_edit = {0};
    CHECK(expect_ok(&backend, "path_edit_node", args, &path_edit));
    response_destroy(&path_edit);
    CHECK(render_frame(&backend, document, document_id, &frame_snapshot,
        &frame_snapshot_length, &frame_changed) && frame_changed);
    char materialized[16384]; size_t materialized_length = 0U;
    CHECK(read_source(document->edit, materialized, sizeof(materialized), &materialized_length));
    CHECK(strstr(materialized, "L 20 30") != NULL);
    (void)snprintf(args, sizeof(args), "{\"documentId\":\"%s\",\"elementId\":\"%s\",\"commands\":\"L 0 10 Z\"}", document_id, path_id);
    size_t history_path_append = document_edit_history_cursor(document->edit);
    mcp_tool_response path_append = {0};
    CHECK(expect_ok(&backend, "path_append", args, &path_append));
    response_destroy(&path_append);
    CHECK(document_edit_history_cursor(document->edit) == history_path_append + 1U);
    CHECK(strcmp(session.last_mutation, "0:1") == 0);
    CHECK(render_frame(&backend, document, document_id, &frame_snapshot,
        &frame_snapshot_length, &frame_changed));

    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"parentId\":\"root\",\"xml\":\"<g id='dupe-group'><rect id='dupe-child' x='45' y='2' width='6' height='6'/></g>\"}",
        document_id);
    mcp_tool_response fragment = {0};
    CHECK(expect_ok(&backend, "element_add_fragment", args, &fragment));
    response_destroy(&fragment);
    size_t history_duplicate = document_edit_history_cursor(document->edit);
    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"elementId\":\"dupe-group\"}", document_id);
    mcp_tool_response duplicate = {0};
    CHECK(expect_ok(&backend, "element_duplicate", args, &duplicate));
    CHECK(strstr(duplicate.structured_json.data, "\"created\":true") != NULL);
    char duplicate_id[256];
    CHECK(response_field_string(&duplicate, "elementId", duplicate_id,
        sizeof(duplicate_id)));
    CHECK(document_edit_history_cursor(document->edit) == history_duplicate + 1U);
    response_destroy(&duplicate);
    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"id\":\"dupe-child\"}", document_id);
    mcp_tool_response child_query = {0};
    CHECK(expect_ok(&backend, "element_query", args, &child_query));
    CHECK(strstr(child_query.structured_json.data, "\"complete\":true") != NULL);
    CHECK(strstr(child_query.structured_json.data, "\"elementId\":\"dupe-child\"") != NULL);
    response_destroy(&child_query);
    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"parentId\":\"%s\",\"includeAttributes\":true}",
        document_id, duplicate_id);
    mcp_tool_response duplicate_tree = {0};
    CHECK(expect_ok(&backend, "element_tree", args, &duplicate_tree));
    CHECK(strstr(duplicate_tree.structured_json.data, "\"elementId\":\"dupe-child\"") == NULL);
    response_destroy(&duplicate_tree);
    CHECK(render_frame(&backend, document, document_id, &frame_snapshot,
        &frame_snapshot_length, &frame_changed));

    size_t history_batch = document_edit_history_cursor(document->edit);
    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"operations\":[{\"op\":\"element_create\",\"tag\":\"circle\",\"parentId\":\"root\",\"attributes\":{\"id\":\"batch-dot\",\"cx\":\"50\",\"cy\":\"50\",\"r\":\"4\"}},{\"op\":\"style_fill\",\"elementId\":\"batch-dot\",\"value\":\"orange\"}]}",
        document_id);
    mcp_tool_response batch = {0};
    CHECK(expect_ok(&backend, "batch", args, &batch));
    CHECK(strstr(batch.structured_json.data, "\"atomic\":true") != NULL);
    CHECK(strstr(batch.structured_json.data, "\"undoSteps\":1") != NULL);
    CHECK(strstr(batch.structured_json.data, "\"index\":0") != NULL &&
          strstr(batch.structured_json.data, "\"index\":1") != NULL);
    CHECK(document_edit_history_cursor(document->edit) == history_batch + 1U);
    response_destroy(&batch);
    CHECK(render_frame(&backend, document, document_id, &frame_snapshot,
        &frame_snapshot_length, &frame_changed) && frame_changed);

    char before_rollback[16384]; size_t before_rollback_length = 0U;
    CHECK(read_source(document->edit, before_rollback, sizeof(before_rollback), &before_rollback_length));
    size_t history_rollback = document_edit_history_cursor(document->edit);
    (void)snprintf(args, sizeof(args),
        "{\"documentId\":\"%s\",\"operations\":[{\"op\":\"element_create\",\"tag\":\"rect\",\"parentId\":\"root\",\"attributes\":{\"id\":\"rollback-me\"}},{\"op\":\"element_set\",\"elementId\":\"missing\",\"attributes\":{\"x\":\"1\"}}]}", document_id);
    CHECK(expect_error(&backend, "batch", args, "batch_rolled_back"));
    CHECK(document_edit_history_cursor(document->edit) == history_rollback);
    CHECK(read_source(document->edit, materialized, sizeof(materialized), &materialized_length));
    CHECK(materialized_length == before_rollback_length &&
          memcmp(materialized, before_rollback, materialized_length) == 0);
    CHECK(strstr(materialized, "rollback-me") == NULL);
    CHECK(render_frame(&backend, document, document_id, &frame_snapshot,
        &frame_snapshot_length, &frame_changed) && !frame_changed);

    (void)snprintf(args, sizeof(args), "{\"documentId\":\"%s\",\"elementId\":\"added\"}", document_id);
    mcp_tool_response selection = {0};
    CHECK(expect_ok(&backend, "selection_set", args, &selection));
    response_destroy(&selection);
    (void)snprintf(args, sizeof(args), "{\"documentId\":\"%s\"}", document_id);
    CHECK(expect_ok(&backend, "selection_get", args, &selection));
    CHECK(strstr(selection.structured_json.data, "\"elementId\":\"added\"") != NULL);
    response_destroy(&selection);

    size_t before_undo = document_edit_history_cursor(document->edit);
    CHECK(expect_ok(&backend, "undo", args, &selection));
    CHECK(document_edit_history_cursor(document->edit) + 1U == before_undo);
    CHECK(strcmp(session.last_mutation, "2:0") == 0);
    response_destroy(&selection);
    CHECK(render_frame(&backend, document, document_id, &frame_snapshot,
        &frame_snapshot_length, &frame_changed) && frame_changed);
    CHECK(expect_ok(&backend, "redo", args, &selection));
    CHECK(strcmp(session.last_mutation, "3:0") == 0);
    response_destroy(&selection);
    CHECK(render_frame(&backend, document, document_id, &frame_snapshot,
        &frame_snapshot_length, &frame_changed) && frame_changed);

    free(frame_snapshot);
    mcp_broker_destroy(broker);
    document_edit_destroy(session.edit);

    fake_session headless_session = {0};
    (void)snprintf(headless_session.path, sizeof(headless_session.path),
        "/tmp/vexel-mcp-headless-scene-test.svg");
    document_edit_result headless_edit = document_edit_create(
        (base_bytes){source, sizeof(source) - 1U}, document_edit_default_limits());
    CHECK(headless_edit.error == DOCUMENT_EDIT_OK && headless_edit.edit != NULL);
    headless_session.edit = headless_edit.edit;
    mcp_broker_host headless_host = {
        .context = &headless_session,
        .current_edit = current_edit,
        .current_path = current_path,
        .destroy_current_edit = destroy_current_edit
    };
    mcp_broker *headless_broker = NULL;
    CHECK(mcp_broker_create(&headless_host, &headless_broker));
    mcp_broker_bind_current(headless_broker);
    mcp_broker_document *headless_document = mcp_broker_current(headless_broker);
    CHECK(headless_document != NULL &&
        strcmp(mcp_broker_document_mode(headless_document), "headless") == 0);
    mcp_protocol_backend headless_backend = mcp_broker_protocol_backend(headless_broker);
    char headless_args[256];
    (void)snprintf(headless_args, sizeof(headless_args),
        "{\"documentId\":\"%s\",\"elementId\":\"seed\"}", headless_document->id);
    mcp_tool_response headless_element = {0};
    CHECK(expect_ok(&headless_backend, "element_get", headless_args, &headless_element));
    CHECK(strstr(headless_element.structured_json.data, "\"mode\":\"headless\"") != NULL);
    response_destroy(&headless_element);
    mcp_broker_destroy(headless_broker);
    document_edit_destroy(headless_session.edit);
    return 0;
}
