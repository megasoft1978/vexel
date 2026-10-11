#include "mcp_broker_internal.h"

#include <stdlib.h>
#include <string.h>

static bool dispatch_tool(mcp_broker *broker, const mcp_json_doc *request,
                          size_t arguments, const char *tool_name,
                          mcp_tool_response *out)
{
    if (strcmp(tool_name, "render_png") == 0) {
        mcp_broker_document *document = mcp_broker_find(broker, request, arguments, "documentId");
        if (document == NULL) return mcp_broker_set_error(out, "document_not_found", "documentId is not open");
        return mcp_broker_render_png(broker, document, request, arguments, out);
    }
    if (strcmp(tool_name, "animation_list") == 0 || strcmp(tool_name, "animation_add") == 0 ||
        strcmp(tool_name, "animation_set") == 0 || strcmp(tool_name, "animation_remove") == 0 ||
        strcmp(tool_name, "export_animation") == 0)
        return mcp_broker_animation_call(broker, request, arguments, tool_name, out);
    if (strcmp(tool_name, "element_query") == 0 &&
        (mcp_json_object_get(request, arguments, "selector") != MCP_JSON_NO_TOKEN ||
         mcp_json_object_get(request, arguments, "hitTest") != MCP_JSON_NO_TOKEN))
        return mcp_broker_scene_call(broker, request, arguments, tool_name, out);
    if (strcmp(tool_name, "element_tree") == 0 ||
        strcmp(tool_name, "element_create") == 0 ||
        strcmp(tool_name, "element_set") == 0 || strcmp(tool_name, "element_delete") == 0 ||
        strcmp(tool_name, "element_duplicate") == 0 || strcmp(tool_name, "element_reorder") == 0 ||
        strcmp(tool_name, "element_group") == 0 || strcmp(tool_name, "element_ungroup") == 0 ||
        strcmp(tool_name, "transform_set") == 0 || strcmp(tool_name, "transform_apply") == 0 ||
        strcmp(tool_name, "style_fill") == 0 || strcmp(tool_name, "style_stroke") == 0 ||
        strcmp(tool_name, "style_opacity") == 0 || strcmp(tool_name, "gradient_create") == 0 ||
        strcmp(tool_name, "gradient_stop_add") == 0 || strcmp(tool_name, "gradient_stop_set") == 0 ||
        strcmp(tool_name, "gradient_stop_remove") == 0 || strcmp(tool_name, "path_from_commands") == 0 ||
        strcmp(tool_name, "path_append") == 0 || strcmp(tool_name, "path_edit_node") == 0 ||
        strcmp(tool_name, "element_align") == 0 || strcmp(tool_name, "element_distribute") == 0 ||
        strcmp(tool_name, "batch") == 0)
        return mcp_broker_scene_call(broker, request, arguments, tool_name, out);
    if (strcmp(tool_name, "document_list") == 0 || strcmp(tool_name, "document_open") == 0 ||
        strcmp(tool_name, "document_close") == 0 || strcmp(tool_name, "document_info") == 0 ||
        strcmp(tool_name, "tree_get") == 0 || strcmp(tool_name, "element_get") == 0 ||
        strcmp(tool_name, "element_query") == 0 ||
        strcmp(tool_name, "selection_get") == 0 ||
        strcmp(tool_name, "selection_set") == 0 || strcmp(tool_name, "export_svg") == 0 ||
        strcmp(tool_name, "save") == 0 || strcmp(tool_name, "save_as") == 0 ||
        strcmp(tool_name, "undo") == 0 || strcmp(tool_name, "redo") == 0)
        return mcp_broker_documents_call(broker, request, arguments, tool_name, out);
    return mcp_broker_edit_call(broker, request, arguments, tool_name, out);
}

static bool append_response_mode(mcp_tool_response *out, const char *mode)
{
    if (out == NULL || mode == NULL || out->structured_json.length == 0U) return false;
    mcp_json_doc response = {0};
    bool parsed = mcp_json_parse((const uint8_t *)out->structured_json.data,
        out->structured_json.length, &response);
    if (!parsed || response.root >= response.token_count ||
        response.tokens[response.root].type != MCP_JSON_OBJECT) {
        mcp_json_doc_destroy(&response);
        return false;
    }
    if (mcp_json_object_get(&response, response.root, "mode") != MCP_JSON_NO_TOKEN) {
        mcp_json_doc_destroy(&response);
        return true;
    }
    mcp_json_doc_destroy(&response);
    if (out->structured_json.limit < 32U ||
        out->structured_json.data[out->structured_json.length - 1U] != '}' ||
        out->structured_json.length > out->structured_json.limit - 32U) return false;

    mcp_json_buffer tagged = {0};
    if (!mcp_json_buffer_init(&tagged, out->structured_json.length + 32U,
            out->structured_json.limit)) return false;
    bool okay = mcp_json_buffer_append(&tagged, out->structured_json.data,
            out->structured_json.length - 1U) &&
        mcp_json_buffer_literal(&tagged, ",\"mode\":" ) &&
        mcp_json_buffer_quoted(&tagged, mode, strlen(mode)) &&
        mcp_json_buffer_literal(&tagged, "}" );
    if (okay) {
        mcp_json_buffer_destroy(&out->structured_json);
        out->structured_json = tagged;
    } else mcp_json_buffer_destroy(&tagged);
    return okay;
}

bool mcp_broker_call_tool(void *context, const mcp_json_doc *request,
                          size_t arguments, const char *tool_name,
                          mcp_tool_response *out)
{
    mcp_broker *broker = context;
    if (broker == NULL || request == NULL || tool_name == NULL || out == NULL) return false;
    mcp_broker_document *document = mcp_broker_find(broker, request, arguments,
        "documentId");
    if (document == NULL) document = mcp_broker_current(broker);
    const char *mode = document == NULL ?
        (broker->host.live_document_count == NULL ? "headless" : "live") :
        mcp_broker_document_mode(document);
    if (!dispatch_tool(broker, request, arguments, tool_name, out)) return false;
    return append_response_mode(out, mode);
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
