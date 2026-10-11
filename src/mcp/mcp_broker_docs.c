#define _POSIX_C_SOURCE 200809L
#define _DARWIN_C_SOURCE

#include "mcp_broker_internal.h"

#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#define MCP_OPEN_DOCUMENT_LIMIT (64U * 1024U * 1024U)
#define MCP_ELEMENT_ATTRIBUTE_LIMIT 128U
#define MCP_EXPORTED_SOURCE_LIMIT (1024U * 1024U)

static bool append_number(mcp_json_buffer *out, uint64_t value)
{
    char text[32];
    int amount = snprintf(text, sizeof(text), "%llu", (unsigned long long)value);
    return amount > 0 && (size_t)amount < sizeof(text) &&
        mcp_json_buffer_append(out, text, (size_t)amount);
}

static bool append_bool(mcp_json_buffer *out, bool value)
{
    return mcp_json_buffer_literal(out, value ? "true" : "false");
}

static bool append_document_entry(mcp_broker *broker, mcp_json_buffer *out,
                                  mcp_broker_document *document, bool active)
{
    return mcp_json_buffer_literal(out, "{\"documentId\":" ) &&
        mcp_json_buffer_quoted(out, mcp_broker_public_id(broker, document),
            strlen(mcp_broker_public_id(broker, document))) &&
        mcp_json_buffer_literal(out, ",\"path\":" ) &&
        mcp_json_buffer_quoted(out, document->path, strlen(document->path)) &&
        mcp_json_buffer_literal(out, ",\"mode\":" ) &&
        mcp_json_buffer_quoted(out, mcp_broker_document_mode(document),
            strlen(mcp_broker_document_mode(document))) &&
        mcp_json_buffer_literal(out, ",\"dirty\":" ) &&
        append_bool(out, document_edit_is_dirty(document->edit)) &&
        mcp_json_buffer_literal(out, ",\"active\":" ) && append_bool(out, active) &&
        mcp_json_buffer_literal(out, ",\"live\":" ) && append_bool(out, document->live) &&
        (document->live ?
            (mcp_json_buffer_literal(out, ",\"windowId\":" ) &&
             mcp_json_buffer_quoted(out, document->window_id,
                 strlen(document->window_id))) : true) &&
        mcp_json_buffer_literal(out, "}" );
}

static bool read_bounded_file(const char *path, uint8_t **out_source, size_t *out_length)
{
    if (path == NULL || out_source == NULL || out_length == NULL) return false;
    *out_source = NULL;
    *out_length = 0U;
    int descriptor = open(path, O_RDONLY | O_NOFOLLOW);
    if (descriptor < 0) return false;
    struct stat status;
    if (fstat(descriptor, &status) != 0 || !S_ISREG(status.st_mode) || status.st_size < 0 ||
        (uint64_t)status.st_size > MCP_OPEN_DOCUMENT_LIMIT) {
        (void)close(descriptor);
        return false;
    }
    size_t length = (size_t)status.st_size;
    uint8_t *source = malloc(length == 0U ? 1U : length);
    if (source == NULL) { (void)close(descriptor); return false; }
    size_t cursor = 0U;
    bool okay = true;
    while (cursor < length) {
        ssize_t amount = read(descriptor, source + cursor, length - cursor);
        if (amount < 0 && errno == EINTR) continue;
        if (amount <= 0) { okay = false; break; }
        cursor += (size_t)amount;
    }
    if (close(descriptor) != 0) okay = false;
    if (!okay) { free(source); return false; }
    *out_source = source;
    *out_length = length;
    return true;
}

static mcp_broker_document *find_path(mcp_broker *broker, const char *canonical)
{
    for (size_t index = 0U; index < MCP_BROKER_MAX_DOCUMENTS; index++) {
        mcp_broker_document *document = &broker->documents[index];
        if (document->used && strcmp(document->path, canonical) == 0) return document;
    }
    return NULL;
}

static mcp_broker_document *open_document(mcp_broker *broker, const char *path,
                                          const char **out_error)
{
    char canonical[MCP_BROKER_PATH_CAP];
    if (!mcp_access_resolve_existing(path, canonical) || !mcp_access_path_allowed(canonical)) {
        *out_error = "path_not_granted";
        return NULL;
    }
    mcp_broker_document *existing = find_path(broker, canonical);
    if (existing != NULL) return existing;
    size_t headless_count = 0U;
    for (size_t index = 0U; index < MCP_BROKER_MAX_DOCUMENTS; index++)
        if (broker->documents[index].used && !broker->documents[index].live)
            headless_count++;
    if (headless_count >= MCP_BROKER_MAX_HEADLESS_DOCUMENTS) {
        *out_error = "document_limit";
        return NULL;
    }
    mcp_broker_document *slot = NULL;
    for (size_t index = 0U; index < MCP_BROKER_MAX_DOCUMENTS; index++)
        if (!broker->documents[index].used) { slot = &broker->documents[index]; break; }
    if (slot == NULL) { *out_error = "document_limit"; return NULL; }
    uint8_t *source = NULL;
    size_t source_length = 0U;
    if (!read_bounded_file(canonical, &source, &source_length)) {
        *out_error = "document_read_failed";
        return NULL;
    }
    document_edit_result created = document_edit_create((base_bytes){source, source_length},
                                                         document_edit_default_limits());
    if (created.error != DOCUMENT_EDIT_OK || created.edit == NULL) {
        free(source);
        *out_error = created.error == DOCUMENT_EDIT_LIMIT ? "document_limit" : "invalid_svg";
        return NULL;
    }
    *slot = (mcp_broker_document){0};
    slot->used = true;
    slot->edit = created.edit;
    mcp_ui_host_init_headless(&slot->ui_host);
    slot->owned_source = source;
    slot->owned_source_length = source_length;
    memcpy(slot->path, canonical, strlen(canonical) + 1U);
    uint32_t serial = broker->next_serial++;
    if (serial == 0U) serial = broker->next_serial++;
    slot->serial = serial;
    int amount = snprintf(slot->id, sizeof(slot->id), "doc-%08x", serial);
    if (amount < 0 || (size_t)amount >= sizeof(slot->id)) {
        document_edit_destroy(slot->edit);
        free(slot->owned_source);
        *slot = (mcp_broker_document){0};
        *out_error = "document_id_exhausted";
        return NULL;
    }
    (void)document_edit_root(slot->edit, &slot->selection);
    slot->selection_valid = true;
    return slot;
}

static bool emit_element(mcp_broker *broker, mcp_broker_document *document,
                         document_handle handle,
                         mcp_json_buffer *out)
{
    (void)broker;
    base_bytes tag = {0};
    if (!document_edit_tag_name(document->edit, handle, &tag) ||
        !mcp_json_buffer_literal(out, "{\"elementId\":" ) ||
        !mcp_broker_element_id(document, handle, out) ||
        !mcp_json_buffer_literal(out, ",\"mode\":" ) ||
        !mcp_json_buffer_quoted(out, mcp_broker_document_mode(document),
            strlen(mcp_broker_document_mode(document))) ||
        !mcp_json_buffer_literal(out, ",\"tag\":" ) ||
        !mcp_json_buffer_quoted(out, (const char *)tag.ptr, tag.len) ||
        !mcp_json_buffer_literal(out, ",\"attributes\":[")) return false;
    size_t count = document_edit_attribute_count(document->edit, handle);
    if (count > MCP_ELEMENT_ATTRIBUTE_LIMIT) count = MCP_ELEMENT_ATTRIBUTE_LIMIT;
    for (size_t index = 0U; index < count; index++) {
        document_edit_attribute attribute;
        base_bytes name = {0}, value = {0};
        if (!document_edit_attribute_at(document->edit, handle, index, &attribute) ||
            !document_edit_span_bytes(document->edit, attribute.name, &name) ||
            !document_edit_span_bytes(document->edit, attribute.value, &value)) return false;
        if (index > 0U && !mcp_json_buffer_literal(out, ",")) return false;
        if (!mcp_json_buffer_literal(out, "{\"name\":" ) ||
            !mcp_json_buffer_quoted(out, (const char *)name.ptr, name.len) ||
            !mcp_json_buffer_literal(out, ",\"value\":" ) ||
            !mcp_json_buffer_quoted(out, (const char *)value.ptr, value.len) ||
            !mcp_json_buffer_literal(out, "}")) return false;
    }
    return mcp_json_buffer_literal(out, "]}" );
}

static bool get_element_argument(mcp_broker_document *document, const mcp_json_doc *request,
                                 size_t args, const char *name, document_handle *out)
{
    char id[256];
    size_t length = 0U;
    return mcp_broker_string_arg(request, args, name, id, sizeof(id), &length) &&
        mcp_broker_resolve_element(document, id, length, out);
}

static bool document_list(mcp_broker *broker, mcp_tool_response *out)
{
    if (!mcp_json_buffer_literal(&out->structured_json, "{\"documents\":[")) return false;
    bool first = true;
    for (size_t index = 0U; index < MCP_BROKER_MAX_DOCUMENTS; index++) {
        mcp_broker_document *document = &broker->documents[index];
        if (!document->used || document->edit == NULL) continue;
        if (!first && !mcp_json_buffer_literal(&out->structured_json, ",")) return false;
        if (!append_document_entry(broker, &out->structured_json, document,
                document == broker->app_document)) return false;
        first = false;
    }
    return mcp_json_buffer_literal(&out->structured_json, "]}" );
}

static bool document_info(mcp_broker *broker, const mcp_json_doc *request,
                          size_t args, mcp_tool_response *out)
{
    mcp_broker_document *document = mcp_broker_find(broker, request, args, "documentId");
    if (document == NULL) return mcp_broker_set_error(out, "document_not_found", "documentId is not open");
    return mcp_json_buffer_literal(&out->structured_json, "{\"documentId\":" ) &&
        mcp_json_buffer_quoted(&out->structured_json,
            mcp_broker_public_id(broker, document),
            strlen(mcp_broker_public_id(broker, document))) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"path\":" ) &&
        mcp_json_buffer_quoted(&out->structured_json, document->path, strlen(document->path)) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"sourceBytes\":" ) &&
        append_number(&out->structured_json, document_edit_byte_length(document->edit)) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"dirty\":" ) &&
        append_bool(&out->structured_json, document_edit_is_dirty(document->edit)) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"undoDepth\":" ) &&
        append_number(&out->structured_json, document_edit_history_cursor(document->edit)) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"redoDepth\":" ) &&
        append_number(&out->structured_json,
            document_edit_history_count(document->edit) - document_edit_history_cursor(document->edit)) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"active\":" ) &&
        append_bool(&out->structured_json, document == broker->app_document) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"live\":" ) &&
        append_bool(&out->structured_json, document->live) &&
        (document->live ?
            (mcp_json_buffer_literal(&out->structured_json, ",\"windowId\":" ) &&
             mcp_json_buffer_quoted(&out->structured_json, document->window_id,
                 strlen(document->window_id))) : true) &&
        mcp_json_buffer_literal(&out->structured_json, "}" );
}

static bool tree_get(mcp_broker *broker, const mcp_json_doc *request,
                     size_t args, mcp_tool_response *out)
{
    mcp_broker_document *document = mcp_broker_find(broker, request, args, "documentId");
    if (document == NULL) return mcp_broker_set_error(out, "document_not_found", "documentId is not open");
    uint64_t cursor = 0U, limit = 128U;
    bool present = false;
    if (!mcp_broker_u64_arg(request, args, "cursor", &cursor, &present) ||
        !mcp_broker_u64_arg(request, args, "limit", &limit, &present) || cursor > SIZE_MAX ||
        limit == 0U || limit > 512U) return mcp_broker_set_error(out, "invalid_arguments", "tree cursor or page size is out of range");
    if (!mcp_broker_tree_json(document, (size_t)cursor, (size_t)limit, &out->structured_json))
        return mcp_broker_set_error(out, "invalid_cursor", "tree cursor is outside the document");
    return true;
}

static bool element_get(mcp_broker *broker, const mcp_json_doc *request,
                        size_t args, mcp_tool_response *out)
{
    mcp_broker_document *document = mcp_broker_find(broker, request, args, "documentId");
    document_handle handle;
    if (document == NULL) return mcp_broker_set_error(out, "document_not_found", "documentId is not open");
    if (!get_element_argument(document, request, args, "elementId", &handle))
        return mcp_broker_set_error(out, "element_not_found", "elementId does not identify a live element");
    return emit_element(broker, document, handle, &out->structured_json);
}

static bool query_matches(mcp_broker_document *document, document_handle handle,
                          const mcp_json_doc *request, size_t args,
                          const char *id_filter, size_t id_length,
                          const char *tag_filter, size_t tag_length,
                          const char *attribute_filter, size_t attribute_length,
                          const char *value_filter, size_t value_length)
{
    if (id_length > 0U) {
        document_handle matched;
        if (!mcp_broker_resolve_element(document, id_filter, id_length, &matched) ||
            matched.slot != handle.slot || matched.generation != handle.generation) return false;
    }
    base_bytes tag = {0};
    if (!document_edit_tag_name(document->edit, handle, &tag)) return false;
    if (tag_length > 0U && (tag.len != tag_length || memcmp(tag.ptr, tag_filter, tag_length) != 0)) return false;
    if (attribute_length == 0U) return true;
    size_t count = document_edit_attribute_count(document->edit, handle);
    for (size_t index = 0U; index < count; index++) {
        document_edit_attribute attribute;
        base_bytes name = {0}, value = {0};
        if (!document_edit_attribute_at(document->edit, handle, index, &attribute) ||
            !document_edit_span_bytes(document->edit, attribute.name, &name) ||
            name.len != attribute_length || memcmp(name.ptr, attribute_filter, attribute_length) != 0 ||
            !document_edit_span_bytes(document->edit, attribute.value, &value)) continue;
        if (value_length == 0U || (value.len == value_length && memcmp(value.ptr, value_filter, value_length) == 0)) return true;
    }
    (void)request;
    (void)args;
    return false;
}

static bool element_query(mcp_broker *broker, const mcp_json_doc *request,
                          size_t args, mcp_tool_response *out)
{
    mcp_broker_document *document = mcp_broker_find(broker, request, args, "documentId");
    if (document == NULL) return mcp_broker_set_error(out, "document_not_found", "documentId is not open");
    char id[256] = {0}, tag[128] = {0}, attr[128] = {0}, value[512] = {0};
    size_t id_length = 0U, tag_length = 0U, attr_length = 0U, value_length = 0U;
    size_t token = mcp_json_object_get(request, args, "id");
    if (token != MCP_JSON_NO_TOKEN && !mcp_broker_string_arg(request, args, "id", id, sizeof(id), &id_length))
        return mcp_broker_set_error(out, "invalid_arguments", "id is not a bounded string");
    token = mcp_json_object_get(request, args, "tag");
    if (token != MCP_JSON_NO_TOKEN && !mcp_broker_string_arg(request, args, "tag", tag, sizeof(tag), &tag_length))
        return mcp_broker_set_error(out, "invalid_arguments", "tag is not a bounded string");
    token = mcp_json_object_get(request, args, "attribute");
    if (token != MCP_JSON_NO_TOKEN && !mcp_broker_string_arg(request, args, "attribute", attr, sizeof(attr), &attr_length))
        return mcp_broker_set_error(out, "invalid_arguments", "attribute is not a bounded string");
    token = mcp_json_object_get(request, args, "value");
    if (token != MCP_JSON_NO_TOKEN && !mcp_broker_string_arg(request, args, "value", value, sizeof(value), &value_length))
        return mcp_broker_set_error(out, "invalid_arguments", "value is not a bounded string");
    if (id_length + tag_length + attr_length == 0U)
        return mcp_broker_set_error(out, "invalid_arguments", "element_query needs an id, tag, or attribute filter");
    uint64_t cursor = 0U, limit = 128U;
    bool present = false;
    if (!mcp_broker_u64_arg(request, args, "cursor", &cursor, &present) ||
        !mcp_broker_u64_arg(request, args, "limit", &limit, &present) ||
        limit == 0U || limit > 512U || cursor > SIZE_MAX)
        return mcp_broker_set_error(out, "invalid_arguments", "query cursor or page size is out of range");
    const document_model *model = document_edit_model(document->edit);
    if (model == NULL) return mcp_broker_set_error(out, "document_invalid", "document tree is unavailable");
    size_t node_count = document_model_node_count(model);
    if (cursor > node_count) return mcp_broker_set_error(out, "invalid_cursor", "query cursor is outside the document");
    if (!mcp_json_buffer_literal(&out->structured_json, "{\"documentId\":" ) ||
        !mcp_json_buffer_quoted(&out->structured_json,
            mcp_broker_public_id(broker, document),
            strlen(mcp_broker_public_id(broker, document))) ||
        !mcp_json_buffer_literal(&out->structured_json, ",\"mode\":" ) ||
        !mcp_json_buffer_quoted(&out->structured_json, mcp_broker_document_mode(document),
            strlen(mcp_broker_document_mode(document))) ||
        !mcp_json_buffer_literal(&out->structured_json, ",\"elements\":[")) return false;
    size_t slot = (size_t)cursor, emitted = 0U;
    for (; slot < node_count && emitted < (size_t)limit; slot++) {
        document_handle handle;
        if (!document_model_node_at(model, slot, &handle) ||
            !query_matches(document, handle, request, args, id, id_length, tag, tag_length,
                           attr, attr_length, value, value_length)) continue;
        if (emitted > 0U && !mcp_json_buffer_literal(&out->structured_json, ",")) return false;
        if (!mcp_json_buffer_literal(&out->structured_json, "{\"elementId\":" ) ||
            !mcp_broker_element_id(document, handle, &out->structured_json) ||
            !mcp_json_buffer_literal(&out->structured_json, ",\"tag\":" )) return false;
        base_bytes tag_name = {0};
        if (!document_edit_tag_name(document->edit, handle, &tag_name) ||
            !mcp_json_buffer_quoted(&out->structured_json, (const char *)tag_name.ptr, tag_name.len) ||
            !mcp_json_buffer_literal(&out->structured_json, "}")) return false;
        emitted++;
    }
    return mcp_json_buffer_literal(&out->structured_json, "],\"cursor\":" ) &&
        append_number(&out->structured_json, slot) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"complete\":" ) &&
        append_bool(&out->structured_json, slot >= node_count) &&
        mcp_json_buffer_literal(&out->structured_json, "}" );
}

static bool export_svg(mcp_broker *broker, const mcp_json_doc *request,
                       size_t args, mcp_tool_response *out)
{
    mcp_broker_document *document = mcp_broker_find(broker, request, args, "documentId");
    if (document == NULL) return mcp_broker_set_error(out, "document_not_found", "documentId is not open");
    size_t length = document_edit_byte_length(document->edit);
    if (length > MCP_EXPORTED_SOURCE_LIMIT) return mcp_broker_set_error(out, "response_limit", "SVG export exceeds the 1 MiB MCP response limit");
    uint8_t *source = malloc(length == 0U ? 1U : length);
    if (source == NULL || !document_edit_materialize(document->edit, source, length, &length)) {
        free(source);
        return mcp_broker_set_error(out, "source_unavailable", "SVG source could not be materialized");
    }
    bool okay = mcp_json_buffer_literal(&out->structured_json, "{\"documentId\":" ) &&
        mcp_json_buffer_quoted(&out->structured_json,
            mcp_broker_public_id(broker, document),
            strlen(mcp_broker_public_id(broker, document))) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"svg\":" ) &&
        mcp_json_buffer_quoted(&out->structured_json, (const char *)source, length) &&
        mcp_json_buffer_literal(&out->structured_json, "}" );
    free(source);
    return okay;
}

static bool selection_get(mcp_broker *broker, const mcp_json_doc *request,
                          size_t args, mcp_tool_response *out)
{
    mcp_broker_document *document = mcp_broker_find(broker, request, args, "documentId");
    if (document == NULL) return mcp_broker_set_error(out, "document_not_found", "documentId is not open");
    if (!document->selection_valid) return mcp_broker_set_error(out, "selection_unavailable", "document has no active selection");
    return mcp_json_buffer_literal(&out->structured_json, "{\"documentId\":" ) &&
        mcp_json_buffer_quoted(&out->structured_json,
            mcp_broker_public_id(broker, document),
            strlen(mcp_broker_public_id(broker, document))) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"elementId\":" ) &&
        mcp_broker_element_id(document, document->selection, &out->structured_json) &&
        mcp_json_buffer_literal(&out->structured_json, "}" );
}

static bool selection_set(mcp_broker *broker, const mcp_json_doc *request,
                          size_t args, mcp_tool_response *out)
{
    mcp_broker_document *document = mcp_broker_find(broker, request, args, "documentId");
    document_handle handle;
    if (document == NULL) return mcp_broker_set_error(out, "document_not_found", "documentId is not open");
    if (!get_element_argument(document, request, args, "elementId", &handle))
        return mcp_broker_set_error(out, "element_not_found", "elementId does not identify a live element");
    document->selection = handle;
    document->selection_valid = true;
    if (document->app_owned && broker->host.selection_changed_for_document != NULL)
        broker->host.selection_changed_for_document(broker->host.context,
            document->host_document, handle);
    else if (document->app_owned && broker->host.selection_changed != NULL)
        broker->host.selection_changed(broker->host.context, handle);
    return mcp_json_buffer_literal(&out->structured_json, "{\"documentId\":" ) &&
        mcp_json_buffer_quoted(&out->structured_json,
            mcp_broker_public_id(broker, document),
            strlen(mcp_broker_public_id(broker, document))) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"elementId\":" ) &&
        mcp_broker_element_id(document, handle, &out->structured_json) &&
        mcp_json_buffer_literal(&out->structured_json, "}" );
}

static bool save_document(mcp_broker *broker, const mcp_json_doc *request,
                          size_t args, const char *tool, mcp_tool_response *out)
{
    mcp_broker_document *document = mcp_broker_find(broker, request, args, "documentId");
    if (document == NULL) return mcp_broker_set_error(out, "document_not_found", "documentId is not open");
    char save_path[MCP_BROKER_PATH_CAP];
    if (strcmp(tool, "save") == 0) memcpy(save_path, document->path, strlen(document->path) + 1U);
    else {
        char requested[MCP_BROKER_PATH_CAP];
        size_t length = 0U;
        if (!mcp_broker_string_arg(request, args, "path", requested, sizeof(requested), &length) ||
            !mcp_access_resolve_save_path(requested, save_path))
            return mcp_broker_set_error(out, "path_not_granted", "Save As path must be inside a user-granted directory and have an existing parent directory");
    }
    document_edit_save_result saved = document_edit_save_as(document->edit,
        (base_bytes){(const uint8_t *)save_path, strlen(save_path)}, NULL);
    if (saved.error != DOCUMENT_EDIT_OK)
        return mcp_broker_set_error(out, document_edit_error_name(saved.error), "document save failed");
    if (strcmp(tool, "save_as") == 0) {
        bool updated = true;
        if (document->app_owned && broker->host.set_document_path != NULL)
            updated = broker->host.set_document_path(broker->host.context,
                document->host_document, save_path);
        else if (document->app_owned && broker->host.set_current_path != NULL)
            updated = broker->host.set_current_path(broker->host.context, save_path);
        else if (document->app_owned) updated = false;
        if (!updated)
            return mcp_broker_set_error(out, "save_as_path_update_failed",
                "document was written but Vexel could not update the open path");
        memcpy(document->path, save_path, strlen(save_path) + 1U);
    }
    if (document->app_owned && broker->host.agent_edited != NULL)
        broker->host.agent_edited(broker->host.context, document->path);
    mcp_broker_commit_mutation(broker, document->id, MCP_BROKER_MUTATION_SAVE);
    return mcp_json_buffer_literal(&out->structured_json, "{\"documentId\":" ) &&
        mcp_json_buffer_quoted(&out->structured_json,
            mcp_broker_public_id(broker, document),
            strlen(mcp_broker_public_id(broker, document))) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"path\":" ) &&
        mcp_json_buffer_quoted(&out->structured_json, document->path, strlen(document->path)) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"saved\":true}" );
}

static bool history_action(mcp_broker *broker, const mcp_json_doc *request,
                           size_t args, bool redo, mcp_tool_response *out)
{
    mcp_broker_document *document = mcp_broker_find(broker, request, args, "documentId");
    if (document == NULL) return mcp_broker_set_error(out, "document_not_found", "documentId is not open");
    document_edit_error error = redo ? document_edit_redo(document->edit) : document_edit_undo(document->edit);
    if (error != DOCUMENT_EDIT_OK) return mcp_broker_set_error(out,
        redo ? "redo_unavailable" : "undo_unavailable", document_edit_error_name(error));
    mcp_broker_after_edit(broker, document, (document_handle){0},
        redo ? MCP_BROKER_MUTATION_REDO : MCP_BROKER_MUTATION_UNDO, false);
    return mcp_json_buffer_literal(&out->structured_json, "{\"documentId\":" ) &&
        mcp_json_buffer_quoted(&out->structured_json,
            mcp_broker_public_id(broker, document),
            strlen(mcp_broker_public_id(broker, document))) &&
        mcp_json_buffer_literal(&out->structured_json, redo ? ",\"action\":\"redo\"}" : ",\"action\":\"undo\"}" );
}

bool mcp_broker_documents_call(mcp_broker *broker, const mcp_json_doc *request,
                               size_t args, const char *tool_name,
                               mcp_tool_response *out)
{
    if (broker == NULL || request == NULL || tool_name == NULL || out == NULL) return false;
    if (strcmp(tool_name, "document_list") == 0) return document_list(broker, out);
    if (strcmp(tool_name, "document_open") == 0) {
        char path[MCP_BROKER_PATH_CAP];
        size_t length = 0U;
        if (!mcp_broker_string_arg(request, args, "path", path, sizeof(path), &length))
            return mcp_broker_set_error(out, "invalid_path", "document path is not bounded");
        const char *error = "document_open_failed";
        mcp_broker_document *document = open_document(broker, path, &error);
        if (document == NULL) return mcp_broker_set_error(out, error, "document could not be opened");
        if (broker->host.agent_edited != NULL)
            broker->host.agent_edited(broker->host.context, document->path);
        return append_document_entry(broker, &out->structured_json, document,
                                     document == broker->app_document);
    }
    if (strcmp(tool_name, "document_close") == 0) {
        mcp_broker_document *document = mcp_broker_find(broker, request, args, "documentId");
        if (document == NULL) return mcp_broker_set_error(out, "document_not_found", "documentId is not open");
        if (document->app_owned) return mcp_broker_set_error(out,
            "active_document", "The document opened by the Vexel UI stays open while its window is active");
        if (document_edit_is_dirty(document->edit)) return mcp_broker_set_error(out,
            "unsaved_changes", "Save or undo edits before closing this document");
        document_edit_destroy(document->edit);
        free(document->owned_source);
        free(document->last_render_base64);
        *document = (mcp_broker_document){0};
        return mcp_json_buffer_literal(&out->structured_json, "{\"closed\":true}" );
    }
    if (strcmp(tool_name, "document_info") == 0) return document_info(broker, request, args, out);
    if (strcmp(tool_name, "tree_get") == 0) return tree_get(broker, request, args, out);
    if (strcmp(tool_name, "element_get") == 0) return element_get(broker, request, args, out);
    if (strcmp(tool_name, "element_query") == 0) return element_query(broker, request, args, out);
    if (strcmp(tool_name, "selection_get") == 0) return selection_get(broker, request, args, out);
    if (strcmp(tool_name, "selection_set") == 0) return selection_set(broker, request, args, out);
    if (strcmp(tool_name, "export_svg") == 0) return export_svg(broker, request, args, out);
    if (strcmp(tool_name, "save") == 0 || strcmp(tool_name, "save_as") == 0)
        return save_document(broker, request, args, tool_name, out);
    if (strcmp(tool_name, "undo") == 0) return history_action(broker, request, args, false, out);
    if (strcmp(tool_name, "redo") == 0) return history_action(broker, request, args, true, out);
    return false;
}
