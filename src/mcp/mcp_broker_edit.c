#define _POSIX_C_SOURCE 200809L

#include "mcp_broker_internal.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MCP_TEMPLATE_ATTRIBUTE_CAP 64U
#define MCP_TEMPLATE_NAME_CAP 128U
#define MCP_TEMPLATE_VALUE_CAP 1024U

typedef struct template_args {
    document_edit_template_attribute attributes[MCP_TEMPLATE_ATTRIBUTE_CAP];
    char names[MCP_TEMPLATE_ATTRIBUTE_CAP][MCP_TEMPLATE_NAME_CAP];
    char values[MCP_TEMPLATE_ATTRIBUTE_CAP][MCP_TEMPLATE_VALUE_CAP];
    size_t count;
    document_edit_template_kind kind;
} template_args;

static bool get_string(const mcp_json_doc *request, size_t args, const char *name,
                       char *buffer, size_t capacity, size_t *length)
{
    return mcp_broker_string_arg(request, args, name, buffer, capacity, length);
}

static bool resolve_arg(mcp_broker_document *document, const mcp_json_doc *request,
                        size_t args, const char *field, document_handle *out)
{
    char id[256];
    size_t length = 0U;
    return get_string(request, args, field, id, sizeof(id), &length) &&
        mcp_broker_resolve_element(document, id, length, out);
}

static bool template_from_string(const char *name, document_edit_template_kind *out)
{
    if (strcmp(name, "rect") == 0) *out = DOCUMENT_EDIT_TEMPLATE_RECT;
    else if (strcmp(name, "circle") == 0) *out = DOCUMENT_EDIT_TEMPLATE_CIRCLE;
    else if (strcmp(name, "ellipse") == 0) *out = DOCUMENT_EDIT_TEMPLATE_ELLIPSE;
    else if (strcmp(name, "line") == 0) *out = DOCUMENT_EDIT_TEMPLATE_LINE;
    else if (strcmp(name, "path") == 0) *out = DOCUMENT_EDIT_TEMPLATE_PATH;
    else if (strcmp(name, "g") == 0) *out = DOCUMENT_EDIT_TEMPLATE_GROUP;
    else return false;
    return true;
}

static document_edit_error collect_template_args(const mcp_json_doc *request, size_t args,
                                                 template_args *out,
                                                 char parent_id[256], size_t *parent_length)
{
    if (out == NULL || parent_id == NULL || parent_length == NULL) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    char template_name[32];
    size_t template_length = 0U;
    if (!get_string(request, args, "template", template_name, sizeof(template_name), &template_length) ||
        !template_from_string(template_name, &out->kind)) return DOCUMENT_EDIT_INVALID_OPERATION;
    if (!get_string(request, args, "parentId", parent_id, 256U, parent_length))
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    size_t attributes = mcp_json_object_get(request, args, "attributes");
    if (attributes == MCP_JSON_NO_TOKEN) return DOCUMENT_EDIT_OK;
    size_t key = request->tokens[attributes].first_child;
    while (key != MCP_JSON_NO_TOKEN) {
        size_t value = request->tokens[key].next_sibling;
        if (value == MCP_JSON_NO_TOKEN || out->count >= MCP_TEMPLATE_ATTRIBUTE_CAP)
            return DOCUMENT_EDIT_LIMIT;
        size_t name_length = 0U, value_length = 0U;
        if (!mcp_json_string_copy(request, key, out->names[out->count],
                sizeof(out->names[0]) - 1U, &name_length) ||
            !mcp_json_string_copy(request, value, out->values[out->count],
                sizeof(out->values[0]) - 1U, &value_length) ||
            memchr(out->names[out->count], '\0', name_length) != NULL ||
            memchr(out->values[out->count], '\0', value_length) != NULL)
            return DOCUMENT_EDIT_INVALID_ARGUMENT;
        out->names[out->count][name_length] = '\0';
        out->values[out->count][value_length] = '\0';
        out->attributes[out->count] = (document_edit_template_attribute){
            {(const uint8_t *)out->names[out->count], name_length},
            {(const uint8_t *)out->values[out->count], value_length}};
        out->count++;
        key = request->tokens[value].next_sibling;
    }
    (void)template_length;
    return DOCUMENT_EDIT_OK;
}

static bool find_attribute(mcp_broker_document *document, document_handle handle,
                           const char *name, size_t name_length)
{
    size_t count = document_edit_attribute_count(document->edit, handle);
    for (size_t index = 0U; index < count; index++) {
        document_edit_attribute attribute;
        base_bytes bytes = {0};
        if (document_edit_attribute_at(document->edit, handle, index, &attribute) &&
            document_edit_span_bytes(document->edit, attribute.name, &bytes) &&
            bytes.len == name_length && memcmp(bytes.ptr, name, name_length) == 0) return true;
    }
    return false;
}

static document_edit_error apply_template(mcp_broker_document *document,
                                          const mcp_json_doc *request, size_t args,
                                          document_handle *out_inserted)
{
    template_args template_value = {0};
    char parent_id[256];
    size_t parent_length = 0U;
    document_edit_error error = collect_template_args(request, args, &template_value,
                                                       parent_id, &parent_length);
    if (error != DOCUMENT_EDIT_OK) return error;
    document_handle parent;
    if (!mcp_broker_resolve_element(document, parent_id, parent_length, &parent))
        return DOCUMENT_EDIT_NOT_FOUND;
    return document_edit_insert_template(document->edit, parent, template_value.kind,
        template_value.attributes, template_value.count, out_inserted);
}

static document_edit_error apply_simple_operation(mcp_broker_document *document,
                                                  const mcp_json_doc *request, size_t args,
                                                  const char *operation,
                                                  document_handle *out_selection)
{
    document_handle target = {0}, result = {0};
    document_edit_error error = DOCUMENT_EDIT_INVALID_OPERATION;
    if (strcmp(operation, "element_add") == 0) {
        error = apply_template(document, request, args, &result);
        if (error == DOCUMENT_EDIT_OK) *out_selection = result;
        return error;
    }
    if (!resolve_arg(document, request, args, "elementId", &target)) return DOCUMENT_EDIT_NOT_FOUND;
    if (strcmp(operation, "attribute_set") == 0) {
        char name[128], value[2048];
        size_t name_length = 0U, value_length = 0U;
        if (!get_string(request, args, "name", name, sizeof(name), &name_length) ||
            !get_string(request, args, "value", value, sizeof(value), &value_length))
            return DOCUMENT_EDIT_INVALID_ARGUMENT;
        base_bytes name_bytes = {(const uint8_t *)name, name_length};
        base_bytes value_bytes = {(const uint8_t *)value, value_length};
        error = find_attribute(document, target, name, name_length) ?
            document_edit_set_attribute(document->edit, target, name_bytes, value_bytes) :
            document_edit_add_attribute(document->edit, target, name_bytes, value_bytes);
    } else if (strcmp(operation, "attribute_remove") == 0) {
        char name[128];
        size_t name_length = 0U;
        if (!get_string(request, args, "name", name, sizeof(name), &name_length))
            return DOCUMENT_EDIT_INVALID_ARGUMENT;
        error = document_edit_remove_attribute(document->edit, target,
            (base_bytes){(const uint8_t *)name, name_length});
    } else if (strcmp(operation, "text_set") == 0) {
        char value[8192];
        size_t value_length = 0U;
        if (!get_string(request, args, "text", value, sizeof(value), &value_length))
            return DOCUMENT_EDIT_INVALID_ARGUMENT;
        error = document_edit_set_text_content(document->edit, target,
            (base_bytes){(const uint8_t *)value, value_length});
    } else if (strcmp(operation, "element_delete") == 0) {
        error = document_edit_delete_element(document->edit, target);
        if (error == DOCUMENT_EDIT_OK) (void)document_edit_root(document->edit, &result);
    } else if (strcmp(operation, "element_duplicate") == 0) {
        error = document_edit_duplicate_element(document->edit, target, &result);
    } else if (strcmp(operation, "element_group") == 0) {
        error = document_edit_wrap_in_group(document->edit, target, &result);
    } else if (strcmp(operation, "element_ungroup") == 0) {
        error = document_edit_ungroup(document->edit, target);
        if (error == DOCUMENT_EDIT_OK) (void)document_edit_root(document->edit, &result);
    } else if (strcmp(operation, "element_reorder") == 0) {
        document_handle sibling;
        bool before = false;
        if (!resolve_arg(document, request, args, "targetId", &sibling) ||
            !mcp_broker_bool_arg(request, args, "before", &before)) return DOCUMENT_EDIT_INVALID_ARGUMENT;
        error = document_edit_reorder_sibling(document->edit, target, sibling, before);
        result = target;
    } else return DOCUMENT_EDIT_INVALID_OPERATION;
    if (error == DOCUMENT_EDIT_OK && result.document_id == 0U) result = target;
    if (error == DOCUMENT_EDIT_OK) *out_selection = result;
    return error;
}

static bool edit_error(mcp_tool_response *out, document_edit_error error)
{
    const char *name = document_edit_error_name(error);
    return mcp_broker_set_error(out, name == NULL ? "edit_failed" : name,
        name == NULL ? "document edit failed" : name);
}

static bool direct_edit(mcp_broker *broker, const mcp_json_doc *request,
                        size_t args, const char *operation, mcp_tool_response *out)
{
    mcp_broker_document *document = mcp_broker_find(broker, request, args, "documentId");
    if (document == NULL) return mcp_broker_set_error(out, "document_not_found", "documentId is not open");
    document_handle selection = {0};
    document_edit_error error = apply_simple_operation(document, request, args, operation, &selection);
    if (error != DOCUMENT_EDIT_OK) return edit_error(out, error);
    base_bytes selected_tag = {0};
    if (!document_edit_tag_name(document->edit, selection, &selected_tag) &&
        !document_edit_root(document->edit, &selection))
        return mcp_broker_set_error(out, "document_invalid", "document edit completed but the selection could not be rebound");
    mcp_broker_after_edit(broker, document, selection);
    if (!mcp_json_buffer_literal(&out->structured_json, "{\"documentId\":" ) ||
        !mcp_json_buffer_quoted(&out->structured_json, document->id, strlen(document->id)) ||
        !mcp_json_buffer_literal(&out->structured_json, ",\"elementId\":" ) ||
        !mcp_broker_element_id(document, selection, &out->structured_json)) return false;
    if (strcmp(operation, "element_add") == 0 || strcmp(operation, "element_duplicate") == 0)
        if (!mcp_json_buffer_literal(&out->structured_json, ",\"created\":true")) return false;
    return mcp_json_buffer_literal(&out->structured_json, "}" );
}

static bool transaction_key_allowed(const mcp_json_doc *doc, size_t object,
                                    const char *operation)
{
    const char *allowed = strcmp(operation, "attribute_set") == 0 ? "op;elementId;name;value" :
        strcmp(operation, "attribute_remove") == 0 ? "op;elementId;name" :
        strcmp(operation, "text_set") == 0 ? "op;elementId;text" :
        strcmp(operation, "element_delete") == 0 ? "op;elementId" :
        strcmp(operation, "element_add") == 0 ? "op;parentId;template;attributes" :
        strcmp(operation, "element_duplicate") == 0 || strcmp(operation, "element_group") == 0 ||
        strcmp(operation, "element_ungroup") == 0 ? "op;elementId" :
        strcmp(operation, "element_reorder") == 0 ? "op;elementId;targetId;before" : "";
    if (allowed[0] == '\0') return false;
    size_t key = doc->tokens[object].first_child;
    while (key != MCP_JSON_NO_TOKEN) {
        char name[64];
        size_t length = 0U;
        size_t value = doc->tokens[key].next_sibling;
        if (value == MCP_JSON_NO_TOKEN ||
            !mcp_json_string_copy(doc, key, name, sizeof(name) - 1U, &length)) return false;
        name[length] = '\0';
        bool found = false;
        const char *cursor = allowed;
        while (*cursor != '\0') {
            const char *end = strchr(cursor, ';');
            size_t field_length = end == NULL ? strlen(cursor) : (size_t)(end - cursor);
            if (field_length == length && memcmp(cursor, name, length) == 0) { found = true; break; }
            if (end == NULL) break;
            cursor = end + 1;
        }
        if (!found) return false;
        key = doc->tokens[value].next_sibling;
    }
    const char *required = strcmp(operation, "attribute_set") == 0 ? "op;elementId;name;value" :
        strcmp(operation, "attribute_remove") == 0 ? "op;elementId;name" :
        strcmp(operation, "text_set") == 0 ? "op;elementId;text" :
        strcmp(operation, "element_add") == 0 ? "op;parentId;template" :
        strcmp(operation, "element_reorder") == 0 ? "op;elementId;targetId;before" : "op;elementId";
    for (const char *cursor = required; *cursor != '\0';) {
        const char *end = strchr(cursor, ';');
        size_t length = end == NULL ? strlen(cursor) : (size_t)(end - cursor);
        char field[64];
        if (length == 0U || length >= sizeof(field)) return false;
        memcpy(field, cursor, length);
        field[length] = '\0';
        if (mcp_json_object_get(doc, object, field) == MCP_JSON_NO_TOKEN) return false;
        if (end == NULL) break;
        cursor = end + 1;
    }
    return true;
}

static bool transaction(mcp_broker *broker, const mcp_json_doc *request,
                        size_t args, mcp_tool_response *out)
{
    mcp_broker_document *document = mcp_broker_find(broker, request, args, "documentId");
    if (document == NULL) return mcp_broker_set_error(out, "document_not_found", "documentId is not open");
    size_t edits = mcp_json_object_get(request, args, "edits");
    size_t count = request->tokens[edits].child_count;
    if (count == 0U || count > 64U) return mcp_broker_set_error(out, "invalid_transaction", "transaction must contain 1 to 64 edits");
    size_t length = document_edit_byte_length(document->edit);
    if (length > MCP_TEMPLATE_VALUE_CAP * 1024U * 64U)
        return mcp_broker_set_error(out, "transaction_limit", "transaction source exceeds its 64 MiB limit");
    uint8_t *source = malloc(length == 0U ? 1U : length);
    if (source == NULL || !document_edit_materialize(document->edit, source, length, &length)) {
        free(source);
        return mcp_broker_set_error(out, "source_unavailable", "transaction snapshot could not be materialized");
    }
    document_edit_result created = document_edit_create((base_bytes){source, length}, document_edit_default_limits());
    if (created.error != DOCUMENT_EDIT_OK || created.edit == NULL) {
        free(source);
        return mcp_broker_set_error(out, "transaction_failed", "transaction snapshot could not be parsed");
    }
    mcp_broker_document scratch = *document;
    scratch.edit = created.edit;
    scratch.owned_source = source;
    scratch.owned_source_length = length;
    document_handle selection = {0};
    size_t edit_token = request->tokens[edits].first_child;
    bool okay = true;
    for (size_t index = 0U; index < count; index++) {
        if (edit_token == MCP_JSON_NO_TOKEN || request->tokens[edit_token].type != MCP_JSON_OBJECT) { okay = false; break; }
        size_t op_token = mcp_json_object_get(request, edit_token, "op");
        char operation[32];
        size_t op_length = 0U;
        if (!mcp_json_string_copy(request, op_token, operation, sizeof(operation) - 1U, &op_length)) { okay = false; break; }
        operation[op_length] = '\0';
        if (!transaction_key_allowed(request, edit_token, operation)) { okay = false; break; }
        document_edit_error error = apply_simple_operation(&scratch, request, edit_token, operation, &selection);
        if (error != DOCUMENT_EDIT_OK) { okay = false; break; }
        edit_token = request->tokens[edit_token].next_sibling;
    }
    if (!okay) {
        document_edit_destroy(scratch.edit);
        free(source);
        return mcp_broker_set_error(out, "transaction_rolled_back", "a transaction edit failed; no document changes were committed");
    }
    if (!document_edit_root(scratch.edit, &selection)) {
        document_edit_destroy(scratch.edit);
        free(source);
        return mcp_broker_set_error(out, "transaction_failed", "transaction result has no document root");
    }
    document_edit *previous = document->edit;
    uint8_t *previous_source = document->owned_source;
    if (document->app_owned) {
        if (broker->host.replace_current_edit == NULL ||
            !broker->host.replace_current_edit(broker->host.context, previous, scratch.edit, selection)) {
            document_edit_destroy(scratch.edit);
            free(source);
            return mcp_broker_set_error(out, "transaction_commit_failed", "Vexel could not install the transaction snapshot");
        }
    } else document_edit_destroy(previous);
    document->edit = scratch.edit;
    document->owned_source = source;
    document->owned_source_length = length;
    free(previous_source);
    mcp_broker_after_edit(broker, document, selection);
    char applied[32];
    int applied_length = snprintf(applied, sizeof(applied), "%zu", count);
    return applied_length > 0 && (size_t)applied_length < sizeof(applied) &&
        mcp_json_buffer_literal(&out->structured_json, "{\"documentId\":" ) &&
        mcp_json_buffer_quoted(&out->structured_json, document->id, strlen(document->id)) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"applied\":" ) &&
        mcp_json_buffer_append(&out->structured_json, applied, (size_t)applied_length) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"atomic\":true}" );
}

bool mcp_broker_edit_call(mcp_broker *broker, const mcp_json_doc *request,
                          size_t args, const char *tool_name,
                          mcp_tool_response *out)
{
    if (strcmp(tool_name, "transaction") == 0) return transaction(broker, request, args, out);
    return direct_edit(broker, request, args, tool_name, out);
}
