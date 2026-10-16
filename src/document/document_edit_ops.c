#include "document_edit_internal.h"

#include <stdlib.h>
#include <string.h>


static bool tokenize_ok(base_bytes bytes, svg_xml_diagnostic *diagnostic)
{
    svg_xml_token_stream *stream = NULL;
    svg_xml_stream_status status = svg_xml_token_stream_start(bytes,
        64U * 1024U * 1024U, NULL, NULL, &stream);
    if (status != SVG_XML_STREAM_OK) return false;
    do { status = svg_xml_token_stream_step(stream, 64U * 1024U); }
    while (status == SVG_XML_STREAM_OK);
    svg_xml_result result = svg_xml_token_stream_result(stream);
    if (diagnostic != NULL) *diagnostic = result.diagnostic;
    svg_xml_token_stream_destroy(stream);
    return status == SVG_XML_STREAM_COMPLETE &&
        result.diagnostic.category == SVG_XML_ERROR_NONE;
}

bool document_edit_validate_replace(document_edit *edit,
    size_t region_offset, size_t region_length, size_t patch_offset,
    size_t remove_length, const uint8_t *insert, size_t insert_length,
    bool fragment, bool start_tag)
{
    if (edit == NULL || (insert_length > 0U && insert == NULL) ||
        region_offset > edit->byte_length ||
        region_length > edit->byte_length - region_offset ||
        patch_offset < region_offset || patch_offset - region_offset > region_length ||
        remove_length > region_length - (patch_offset - region_offset)) return false;
    size_t candidate_length = region_length - remove_length;
    if (!base_size_add(candidate_length, insert_length, &candidate_length)) return false;
    size_t candidate_size = candidate_length == 0U ? 1U : candidate_length;
    uint8_t *candidate = malloc(candidate_size);
    if (candidate == NULL) return false;
    size_t prefix_length = patch_offset - region_offset;
    size_t suffix_offset = prefix_length + remove_length;
    size_t suffix_length = region_length - suffix_offset;
    bool copied = document_edit_read_range(edit, region_offset, prefix_length, candidate) &&
        (insert_length == 0U || (memcpy(candidate + prefix_length, insert, insert_length), true)) &&
        document_edit_read_range(edit, patch_offset + remove_length, suffix_length,
                                 candidate + prefix_length + insert_length);
    if (!copied) { free(candidate); return false; }

    uint8_t *validation = candidate;
    size_t validation_length = candidate_length;
    bool owns_validation = false;
    if (start_tag) {
        if (validation_length == 0U || validation[validation_length - 1U] != '>') {
            free(candidate); return false;
        }
        bool already_empty = validation_length >= 2U &&
            validation[validation_length - 2U] == '/';
        if (!already_empty) {
            uint8_t *expanded = malloc(validation_length + 1U);
            if (expanded == NULL) { free(candidate); return false; }
            memcpy(expanded, validation, validation_length - 1U);
            expanded[validation_length - 1U] = '/';
            expanded[validation_length] = '>';
            free(candidate);
            validation = expanded;
            ++validation_length;
            owns_validation = true;
        }
    }
    if (fragment) {
        static const uint8_t prefix[] = "<probe>";
        static const uint8_t suffix[] = "</probe>";
        size_t wrapped_length;
        if (!base_size_add(validation_length, sizeof(prefix) - 1U, &wrapped_length) ||
            !base_size_add(wrapped_length, sizeof(suffix) - 1U, &wrapped_length)) {
            if (owns_validation) free(validation);
            else free(candidate);
            return false;
        }
        uint8_t *wrapped = malloc(wrapped_length);
        if (wrapped == NULL) {
            if (owns_validation) free(validation);
            else free(candidate);
            return false;
        }
        memcpy(wrapped, prefix, sizeof(prefix) - 1U);
        if (validation_length > 0U)
            memcpy(wrapped + sizeof(prefix) - 1U, validation, validation_length);
        memcpy(wrapped + sizeof(prefix) - 1U + validation_length,
               suffix, sizeof(suffix) - 1U);
        if (owns_validation) free(validation);
        else free(candidate);
        validation = wrapped;
        validation_length = wrapped_length;
        owns_validation = true;
    }
    svg_xml_diagnostic diagnostic = {0};
    bool valid = validation_length <= svg_xml_default_options().limits.input_bytes &&
        tokenize_ok((base_bytes){validation, validation_length}, &diagnostic);
    if (owns_validation) free(validation);
    else free(candidate);
    return valid;
}

static bool emit_escaped(base_bytes input, uint8_t quote, bool text,
                         uint8_t **out_bytes, size_t *out_length)
{
    if (!base_bytes_valid(input) || out_bytes == NULL || out_length == NULL) return false;
    size_t capacity;
    if (!base_size_mul(input.len, 6U, &capacity) ||
        !base_size_add(capacity, 1U, &capacity)) return false;
    uint8_t *output = capacity == 1U ? NULL : malloc(capacity);
    if (capacity > 1U && output == NULL) return false;
    size_t used = 0U;
    for (size_t i = 0U; i < input.len; ++i) {
        const char *replacement = NULL;
        size_t replacement_length = 0U;
        uint8_t c = input.ptr[i];
        if (c == '&') { replacement = "&amp;"; replacement_length = 5U; }
        else if (c == '<') { replacement = "&lt;"; replacement_length = 4U; }
        else if (text && c == '>') { replacement = "&gt;"; replacement_length = 4U; }
        else if (!text && c == quote && quote == '"') {
            replacement = "&quot;"; replacement_length = 6U;
        } else if (!text && c == quote && quote == '\'') {
            replacement = "&apos;"; replacement_length = 6U;
        }
        if (replacement != NULL) {
            memcpy(output + used, replacement, replacement_length);
            used += replacement_length;
        } else output[used++] = c;
    }
    *out_bytes = output;
    *out_length = used;
    return true;
}

bool document_edit_encode_attribute(base_bytes input, uint8_t quote,
                                    uint8_t **out_bytes, size_t *out_length)
{
    if (quote != '\'' && quote != '"') return false;
    return emit_escaped(input, quote, false, out_bytes, out_length);
}

bool document_edit_encode_text(base_bytes input, uint8_t **out_bytes,
                               size_t *out_length)
{
    return emit_escaped(input, 0U, true, out_bytes, out_length);
}

bool document_edit_name_valid(base_bytes name)
{
    if (!base_bytes_valid(name) || name.len == 0U || name.len > 4096U) return false;
    static const uint8_t prefix[] = "<probe ";
    static const uint8_t middle[] = "=\"x\"/>";
    size_t length;
    if (!base_size_add(sizeof(prefix) - 1U, name.len, &length) ||
        !base_size_add(length, sizeof(middle) - 1U, &length)) return false;
    uint8_t *candidate = malloc(length);
    if (candidate == NULL) return false;
    memcpy(candidate, prefix, sizeof(prefix) - 1U);
    memcpy(candidate + sizeof(prefix) - 1U, name.ptr, name.len);
    memcpy(candidate + sizeof(prefix) - 1U + name.len, middle, sizeof(middle) - 1U);
    bool valid = tokenize_ok((base_bytes){candidate, length}, NULL);
    free(candidate);
    return valid;
}

static bool attribute_index(document_edit *edit, document_edit_node_info *node,
                            base_bytes name, size_t *out_index)
{
    for (size_t i = 0U; i < node->attribute_count; ++i) {
        document_edit_attribute_info *attribute =
            &edit->attributes[node->first_attribute + i];
        if (document_edit_span_equals(edit, attribute->name, name)) {
            *out_index = node->first_attribute + i;
            return true;
        }
    }
    return false;
}

static size_t start_tag_trailing_start(const uint8_t *bytes, size_t length)
{
    if (length < 2U || bytes[length - 1U] != '>') return length;
    size_t delimiter = length - 1U;
    if (delimiter > 0U && bytes[delimiter - 1U] == '/') --delimiter;
    while (delimiter > 0U) {
        uint8_t c = bytes[delimiter - 1U];
        if (c != ' ' && c != '\t' && c != '\r' && c != '\n') break;
        --delimiter;
    }
    return delimiter;
}

static document_edit_error add_attribute_encoded(document_edit *edit,
    document_handle node_handle, base_bytes name, const uint8_t *encoded,
    size_t encoded_length, bool validate_name)
{
    document_edit_node_info *node = NULL;
    if (!document_edit_handle_info(edit, node_handle, &node))
        return DOCUMENT_EDIT_STALE_HANDLE;
    if (validate_name && !document_edit_name_valid(name)) return DOCUMENT_EDIT_INVALID_NAME;
    size_t existing = 0U;
    if (attribute_index(edit, node, name, &existing))
        return DOCUMENT_EDIT_DUPLICATE_ATTRIBUTE;
    size_t insert_at = start_tag_trailing_start(edit->snapshot + node->open.offset,
                                                node->open.length);
    uint8_t *attribute = NULL;
    size_t attribute_length;
    if (!base_size_add(1U, name.len, &attribute_length) ||
        !base_size_add(attribute_length, 3U, &attribute_length) ||
        !base_size_add(attribute_length, encoded_length, &attribute_length))
        return DOCUMENT_EDIT_LIMIT;
    attribute = malloc(attribute_length);
    if (attribute == NULL) return DOCUMENT_EDIT_OUT_OF_MEMORY;
    size_t at = 0U;
    attribute[at++] = ' ';
    memcpy(attribute + at, name.ptr, name.len); at += name.len;
    attribute[at++] = '=';
    attribute[at++] = '"';
    if (encoded_length > 0U) memcpy(attribute + at, encoded, encoded_length);
    at += encoded_length;
    attribute[at++] = '"';
    size_t patch = node->open.offset + insert_at;
    bool valid = document_edit_validate_replace(edit, node->open.offset,
        node->open.length, patch, 0U, attribute, at, false, true);
    document_edit_error result = valid ? document_edit_commit_replace(edit, patch,
        0U, attribute, at, node_handle.slot, false,
        DOCUMENT_EDIT_HISTORY_ACTION_ADD_ATTRIBUTE) : DOCUMENT_EDIT_MALFORMED_XML;
    free(attribute);
    return result;
}

document_edit_error document_edit_add_attribute(document_edit *edit,
    document_handle node_handle, base_bytes name, base_bytes value)
{
    if (edit == NULL || !base_bytes_valid(name) || !base_bytes_valid(value))
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    if (!document_edit_name_valid(name)) return DOCUMENT_EDIT_INVALID_NAME;
    uint8_t *encoded = NULL;
    size_t encoded_length = 0U;
    if (!document_edit_encode_attribute(value, '"', &encoded, &encoded_length))
        return DOCUMENT_EDIT_OUT_OF_MEMORY;
    document_edit_error result = add_attribute_encoded(edit, node_handle, name,
        encoded, encoded_length, false);
    free(encoded);
    return result;
}

document_edit_error document_edit_set_attribute(document_edit *edit,
    document_handle node_handle, base_bytes name, base_bytes value)
{
    if (edit == NULL || !base_bytes_valid(name) || !base_bytes_valid(value))
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    if (!document_edit_name_valid(name)) return DOCUMENT_EDIT_INVALID_NAME;
    document_edit_node_info *node = NULL;
    if (!document_edit_handle_info(edit, node_handle, &node))
        return DOCUMENT_EDIT_STALE_HANDLE;
    size_t index = 0U;
    if (!attribute_index(edit, node, name, &index))
        return document_edit_add_attribute(edit, node_handle, name, value);
    document_edit_attribute_info *attribute = &edit->attributes[index];
    uint8_t *encoded = NULL;
    size_t encoded_length = 0U;
    if (!document_edit_encode_attribute(value, attribute->quote, &encoded,
                                        &encoded_length))
        return DOCUMENT_EDIT_OUT_OF_MEMORY;
    bool valid = document_edit_validate_replace(edit, node->open.offset,
        node->open.length, attribute->value.offset, attribute->value.length,
        encoded, encoded_length, false, true);
    document_edit_error result = valid ? document_edit_commit_replace(edit,
        attribute->value.offset, attribute->value.length, encoded, encoded_length,
        node_handle.slot, true, DOCUMENT_EDIT_HISTORY_ACTION_EDIT_ATTRIBUTE) : DOCUMENT_EDIT_MALFORMED_XML;
    free(encoded);
    return result;
}

document_edit_error document_edit_remove_attribute(document_edit *edit,
    document_handle node_handle, base_bytes name)
{
    if (edit == NULL || !base_bytes_valid(name)) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    document_edit_node_info *node = NULL;
    if (!document_edit_handle_info(edit, node_handle, &node))
        return DOCUMENT_EDIT_STALE_HANDLE;
    size_t index = 0U;
    if (!attribute_index(edit, node, name, &index)) return DOCUMENT_EDIT_NOT_FOUND;
    document_edit_attribute_info *attribute = &edit->attributes[index];
    bool valid = document_edit_validate_replace(edit, node->open.offset,
        node->open.length, attribute->source.offset, attribute->source.length,
        NULL, 0U, false, true);
    if (!valid) return DOCUMENT_EDIT_MALFORMED_XML;
    return document_edit_commit_replace(edit, attribute->source.offset,
        attribute->source.length, NULL, 0U, node_handle.slot, false,
        DOCUMENT_EDIT_HISTORY_ACTION_REMOVE_ATTRIBUTE);
}

static bool handle_is_root(document_edit *edit, document_handle node)
{
    document_handle root;
    return document_model_root(edit->model, &root) && root.slot == node.slot;
}

document_edit_error document_edit_set_text_content(document_edit *edit,
    document_handle node_handle, base_bytes text)
{
    if (edit == NULL || !base_bytes_valid(text)) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    document_edit_node_info *node = NULL;
    if (!document_edit_handle_info(edit, node_handle, &node))
        return DOCUMENT_EDIT_STALE_HANDLE;
    uint8_t *encoded = NULL;
    size_t encoded_length = 0U;
    if (!document_edit_encode_text(text, &encoded, &encoded_length))
        return DOCUMENT_EDIT_OUT_OF_MEMORY;
    size_t patch_offset, remove_length;
    uint8_t *replacement = encoded;
    size_t replacement_length = encoded_length;
    if (node->empty) {
        base_bytes tag_name;
        if (!document_model_tag_name(edit->model, node_handle, &tag_name)) {
            free(encoded); return DOCUMENT_EDIT_STALE_HANDLE;
        }
        size_t close_length;
        if (!base_size_add(tag_name.len, 3U, &close_length)) {
            free(encoded); return DOCUMENT_EDIT_LIMIT;
        }
        size_t total;
        size_t close_with_gt;
        if (!base_size_add(close_length, 1U, &close_with_gt) ||
            !base_size_add(encoded_length, close_with_gt, &total)) {
            free(encoded); return DOCUMENT_EDIT_LIMIT;
        }
        replacement = malloc(total);
        if (replacement == NULL) { free(encoded); return DOCUMENT_EDIT_OUT_OF_MEMORY; }
        replacement[0] = '>';
        if (encoded_length > 0U) memcpy(replacement + 1U, encoded, encoded_length);
        size_t at = encoded_length + 1U;
        replacement[at++] = '<'; replacement[at++] = '/';
        memcpy(replacement + at, tag_name.ptr, tag_name.len); at += tag_name.len;
        replacement[at++] = '>';
        replacement_length = at;
        patch_offset = node->open.offset + node->open.length - 2U;
        remove_length = 2U;
        free(encoded);
    } else {
        patch_offset = node->open.offset + node->open.length;
        remove_length = node->close.offset - patch_offset;
    }
    bool valid = document_edit_validate_replace(edit, node->span.offset,
        node->span.length, patch_offset, remove_length, replacement,
        replacement_length, false, false);
    document_edit_error result = valid ? document_edit_commit_replace(edit,
        patch_offset, remove_length, replacement, replacement_length,
        node_handle.slot, false, DOCUMENT_EDIT_HISTORY_ACTION_EDIT_TEXT) : DOCUMENT_EDIT_MALFORMED_XML;
    free(replacement);
    return result;
}

document_edit_error document_edit_delete_element(document_edit *edit,
    document_handle node_handle)
{
    if (edit == NULL) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    document_edit_node_info *node = NULL;
    if (!document_edit_handle_info(edit, node_handle, &node))
        return DOCUMENT_EDIT_STALE_HANDLE;
    if (handle_is_root(edit, node_handle)) return DOCUMENT_EDIT_ROOT_DELETE;
    document_handle parent;
    document_source_span parent_span;
    if (!document_model_parent(edit->model, node_handle, &parent) ||
        !document_model_source_span(edit->model, parent, &parent_span))
        return DOCUMENT_EDIT_STALE_HANDLE;
    if (!document_edit_validate_replace(edit, parent_span.offset, parent_span.length,
            node->span.offset, node->span.length, NULL, 0U, false, false))
        return DOCUMENT_EDIT_MALFORMED_XML;
    return document_edit_commit_replace(edit, node->span.offset, node->span.length,
        NULL, 0U, node_handle.slot, false, DOCUMENT_EDIT_HISTORY_ACTION_DELETE);
}

document_edit_error document_edit_duplicate_element(document_edit *edit,
    document_handle node_handle, document_handle *out_duplicate)
{
    if (edit == NULL || out_duplicate == NULL)
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    document_edit_node_info *node = NULL;
    if (!document_edit_handle_info(edit, node_handle, &node))
        return DOCUMENT_EDIT_STALE_HANDLE;
    if (handle_is_root(edit, node_handle)) return DOCUMENT_EDIT_INVALID_OPERATION;
    document_handle parent;
    document_source_span parent_span;
    if (!document_model_parent(edit->model, node_handle, &parent) ||
        !document_model_source_span(edit->model, parent, &parent_span))
        return DOCUMENT_EDIT_STALE_HANDLE;
    uint8_t *copy = malloc(node->span.length);
    if (copy == NULL) return DOCUMENT_EDIT_OUT_OF_MEMORY;
    if (!document_edit_read_range(edit, node->span.offset, node->span.length, copy)) {
        free(copy); return DOCUMENT_EDIT_INVALID_OPERATION;
    }
    size_t insert_at = node->span.offset + node->span.length;
    bool valid = document_edit_validate_replace(edit, parent_span.offset,
        parent_span.length, insert_at, 0U, copy, node->span.length, false, false);
    document_edit_error result = valid ? document_edit_commit_replace(edit,
        insert_at, 0U, copy, node->span.length, node_handle.slot, false,
        DOCUMENT_EDIT_HISTORY_ACTION_DUPLICATE) :
        DOCUMENT_EDIT_MALFORMED_XML;
    free(copy);
    if (result != DOCUMENT_EDIT_OK) return result;
    return document_edit_element_at_offset(edit, insert_at, out_duplicate)
        ? DOCUMENT_EDIT_OK : DOCUMENT_EDIT_INVALID_OPERATION;
}

static bool same_parent(document_edit *edit, document_handle left,
                       document_handle right)
{
    document_handle left_parent, right_parent;
    bool left_has = document_model_parent(edit->model, left, &left_parent);
    bool right_has = document_model_parent(edit->model, right, &right_parent);
    return left_has && right_has && left_parent.slot == right_parent.slot;
}

document_edit_error document_edit_reorder_sibling(document_edit *edit,
    document_handle node_handle, document_handle target_handle, bool before)
{
    if (edit == NULL) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    document_edit_node_info *node = NULL, *target = NULL;
    if (!document_edit_handle_info(edit, node_handle, &node) ||
        !document_edit_handle_info(edit, target_handle, &target))
        return DOCUMENT_EDIT_STALE_HANDLE;
    if (node_handle.slot == target_handle.slot ||
        !same_parent(edit, node_handle, target_handle))
        return DOCUMENT_EDIT_INVALID_OPERATION;
    bool already = before ? node->span.offset < target->span.offset :
                            node->span.offset > target->span.offset;
    if (already) return DOCUMENT_EDIT_OK;
    size_t start, end, node_start = node->span.offset, node_end = node_start + node->span.length;
    size_t target_start = target->span.offset, target_end = target_start + target->span.length;
    if (node_start < target_start) { start = node_start; end = target_end; }
    else { start = target_start; end = node_end; }
    size_t span_length = end - start;
    uint8_t *old_region = malloc(span_length);
    uint8_t *new_region = malloc(span_length);
    if ((span_length > 0U && old_region == NULL) ||
        (span_length > 0U && new_region == NULL)) {
        free(old_region); free(new_region); return DOCUMENT_EDIT_OUT_OF_MEMORY;
    }
    if (!document_edit_read_range(edit, start, span_length, old_region)) {
        free(old_region); free(new_region); return DOCUMENT_EDIT_INVALID_OPERATION;
    }
    if (node_start < target_start) {
        size_t gap_length = target_start - node_end;
        if (before) { free(old_region); free(new_region); return DOCUMENT_EDIT_OK; }
        memcpy(new_region, old_region + node->span.length, gap_length);
        memcpy(new_region + gap_length, old_region + node->span.length + gap_length,
               target->span.length);
        memcpy(new_region + gap_length + target->span.length, old_region,
               node->span.length);
    } else {
        size_t gap_length = node_start - target_end;
        if (!before) { free(old_region); free(new_region); return DOCUMENT_EDIT_OK; }
        memcpy(new_region, old_region + target->span.length + gap_length,
               node->span.length);
        memcpy(new_region + node->span.length, old_region, target->span.length);
        memcpy(new_region + node->span.length + target->span.length,
               old_region + target->span.length, gap_length);
    }
    document_handle parent;
    document_source_span parent_span;
    if (!document_model_parent(edit->model, node_handle, &parent) ||
        !document_model_source_span(edit->model, parent, &parent_span)) {
        free(old_region); free(new_region); return DOCUMENT_EDIT_STALE_HANDLE;
    }
    bool valid = document_edit_validate_replace(edit, parent_span.offset,
        parent_span.length, start, span_length, new_region, span_length, false, false);
    document_edit_error result = valid ? document_edit_commit_replace(edit, start,
        span_length, new_region, span_length, node_handle.slot, false,
        DOCUMENT_EDIT_HISTORY_ACTION_MOVE) :
        DOCUMENT_EDIT_MALFORMED_XML;
    free(old_region); free(new_region);
    return result;
}

document_edit_error document_edit_wrap_in_group(document_edit *edit,
    document_handle node_handle, document_handle *out_group)
{
    if (edit == NULL || out_group == NULL)
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    document_edit_node_info *node = NULL;
    if (!document_edit_handle_info(edit, node_handle, &node))
        return DOCUMENT_EDIT_STALE_HANDLE;
    if (handle_is_root(edit, node_handle)) return DOCUMENT_EDIT_INVALID_OPERATION;
    size_t node_offset = node->span.offset;
    static const uint8_t prefix[] = "<g>";
    static const uint8_t suffix[] = "</g>";
    size_t new_length;
    if (!base_size_add(node->span.length, sizeof(prefix) - 1U, &new_length) ||
        !base_size_add(new_length, sizeof(suffix) - 1U, &new_length))
        return DOCUMENT_EDIT_LIMIT;
    uint8_t *wrapped = malloc(new_length);
    if (wrapped == NULL) return DOCUMENT_EDIT_OUT_OF_MEMORY;
    memcpy(wrapped, prefix, sizeof(prefix) - 1U);
    if (!document_edit_read_range(edit, node->span.offset, node->span.length,
            wrapped + sizeof(prefix) - 1U)) {
        free(wrapped); return DOCUMENT_EDIT_INVALID_OPERATION;
    }
    memcpy(wrapped + sizeof(prefix) - 1U + node->span.length,
           suffix, sizeof(suffix) - 1U);
    bool valid = document_edit_validate_replace(edit, node->span.offset,
        node->span.length, node->span.offset, node->span.length, wrapped,
        new_length, false, false);
    document_edit_error result = valid ? document_edit_commit_replace(edit,
        node_offset, node->span.length, wrapped, new_length,
        node_handle.slot, false, DOCUMENT_EDIT_HISTORY_ACTION_GROUP) : DOCUMENT_EDIT_MALFORMED_XML;
    free(wrapped);
    if (result != DOCUMENT_EDIT_OK) return result;
    return document_edit_element_at_offset(edit, node_offset, out_group)
        ? DOCUMENT_EDIT_OK : DOCUMENT_EDIT_INVALID_OPERATION;
}

document_edit_error document_edit_ungroup(document_edit *edit,
    document_handle group_handle)
{
    if (edit == NULL) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    document_edit_node_info *group = NULL;
    if (!document_edit_handle_info(edit, group_handle, &group))
        return DOCUMENT_EDIT_STALE_HANDLE;
    base_bytes tag;
    if (!document_model_tag_name(edit->model, group_handle, &tag))
        return DOCUMENT_EDIT_STALE_HANDLE;
    if (tag.len != 1U || tag.ptr[0] != 'g') return DOCUMENT_EDIT_INVALID_OPERATION;
    size_t inner_start = group->empty ? group->span.offset + group->span.length - 2U :
        group->open.offset + group->open.length;
    size_t inner_end = group->empty ? inner_start : group->close.offset;
    size_t inner_length = inner_end - inner_start;
    uint8_t *inner = inner_length == 0U ? NULL : malloc(inner_length);
    if (inner_length > 0U && inner == NULL) return DOCUMENT_EDIT_OUT_OF_MEMORY;
    if (inner_length > 0U && !document_edit_read_range(edit, inner_start,
            inner_length, inner)) { free(inner); return DOCUMENT_EDIT_INVALID_OPERATION; }
    size_t patch_offset = group->span.offset;
    size_t patch_length = group->span.length;
    size_t region_offset = patch_offset, region_length = patch_length;
    bool fragment = true;
    if (handle_is_root(edit, group_handle)) {
        region_offset = 0U;
        region_length = edit->byte_length;
        fragment = false;
    } else {
        document_handle parent;
        document_source_span parent_span;
        if (!document_model_parent(edit->model, group_handle, &parent) ||
            !document_model_source_span(edit->model, parent, &parent_span)) {
            free(inner); return DOCUMENT_EDIT_STALE_HANDLE;
        }
        region_offset = parent_span.offset;
        region_length = parent_span.length;
    }
    bool valid = document_edit_validate_replace(edit, region_offset, region_length,
        patch_offset, patch_length, inner, inner_length, fragment, false);
    document_edit_error result = valid ? document_edit_commit_replace(edit,
        patch_offset, patch_length, inner, inner_length,
        group_handle.slot, false, DOCUMENT_EDIT_HISTORY_ACTION_UNGROUP) : DOCUMENT_EDIT_MALFORMED_XML;
    free(inner);
    return result;
}

document_edit_save_result document_edit_save_as(document_edit *edit,
    base_bytes path, const platform_atomic_file_hooks *hooks)
{
    document_edit_save_result result = {0};
    result.error = DOCUMENT_EDIT_INVALID_ARGUMENT;
    if (edit == NULL || !base_bytes_valid(path) || path.len == 0U ||
        !document_edit_ensure_index(edit)) return result;
    result.source_result = source_save_noop((base_bytes){edit->snapshot,
        edit->snapshot_length}, path, edit->tokens, edit->token_count, hooks);
    if (result.source_result.error == SOURCE_SAVE_OK ||
        result.source_result.file_result.renamed) {
        edit->saved_state_id = edit->state_id;
    }
    result.error = result.source_result.error == SOURCE_SAVE_OK ? DOCUMENT_EDIT_OK :
        DOCUMENT_EDIT_SAVE_FAILED;
    return result;
}
