#define _POSIX_C_SOURCE 200809L

#include "mcp_broker_internal.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void clear_document(mcp_broker_document *document, bool destroy_edit)
{
    if (document == NULL) return;
    if (destroy_edit && document->edit != NULL) document_edit_destroy(document->edit);
    free(document->owned_source);
    free(document->last_render_base64);
    *document = (mcp_broker_document){0};
}

static mcp_broker_document *free_slot(mcp_broker *broker)
{
    for (size_t index = 0U; index < MCP_BROKER_MAX_DOCUMENTS; index++)
        if (!broker->documents[index].used) return &broker->documents[index];
    return NULL;
}

static void assign_id(mcp_broker *broker, mcp_broker_document *document)
{
    uint32_t serial = broker->next_serial++;
    if (serial == 0U) serial = broker->next_serial++;
    document->serial = serial;
    int amount = snprintf(document->id, sizeof(document->id), "doc-%08x", serial);
    if (amount < 0 || (size_t)amount >= sizeof(document->id)) document->id[0] = '\0';
}

bool mcp_broker_create(const mcp_broker_host *host, mcp_broker **out)
{
    if (out == NULL) return false;
    *out = NULL;
    mcp_broker *broker = calloc(1U, sizeof(*broker));
    if (broker == NULL) return false;
    if (host != NULL) broker->host = *host;
    broker->next_serial = 1U;
    *out = broker;
    return true;
}

void mcp_broker_destroy(mcp_broker *broker)
{
    if (broker == NULL) return;
    for (size_t index = 0U; index < MCP_BROKER_MAX_DOCUMENTS; index++) {
        mcp_broker_document *document = &broker->documents[index];
        bool destroy_edit = document->used && !document->app_owned;
        if (document->used && document->app_owned && document->owned_source != NULL) {
            if (broker->host.destroy_current_edit != NULL &&
                broker->host.destroy_current_edit(broker->host.context, document->edit)) {
                document->edit = NULL;
            } else {
                /* Keep borrowed bytes alive if the host still owns the edit object. */
                document->owned_source = NULL;
            }
        }
        clear_document(document, destroy_edit);
    }
    free(broker);
}

void mcp_broker_bind_current(mcp_broker *broker)
{
    if (broker == NULL || broker->host.current_edit == NULL || broker->host.current_path == NULL) return;
    document_edit *edit = broker->host.current_edit(broker->host.context);
    const char *path = broker->host.current_path(broker->host.context);
    if (edit == NULL || path == NULL || path[0] == '\0' ||
        strnlen(path, MCP_BROKER_PATH_CAP) >= MCP_BROKER_PATH_CAP) {
        if (broker->app_document != NULL) {
            clear_document(broker->app_document, false);
            broker->app_document = NULL;
        }
        return;
    }
    if (broker->app_document != NULL && strcmp(broker->app_document->path, path) == 0) {
        if (broker->app_document->edit != edit && broker->app_document->owned_source != NULL) {
            free(broker->app_document->owned_source);
            broker->app_document->owned_source = NULL;
            broker->app_document->owned_source_length = 0U;
        }
        broker->app_document->edit = edit;
        document_handle selection;
        if (broker->host.current_selection != NULL &&
            broker->host.current_selection(broker->host.context, &selection)) {
            broker->app_document->selection = selection;
            broker->app_document->selection_valid = true;
        }
        return;
    }
    if (broker->app_document != NULL) {
        clear_document(broker->app_document, false);
        broker->app_document = NULL;
    }
    mcp_broker_document *document = free_slot(broker);
    if (document == NULL) return;
    document->used = true;
    document->app_owned = true;
    document->edit = edit;
    memcpy(document->path, path, strlen(path) + 1U);
    assign_id(broker, document);
    broker->app_document = document;
    if (document_edit_root(edit, &document->selection)) document->selection_valid = true;
}

mcp_broker_document *mcp_broker_current(mcp_broker *broker)
{
    return broker == NULL ? NULL : broker->app_document;
}

mcp_protocol_backend mcp_broker_protocol_backend(mcp_broker *broker)
{
    return (mcp_protocol_backend){mcp_broker_call_tool, mcp_broker_read_resource, broker};
}

mcp_broker_document *mcp_broker_find(mcp_broker *broker, const mcp_json_doc *doc,
                                     size_t args, const char *key)
{
    if (broker == NULL || doc == NULL || key == NULL) return NULL;
    char id[32];
    size_t length = 0U;
    if (!mcp_broker_string_arg(doc, args, key, id, sizeof(id), &length)) return NULL;
    id[length] = '\0';
    for (size_t index = 0U; index < MCP_BROKER_MAX_DOCUMENTS; index++) {
        mcp_broker_document *document = &broker->documents[index];
        if (document->used && strcmp(document->id, id) == 0 && document->edit != NULL) return document;
    }
    return NULL;
}

bool mcp_broker_string_arg(const mcp_json_doc *doc, size_t args, const char *name,
                           char *out, size_t capacity, size_t *out_length)
{
    if (doc == NULL || out == NULL || capacity == 0U || out_length == NULL) return false;
    size_t token = mcp_json_object_get(doc, args, name);
    size_t length = 0U;
    if (!mcp_json_string_copy(doc, token, out, capacity - 1U, &length) ||
        memchr(out, '\0', length) != NULL) return false;
    out[length] = '\0';
    *out_length = length;
    return true;
}

bool mcp_broker_u64_arg(const mcp_json_doc *doc, size_t args, const char *name,
                        uint64_t *out, bool *present)
{
    if (doc == NULL || out == NULL || present == NULL) return false;
    size_t token = mcp_json_object_get(doc, args, name);
    *present = token != MCP_JSON_NO_TOKEN;
    if (!*present) return true;
    return mcp_json_uint(doc, token, out);
}

bool mcp_broker_bool_arg(const mcp_json_doc *doc, size_t args, const char *name,
                         bool *out)
{
    return doc != NULL && out != NULL &&
        mcp_json_bool(doc, mcp_json_object_get(doc, args, name), out);
}

bool mcp_broker_set_result(mcp_tool_response *out, const char *json)
{
    if (out == NULL || json == NULL) return false;
    out->is_error = false;
    return mcp_json_buffer_literal(&out->structured_json, json);
}

bool mcp_broker_set_error(mcp_tool_response *out, const char *code, const char *message)
{
    if (out == NULL || code == NULL || message == NULL) return false;
    out->is_error = true;
    return mcp_json_buffer_literal(&out->structured_json, "{\"error\":{\"code\":" ) &&
        mcp_json_buffer_quoted(&out->structured_json, code, strlen(code)) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"message\":" ) &&
        mcp_json_buffer_quoted(&out->structured_json, message, strlen(message)) &&
        mcp_json_buffer_literal(&out->structured_json, "}}" ) &&
        mcp_json_buffer_literal(&out->content_json, "[{\"type\":\"text\",\"text\":" ) &&
        mcp_json_buffer_quoted(&out->content_json, message, strlen(message)) &&
        mcp_json_buffer_literal(&out->content_json, "}]" );
}

static bool handle_handle_id(const char *id, size_t length,
                             document_handle *out, size_t *slot, uint32_t *generation)
{
    if (length < 4U || id[0] != '@') return false;
    size_t cursor = 1U;
    uint64_t first = 0U;
    if (cursor >= length || id[cursor] < '0' || id[cursor] > '9') return false;
    while (cursor < length && id[cursor] >= '0' && id[cursor] <= '9') {
        uint32_t digit = (uint32_t)(id[cursor++] - '0');
        if (first > (SIZE_MAX - digit) / 10U) return false;
        first = first * 10U + digit;
    }
    if (cursor >= length || id[cursor++] != ':') return false;
    uint64_t second = 0U;
    if (cursor >= length) return false;
    while (cursor < length && id[cursor] >= '0' && id[cursor] <= '9') {
        uint32_t digit = (uint32_t)(id[cursor++] - '0');
        if (second > (UINT32_MAX - digit) / 10U) return false;
        second = second * 10U + digit;
    }
    if (cursor != length || first > UINT32_MAX || second == 0U) return false;
    *slot = (size_t)first;
    *generation = (uint32_t)second;
    *out = (document_handle){0U, (uint32_t)first, (uint32_t)second};
    return true;
}

bool mcp_broker_resolve_element(mcp_broker_document *document, const char *id,
                                size_t id_length, document_handle *out)
{
    if (document == NULL || document->edit == NULL || id == NULL || out == NULL || id_length == 0U) return false;
    size_t slot = 0U;
    uint32_t generation = 0U;
    document_handle parsed = {0};
    const document_model *model = document_edit_model(document->edit);
    if (model != NULL && handle_handle_id(id, id_length, &parsed, &slot, &generation)) {
        document_handle candidate;
        if (document_model_node_at(model, slot, &candidate) && candidate.generation == generation) {
            *out = candidate;
            return true;
        }
    }
    return document_edit_find_id(document->edit, (base_bytes){(const uint8_t *)id, id_length}, out);
}

bool mcp_broker_element_id(mcp_broker_document *document, document_handle handle,
                           mcp_json_buffer *out)
{
    if (document == NULL || document->edit == NULL || out == NULL) return false;
    size_t count = document_edit_attribute_count(document->edit, handle);
    for (size_t index = 0U; index < count; index++) {
        document_edit_attribute attribute;
        base_bytes name = {0}, value = {0};
        if (!document_edit_attribute_at(document->edit, handle, index, &attribute) ||
            !document_edit_span_bytes(document->edit, attribute.name, &name) ||
            name.len != 2U || memcmp(name.ptr, "id", 2U) != 0 ||
            !document_edit_span_bytes(document->edit, attribute.value, &value)) continue;
        char decoded[256];
        size_t length = 0U;
        if (!document_edit_decode_span(document->edit, attribute.value,
                (uint8_t *)decoded, sizeof(decoded) - 1U, &length) ||
            length == 0U || memchr(decoded, '\0', length) != NULL) break;
        document_handle first_match;
        if (document_edit_find_id(document->edit,
                (base_bytes){(const uint8_t *)decoded, length}, &first_match) &&
            (first_match.slot != handle.slot || first_match.generation != handle.generation)) break;
        return mcp_json_buffer_quoted(out, decoded, length);
    }
    char handle_text[64];
    int amount = snprintf(handle_text, sizeof(handle_text), "@%u:%u", handle.slot, handle.generation);
    return amount > 0 && (size_t)amount < sizeof(handle_text) &&
        mcp_json_buffer_quoted(out, handle_text, (size_t)amount);
}

bool mcp_broker_tree_json(mcp_broker_document *document, size_t cursor, size_t limit,
                          mcp_json_buffer *out)
{
    if (document == NULL || document->edit == NULL || out == NULL || limit == 0U || limit > 512U) return false;
    const document_model *model = document_edit_model(document->edit);
    if (model == NULL) return false;
    size_t node_count = document_model_node_count(model);
    if (cursor > node_count) return false;
    if (!mcp_json_buffer_literal(out, "{\"documentId\":" ) ||
        !mcp_json_buffer_quoted(out, document->id, strlen(document->id)) ||
        !mcp_json_buffer_literal(out, ",\"elements\":[")) return false;
    size_t slot = cursor;
    size_t emitted = 0U;
    for (; slot < node_count && emitted < limit; slot++) {
        document_handle handle;
        if (!document_model_node_at(model, slot, &handle)) continue;
        base_bytes tag = {0};
        size_t depth = 0U;
        if (!document_edit_tag_name(document->edit, handle, &tag) ||
            !document_model_depth(model, handle, &depth)) return false;
        if (emitted > 0U && !mcp_json_buffer_literal(out, ",")) return false;
        if (!mcp_json_buffer_literal(out, "{\"elementId\":" ) ||
            !mcp_broker_element_id(document, handle, out) ||
            !mcp_json_buffer_literal(out, ",\"tag\":" ) ||
            !mcp_json_buffer_quoted(out, (const char *)tag.ptr, tag.len) ||
            !mcp_json_buffer_literal(out, ",\"depth\":" )) return false;
        char number[32];
        int amount = snprintf(number, sizeof(number), "%zu", depth);
        if (amount < 0 || (size_t)amount >= sizeof(number) ||
            !mcp_json_buffer_append(out, number, (size_t)amount) ||
            !mcp_json_buffer_literal(out, "}")) return false;
        emitted++;
    }
    if (!mcp_json_buffer_literal(out, "],\"cursor\":" )) return false;
    char number[32];
    int amount = snprintf(number, sizeof(number), "%zu", slot);
    if (amount < 0 || (size_t)amount >= sizeof(number) ||
        !mcp_json_buffer_append(out, number, (size_t)amount) ||
        !mcp_json_buffer_literal(out, ",\"complete\":" ) ||
        !mcp_json_buffer_literal(out, slot >= node_count ? "true}" : "false}")) return false;
    return true;
}

void mcp_broker_after_edit(mcp_broker *broker, mcp_broker_document *document,
                           document_handle selection)
{
    if (broker == NULL || document == NULL || document->edit == NULL) return;
    document->selection = selection;
    document->selection_valid = true;
    if (broker->host.selection_changed != NULL && document->app_owned)
        broker->host.selection_changed(broker->host.context, selection);
    if (broker->host.agent_edited != NULL)
        broker->host.agent_edited(broker->host.context, document->path);
}
