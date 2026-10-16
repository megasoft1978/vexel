#include "document_edit_internal.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define DOCUMENT_EDIT_TEMPLATE_LIMIT (64U * 1024U)
#define DOCUMENT_EDIT_TEMPLATE_ATTR_LIMIT 128U
#define DOCUMENT_EDIT_AUTO_ID_ATTEMPTS 4096U

typedef struct prepared_fragment {
    const uint8_t *bytes;
    size_t length;
    uint8_t *owned;
    size_t root_offset;
    bool generated_id;
    uint64_t next_id;
} prepared_fragment;

static bool graphics_tag(base_bytes tag)
{
    static const char *const names[] = {
        "a", "circle", "ellipse", "image", "line", "path", "polygon",
        "polyline", "rect", "text", "use", "g"
    };
    size_t start = 0U;
    for (size_t index = 0U; index < tag.len; ++index)
        if (tag.ptr[index] == ':') start = index + 1U;
    base_bytes local = {tag.ptr + start, tag.len - start};
    for (size_t index = 0U; index < sizeof(names) / sizeof(names[0]); ++index) {
        size_t length = strlen(names[index]);
        if (local.len == length && memcmp(local.ptr, names[index], length) == 0)
            return true;
    }
    return false;
}

static bool fragment_tag_end(base_bytes fragment, size_t start, size_t *out_end)
{
    uint8_t quote = 0U;
    for (size_t index = start; index < fragment.len; ++index) {
        uint8_t value = fragment.ptr[index];
        if (quote != 0U) {
            if (value == quote) quote = 0U;
        } else if (value == '"' || value == '\'') quote = value;
        else if (value == '>') {
            *out_end = index > start && fragment.ptr[index - 1U] == '/' ?
                index - 1U : index;
            return true;
        }
    }
    return false;
}

static document_edit_error document_id_exists(document_edit *edit, base_bytes id,
                                              bool *out_exists)
{
    static const uint8_t id_name[] = "id";
    if (edit == NULL || out_exists == NULL || !base_bytes_valid(id) ||
        !document_edit_ensure_index(edit)) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    *out_exists = false;
    for (size_t node_index = 0U; node_index < edit->node_count; ++node_index) {
        const document_edit_node_info *node = &edit->nodes[node_index];
        for (size_t index = 0U; index < node->attribute_count; ++index) {
            const document_edit_attribute_info *attribute =
                &edit->attributes[node->first_attribute + index];
            if (!document_edit_span_equals(edit, attribute->name,
                    (base_bytes){id_name, sizeof(id_name) - 1U})) continue;
            base_bytes raw = {edit->snapshot + attribute->value.offset,
                attribute->value.length};
            if (raw.len == id.len &&
                (id.len == 0U || memcmp(raw.ptr, id.ptr, id.len) == 0)) {
                *out_exists = true;
                return DOCUMENT_EDIT_OK;
            }
            if (memchr(raw.ptr, '&', raw.len) == NULL) continue;
            size_t capacity = raw.len == 0U ? 1U : raw.len;
            uint8_t *decoded = malloc(capacity);
            if (decoded == NULL) return DOCUMENT_EDIT_OUT_OF_MEMORY;
            size_t decoded_length = 0U;
            bool okay = document_edit_decode_bytes(raw, decoded, capacity,
                &decoded_length);
            bool match = okay && decoded_length == id.len &&
                (id.len == 0U || memcmp(decoded, id.ptr, id.len) == 0);
            free(decoded);
            if (!okay) return DOCUMENT_EDIT_MALFORMED_XML;
            if (match) {
                *out_exists = true;
                return DOCUMENT_EDIT_OK;
            }
        }
    }
    return DOCUMENT_EDIT_OK;
}

static document_edit_error prepare_fragment(document_edit *edit,
    base_bytes fragment, prepared_fragment *out)
{
    static const uint8_t prefix[] = "<probe>";
    static const uint8_t suffix[] = "</probe>";
    static const uint8_t id_name[] = "id";
    if (edit == NULL || out == NULL || !base_bytes_valid(fragment) ||
        fragment.len == 0U || fragment.len > DOCUMENT_EDIT_XML_FRAGMENT_LIMIT)
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    memset(out, 0, sizeof(*out));
    size_t wrapped_length;
    if (!base_size_add(sizeof(prefix) - 1U, fragment.len, &wrapped_length) ||
        !base_size_add(wrapped_length, sizeof(suffix) - 1U, &wrapped_length))
        return DOCUMENT_EDIT_LIMIT;
    uint8_t *wrapped = malloc(wrapped_length);
    if (wrapped == NULL) return DOCUMENT_EDIT_OUT_OF_MEMORY;
    memcpy(wrapped, prefix, sizeof(prefix) - 1U);
    memcpy(wrapped + sizeof(prefix) - 1U, fragment.ptr, fragment.len);
    memcpy(wrapped + sizeof(prefix) - 1U + fragment.len,
        suffix, sizeof(suffix) - 1U);
    document_model_result parsed = document_model_parse(
        (base_bytes){wrapped, wrapped_length});
    if (parsed.error != DOCUMENT_MODEL_OK || parsed.model == NULL) {
        free(wrapped);
        return parsed.error == DOCUMENT_MODEL_OUT_OF_MEMORY ?
            DOCUMENT_EDIT_OUT_OF_MEMORY : DOCUMENT_EDIT_MALFORMED_XML;
    }
    document_handle wrapper, root, next;
    document_source_span root_span;
    bool valid = document_model_root(parsed.model, &wrapper) &&
        document_model_first_child(parsed.model, wrapper, &root) &&
        !document_model_next_sibling(parsed.model, root, &next) &&
        document_model_source_span(parsed.model, root, &root_span) &&
        root_span.offset >= sizeof(prefix) - 1U &&
        root_span.offset - (sizeof(prefix) - 1U) < fragment.len;
    if (!valid) {
        document_model_destroy(parsed.model);
        free(wrapped);
        return DOCUMENT_EDIT_INVALID_OPERATION;
    }
    out->root_offset = root_span.offset - (sizeof(prefix) - 1U);
    base_bytes tag = {0};
    bool drawable = document_model_tag_name(parsed.model, root, &tag) &&
        graphics_tag(tag);
    bool has_id = false;
    base_bytes id_value = {0};
    for (size_t index = 0U; index < document_model_attribute_count(parsed.model, root);
         ++index) {
        base_bytes name = {0}, value = {0};
        if (!document_model_attribute_at(parsed.model, root, index, &name, &value)) {
            valid = false;
            break;
        }
        if (name.len == sizeof(id_name) - 1U &&
            memcmp(name.ptr, id_name, sizeof(id_name) - 1U) == 0) {
            has_id = true;
            id_value = value;
            break;
        }
    }
    if (!valid) {
        document_model_destroy(parsed.model);
        free(wrapped);
        return DOCUMENT_EDIT_INVALID_OPERATION;
    }
    if (has_id) {
        size_t capacity = id_value.len == 0U ? 1U : id_value.len;
        uint8_t *decoded = malloc(capacity);
        if (decoded == NULL) {
            document_model_destroy(parsed.model);
            free(wrapped);
            return DOCUMENT_EDIT_OUT_OF_MEMORY;
        }
        size_t decoded_length = 0U;
        bool decoded_ok = document_edit_decode_bytes(id_value, decoded,
            capacity, &decoded_length);
        bool duplicate = false;
        document_edit_error lookup_error = decoded_ok ? document_id_exists(edit,
            (base_bytes){decoded, decoded_length}, &duplicate) :
            DOCUMENT_EDIT_MALFORMED_XML;
        free(decoded);
        if (lookup_error != DOCUMENT_EDIT_OK || duplicate) {
            document_model_destroy(parsed.model);
            free(wrapped);
            return duplicate ? DOCUMENT_EDIT_DUPLICATE_ID : lookup_error;
        }
    }
    document_model_destroy(parsed.model);
    free(wrapped);
    out->bytes = fragment.ptr;
    out->length = fragment.len;
    out->next_id = edit->next_auto_object_id;
    if (!drawable || has_id) return DOCUMENT_EDIT_OK;

    char id[48];
    size_t id_length = 0U, insert_at = 0U;
    bool available = false;
    for (size_t attempt = 0U; attempt < DOCUMENT_EDIT_AUTO_ID_ATTEMPTS; ++attempt) {
        if (out->next_id == UINT64_MAX) return DOCUMENT_EDIT_LIMIT;
        int amount = snprintf(id, sizeof(id), "vexel-object-%llu",
            (unsigned long long)out->next_id);
        if (amount <= 0 || (size_t)amount >= sizeof(id)) return DOCUMENT_EDIT_LIMIT;
        id_length = (size_t)amount;
        ++out->next_id;
        bool exists = false;
        document_edit_error lookup_error = document_id_exists(edit,
            (base_bytes){(const uint8_t *)id, id_length}, &exists);
        if (lookup_error != DOCUMENT_EDIT_OK) return lookup_error;
        if (!exists) {
            available = true;
            break;
        }
    }
    if (!available) return DOCUMENT_EDIT_LIMIT;
    if (!fragment_tag_end(fragment, out->root_offset, &insert_at))
        return DOCUMENT_EDIT_MALFORMED_XML;
    size_t attribute_length, total;
    if (!base_size_add(id_length, 6U, &attribute_length) ||
        !base_size_add(fragment.len, attribute_length, &total) ||
        total > DOCUMENT_EDIT_XML_FRAGMENT_LIMIT) return DOCUMENT_EDIT_LIMIT;
    out->owned = malloc(total);
    if (out->owned == NULL) return DOCUMENT_EDIT_OUT_OF_MEMORY;
    memcpy(out->owned, fragment.ptr, insert_at);
    size_t cursor = insert_at;
    memcpy(out->owned + cursor, " id=\"", 5U); cursor += 5U;
    memcpy(out->owned + cursor, id, id_length); cursor += id_length;
    out->owned[cursor++] = '"';
    memcpy(out->owned + cursor, fragment.ptr + insert_at,
        fragment.len - insert_at);
    out->bytes = out->owned;
    out->length = total;
    out->generated_id = true;
    return DOCUMENT_EDIT_OK;
}

static bool bytes_equal(base_bytes left, base_bytes right)
{
    return base_bytes_valid(left) && base_bytes_valid(right) && left.len == right.len &&
        (left.len == 0U || memcmp(left.ptr, right.ptr, left.len) == 0);
}

static bool template_tag(document_edit_template_kind kind, base_bytes *out_tag)
{
    static const uint8_t *const names[] = {
        (const uint8_t *)"rect", (const uint8_t *)"circle",
        (const uint8_t *)"ellipse", (const uint8_t *)"line",
        (const uint8_t *)"path", (const uint8_t *)"g",
        (const uint8_t *)"text"
    };
    static const size_t lengths[] = {4U, 6U, 7U, 4U, 4U, 1U, 4U};
    if ((unsigned)kind >= sizeof(lengths) / sizeof(lengths[0])) return false;
    *out_tag = (base_bytes){names[(size_t)kind], lengths[(size_t)kind]};
    return true;
}

document_edit_error document_edit_insert_template(document_edit *edit,
    document_handle parent_handle, document_edit_template_kind kind,
    const document_edit_template_attribute *attributes, size_t attribute_count,
    document_handle *out_inserted)
{
    if (edit == NULL || out_inserted == NULL ||
        (attribute_count > 0U && attributes == NULL) ||
        attribute_count > DOCUMENT_EDIT_TEMPLATE_ATTR_LIMIT)
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    document_edit_node_info *parent = NULL;
    if (!document_edit_handle_info(edit, parent_handle, &parent))
        return DOCUMENT_EDIT_STALE_HANDLE;
    bool parent_was_empty = parent->empty;
    base_bytes tag;
    if (!template_tag(kind, &tag)) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    size_t capacity = DOCUMENT_EDIT_TEMPLATE_LIMIT;
    uint8_t *markup = malloc(capacity);
    if (markup == NULL) return DOCUMENT_EDIT_OUT_OF_MEMORY;
    size_t used = 0U;
    markup[used++] = '<';
    memcpy(markup + used, tag.ptr, tag.len); used += tag.len;
    for (size_t i = 0U; i < attribute_count; ++i) {
        const document_edit_template_attribute *attribute = &attributes[i];
        if (!base_bytes_valid(attribute->name) || !base_bytes_valid(attribute->value) ||
            !document_edit_name_valid(attribute->name)) {
            free(markup); return DOCUMENT_EDIT_INVALID_NAME;
        }
        for (size_t previous = 0U; previous < i; ++previous) {
            if (bytes_equal(attributes[previous].name, attribute->name)) {
                free(markup); return DOCUMENT_EDIT_DUPLICATE_ATTRIBUTE;
            }
        }
        uint8_t *encoded = NULL;
        size_t encoded_length = 0U;
        if (!document_edit_encode_attribute(attribute->value, '"', &encoded,
                                            &encoded_length)) {
            free(markup); return DOCUMENT_EDIT_OUT_OF_MEMORY;
        }
        size_t addition;
        if (!base_size_add(1U, attribute->name.len, &addition) ||
            !base_size_add(addition, 3U, &addition) ||
            !base_size_add(addition, encoded_length, &addition) ||
            addition > capacity - used) {
            free(encoded); free(markup); return DOCUMENT_EDIT_LIMIT;
        }
        markup[used++] = ' ';
        memcpy(markup + used, attribute->name.ptr, attribute->name.len);
        used += attribute->name.len;
        markup[used++] = '='; markup[used++] = '"';
        if (encoded_length > 0U) memcpy(markup + used, encoded, encoded_length);
        used += encoded_length;
        markup[used++] = '"';
        free(encoded);
    }
    if (capacity - used < 2U) { free(markup); return DOCUMENT_EDIT_LIMIT; }
    markup[used++] = '/'; markup[used++] = '>';
    prepared_fragment prepared;
    document_edit_error prepared_error = prepare_fragment(edit,
        (base_bytes){markup, used}, &prepared);
    if (prepared_error != DOCUMENT_EDIT_OK) {
        free(markup);
        return prepared_error;
    }
    if (prepared.owned != NULL) {
        free(markup);
        markup = prepared.owned;
        used = prepared.length;
    }
    size_t patch_offset, remove_length = 0U;
    uint8_t *replacement = markup;
    size_t replacement_length = used;
    if (parent->empty) {
        base_bytes parent_tag;
        if (!document_model_tag_name(edit->model, parent_handle, &parent_tag)) {
            free(markup); return DOCUMENT_EDIT_STALE_HANDLE;
        }
        size_t close_length;
        if (!base_size_add(parent_tag.len, 3U, &close_length)) {
            free(markup); return DOCUMENT_EDIT_LIMIT;
        }
        size_t total;
        size_t close_with_gt;
        if (!base_size_add(close_length, 1U, &close_with_gt) ||
            !base_size_add(used, close_with_gt, &total) ||
            total > DOCUMENT_EDIT_TEMPLATE_LIMIT) {
            free(markup); return DOCUMENT_EDIT_LIMIT;
        }
        replacement = malloc(total);
        if (replacement == NULL) { free(markup); return DOCUMENT_EDIT_OUT_OF_MEMORY; }
        replacement[0] = '>';
        memcpy(replacement + 1U, markup, used);
        size_t at = used + 1U;
        replacement[at++] = '<'; replacement[at++] = '/';
        memcpy(replacement + at, parent_tag.ptr, parent_tag.len); at += parent_tag.len;
        replacement[at++] = '>';
        replacement_length = at;
        patch_offset = parent->open.offset + parent->open.length - 2U;
        remove_length = 2U;
        free(markup);
    } else {
        patch_offset = parent->close.offset;
    }
    bool valid = document_edit_validate_replace(edit, parent->span.offset,
        parent->span.length, patch_offset, remove_length, replacement,
        replacement_length, false, false);
    document_edit_error result = valid ? document_edit_commit_replace(edit,
        patch_offset, remove_length, replacement, replacement_length,
        parent_handle.slot, false, document_edit_history_template_action(kind)) : DOCUMENT_EDIT_MALFORMED_XML;
    size_t inserted_offset = patch_offset + (parent_was_empty ? 1U : 0U);
    free(replacement);
    if (result != DOCUMENT_EDIT_OK) return result;
    if (prepared.generated_id) edit->next_auto_object_id = prepared.next_id;
    return document_edit_element_at_offset(edit, inserted_offset, out_inserted)
        ? DOCUMENT_EDIT_OK : DOCUMENT_EDIT_INVALID_OPERATION;
}

document_edit_error document_edit_insert_xml_fragment(document_edit *edit,
    document_handle parent_handle, base_bytes fragment,
    document_handle *out_inserted)
{
    if (edit == NULL || out_inserted == NULL || !base_bytes_valid(fragment) ||
        fragment.len == 0U) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    if (fragment.len > DOCUMENT_EDIT_XML_FRAGMENT_LIMIT) return DOCUMENT_EDIT_LIMIT;
    document_edit_node_info *parent = NULL;
    if (!document_edit_handle_info(edit, parent_handle, &parent))
        return DOCUMENT_EDIT_STALE_HANDLE;
    prepared_fragment prepared;
    document_edit_error root_error = prepare_fragment(edit, fragment, &prepared);
    if (root_error != DOCUMENT_EDIT_OK) return root_error;

    size_t patch_offset = parent->close.offset;
    size_t remove_length = 0U;
    const uint8_t *replacement = prepared.bytes;
    size_t replacement_length = prepared.length;
    uint8_t *owned_replacement = NULL;
    if (parent->empty) {
        base_bytes parent_tag;
        if (!document_model_tag_name(edit->model, parent_handle, &parent_tag)) {
            free(prepared.owned);
            return DOCUMENT_EDIT_STALE_HANDLE;
        }
        size_t total;
        if (!base_size_add(1U, prepared.length, &total) ||
            !base_size_add(total, parent_tag.len, &total) ||
            !base_size_add(total, 3U, &total) ||
            total > DOCUMENT_EDIT_XML_FRAGMENT_LIMIT + 4096U) {
            free(prepared.owned);
            return DOCUMENT_EDIT_LIMIT;
        }
        owned_replacement = malloc(total);
        if (owned_replacement == NULL) {
            free(prepared.owned);
            return DOCUMENT_EDIT_OUT_OF_MEMORY;
        }
        size_t cursor = 0U;
        owned_replacement[cursor++] = '>';
        memcpy(owned_replacement + cursor, prepared.bytes, prepared.length);
        cursor += prepared.length;
        owned_replacement[cursor++] = '<';
        owned_replacement[cursor++] = '/';
        memcpy(owned_replacement + cursor, parent_tag.ptr, parent_tag.len);
        cursor += parent_tag.len;
        owned_replacement[cursor++] = '>';
        replacement = owned_replacement;
        replacement_length = cursor;
        patch_offset = parent->open.offset + parent->open.length - 2U;
        remove_length = 2U;
    }
    bool valid = document_edit_validate_replace(edit, parent->span.offset,
        parent->span.length, patch_offset, remove_length, replacement,
        replacement_length, false, false);
    size_t inserted_offset = patch_offset +
        (parent->empty ? 1U : 0U) + prepared.root_offset;
    document_edit_error result = valid ? document_edit_commit_replace(edit,
        patch_offset, remove_length, replacement, replacement_length,
        parent_handle.slot, false, DOCUMENT_EDIT_HISTORY_ACTION_ADD_SHAPE) : DOCUMENT_EDIT_MALFORMED_XML;
    free(owned_replacement);
    free(prepared.owned);
    if (result != DOCUMENT_EDIT_OK) return result;
    if (prepared.generated_id) edit->next_auto_object_id = prepared.next_id;
    return document_edit_element_at_offset(edit, inserted_offset, out_inserted)
        ? DOCUMENT_EDIT_OK : DOCUMENT_EDIT_INVALID_OPERATION;
}

