#include "mcp_broker_internal.h"

#include <stdlib.h>
#include <string.h>

static bool path_has_vexel_extension(const char *path, size_t length)
{
    static const char extension[] = "vexel";
    if (path == NULL || length <= sizeof(extension)) return false;
    size_t start = length - (sizeof(extension) - 1U);
    if (path[start - 1U] != '.') return false;
    for (size_t index = 0U; index < sizeof(extension) - 1U; ++index) {
        char value = path[start + index];
        if (value >= 'A' && value <= 'Z') value = (char)(value - 'A' + 'a');
        if (value != extension[index]) return false;
    }
    return true;
}

static bool save_active_project(mcp_broker *broker, const mcp_json_doc *request,
                                size_t arguments, const char *tool_name,
                                mcp_tool_response *out)
{
    mcp_broker_document *document = mcp_broker_find(broker, request,
        arguments, "documentId");
    if (document == NULL || !document->app_owned ||
        document != mcp_broker_current(broker) ||
        broker->host.current_project_bundle == NULL ||
        broker->host.current_project_bundle(broker->host.context) == NULL)
        return false;
    if (broker->host.save_current_project == NULL)
        return mcp_broker_set_error(out, "project_save_unavailable",
            "the active project save route is unavailable");
    bool save_as = strcmp(tool_name, "save_as") == 0;
    char resolved[MCP_BROKER_PATH_CAP];
    const char *path = document->path;
    size_t path_length = strlen(path);
    if (save_as) {
        char requested[MCP_BROKER_PATH_CAP];
        size_t requested_length = 0U;
        if (!mcp_broker_string_arg(request, arguments, "path", requested,
                sizeof(requested), &requested_length) || requested_length == 0U ||
            memchr(requested, '\0', requested_length) != NULL ||
            !mcp_access_resolve_save_path(requested, resolved))
            return mcp_broker_set_error(out, "path_not_granted",
                "Save As path must be inside a user-granted directory and have an existing parent directory");
        path = resolved;
        path_length = strlen(path);
    }
    if (!broker->host.save_current_project(broker->host.context, path,
            path_length, save_as))
        return mcp_broker_set_error(out, "project_save_failed",
            "Vexel could not save the active project in the selected format");
    if (save_as) memcpy(document->path, path, path_length + 1U);
    return mcp_json_buffer_literal(&out->structured_json,
        path_has_vexel_extension(path, path_length) ?
        "{\"saved\":true,\"format\":\"vexel\"}" :
        "{\"saved\":true,\"format\":\"svg\"}");
}

static bool mcp_broker_dispatch_tool(void *context,
                                    const mcp_json_doc *request,
                                    size_t arguments, const char *tool_name,
                                    mcp_tool_response *out)
{
    mcp_broker *broker = context;
    if (broker == NULL || request == NULL || tool_name == NULL || out == NULL) return false;
    if (strcmp(tool_name, "render_png") == 0) {
        mcp_broker_document *document = mcp_broker_find(broker, request, arguments, "documentId");
        if (document == NULL) return mcp_broker_set_error(out, "document_not_found", "documentId is not open");
        return mcp_broker_render_png(broker, document, request, arguments, out);
    }
    if (strcmp(tool_name, "animation_list") == 0 || strcmp(tool_name, "animation_add") == 0 ||
        strcmp(tool_name, "animation_set") == 0 || strcmp(tool_name, "animation_remove") == 0 ||
        strcmp(tool_name, "export_animation") == 0)
        return mcp_broker_animation_call(broker, request, arguments, tool_name, out);
    if (strcmp(tool_name, "native_timeline_get") == 0 ||
        strcmp(tool_name, "native_timeline_add_track") == 0 ||
        strcmp(tool_name, "native_timeline_add_drawing_asset") == 0 ||
        strcmp(tool_name, "native_timeline_add_property_track") == 0 ||
        strcmp(tool_name, "native_timeline_set_property_key") == 0 ||
        strcmp(tool_name, "native_timeline_set_loop") == 0 ||
        strcmp(tool_name, "native_timeline_set_cel") == 0 ||
        strcmp(tool_name, "native_timeline_move_item") == 0 ||
        strcmp(tool_name, "native_timeline_remove_cel") == 0 ||
        strcmp(tool_name, "native_timeline_undo") == 0 ||
        strcmp(tool_name, "native_timeline_redo") == 0) {
        mcp_broker_document *document = mcp_broker_find(broker, request,
            arguments, "documentId");
        if (document == NULL)
            return mcp_broker_set_error(out, "document_not_found",
                "documentId is not open");
        return mcp_broker_native_timeline_call(broker, document, request,
            arguments, tool_name, out);
    }
    if (strcmp(tool_name, "save") == 0 || strcmp(tool_name, "save_as") == 0) {
        bool routed = save_active_project(broker, request, arguments,
            tool_name, out);
        if (routed) return true;
        if (out->is_error) return true;
    }
    if (strcmp(tool_name, "document_list") == 0 || strcmp(tool_name, "document_open") == 0 ||
        strcmp(tool_name, "document_close") == 0 || strcmp(tool_name, "document_info") == 0 ||
        strcmp(tool_name, "tree_get") == 0 || strcmp(tool_name, "element_get") == 0 ||
        strcmp(tool_name, "element_query") == 0 || strcmp(tool_name, "selection_get") == 0 ||
        strcmp(tool_name, "selection_set") == 0 || strcmp(tool_name, "export_svg") == 0 ||
        strcmp(tool_name, "save") == 0 || strcmp(tool_name, "save_as") == 0 ||
        strcmp(tool_name, "undo") == 0 || strcmp(tool_name, "redo") == 0)
        return mcp_broker_documents_call(broker, request, arguments, tool_name, out);
    return mcp_broker_edit_call(broker, request, arguments, tool_name, out);
}

bool mcp_broker_call_tool(void *context, const mcp_json_doc *request,
                          size_t arguments, const char *tool_name,
                          mcp_tool_response *out)
{
    mcp_broker *broker = context;
    if (broker == NULL || request == NULL || tool_name == NULL || out == NULL)
        return false;
    bool dispatched = mcp_broker_dispatch_tool(context, request, arguments,
                                                tool_name, out);
    if (broker->host.agent_action_completed != NULL)
        broker->host.agent_action_completed(broker->host.context, tool_name,
                                            dispatched && !out->is_error);
    return dispatched;
}

bool mcp_broker_read_resource(void *context, const mcp_json_doc *request,
                              const char *uri, mcp_json_buffer *out_contents)
{
    mcp_broker *broker = context;
    if (broker == NULL || request == NULL || uri == NULL || out_contents == NULL) return false;
    mcp_broker_document *document = mcp_broker_current(broker);
    if (document == NULL) return false;
    if (strcmp(uri, "vexel://documents/current/source") == 0) {
        uint8_t *source = NULL;
        size_t length = 0U;
        if (document_edit_byte_length(document->edit) > 1024U * 1024U) return false;
        length = document_edit_byte_length(document->edit);
        source = malloc(length == 0U ? 1U : length);
        if (source == NULL || !document_edit_materialize(document->edit, source, length, &length)) {
            free(source);
            return false;
        }
        bool okay = mcp_json_buffer_literal(out_contents,
            "[{\"uri\":\"vexel://documents/current/source\",\"mimeType\":\"image/svg+xml\",\"text\":" ) &&
            mcp_json_buffer_quoted(out_contents, (const char *)source, length) &&
            mcp_json_buffer_literal(out_contents, "}]" );
        free(source);
        return okay;
    }
    if (strcmp(uri, "vexel://documents/current/tree") == 0) {
        mcp_json_buffer tree = {0};
        if (!mcp_json_buffer_init(&tree, 256U, MCP_JSON_MAX_BYTES)) return false;
        bool okay = mcp_broker_tree_json(document, 0U, 128U, &tree) &&
            mcp_json_buffer_literal(out_contents,
                "[{\"uri\":\"vexel://documents/current/tree\",\"mimeType\":\"application/json\",\"text\":" ) &&
            mcp_json_buffer_quoted(out_contents, tree.data, tree.length) &&
            mcp_json_buffer_literal(out_contents, "}]" );
        mcp_json_buffer_destroy(&tree);
        return okay;
    }
    if (strcmp(uri, "vexel://documents/current/render") == 0) {
        if (document->last_render_base64 == NULL || document->last_render_length == 0U) return false;
        return mcp_json_buffer_literal(out_contents,
            "[{\"uri\":\"vexel://documents/current/render\",\"mimeType\":\"image/png\",\"blob\":" ) &&
            mcp_json_buffer_quoted(out_contents, document->last_render_base64,
                                   document->last_render_length) &&
            mcp_json_buffer_literal(out_contents, "}]" );
    }
    return false;
}
