#include "mcp_broker_scene_internal.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MCP_SCENE_DUPLICATE_NODE_LIMIT 4096U
#define MCP_SCENE_DUPLICATE_ID_LIMIT 512U
#define MCP_SCENE_DUPLICATE_PATH_CAP 2048U
#define MCP_SCENE_DUPLICATE_ID_CAP 2048U

typedef struct duplicate_identity {
    char path[MCP_SCENE_DUPLICATE_PATH_CAP];
    size_t path_length;
} duplicate_identity;

static bool new_duplicate_id(mcp_broker_document *document, char output[64])
{
    for (uint32_t serial = 0U; serial < 10000U; serial++) {
        int amount = snprintf(output, 64U, "mcp-duplicate-%u-%u",
                              document->serial, serial);
        if (amount < 0 || amount >= 64) return false;
        document_handle existing;
        if (!document_edit_find_id(document->edit,
                (base_bytes){(const uint8_t *)output, (size_t)amount},
                &existing)) return true;
    }
    return false;
}

static bool duplicate_has_id(mcp_broker_document *document,
                             document_handle element, bool *out_present)
{
    char id[MCP_SCENE_DUPLICATE_ID_CAP];
    size_t length = 0U;
    return mcp_scene_attribute_text(document, element, "id", id, sizeof(id),
                                    &length, out_present);
}

static bool remember_identity(mcp_broker_document *document,
    document_handle element, duplicate_identity *identity)
{
    return document_edit_element_path(document->edit, element,
        (uint8_t *)identity->path, sizeof(identity->path) - 1U,
        &identity->path_length) && identity->path_length < sizeof(identity->path);
}

static bool identity_exists(document_handle element,
                            document_handle duplicate_root)
{
    return element.document_id == duplicate_root.document_id &&
        element.slot == duplicate_root.slot &&
        element.generation == duplicate_root.generation;
}

static bool collect_duplicate_identities(mcp_broker_document *document,
    document_handle duplicate_root, duplicate_identity *identities,
    size_t *out_count)
{
    document_handle *stack = calloc(MCP_SCENE_DUPLICATE_NODE_LIMIT, sizeof(*stack));
    if (stack == NULL) return false;
    const document_model *model = document_edit_model(document->edit);
    if (model == NULL) { free(stack); return false; }
    size_t top = 0U, visited = 0U, identity_count = 0U;
    stack[top++] = duplicate_root;
    bool okay = true;
    while (top > 0U && okay) {
        document_handle current = stack[--top];
        if (++visited > MCP_SCENE_DUPLICATE_NODE_LIMIT) { okay = false; break; }
        bool has_id = false;
        if (!duplicate_has_id(document, current, &has_id)) { okay = false; break; }
        if (has_id || identity_exists(current, duplicate_root)) {
            if (identity_count >= MCP_SCENE_DUPLICATE_ID_LIMIT ||
                !remember_identity(document, current, &identities[identity_count])) {
                okay = false;
                break;
            }
            identity_count++;
        }
        document_handle child;
        if (!document_model_first_child(model, current, &child)) continue;
        do {
            if (top >= MCP_SCENE_DUPLICATE_NODE_LIMIT) { okay = false; break; }
            stack[top++] = child;
            document_handle next;
            if (!document_model_next_sibling(model, child, &next)) break;
            child = next;
        } while (true);
    }
    free(stack);
    if (okay) *out_count = identity_count;
    return okay;
}

static document_edit_error assign_duplicate_id(mcp_broker_document *document,
    const duplicate_identity *identity)
{
    document_handle element;
    if (!mcp_scene_resolve_element_path(document, identity->path,
            identity->path_length, &element)) return DOCUMENT_EDIT_STALE_HANDLE;
    char id[64];
    if (!new_duplicate_id(document, id)) return DOCUMENT_EDIT_LIMIT;
    size_t attribute_count = document_edit_attribute_count(document->edit, element);
    bool has_id = false;
    for (size_t index = 0U; index < attribute_count; index++) {
        document_edit_attribute attribute;
        base_bytes name = {0};
        if (!document_edit_attribute_at(document->edit, element, index, &attribute) ||
            !document_edit_span_bytes(document->edit, attribute.name, &name))
            return DOCUMENT_EDIT_STALE_HANDLE;
        if (name.len == 2U && memcmp(name.ptr, "id", 2U) == 0) {
            has_id = true;
            break;
        }
    }
    return has_id ? document_edit_set_attribute(document->edit, element,
            (base_bytes){(const uint8_t *)"id", 2U},
            (base_bytes){(const uint8_t *)id, strlen(id)}) :
        document_edit_add_attribute(document->edit, element,
            (base_bytes){(const uint8_t *)"id", 2U},
            (base_bytes){(const uint8_t *)id, strlen(id)});
}

document_edit_error mcp_broker_scene_duplicate(
    mcp_broker_document *document, document_handle duplicate,
    document_handle *out_duplicate)
{
    if (document == NULL || out_duplicate == NULL) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    duplicate_identity *identities = calloc(MCP_SCENE_DUPLICATE_ID_LIMIT,
                                             sizeof(*identities));
    if (identities == NULL) return DOCUMENT_EDIT_OUT_OF_MEMORY;
    size_t count = 0U;
    if (!collect_duplicate_identities(document, duplicate, identities, &count) || count == 0U) {
        free(identities);
        return DOCUMENT_EDIT_LIMIT;
    }
    for (size_t index = 0U; index < count; index++) {
        document_edit_error error = assign_duplicate_id(document, &identities[index]);
        if (error != DOCUMENT_EDIT_OK) { free(identities); return error; }
    }
    bool rebound = mcp_scene_resolve_element_path(document, identities[0].path,
        identities[0].path_length, out_duplicate);
    free(identities);
    return rebound ? DOCUMENT_EDIT_OK : DOCUMENT_EDIT_STALE_HANDLE;
}
