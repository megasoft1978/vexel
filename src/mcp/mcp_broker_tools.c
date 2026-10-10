#include "mcp_broker_internal.h"

#include <stdlib.h>
#include <string.h>

bool mcp_broker_call_tool(void *context, const mcp_json_doc *request,
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
