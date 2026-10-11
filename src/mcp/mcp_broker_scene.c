#include "mcp_broker_scene_internal.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MCP_SCENE_BATCH_LIMIT 32U
#define MCP_SCENE_SOURCE_LIMIT (64U * 1024U * 1024U)
#define MCP_SCENE_CHANGE_SPAN_LIMIT DOCUMENT_EDIT_XML_FRAGMENT_LIMIT
#define MCP_SCENE_PATH_REF_CAP 2048U

static const char *scene_error_message(document_edit_error error)
{
    switch (error) {
        case DOCUMENT_EDIT_INVALID_ARGUMENT: return "required values are missing, malformed, or outside their supported range";
        case DOCUMENT_EDIT_STALE_HANDLE: return "the target changed while the operation was being staged; query it again and retry";
        case DOCUMENT_EDIT_NOT_FOUND: return "the requested element or referenced object does not exist";
        case DOCUMENT_EDIT_DUPLICATE_ATTRIBUTE: return "the edit would create a duplicate XML attribute";
        case DOCUMENT_EDIT_INVALID_NAME: return "the attribute or style property is not supported";
        case DOCUMENT_EDIT_MALFORMED_XML: return "the generated SVG fragment is not valid XML";
        case DOCUMENT_EDIT_ROOT_DELETE: return "the SVG root element cannot be deleted";
        case DOCUMENT_EDIT_INVALID_OPERATION: return "the operation is not supported for this element or content";
        case DOCUMENT_EDIT_LIMIT: return "the operation exceeds a document, source-span, or item limit";
        case DOCUMENT_EDIT_HISTORY_LIMIT: return "the shared undo history is full";
        case DOCUMENT_EDIT_OUT_OF_MEMORY: return "there is not enough memory to stage this operation; no edit was committed";
        case DOCUMENT_EDIT_SAVE_FAILED: return "the document could not be saved";
        case DOCUMENT_EDIT_OK: return "no error";
    }
    return "scene operation failed without changing the document";
}

static bool append_u64(mcp_json_buffer *out, uint64_t value)
{
    char text[32];
    int amount = snprintf(text, sizeof(text), "%llu", (unsigned long long)value);
    return amount > 0 && (size_t)amount < sizeof(text) &&
        mcp_json_buffer_append(out, text, (size_t)amount);
}

static bool operation_keys_allowed(const mcp_json_doc *request, size_t object,
                                   const char *operation)
{
    const char *allowed = strcmp(operation, "element_create") == 0 ?
        "op;tag;parentId;index;attributes;style;text" :
        strcmp(operation, "element_set") == 0 ? "op;elementId;attributes;style" :
        strcmp(operation, "element_delete") == 0 || strcmp(operation, "element_duplicate") == 0 ||
        strcmp(operation, "element_group") == 0 || strcmp(operation, "element_ungroup") == 0 ?
        "op;elementId" :
        strcmp(operation, "element_reorder") == 0 ? "op;elementId;targetId;before" :
        strcmp(operation, "transform_set") == 0 ? "op;elementId;value" :
        strcmp(operation, "transform_apply") == 0 ? "op;elementId;operation;x;y;sx;sy;angle;cx;cy;a;b;c;d;e;f" :
        strcmp(operation, "path_from_commands") == 0 ? "op;parentId;commands" :
        strcmp(operation, "path_append") == 0 ? "op;elementId;commands" :
        strcmp(operation, "path_edit_node") == 0 ? "op;elementId;nodeIndex;values" :
        strcmp(operation, "style_fill") == 0 || strcmp(operation, "style_stroke") == 0 ||
        strcmp(operation, "style_opacity") == 0 ? "op;elementId;value" :
        strcmp(operation, "gradient_create") == 0 ? "op;parentId;kind;attributes;index" :
        strcmp(operation, "gradient_stop_add") == 0 ? "op;gradientId;index;offset;color;opacity" :
        strcmp(operation, "gradient_stop_set") == 0 ? "op;gradientId;index;offset;color;opacity" :
        strcmp(operation, "gradient_stop_remove") == 0 ? "op;gradientId;index" :
        strcmp(operation, "element_align") == 0 ? "op;elementIds;axis;alignment" :
        strcmp(operation, "element_distribute") == 0 ? "op;elementIds;axis" : "";
    if (allowed[0] == '\0') return false;
    size_t key = request->tokens[object].first_child;
    while (key != MCP_JSON_NO_TOKEN) {
        size_t value = request->tokens[key].next_sibling;
        char name[64]; size_t name_length = 0U;
        if (value == MCP_JSON_NO_TOKEN || !mcp_json_string_copy(request, key,
                name, sizeof(name) - 1U, &name_length)) return false;
        name[name_length] = '\0';
        bool found = false;
        const char *cursor = allowed;
        while (*cursor != '\0') {
            const char *end = strchr(cursor, ';');
            size_t length = end == NULL ? strlen(cursor) : (size_t)(end - cursor);
            if (length == name_length && memcmp(cursor, name, length) == 0) { found = true; break; }
            if (end == NULL) break;
            cursor = end + 1;
        }
        if (!found) return false;
        key = request->tokens[value].next_sibling;
    }
    size_t op_token = mcp_json_object_get(request, object, "op");
    if (op_token == MCP_JSON_NO_TOKEN || request->tokens[op_token].type != MCP_JSON_STRING) return false;
    return true;
}

static bool copy_document_edit(mcp_broker_document *document,
                               mcp_broker_document *scratch,
                               uint8_t **out_source, size_t *out_length)
{
    size_t length = document_edit_byte_length(document->edit);
    if (length > MCP_SCENE_SOURCE_LIMIT) return false;
    uint8_t *source = malloc(length == 0U ? 1U : length);
    if (source == NULL || !document_edit_materialize(document->edit, source,
            length, &length)) { free(source); return false; }
    document_edit_result created = document_edit_create((base_bytes){source, length},
        document_edit_default_limits());
    if (created.error != DOCUMENT_EDIT_OK || created.edit == NULL) { free(source); return false; }
    *scratch = *document;
    scratch->edit = created.edit;
    scratch->owned_source = source;
    scratch->owned_source_length = length;
    *out_source = source;
    *out_length = length;
    return true;
}

static void destroy_scratch(mcp_broker_document *scratch, uint8_t *source)
{
    if (scratch != NULL) document_edit_destroy(scratch->edit);
    free(source);
}

static bool commit_staged(mcp_broker *broker, mcp_broker_document *document,
    mcp_broker_document *scratch, const uint8_t *original, size_t original_length,
    const char *mutation_kind, const char *last_id, bool *out_changed,
    mcp_tool_response *out)
{
    size_t final_length = document_edit_byte_length(scratch->edit);
    if (final_length > MCP_SCENE_SOURCE_LIMIT)
        { (void)mcp_broker_set_error(out, "document_limit", "scene edit would exceed the 64 MiB document limit"); return false; }
    uint8_t *final_source = malloc(final_length == 0U ? 1U : final_length);
    if (final_source == NULL || !document_edit_materialize(scratch->edit,
            final_source, final_length, &final_length)) {
        free(final_source);
        { (void)mcp_broker_set_error(out, "source_unavailable", "staged source could not be materialized"); return false; }
    }
    size_t prefix = 0U;
    while (prefix < original_length && prefix < final_length && original[prefix] == final_source[prefix]) prefix++;
    size_t suffix = 0U;
    while (suffix < original_length - prefix && suffix < final_length - prefix &&
        original[original_length - suffix - 1U] == final_source[final_length - suffix - 1U]) suffix++;
    size_t removed = original_length - prefix - suffix;
    size_t inserted = final_length - prefix - suffix;
    bool changed = removed != 0U || inserted != 0U;
    if (out_changed != NULL) *out_changed = changed;
    if (removed > MCP_SCENE_CHANGE_SPAN_LIMIT || inserted > MCP_SCENE_CHANGE_SPAN_LIMIT) {
        free(final_source);
        { (void)mcp_broker_set_error(out, "batch_span_limit", "scene edit changes exceed the 64 KiB atomic source-span limit"); return false; }
    }
    document_edit_error error = DOCUMENT_EDIT_OK;
    if (changed)
        error = document_edit_replace_source_span(document->edit,
            (document_source_span){prefix, removed},
            (base_bytes){final_source + prefix, inserted});
    free(final_source);
    if (error != DOCUMENT_EDIT_OK) {
        (void)mcp_broker_set_error(out, document_edit_error_name(error),
            "staged scene edit could not be committed through document_edit");
        return false;
    }
    if (changed) {
        document_handle selection = {0};
        bool has_selection = last_id != NULL && document_edit_find_id(document->edit,
            (base_bytes){(const uint8_t *)last_id, strlen(last_id)}, &selection);
        if (!has_selection && !document_edit_root(document->edit, &selection)) {
            (void)mcp_broker_set_error(out, "document_invalid", "scene edit committed but its selection could not be rebound");
            return false;
        }
        mcp_broker_mutation_kind kind = strcmp(mutation_kind, "undo") == 0 ?
            MCP_BROKER_MUTATION_UNDO : strcmp(mutation_kind, "redo") == 0 ?
            MCP_BROKER_MUTATION_REDO : strcmp(mutation_kind, "transaction") == 0 ?
            MCP_BROKER_MUTATION_TRANSACTION : MCP_BROKER_MUTATION_EDIT;
        mcp_broker_after_edit(broker, document, selection, kind, true);
    }
    return true;
}

static bool scene_single(mcp_broker *broker, mcp_broker_document *document,
    const mcp_json_doc *request, size_t args, const char *operation,
    mcp_tool_response *out)
{
    mcp_broker_document scratch; uint8_t *original = NULL; size_t original_length = 0U;
    if (!copy_document_edit(document, &scratch, &original, &original_length))
        return mcp_broker_set_error(out, "source_unavailable", "document snapshot could not be staged");
    document_handle changed = {0}; bool created = false;
    document_edit_error error = mcp_broker_scene_apply_operation(&scratch, request,
        args, operation, &changed, &created);
    if (error != DOCUMENT_EDIT_OK) {
        destroy_scratch(&scratch, original);
        const char *name = document_edit_error_name(error);
        return mcp_broker_set_error(out, name == NULL ? "scene_edit_failed" : name,
            scene_error_message(error));
    }
    char result_id[256]; size_t result_id_length = 0U;
    bool result_id_present = false;
    if (!mcp_scene_attribute_text(&scratch, changed, "id",
            result_id, sizeof(result_id), &result_id_length, &result_id_present)) result_id_present = false;
    if (!result_id_present) result_id_length = 0U;
    if (result_id_length < sizeof(result_id)) result_id[result_id_length] = '\0';
    char last_id[256];
    if (result_id_present) memcpy(last_id, result_id, result_id_length + 1U);
    else last_id[0] = '\0';
    bool did_change = false;
    if (!commit_staged(broker, document, &scratch, original, original_length,
            operation, last_id[0] == '\0' ? NULL : last_id, &did_change, out)) {
        destroy_scratch(&scratch, original);
        return out->is_error;
    }
    if (!mcp_json_buffer_literal(&out->structured_json, "{\"documentId\":" ) ||
        !mcp_json_buffer_quoted(&out->structured_json, document->id, strlen(document->id)) ||
        !mcp_json_buffer_literal(&out->structured_json, ",\"mode\":" ) ||
        !mcp_json_buffer_quoted(&out->structured_json, mcp_broker_document_mode(document),
                               strlen(mcp_broker_document_mode(document))) ||
        !mcp_json_buffer_literal(&out->structured_json, ",\"revision\":" ) ||
        !append_u64(&out->structured_json, (uint64_t)document_edit_history_cursor(document->edit))) {
        destroy_scratch(&scratch, original);
        return false;
    }
    if (result_id_present && (!mcp_json_buffer_literal(&out->structured_json, ",\"elementId\":" ) ||
        !mcp_json_buffer_quoted(&out->structured_json, result_id, result_id_length))) {
        destroy_scratch(&scratch, original);
        return false;
    }
    bool okay = created ? mcp_json_buffer_literal(&out->structured_json, ",\"created\":true}") :
        mcp_json_buffer_literal(&out->structured_json, did_change ? ",\"changed\":true}" : ",\"changed\":false}");
    destroy_scratch(&scratch, original);
    return okay;
}

static bool batch_error(mcp_tool_response *out, size_t index,
                        const char *operation, const char *cause)
{
    char message[256];
    int amount = snprintf(message, sizeof(message),
        "operation %zu (%s) failed: %s; the batch was rolled back",
        index, operation == NULL ? "unknown" : operation,
        cause == NULL ? "invalid operation" : cause);
    if (amount < 0 || (size_t)amount >= sizeof(message))
        return mcp_broker_set_error(out, "batch_rolled_back", "a scene batch operation failed; the batch was rolled back");
    return mcp_broker_set_error(out, "batch_rolled_back", message);
}

static bool scene_batch(mcp_broker *broker, const mcp_json_doc *request,
                        size_t args, mcp_tool_response *out)
{
    mcp_broker_document *document = mcp_broker_find(broker, request, args, "documentId");
    if (document == NULL) return mcp_broker_set_error(out, "document_not_found", "documentId is not open");
    size_t operations = mcp_json_object_get(request, args, "operations");
    if (operations == MCP_JSON_NO_TOKEN || request->tokens[operations].type != MCP_JSON_ARRAY ||
        request->tokens[operations].child_count == 0U ||
        request->tokens[operations].child_count > MCP_SCENE_BATCH_LIMIT)
        return mcp_broker_set_error(out, "invalid_batch", "batch requires 1 to 32 operation objects");
    mcp_broker_document scratch; uint8_t *original = NULL; size_t original_length = 0U;
    if (!copy_document_edit(document, &scratch, &original, &original_length))
        return mcp_broker_set_error(out, "source_unavailable", "document snapshot could not be staged");
    mcp_json_buffer results = {0};
    if (!mcp_json_buffer_init(&results, 256U, 64U * 1024U)) {
        destroy_scratch(&scratch, original);
        return mcp_broker_set_error(out, "out_of_memory", "batch result allocation failed");
    }
    bool okay = mcp_json_buffer_literal(&results, "[");
    size_t operation_token = request->tokens[operations].first_child;
    size_t count = request->tokens[operations].child_count;
    char last_id[256] = {0};
    for (size_t index = 0U; okay && index < count; index++) {
        if (operation_token == MCP_JSON_NO_TOKEN || request->tokens[operation_token].type != MCP_JSON_OBJECT) {
            okay = false;
            (void)batch_error(out, index, "unknown", "operation must be an object");
            break;
        }
        char operation[32]; size_t operation_length = 0U;
        if (!mcp_scene_string_arg(request, operation_token, "op", operation,
                sizeof(operation), &operation_length) ||
            !operation_keys_allowed(request, operation_token, operation)) {
            okay = false;
            (void)batch_error(out, index, "unknown", "operation name or fields are invalid");
            break;
        }
        document_handle result_element = {0}; bool created = false;
        document_edit_error error = mcp_broker_scene_apply_operation(&scratch,
            request, operation_token, operation, &result_element, &created);
        if (error != DOCUMENT_EDIT_OK) {
            okay = false;
            (void)batch_error(out, index, operation, scene_error_message(error));
            break;
        }
        char result_id[256]; size_t result_id_length = 0U; bool has_id = false;
        if (mcp_scene_attribute_text(&scratch, result_element, "id", result_id,
                sizeof(result_id), &result_id_length, &has_id) && has_id &&
            result_id_length < sizeof(result_id)) {
            result_id[result_id_length] = '\0';
            memcpy(last_id, result_id, result_id_length + 1U);
        } else has_id = false;
        if (index > 0U) okay = mcp_json_buffer_literal(&results, ",");
        okay = okay && mcp_json_buffer_literal(&results, "{\"index\":" ) &&
            append_u64(&results, (uint64_t)index) &&
            mcp_json_buffer_literal(&results, ",\"op\":" ) &&
            mcp_json_buffer_quoted(&results, operation, operation_length) &&
            mcp_json_buffer_literal(&results, ",\"applied\":true");
        if (okay && has_id) okay = mcp_json_buffer_literal(&results, ",\"elementId\":" ) &&
            mcp_json_buffer_quoted(&results, result_id, result_id_length);
        if (okay && created) okay = mcp_json_buffer_literal(&results, ",\"created\":true");
        if (okay) okay = mcp_json_buffer_literal(&results, "}");
        operation_token = request->tokens[operation_token].next_sibling;
    }
    if (okay) okay = mcp_json_buffer_literal(&results, "]");
    if (!okay) {
        mcp_json_buffer_destroy(&results);
        destroy_scratch(&scratch, original);
        if (!out->is_error) return mcp_broker_set_error(out, "batch_rolled_back", "scene batch failed; the batch was rolled back");
        return true;
    }
    bool changed = false;
    if (!commit_staged(broker, document, &scratch, original, original_length,
            "batch", last_id[0] == '\0' ? NULL : last_id, &changed, out)) {
        mcp_json_buffer_destroy(&results);
        destroy_scratch(&scratch, original);
        return out->is_error;
    }
    bool written = mcp_json_buffer_literal(&out->structured_json, "{\"documentId\":" ) &&
        mcp_json_buffer_quoted(&out->structured_json, document->id, strlen(document->id)) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"mode\":" ) &&
        mcp_json_buffer_quoted(&out->structured_json, mcp_broker_document_mode(document),
                               strlen(mcp_broker_document_mode(document))) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"revision\":" ) &&
        append_u64(&out->structured_json,
            (uint64_t)document_edit_history_cursor(document->edit)) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"applied\":" ) &&
        append_u64(&out->structured_json, (uint64_t)count) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"atomic\":true,\"undoSteps\":" ) &&
        mcp_json_buffer_literal(&out->structured_json, changed ? "1" : "0") &&
        mcp_json_buffer_literal(&out->structured_json, ",\"results\":" ) &&
        mcp_json_buffer_append(&out->structured_json, results.data, results.length) &&
        mcp_json_buffer_literal(&out->structured_json, "}" );
    mcp_json_buffer_destroy(&results);
    destroy_scratch(&scratch, original);
    return written;
}

bool mcp_broker_scene_call(mcp_broker *broker, const mcp_json_doc *request,
                           size_t args, const char *tool_name,
                           mcp_tool_response *out)
{
    if (broker == NULL || request == NULL || tool_name == NULL || out == NULL) return false;
    if (strcmp(tool_name, "element_tree") == 0 || strcmp(tool_name, "element_get") == 0 ||
        strcmp(tool_name, "element_query") == 0)
        return mcp_broker_scene_query_call(broker, request, args, tool_name, out);
    if (strcmp(tool_name, "batch") == 0) return scene_batch(broker, request, args, out);
    mcp_broker_document *document = mcp_broker_find(broker, request, args, "documentId");
    if (document == NULL) return mcp_broker_set_error(out, "document_not_found", "documentId is not open");
    return scene_single(broker, document, request, args, tool_name, out);
}
