#include "mcp_broker_internal.h"

#include <stdlib.h>
#include <string.h>

#define NATIVE_TIMELINE_SOURCE_LIMIT (16U * 1024U * 1024U)
#define NATIVE_DRAWING_ASSET_SOURCE_LIMIT (1024U * 1024U)

typedef struct native_timeline_snapshot {
    uint8_t *scene;
    size_t scene_length;
    project_timeline_session *session;
} native_timeline_snapshot;

static bool set_error_and_fail(mcp_tool_response *out, const char *code,
                               const char *message)
{
    (void)mcp_broker_set_error(out, code, message);
    return false;
}

static void snapshot_destroy(native_timeline_snapshot *snapshot)
{
    if (snapshot == NULL) return;
    free(snapshot->scene);
    *snapshot = (native_timeline_snapshot){0};
}

static bool snapshot_base(mcp_broker *broker, mcp_broker_document *document,
                          native_timeline_snapshot *snapshot,
                          mcp_tool_response *out,
                          bool allow_oversized_timeline)
{
    if (broker == NULL || document == NULL || snapshot == NULL || out == NULL ||
        document != mcp_broker_current(broker) || !document->app_owned ||
        broker->host.current_timeline_session == NULL ||
        broker->host.current_project_bundle == NULL)
        return set_error_and_fail(out, "native_timeline_unavailable",
            "native timeline tools require the active app-owned .vexel document");
    snapshot->session = broker->host.current_timeline_session(broker->host.context);
    const project_bundle_view *bundle =
        broker->host.current_project_bundle(broker->host.context);
    if (snapshot->session == NULL || bundle == NULL ||
        bundle->version != PROJECT_BUNDLE_VERSION)
        return set_error_and_fail(out, "native_timeline_unavailable",
            "the active document has no editable native-v1 timeline session");
    base_bytes timeline = project_timeline_session_source(snapshot->session);
    size_t timeline_limit = allow_oversized_timeline ?
        PROJECT_BUNDLE_MAX_TIMELINE_BYTES : NATIVE_TIMELINE_SOURCE_LIMIT;
    if (timeline.ptr == NULL || timeline.len == 0U ||
        timeline.len > timeline_limit)
        return set_error_and_fail(out, "native_timeline_source_limit",
            allow_oversized_timeline ?
            "native timeline exceeds the 64 MiB recovery limit" :
            "native timeline source is empty or exceeds the 16 MiB MCP edit limit");
    if (document->edit == NULL) return set_error_and_fail(out,
        "document_invalid", "the active document edit source is unavailable");
    snapshot->scene_length = document_edit_byte_length(document->edit);
    if (snapshot->scene_length == 0U ||
        snapshot->scene_length > NATIVE_TIMELINE_SOURCE_LIMIT)
        return set_error_and_fail(out, "native_timeline_source_limit",
            "native timeline MCP operations are limited to 16 MiB scene sources");
    snapshot->scene = malloc(snapshot->scene_length);
    if (snapshot->scene == NULL || !document_edit_materialize(document->edit,
            snapshot->scene, snapshot->scene_length, &snapshot->scene_length)) {
        free(snapshot->scene);
        snapshot->scene = NULL;
        return set_error_and_fail(out, "native_timeline_read_failed",
            "the active project scene could not be materialized");
    }
    return true;
}

static bool snapshot_refresh(mcp_broker *broker, mcp_broker_document *document,
                             native_timeline_snapshot *snapshot,
                             mcp_tool_response *out,
                             bool allow_oversized_timeline)
{
    if (!snapshot_base(broker, document, snapshot, out,
            allow_oversized_timeline)) return false;
    const project_bundle_view *bundle =
        broker->host.current_project_bundle(broker->host.context);
    project_timeline_edit_result refreshed = project_timeline_session_refresh_context(
        snapshot->session, bundle,
        (base_bytes){snapshot->scene, snapshot->scene_length}, NULL, NULL);
    if (refreshed.error != PROJECT_TIMELINE_OK) {
        snapshot_destroy(snapshot);
        return set_error_and_fail(out, "native_timeline_invalid_reference",
            project_timeline_error_name(refreshed.error));
    }
    return true;
}

static bool parse_track_id(const mcp_json_doc *request, size_t args,
                           char out[257], size_t *length)
{
    return mcp_broker_string_arg(request, args, "trackId", out, 257U, length) &&
        *length > 0U && memchr(out, '\0', *length) == NULL;
}

static bool property_value_arg(const mcp_json_doc *request, size_t args,
    base_bytes *out_value)
{
    size_t token = mcp_json_object_get(request, args, "value");
    if (token >= request->token_count) return false;
    const mcp_json_token *value = &request->tokens[token];
    if (value->end < value->start || value->end - value->start > 512U ||
        value->end > request->source_length) return false;
    if (value->type == MCP_JSON_ARRAY) {
        if (value->child_count == 0U || value->child_count > 6U) return false;
        size_t child = value->first_child;
        while (child != MCP_JSON_NO_TOKEN) {
            if (request->tokens[child].type != MCP_JSON_NUMBER) return false;
            child = request->tokens[child].next_sibling;
        }
    } else if (value->type != MCP_JSON_NUMBER &&
        value->type != MCP_JSON_TRUE && value->type != MCP_JSON_FALSE) {
        return false;
    }
    *out_value = (base_bytes){request->source + value->start,
        value->end - value->start};
    return true;
}

static bool native_edit(mcp_broker *broker, mcp_broker_document *document,
                        const mcp_json_doc *request, size_t args,
                        const char *tool, mcp_tool_response *out)
{
    native_timeline_snapshot snapshot = {0};
    bool history_action = strcmp(tool, "native_timeline_undo") == 0 ||
        strcmp(tool, "native_timeline_redo") == 0;
    if (!snapshot_refresh(broker, document, &snapshot, out, history_action))
        return out->is_error;
    project_timeline_edit_result result = {PROJECT_TIMELINE_INVALID_ARGUMENT, 0U};
    bool changed = true;
    if (strcmp(tool, "native_timeline_add_drawing_asset") == 0) {
        char name[PROJECT_BUNDLE_MAX_ASSET_NAME_BYTES + 1U];
        size_t name_length = 0U, svg_length = 0U;
        char *svg = malloc(NATIVE_DRAWING_ASSET_SOURCE_LIMIT + 1U);
        bool valid = svg != NULL &&
            mcp_broker_string_arg(request, args, "assetName", name,
                sizeof(name), &name_length) && name_length > 0U &&
            mcp_broker_string_arg(request, args, "svg", svg,
                NATIVE_DRAWING_ASSET_SOURCE_LIMIT + 1U, &svg_length) &&
            svg_length > 0U && svg_length <= NATIVE_DRAWING_ASSET_SOURCE_LIMIT;
        if (!valid) {
            free(svg);
            snapshot_destroy(&snapshot);
            return mcp_broker_set_error(out, "drawing_asset_invalid",
                "assetName must be safe and svg must be a complete SVG no larger than 1 MiB");
        }
        project_bundle_result added = broker->host.add_native_drawing_asset == NULL ?
            (project_bundle_result){PROJECT_BUNDLE_INVALID_ARGUMENT, 0U, 0U} :
            broker->host.add_native_drawing_asset(broker->host.context,
                (base_bytes){(const uint8_t *)name, name_length},
                (base_bytes){(const uint8_t *)svg, svg_length});
        free(svg);
        snapshot_destroy(&snapshot);
        if (added.error != PROJECT_BUNDLE_OK)
            return mcp_broker_set_error(out, "drawing_asset_add_failed",
                project_bundle_error_name(added.error));
        return mcp_json_buffer_literal(&out->structured_json,
            "{\"assetAdded\":true}");
    } else if (strcmp(tool, "native_timeline_add_track") == 0) {
        char track_id[257], target_id[257], asset[257], name[257];
        size_t track_length = 0U, target_length = 0U, asset_length = 0U,
            name_length = 0U;
        bool valid = mcp_broker_string_arg(request, args, "trackId", track_id,
                sizeof(track_id), &track_length) && track_length > 0U &&
            mcp_broker_string_arg(request, args, "targetId", target_id,
                sizeof(target_id), &target_length) && target_length > 0U &&
            mcp_broker_string_arg(request, args, "assetName", asset,
                sizeof(asset), &asset_length) && asset_length > 0U;
        size_t name_token = mcp_json_object_get(request, args, "name");
        if (name_token != MCP_JSON_NO_TOKEN)
            valid = valid && mcp_broker_string_arg(request, args, "name", name,
                sizeof(name), &name_length);
        if (valid)
            result = project_timeline_session_add_drawing_track(snapshot.session,
                (base_bytes){(const uint8_t *)track_id, track_length},
                (base_bytes){(const uint8_t *)target_id, target_length},
                (base_bytes){(const uint8_t *)asset, asset_length},
                (base_bytes){(const uint8_t *)name, name_length});
    } else if (strcmp(tool, "native_timeline_add_property_track") == 0) {
        char track_id[129], target_id[257], property[129], value_type[32];
        char interpolation[16], name[257];
        size_t track_length = 0U, target_length = 0U, property_length = 0U;
        size_t type_length = 0U, interpolation_length = 0U, name_length = 0U;
        base_bytes value = {0};
        bool valid = mcp_broker_string_arg(request, args, "trackId", track_id,
                sizeof(track_id), &track_length) && track_length > 0U &&
            mcp_broker_string_arg(request, args, "targetId", target_id,
                sizeof(target_id), &target_length) && target_length > 0U &&
            mcp_broker_string_arg(request, args, "property", property,
                sizeof(property), &property_length) && property_length > 0U &&
            mcp_broker_string_arg(request, args, "valueType", value_type,
                sizeof(value_type), &type_length) && type_length > 0U &&
            mcp_broker_string_arg(request, args, "interpolation", interpolation,
                sizeof(interpolation), &interpolation_length) && interpolation_length > 0U &&
            property_value_arg(request, args, &value);
        size_t name_token = mcp_json_object_get(request, args, "name");
        if (name_token != MCP_JSON_NO_TOKEN)
            valid = valid && mcp_broker_string_arg(request, args, "name", name,
                sizeof(name), &name_length);
        if (valid)
            result = project_timeline_session_add_property_track(snapshot.session,
                (base_bytes){(const uint8_t *)track_id, track_length},
                (base_bytes){(const uint8_t *)target_id, target_length},
                (base_bytes){(const uint8_t *)property, property_length},
                (base_bytes){(const uint8_t *)value_type, type_length},
                (base_bytes){(const uint8_t *)interpolation, interpolation_length},
                value, (base_bytes){(const uint8_t *)name, name_length});
    } else if (strcmp(tool, "native_timeline_set_loop") == 0) {
        char mode[32]; size_t mode_length = 0U;
        project_timeline_loop_mode parsed_mode;
        bool valid = mcp_broker_string_arg(request, args, "mode", mode,
            sizeof(mode), &mode_length);
        if (valid && mode_length == 4U && memcmp(mode, "once", 4U) == 0)
            parsed_mode = PROJECT_TIMELINE_LOOP_ONCE;
        else if (valid && mode_length == 4U && memcmp(mode, "loop", 4U) == 0)
            parsed_mode = PROJECT_TIMELINE_LOOP_REPEAT;
        else if (valid && mode_length == 9U && memcmp(mode, "ping_pong", 9U) == 0)
            parsed_mode = PROJECT_TIMELINE_LOOP_PING_PONG;
        else valid = false;
        if (valid)
            result = project_timeline_session_set_loop(snapshot.session,
                parsed_mode);
    } else if (strcmp(tool, "native_timeline_set_property_key") == 0) {
        char track_id[257]; size_t track_length = 0U;
        uint64_t tick = 0U; bool present = false; base_bytes value = {0};
        bool valid = parse_track_id(request, args, track_id, &track_length) &&
            mcp_broker_u64_arg(request, args, "tick", &tick, &present) && present &&
            tick <= UINT32_MAX && property_value_arg(request, args, &value);
        if (valid)
            result = project_timeline_session_set_property_key(snapshot.session,
                (base_bytes){(const uint8_t *)track_id, track_length},
                (uint32_t)tick, value);
    } else if (strcmp(tool, "native_timeline_set_cel") == 0) {
        char track_id[257], asset[257], label[257];
        size_t track_length = 0U, asset_length = 0U, label_length = 0U;
        uint64_t tick = 0U;
        bool present = false;
        bool valid = parse_track_id(request, args, track_id, &track_length) &&
            mcp_broker_u64_arg(request, args, "tick", &tick, &present) && present &&
            tick <= UINT32_MAX && mcp_broker_string_arg(request, args, "assetName",
                asset, sizeof(asset), &asset_length) && asset_length > 0U;
        size_t label_token = mcp_json_object_get(request, args, "label");
        if (label_token != MCP_JSON_NO_TOKEN)
            valid = valid && mcp_broker_string_arg(request, args, "label", label,
                sizeof(label), &label_length);
        if (valid)
            result = project_timeline_session_set_cel(snapshot.session,
                (base_bytes){(const uint8_t *)track_id, track_length},
                (uint32_t)tick,
                (base_bytes){(const uint8_t *)asset, asset_length},
                (base_bytes){(const uint8_t *)label, label_length});
    } else if (strcmp(tool, "native_timeline_remove_cel") == 0) {
        char track_id[257];
        size_t track_length = 0U;
        uint64_t tick = 0U;
        bool present = false;
        bool valid = parse_track_id(request, args, track_id, &track_length) &&
            mcp_broker_u64_arg(request, args, "tick", &tick, &present) && present &&
            tick <= UINT32_MAX;
        if (valid)
            result = project_timeline_session_remove_cel(snapshot.session,
                (base_bytes){(const uint8_t *)track_id, track_length},
                (uint32_t)tick);
    } else if (strcmp(tool, "native_timeline_move_item") == 0) {
        char track_id[257];
        size_t track_length = 0U;
        uint64_t from_tick = 0U, to_tick = 0U;
        bool from_present = false, to_present = false;
        bool valid = parse_track_id(request, args, track_id, &track_length) &&
            mcp_broker_u64_arg(request, args, "fromTick", &from_tick, &from_present) &&
            from_present && from_tick <= UINT32_MAX &&
            mcp_broker_u64_arg(request, args, "toTick", &to_tick, &to_present) &&
            to_present && to_tick <= UINT32_MAX;
        if (valid) {
            base_bytes before = project_timeline_session_source(snapshot.session);
            result = project_timeline_session_move_item(snapshot.session,
                (base_bytes){(const uint8_t *)track_id, track_length},
                (uint32_t)from_tick, (uint32_t)to_tick);
            base_bytes after = project_timeline_session_source(snapshot.session);
            changed = before.len != after.len ||
                (before.len != 0U && memcmp(before.ptr, after.ptr, before.len) != 0);
        }
    } else if (strcmp(tool, "native_timeline_undo") == 0) {
        result = project_timeline_session_undo(snapshot.session);
    } else {
        result = project_timeline_session_redo(snapshot.session);
    }
    snapshot_destroy(&snapshot);
    if (result.error != PROJECT_TIMELINE_OK)
        return mcp_broker_set_error(out, "native_timeline_edit_failed",
            project_timeline_error_name(result.error));
    if (changed && broker->host.native_timeline_changed != NULL)
        broker->host.native_timeline_changed(broker->host.context);
    return mcp_json_buffer_literal(&out->structured_json,
        changed ? "{\"changed\":true,\"savedAs\":\".vexel\"}" :
            "{\"changed\":false,\"savedAs\":\".vexel\"}");
}

static bool native_timeline_read(mcp_broker *broker,
                                 mcp_broker_document *document,
                                 mcp_tool_response *out)
{
    if (document != mcp_broker_current(broker) || !document->app_owned ||
        broker->host.current_timeline_session == NULL ||
        broker->host.current_project_bundle == NULL)
        return mcp_broker_set_error(out, "native_timeline_unavailable",
            "native timeline tools require the active app-owned .vexel document");
    project_timeline_session *session =
        broker->host.current_timeline_session(broker->host.context);
    const project_bundle_view *bundle =
        broker->host.current_project_bundle(broker->host.context);
    if (session == NULL || bundle == NULL ||
        bundle->version != PROJECT_BUNDLE_VERSION)
        return mcp_broker_set_error(out, "native_timeline_unavailable",
            "the active document has no editable native-v1 timeline session");
    base_bytes timeline = project_timeline_session_source(session);
    if (timeline.ptr == NULL || timeline.len == 0U ||
        timeline.len > NATIVE_TIMELINE_SOURCE_LIMIT)
        return mcp_broker_set_error(out, "native_timeline_source_limit",
            "native timeline source is empty or exceeds the 16 MiB MCP limit");
    bool okay = mcp_json_buffer_literal(&out->structured_json,
            "{\"documentId\":") &&
        mcp_json_buffer_quoted(&out->structured_json, document->id,
            strlen(document->id)) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"timeline\":") &&
        mcp_json_buffer_append(&out->structured_json,
            (const char *)timeline.ptr, timeline.len) &&
        mcp_json_buffer_literal(&out->structured_json, "}");
    if (okay) return true;
    out->structured_json.length = 0U;
    if (out->structured_json.data != NULL) out->structured_json.data[0] = '\0';
    return mcp_broker_set_error(out, "native_timeline_result_limit",
        "native timeline result exceeds the bounded MCP response capacity");
}

bool mcp_broker_native_timeline_call(mcp_broker *broker,
    mcp_broker_document *document, const mcp_json_doc *request, size_t args,
    const char *tool, mcp_tool_response *out)
{
    if (broker == NULL || document == NULL || request == NULL || tool == NULL ||
        out == NULL) return false;
    if (strcmp(tool, "native_timeline_get") != 0)
        return native_edit(broker, document, request, args, tool, out);
    return native_timeline_read(broker, document, out);
}
