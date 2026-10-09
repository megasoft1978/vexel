#include "document_model.h"

#include <stdatomic.h>
#include <stdlib.h>
#include <string.h>

#define DOCUMENT_MODEL_MAX_TOKEN_BYTES (64U * 1024U * 1024U)
#define DOCUMENT_MODEL_MAX_NODES 1000000U
#define DOCUMENT_MODEL_MAX_DEPTH 256U

typedef struct document_node {
    svg_xml_span name;
    document_source_span source;
    size_t first_attribute;
    size_t attribute_count;
    size_t depth;
    uint32_t parent;
    uint32_t first_child;
    uint32_t last_child;
    uint32_t next_sibling;
    uint32_t generation;
    bool live;
} document_node;

typedef struct document_attribute {
    svg_xml_span name;
    svg_xml_span value;
} document_attribute;

struct document_model {
    base_bytes source;
    uint64_t id;
    document_node *nodes;
    document_attribute *attributes;
    size_t node_count;
    size_t attribute_count;
    size_t live_node_count;
    uint32_t root;
};

static _Atomic uint64_t next_document_id = 1U;

static bool allocate_document_id(uint64_t *out_id)
{
    uint64_t current = atomic_load_explicit(&next_document_id, memory_order_relaxed);
    for (;;) {
        if (current == 0U || current == UINT64_MAX) return false;
        if (atomic_compare_exchange_weak_explicit(&next_document_id, &current,
                current + 1U, memory_order_relaxed, memory_order_relaxed)) {
            *out_id = current;
            return true;
        }
    }
}

static void append_child(document_model *model, uint32_t parent, uint32_t child)
{
    document_node *parent_node = &model->nodes[parent];
    if (parent_node->first_child == UINT32_MAX) parent_node->first_child = child;
    else model->nodes[parent_node->last_child].next_sibling = child;
    parent_node->last_child = child;
}

static document_model_error create_nodes(document_model *model, const svg_xml_token *tokens,
                                         size_t token_count, size_t element_count,
                                         size_t attribute_count)
{
    if (element_count == 0U || element_count > DOCUMENT_MODEL_MAX_NODES ||
        element_count > UINT32_MAX || element_count > SIZE_MAX / sizeof(*model->nodes))
        return DOCUMENT_MODEL_LIMIT;
    model->nodes = calloc(element_count, sizeof(*model->nodes));
    if (model->nodes == NULL) return DOCUMENT_MODEL_OUT_OF_MEMORY;
    model->node_count = element_count;
    model->live_node_count = element_count;
    if (attribute_count > 0U) {
        if (attribute_count > SIZE_MAX / sizeof(*model->attributes)) return DOCUMENT_MODEL_LIMIT;
        model->attributes = calloc(attribute_count, sizeof(*model->attributes));
        if (model->attributes == NULL) return DOCUMENT_MODEL_OUT_OF_MEMORY;
    }
    model->attribute_count = attribute_count;
    uint32_t stack[DOCUMENT_MODEL_MAX_DEPTH];
    size_t depth = 0U;
    size_t next_slot = 0U;
    size_t next_attribute = 0U;
    for (size_t i = 0U; i < token_count; ++i) {
        const svg_xml_token *token = &tokens[i];
        if (token->kind == SVG_XML_TOKEN_END_ELEMENT) {
            if (depth == 0U) return DOCUMENT_MODEL_PARSE;
            document_node *node = &model->nodes[stack[depth - 1U]];
            if (node->source.offset > token->source.offset) return DOCUMENT_MODEL_PARSE;
            size_t end = token->source.offset + token->source.length;
            if (end < token->source.offset || end > model->source.len) return DOCUMENT_MODEL_PARSE;
            node->source.length = end - node->source.offset;
            --depth;
        } else if (token->kind == SVG_XML_TOKEN_START_ELEMENT ||
                   token->kind == SVG_XML_TOKEN_EMPTY_ELEMENT) {
            if (next_slot >= element_count || token->name.offset > model->source.len ||
                token->name.length > model->source.len - token->name.offset ||
                token->source.offset > model->source.len ||
                token->source.length > model->source.len - token->source.offset)
                return DOCUMENT_MODEL_PARSE;
            document_node *node = &model->nodes[next_slot];
            node->name = token->name;
            node->source.offset = token->source.offset;
            node->source.length = token->source.length;
            node->parent = depth == 0U ? UINT32_MAX : stack[depth - 1U];
            node->depth = depth;
            node->first_attribute = next_attribute;
            while (i + 1U < token_count && tokens[i + 1U].kind == SVG_XML_TOKEN_ATTRIBUTE) {
                const svg_xml_token *attribute = &tokens[++i];
                if (next_attribute >= attribute_count ||
                    attribute->name.offset > model->source.len ||
                    attribute->name.length > model->source.len - attribute->name.offset ||
                    attribute->value.offset > model->source.len ||
                    attribute->value.length > model->source.len - attribute->value.offset)
                    return DOCUMENT_MODEL_PARSE;
                model->attributes[next_attribute].name = attribute->name;
                model->attributes[next_attribute].value = attribute->value;
                ++next_attribute;
                ++node->attribute_count;
            }
            node->first_child = UINT32_MAX;
            node->last_child = UINT32_MAX;
            node->next_sibling = UINT32_MAX;
            node->generation = 1U;
            node->live = true;
            if (depth == 0U) model->root = (uint32_t)next_slot;
            else append_child(model, node->parent, (uint32_t)next_slot);
            if (token->kind == SVG_XML_TOKEN_START_ELEMENT) {
                if (depth >= DOCUMENT_MODEL_MAX_DEPTH) return DOCUMENT_MODEL_LIMIT;
                stack[depth++] = (uint32_t)next_slot;
            }
            ++next_slot;
        }
    }
    return next_slot == element_count && next_attribute == attribute_count && depth == 0U ?
        DOCUMENT_MODEL_OK : DOCUMENT_MODEL_PARSE;
}

document_model_result document_model_parse(base_bytes source)
{
    document_model_result result = {0};
    result.error = DOCUMENT_MODEL_INVALID_ARGUMENT;
    if (!base_bytes_valid(source)) return result;
    svg_xml_options options = svg_xml_default_options();
    if (source.len > options.limits.input_bytes) {
        result.error = DOCUMENT_MODEL_LIMIT;
        result.parse_diagnostic.category = SVG_XML_ERROR_INPUT_LIMIT;
        return result;
    }
    svg_xml_token_stream *stream = NULL;
    svg_xml_stream_status status = svg_xml_token_stream_start(source,
        DOCUMENT_MODEL_MAX_TOKEN_BYTES, NULL, NULL, &stream);
    if (status != SVG_XML_STREAM_OK) {
        result.error = DOCUMENT_MODEL_OUT_OF_MEMORY;
        return result;
    }
    do { status = svg_xml_token_stream_step(stream, 64U * 1024U); }
    while (status == SVG_XML_STREAM_OK);
    svg_xml_result parsed = svg_xml_token_stream_result(stream);
    result.parse_diagnostic = parsed.diagnostic;
    if (status != SVG_XML_STREAM_COMPLETE || parsed.diagnostic.category != SVG_XML_ERROR_NONE) {
        if (status == SVG_XML_STREAM_FAILED &&
            parsed.diagnostic.category == SVG_XML_ERROR_OUTPUT_EXHAUSTED) {
            /* The shared tokenizer uses one code for its storage cap and realloc failure. */
            result.error = DOCUMENT_MODEL_RESOURCE_EXHAUSTED;
        } else result.error = DOCUMENT_MODEL_PARSE;
        svg_xml_token_stream_destroy(stream);
        return result;
    }
    svg_xml_token *tokens = NULL;
    size_t token_count = 0U;
    bool taken = svg_xml_token_stream_take(stream, &tokens, &token_count);
    svg_xml_token_stream_destroy(stream);
    if (!taken) {
        result.error = DOCUMENT_MODEL_PARSE;
        return result;
    }
    document_model *model = calloc(1U, sizeof(*model));
    if (model == NULL) {
        free(tokens);
        result.error = DOCUMENT_MODEL_OUT_OF_MEMORY;
        return result;
    }
    model->source = source;
    model->root = UINT32_MAX;
    bool id_ok = allocate_document_id(&model->id);
    document_model_error nodes_error = id_ok ? create_nodes(model, tokens, token_count,
        parsed.element_count, parsed.attribute_count) : DOCUMENT_MODEL_ID_EXHAUSTED;
    free(tokens);
    if (nodes_error != DOCUMENT_MODEL_OK) {
        free(model->nodes);
        free(model->attributes);
        free(model);
        result.error = nodes_error;
        return result;
    }
    result.error = DOCUMENT_MODEL_OK;
    result.model = model;
    return result;
}

void document_model_destroy(document_model *model)
{
    if (model == NULL) return;
    free(model->nodes);
    free(model->attributes);
    memset(model, 0, sizeof(*model));
    free(model);
}

size_t document_model_node_count(const document_model *model)
{
    return model == NULL ? 0U : model->node_count;
}

size_t document_model_live_node_count(const document_model *model)
{
    return model == NULL ? 0U : model->live_node_count;
}

static const document_node *resolve(const document_model *model, document_handle handle)
{
    if (model == NULL || handle.document_id != model->id ||
        handle.slot >= model->node_count) return NULL;
    document_node *node = &model->nodes[handle.slot];
    return node->live && node->generation == handle.generation ? node : NULL;
}

static document_handle make_handle(const document_model *model, uint32_t slot)
{
    document_handle handle = {model->id, slot, model->nodes[slot].generation};
    return handle;
}

bool document_model_root(const document_model *model, document_handle *out)
{
    if (model == NULL || out == NULL || model->root == UINT32_MAX ||
        !model->nodes[model->root].live) return false;
    *out = make_handle(model, model->root);
    return true;
}

bool document_model_first_child(const document_model *model, document_handle parent,
                                document_handle *out)
{
    const document_node *node = resolve(model, parent);
    if (node == NULL || out == NULL) return false;
    uint32_t child = node->first_child;
    while (child != UINT32_MAX && !model->nodes[child].live)
        child = model->nodes[child].next_sibling;
    if (child == UINT32_MAX) return false;
    *out = make_handle(model, child);
    return true;
}

bool document_model_next_sibling(const document_model *model, document_handle handle,
                                 document_handle *out)
{
    const document_node *node = resolve(model, handle);
    if (node == NULL || out == NULL) return false;
    uint32_t sibling = node->next_sibling;
    while (sibling != UINT32_MAX && !model->nodes[sibling].live)
        sibling = model->nodes[sibling].next_sibling;
    if (sibling == UINT32_MAX) return false;
    *out = make_handle(model, sibling);
    return true;
}

bool document_model_parent(const document_model *model, document_handle handle,
                           document_handle *out)
{
    const document_node *node = resolve(model, handle);
    if (node == NULL || out == NULL || node->parent == UINT32_MAX ||
        !model->nodes[node->parent].live) return false;
    *out = make_handle(model, node->parent);
    return true;
}

bool document_model_tag_name(const document_model *model, document_handle handle,
                             base_bytes *out_name)
{
    const document_node *node = resolve(model, handle);
    if (node == NULL || out_name == NULL) return false;
    out_name->ptr = model->source.ptr + node->name.offset;
    out_name->len = node->name.length;
    return true;
}

size_t document_model_attribute_count(const document_model *model, document_handle handle)
{
    const document_node *node = resolve(model, handle);
    return node == NULL ? 0U : node->attribute_count;
}

bool document_model_attribute_at(const document_model *model, document_handle handle,
                                 size_t index, base_bytes *out_name,
                                 base_bytes *out_value)
{
    const document_node *node = resolve(model, handle);
    if (node == NULL || out_name == NULL || out_value == NULL ||
        index >= node->attribute_count) return false;
    const document_attribute *attribute =
        &model->attributes[node->first_attribute + index];
    out_name->ptr = model->source.ptr + attribute->name.offset;
    out_name->len = attribute->name.length;
    out_value->ptr = model->source.ptr + attribute->value.offset;
    out_value->len = attribute->value.length;
    return true;
}

bool document_model_depth(const document_model *model, document_handle handle,
                          size_t *out_depth)
{
    const document_node *node = resolve(model, handle);
    if (node == NULL || out_depth == NULL) return false;
    *out_depth = node->depth;
    return true;
}

bool document_model_node_at(const document_model *model, size_t slot,
                            document_handle *out)
{
    if (model == NULL || out == NULL || slot >= model->node_count ||
        !model->nodes[slot].live) return false;
    *out = make_handle(model, (uint32_t)slot);
    return true;
}

bool document_model_source_span(const document_model *model, document_handle handle,
                                document_source_span *out_span)
{
    const document_node *node = resolve(model, handle);
    if (node == NULL || out_span == NULL) return false;
    *out_span = node->source;
    return true;
}

bool document_model_remove_subtree(document_model *model, document_handle handle)
{
    if (resolve(model, handle) == NULL) return false;
    for (size_t i = 0U; i < model->node_count; ++i) {
        uint32_t parent = (uint32_t)i;
        bool descendant = false;
        while (parent != UINT32_MAX) {
            if (parent == handle.slot) { descendant = true; break; }
            parent = model->nodes[parent].parent;
        }
        if (!descendant) continue;
        document_node *entry = &model->nodes[i];
        if (entry->live) {
            entry->live = false;
            --model->live_node_count;
            if (entry->generation != UINT32_MAX) ++entry->generation;
        }
    }
    if (handle.slot == model->root) model->root = UINT32_MAX;
    return true;
}

const char *document_model_error_name(document_model_error error)
{
    switch (error) {
        case DOCUMENT_MODEL_OK: return "ok";
        case DOCUMENT_MODEL_INVALID_ARGUMENT: return "invalid_argument";
        case DOCUMENT_MODEL_PARSE: return "parse";
        case DOCUMENT_MODEL_LIMIT: return "limit";
        case DOCUMENT_MODEL_RESOURCE_EXHAUSTED: return "resource_exhausted";
        case DOCUMENT_MODEL_OUT_OF_MEMORY: return "out_of_memory";
        case DOCUMENT_MODEL_ID_EXHAUSTED: return "id_exhausted";
        default: return "unknown";
    }
}
