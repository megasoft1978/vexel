#include "document_edit_internal.h"

#include <stdlib.h>
#include <string.h>

#define DOCUMENT_EDIT_TOKEN_BYTES (64U * 1024U * 1024U)
#define DOCUMENT_EDIT_MAX_COMMANDS 300000U
#define DOCUMENT_EDIT_MAX_HISTORY_BYTES (1024U * 1024U * 1024U)
#define DOCUMENT_EDIT_MAX_PATH_DEPTH 256U

static bool size_for_count(size_t count, size_t item_size, size_t *out_size)
{
    return count == 0U || base_size_mul(count, item_size, out_size);
}

static bool token_stream(base_bytes source, svg_xml_token **out_tokens,
                         size_t *out_count, svg_xml_result *out_result)
{
    svg_xml_token_stream *stream = NULL;
    svg_xml_stream_status status = svg_xml_token_stream_start(source,
        DOCUMENT_EDIT_TOKEN_BYTES, NULL, NULL, &stream);
    if (status != SVG_XML_STREAM_OK) return false;
    do { status = svg_xml_token_stream_step(stream, 64U * 1024U); }
    while (status == SVG_XML_STREAM_OK);
    *out_result = svg_xml_token_stream_result(stream);
    bool taken = status == SVG_XML_STREAM_COMPLETE &&
        out_result->diagnostic.category == SVG_XML_ERROR_NONE &&
        svg_xml_token_stream_take(stream, out_tokens, out_count);
    svg_xml_token_stream_destroy(stream);
    return taken;
}

void document_edit_invalidate_index(document_edit *edit)
{
    if (edit == NULL) return;
    document_model_destroy(edit->model);
    edit->model = NULL;
    free(edit->snapshot);
    free(edit->tokens);
    free(edit->nodes);
    free(edit->attributes);
    free(edit->texts);
    edit->snapshot = NULL;
    edit->tokens = NULL;
    edit->nodes = NULL;
    edit->attributes = NULL;
    edit->texts = NULL;
    edit->snapshot_length = 0U;
    edit->token_count = 0U;
    edit->node_count = 0U;
    edit->attribute_count = 0U;
    edit->text_count = 0U;
}

static bool build_index(document_edit *edit)
{
    edit->last_error = DOCUMENT_EDIT_OUT_OF_MEMORY;
    memset(&edit->last_diagnostic, 0, sizeof(edit->last_diagnostic));
    size_t snapshot_size = edit->byte_length == 0U ? 1U : edit->byte_length;
    uint8_t *snapshot = malloc(snapshot_size);
    if (snapshot == NULL) return false;
    if (edit->byte_length > 0U &&
        !document_edit_read_range(edit, 0U, edit->byte_length, snapshot)) {
        free(snapshot);
        return false;
    }
    base_bytes bytes = {snapshot, edit->byte_length};
    document_model_result model_result = document_model_parse(bytes);
    if (model_result.error != DOCUMENT_MODEL_OK || model_result.model == NULL) {
        edit->last_diagnostic = model_result.parse_diagnostic;
        if (model_result.error == DOCUMENT_MODEL_LIMIT ||
            model_result.error == DOCUMENT_MODEL_RESOURCE_EXHAUSTED)
            edit->last_error = DOCUMENT_EDIT_LIMIT;
        else if (model_result.error == DOCUMENT_MODEL_PARSE)
            edit->last_error = DOCUMENT_EDIT_MALFORMED_XML;
        else edit->last_error = DOCUMENT_EDIT_OUT_OF_MEMORY;
        free(snapshot);
        return false;
    }
    svg_xml_token *tokens = NULL;
    size_t token_count = 0U;
    svg_xml_result parsed = {0};
    if (!token_stream(bytes, &tokens, &token_count, &parsed)) {
        edit->last_diagnostic = parsed.diagnostic;
        edit->last_error = parsed.diagnostic.category == SVG_XML_ERROR_OUTPUT_EXHAUSTED ?
            DOCUMENT_EDIT_LIMIT : DOCUMENT_EDIT_OUT_OF_MEMORY;
        document_model_destroy(model_result.model);
        free(snapshot);
        return false;
    }
    size_t node_count = parsed.element_count;
    size_t attribute_count = parsed.attribute_count;
    size_t text_count = 0U;
    for (size_t i = 0U; i < token_count; ++i) {
        if ((tokens[i].kind == SVG_XML_TOKEN_TEXT ||
             tokens[i].kind == SVG_XML_TOKEN_CDATA) && tokens[i].depth > 0U)
            ++text_count;
    }
    size_t node_bytes = 0U, attribute_bytes = 0U, text_bytes = 0U;
    if (!size_for_count(node_count, sizeof(document_edit_node_info), &node_bytes) ||
        !size_for_count(attribute_count, sizeof(document_edit_attribute_info),
                        &attribute_bytes) ||
        !size_for_count(text_count, sizeof(document_edit_text_info), &text_bytes)) {
        edit->last_error = DOCUMENT_EDIT_LIMIT;
        free(tokens);
        document_model_destroy(model_result.model);
        free(snapshot);
        return false;
    }
    document_edit_node_info *nodes = node_count == 0U ? NULL : calloc(1U, node_bytes);
    document_edit_attribute_info *attributes = attribute_count == 0U ? NULL :
        calloc(1U, attribute_bytes);
    document_edit_text_info *texts = text_count == 0U ? NULL : calloc(1U, text_bytes);
    if ((node_count > 0U && nodes == NULL) ||
        (attribute_count > 0U && attributes == NULL) ||
        (text_count > 0U && texts == NULL)) {
        edit->last_error = DOCUMENT_EDIT_OUT_OF_MEMORY;
        free(nodes); free(attributes); free(texts); free(tokens);
        document_model_destroy(model_result.model);
        free(snapshot);
        return false;
    }

    uint32_t stack[DOCUMENT_EDIT_MAX_PATH_DEPTH];
    size_t depth = 0U, next_node = 0U, next_attribute = 0U, next_text = 0U;
    bool valid = true;
    for (size_t i = 0U; i < token_count && valid; ++i) {
        const svg_xml_token *token = &tokens[i];
        if (token->kind == SVG_XML_TOKEN_START_ELEMENT ||
            token->kind == SVG_XML_TOKEN_EMPTY_ELEMENT) {
            if (next_node >= node_count || token->source.offset > bytes.len ||
                token->source.length > bytes.len - token->source.offset ||
                depth >= DOCUMENT_EDIT_MAX_PATH_DEPTH) { valid = false; break; }
            document_edit_node_info *node = &nodes[next_node];
            node->span = (document_source_span){token->source.offset, token->source.length};
            node->name = token->name;
            node->open = token->source;
            node->close = (svg_xml_span){token->source.offset + token->source.length, 0U};
            node->first_attribute = next_attribute;
            node->empty = token->kind == SVG_XML_TOKEN_EMPTY_ELEMENT;
            while (i + 1U < token_count &&
                   tokens[i + 1U].kind == SVG_XML_TOKEN_ATTRIBUTE) {
                const svg_xml_token *attribute = &tokens[++i];
                if (next_attribute >= attribute_count || attribute->value.offset == 0U ||
                    attribute->source.offset > bytes.len ||
                    attribute->source.length > bytes.len - attribute->source.offset ||
                    attribute->value.offset > bytes.len) { valid = false; break; }
                uint8_t quote = snapshot[attribute->value.offset - 1U];
                if (quote != '\'' && quote != '"') { valid = false; break; }
                attributes[next_attribute++] = (document_edit_attribute_info){
                    attribute->name, attribute->value, attribute->source, quote
                };
                ++node->attribute_count;
            }
            if (!valid) break;
            if (!node->empty) stack[depth++] = (uint32_t)next_node;
            ++next_node;
        } else if (token->kind == SVG_XML_TOKEN_END_ELEMENT) {
            if (depth == 0U || token->source.offset > bytes.len ||
                token->source.length > bytes.len - token->source.offset) {
                valid = false; break;
            }
            document_edit_node_info *node = &nodes[stack[--depth]];
            size_t end = token->source.offset + token->source.length;
            if (end < node->span.offset) { valid = false; break; }
            node->span.length = end - node->span.offset;
            node->close = token->source;
        } else if ((token->kind == SVG_XML_TOKEN_TEXT ||
                    token->kind == SVG_XML_TOKEN_CDATA) && depth > 0U) {
            if (next_text >= text_count || token->value.offset > bytes.len ||
                token->value.length > bytes.len - token->value.offset) {
                valid = false; break;
            }
            texts[next_text++] = (document_edit_text_info){stack[depth - 1U],
                token->value, token->kind == SVG_XML_TOKEN_CDATA};
        }
    }
    if (depth != 0U || next_node != node_count || next_attribute != attribute_count ||
        next_text != text_count) valid = false;
    if (!valid) {
        edit->last_error = DOCUMENT_EDIT_MALFORMED_XML;
        free(nodes); free(attributes); free(texts); free(tokens);
        document_model_destroy(model_result.model);
        free(snapshot);
        return false;
    }
    edit->snapshot = snapshot;
    edit->snapshot_length = edit->byte_length;
    edit->model = model_result.model;
    edit->tokens = tokens;
    edit->token_count = token_count;
    edit->nodes = nodes;
    edit->node_count = node_count;
    edit->attributes = attributes;
    edit->attribute_count = attribute_count;
    edit->texts = texts;
    edit->text_count = text_count;
    edit->last_error = DOCUMENT_EDIT_OK;
    return true;
}

bool document_edit_ensure_index(document_edit *edit)
{
    return edit != NULL && (edit->model != NULL || build_index(edit));
}

bool document_edit_handle_info(document_edit *edit, document_handle handle,
                               document_edit_node_info **out_info)
{
    if (edit == NULL || out_info == NULL || edit->model == NULL ||
        handle.slot >= edit->node_count ||
        !document_model_tag_name(edit->model, handle, &(base_bytes){0})) return false;
    *out_info = &edit->nodes[handle.slot];
    return true;
}

document_edit_limits document_edit_default_limits(void)
{
    return (document_edit_limits){1000U, 64U * 1024U * 1024U};
}

document_edit_result document_edit_create(base_bytes source, document_edit_limits limits)
{
    document_edit_result result = {0};
    result.error = DOCUMENT_EDIT_INVALID_ARGUMENT;
    if (!base_bytes_valid(source)) return result;
    svg_xml_options options = svg_xml_default_options();
    if (source.len > options.limits.input_bytes) {
        result.error = DOCUMENT_EDIT_LIMIT;
        result.parse_diagnostic.category = SVG_XML_ERROR_INPUT_LIMIT;
        return result;
    }
    document_edit_limits defaults = document_edit_default_limits();
    if (limits.history_commands == 0U) limits.history_commands = defaults.history_commands;
    if (limits.history_bytes == 0U) limits.history_bytes = defaults.history_bytes;
    if (limits.history_commands > DOCUMENT_EDIT_MAX_COMMANDS ||
        limits.history_bytes > DOCUMENT_EDIT_MAX_HISTORY_BYTES) {
        result.error = DOCUMENT_EDIT_LIMIT;
        return result;
    }
    document_edit *edit = calloc(1U, sizeof(*edit));
    if (edit == NULL) {
        result.error = DOCUMENT_EDIT_OUT_OF_MEMORY;
        return result;
    }
    edit->original = source;
    edit->limits = limits;
    edit->byte_length = source.len;
    edit->state_id = 1U;
    edit->saved_state_id = 1U;
    edit->next_state_id = 2U;
    if (source.len > 0U) {
        edit->pieces = calloc(1U, sizeof(*edit->pieces));
        if (edit->pieces == NULL) {
            free(edit);
            result.error = DOCUMENT_EDIT_OUT_OF_MEMORY;
            return result;
        }
        edit->pieces[0] = (document_edit_piece){0U, source.len, NULL};
        edit->piece_count = 1U;
        edit->piece_capacity = 1U;
    }
    if (!document_edit_ensure_index(edit)) {
        result.error = edit->last_error;
        result.parse_diagnostic = edit->last_diagnostic;
        document_edit_destroy(edit);
        return result;
    }
    result.error = DOCUMENT_EDIT_OK;
    result.edit = edit;
    return result;
}

void document_edit_destroy(document_edit *edit)
{
    if (edit == NULL) return;
    document_edit_invalidate_index(edit);
    document_edit_piece_table_destroy(edit);
    memset(edit, 0, sizeof(*edit));
    free(edit);
}

const document_model *document_edit_model(document_edit *edit)
{
    return document_edit_ensure_index(edit) ? edit->model : NULL;
}

bool document_edit_root(document_edit *edit, document_handle *out)
{
    return document_edit_ensure_index(edit) && document_model_root(edit->model, out);
}

bool document_edit_span_bytes(document_edit *edit, document_source_span span,
                              base_bytes *out_bytes)
{
    if (edit == NULL || out_bytes == NULL || !document_edit_ensure_index(edit) ||
        span.offset > edit->snapshot_length ||
        span.length > edit->snapshot_length - span.offset) return false;
    out_bytes->ptr = edit->snapshot + span.offset;
    out_bytes->len = span.length;
    return true;
}

bool document_edit_source_span(document_edit *edit, document_handle node,
                               document_source_span *out_span)
{
    return document_edit_ensure_index(edit) &&
        document_model_source_span(edit->model, node, out_span);
}

bool document_edit_tag_name(document_edit *edit, document_handle node,
                            base_bytes *out_name)
{
    return document_edit_ensure_index(edit) &&
        document_model_tag_name(edit->model, node, out_name);
}

size_t document_edit_attribute_count(document_edit *edit, document_handle node)
{
    if (!document_edit_ensure_index(edit)) return 0U;
    return document_model_attribute_count(edit->model, node);
}

bool document_edit_attribute_at(document_edit *edit, document_handle node, size_t index,
                                document_edit_attribute *out_attribute)
{
    document_edit_node_info *info = NULL;
    if (out_attribute == NULL || !document_edit_handle_info(edit, node, &info) ||
        index >= info->attribute_count) return false;
    document_edit_attribute_info *attribute =
        &edit->attributes[info->first_attribute + index];
    *out_attribute = (document_edit_attribute){
        {attribute->name.offset, attribute->name.length},
        {attribute->value.offset, attribute->value.length}, attribute->quote
    };
    return true;
}

static bool emit_bytes(uint8_t *buffer, size_t capacity, size_t *used,
                       const uint8_t *bytes, size_t length)
{
    size_t end;
    if (!base_size_add(*used, length, &end)) return false;
    if (buffer != NULL && end <= capacity && length > 0U)
        memcpy(buffer + *used, bytes, length);
    *used = end;
    return buffer == NULL || end <= capacity;
}

static size_t encode_codepoint(uint32_t codepoint, uint8_t out[4])
{
    if (codepoint <= 0x7fU) { out[0] = (uint8_t)codepoint; return 1U; }
    if (codepoint <= 0x7ffU) {
        out[0] = (uint8_t)(0xc0U | (codepoint >> 6U));
        out[1] = (uint8_t)(0x80U | (codepoint & 0x3fU)); return 2U;
    }
    if (codepoint <= 0xffffU) {
        out[0] = (uint8_t)(0xe0U | (codepoint >> 12U));
        out[1] = (uint8_t)(0x80U | ((codepoint >> 6U) & 0x3fU));
        out[2] = (uint8_t)(0x80U | (codepoint & 0x3fU)); return 3U;
    }
    out[0] = (uint8_t)(0xf0U | (codepoint >> 18U));
    out[1] = (uint8_t)(0x80U | ((codepoint >> 12U) & 0x3fU));
    out[2] = (uint8_t)(0x80U | ((codepoint >> 6U) & 0x3fU));
    out[3] = (uint8_t)(0x80U | (codepoint & 0x3fU)); return 4U;
}

bool document_edit_decode_bytes(base_bytes raw, uint8_t *buffer,
                                size_t capacity, size_t *out_length)
{
    if (!base_bytes_valid(raw) || out_length == NULL ||
        (capacity > 0U && buffer == NULL)) return false;
    size_t p = 0U, used = 0U;
    bool fits = true;
    while (p < raw.len) {
        if (raw.ptr[p] != '&') {
            size_t start = p++;
            while (p < raw.len && raw.ptr[p] != '&') ++p;
            if (!emit_bytes(buffer, capacity, &used, raw.ptr + start, p - start)) fits = false;
            continue;
        }
        size_t start = ++p;
        while (p < raw.len && raw.ptr[p] != ';') ++p;
        if (p >= raw.len) return false;
        size_t length = p - start;
        ++p;
        const uint8_t *replacement = NULL;
        size_t replacement_length = 0U;
        uint8_t encoded[4];
        if (length == 3U && memcmp(raw.ptr + start, "amp", 3U) == 0) {
            replacement = (const uint8_t *)"&"; replacement_length = 1U;
        } else if (length == 2U && memcmp(raw.ptr + start, "lt", 2U) == 0) {
            replacement = (const uint8_t *)"<"; replacement_length = 1U;
        } else if (length == 2U && memcmp(raw.ptr + start, "gt", 2U) == 0) {
            replacement = (const uint8_t *)">"; replacement_length = 1U;
        } else if (length == 4U && memcmp(raw.ptr + start, "quot", 4U) == 0) {
            replacement = (const uint8_t *)"\""; replacement_length = 1U;
        } else if (length == 4U && memcmp(raw.ptr + start, "apos", 4U) == 0) {
            replacement = (const uint8_t *)"'"; replacement_length = 1U;
        } else if (length >= 2U && raw.ptr[start] == '#') {
            size_t digit = start + 1U;
            unsigned base = 10U;
            if (digit < start + length &&
                (raw.ptr[digit] == 'x' || raw.ptr[digit] == 'X')) {
                base = 16U;
                ++digit;
            }
            if (digit >= start + length) return false;
            uint32_t codepoint = 0U;
            for (; digit < start + length; ++digit) {
                uint8_t c = raw.ptr[digit];
                unsigned value;
                if (c >= '0' && c <= '9') value = (unsigned)(c - '0');
                else if (base == 16U && c >= 'a' && c <= 'f') value = (unsigned)(c - 'a') + 10U;
                else if (base == 16U && c >= 'A' && c <= 'F') value = (unsigned)(c - 'A') + 10U;
                else return false;
                if (value >= base || codepoint > (UINT32_MAX - value) / base) return false;
                codepoint = codepoint * base + value;
            }
            replacement_length = encode_codepoint(codepoint, encoded);
            replacement = encoded;
        } else return false;
        if (!emit_bytes(buffer, capacity, &used, replacement, replacement_length)) fits = false;
    }
    *out_length = used;
    return fits;
}

bool document_edit_decode_span(document_edit *edit, document_source_span span,
                               uint8_t *buffer, size_t capacity, size_t *out_length)
{
    base_bytes bytes;
    return document_edit_span_bytes(edit, span, &bytes) &&
        document_edit_decode_bytes(bytes, buffer, capacity, out_length);
}

bool document_edit_span_equals(document_edit *edit, svg_xml_span span,
                               base_bytes bytes)
{
    if (edit == NULL || !base_bytes_valid(bytes) || span.offset > edit->snapshot_length ||
        span.length > edit->snapshot_length - span.offset || span.length != bytes.len)
        return false;
    return bytes.len == 0U || memcmp(edit->snapshot + span.offset, bytes.ptr, bytes.len) == 0;
}

bool document_edit_find_id(document_edit *edit, base_bytes id, document_handle *out)
{
    if (edit == NULL || out == NULL || !base_bytes_valid(id) ||
        !document_edit_ensure_index(edit)) return false;
    static const uint8_t id_name[] = "id";
    for (size_t node_index = 0U; node_index < edit->node_count; ++node_index) {
        const document_edit_node_info *node = &edit->nodes[node_index];
        for (size_t i = 0U; i < node->attribute_count; ++i) {
            const document_edit_attribute_info *attribute =
                &edit->attributes[node->first_attribute + i];
            if (!document_edit_span_equals(edit, attribute->name,
                    (base_bytes){id_name, sizeof(id_name) - 1U})) continue;
            uint8_t *decoded = attribute->value.length == 0U ? NULL :
                malloc(attribute->value.length);
            if (attribute->value.length > 0U && decoded == NULL) return false;
            size_t decoded_length = 0U;
            bool decoded_ok = document_edit_decode_bytes((base_bytes){
                edit->snapshot + attribute->value.offset, attribute->value.length},
                decoded, attribute->value.length, &decoded_length);
            bool match = decoded_ok && decoded_length == id.len &&
                (id.len == 0U || memcmp(decoded, id.ptr, id.len) == 0);
            free(decoded);
            if (match) return document_model_node_at(edit->model, node_index, out);
        }
    }
    return false;
}

bool document_edit_element_at_offset(document_edit *edit, size_t offset,
                                     document_handle *out)
{
    if (edit == NULL || out == NULL || !document_edit_ensure_index(edit) ||
        offset >= edit->snapshot_length) return false;
    size_t best_depth = 0U;
    bool found = false;
    size_t best_slot = 0U;
    for (size_t i = 0U; i < edit->node_count; ++i) {
        const document_edit_node_info *node = &edit->nodes[i];
        if (offset < node->span.offset || offset - node->span.offset >= node->span.length)
            continue;
        document_handle candidate;
        size_t depth = 0U;
        if (!document_model_node_at(edit->model, i, &candidate) ||
            !document_model_depth(edit->model, candidate, &depth)) continue;
        if (!found || depth > best_depth) {
            found = true; best_depth = depth; best_slot = i;
        }
    }
    return found && document_model_node_at(edit->model, best_slot, out);
}

static bool append_path_segment(document_edit *edit, document_handle node,
                                uint8_t *buffer, size_t capacity, size_t *used)
{
    base_bytes name;
    if (!document_model_tag_name(edit->model, node, &name)) return false;
    size_t same_tag_index = 1U;
    document_handle parent;
    if (document_model_parent(edit->model, node, &parent)) {
        document_handle sibling;
        if (document_model_first_child(edit->model, parent, &sibling)) {
            while (sibling.slot != node.slot) {
                base_bytes sibling_name;
                if (document_model_tag_name(edit->model, sibling, &sibling_name) &&
                    sibling_name.len == name.len &&
                    memcmp(sibling_name.ptr, name.ptr, name.len) == 0) ++same_tag_index;
                document_handle next;
                if (!document_model_next_sibling(edit->model, sibling, &next)) break;
                sibling = next;
            }
        }
    }
    if (!emit_bytes(buffer, capacity, used, (const uint8_t *)"/", 1U) ||
        !emit_bytes(buffer, capacity, used, name.ptr, name.len)) return false;
    uint8_t digits[24];
    size_t digit_count = 0U;
    size_t value = same_tag_index;
    do {
        digits[digit_count++] = (uint8_t)('0' + (value % 10U));
        value /= 10U;
    } while (value > 0U && digit_count < sizeof(digits));
    if (!emit_bytes(buffer, capacity, used, (const uint8_t *)"[", 1U)) return false;
    while (digit_count > 0U) {
        uint8_t digit = digits[--digit_count];
        if (!emit_bytes(buffer, capacity, used, &digit, 1U)) return false;
    }
    return emit_bytes(buffer, capacity, used, (const uint8_t *)"]", 1U);
}

bool document_edit_element_path(document_edit *edit, document_handle node,
                                uint8_t *buffer, size_t capacity, size_t *out_length)
{
    if (edit == NULL || out_length == NULL || (capacity > 0U && buffer == NULL) ||
        !document_edit_ensure_index(edit)) return false;
    document_handle chain[DOCUMENT_EDIT_MAX_PATH_DEPTH];
    size_t count = 0U;
    document_handle current = node;
    while (count < DOCUMENT_EDIT_MAX_PATH_DEPTH) {
        if (!document_model_tag_name(edit->model, current, &(base_bytes){0})) return false;
        chain[count++] = current;
        document_handle parent;
        if (!document_model_parent(edit->model, current, &parent)) break;
        current = parent;
    }
    if (count == DOCUMENT_EDIT_MAX_PATH_DEPTH) return false;
    size_t used = 0U;
    while (count > 0U) {
        if (!append_path_segment(edit, chain[--count], buffer, capacity, &used)) {
            *out_length = used;
            return false;
        }
    }
    *out_length = used;
    return true;
}

bool document_edit_text_content(document_edit *edit, document_handle node,
                                uint8_t *buffer, size_t capacity, size_t *out_length)
{
    document_edit_node_info *info = NULL;
    if (out_length == NULL || (capacity > 0U && buffer == NULL) ||
        !document_edit_handle_info(edit, node, &info)) return false;
    size_t used = 0U;
    bool fits = true;
    size_t node_end = info->span.offset + info->span.length;
    for (size_t i = 0U; i < edit->text_count; ++i) {
        const document_edit_text_info *text = &edit->texts[i];
        if (text->value.offset < info->span.offset ||
            text->value.offset >= node_end ||
            text->value.length > node_end - text->value.offset) continue;
        const uint8_t *raw = edit->snapshot + text->value.offset;
        if (text->cdata) {
            if (!emit_bytes(buffer, capacity, &used, raw, text->value.length)) fits = false;
        } else {
            size_t decoded_length = 0U;
            bool decoded = document_edit_decode_bytes((base_bytes){raw, text->value.length},
                                                       NULL, 0U, &decoded_length);
            if (!decoded && decoded_length == 0U && text->value.length > 0U) return false;
            if (buffer != NULL && used <= capacity && decoded_length <= capacity - used) {
                size_t written = 0U;
                if (!document_edit_decode_bytes((base_bytes){raw, text->value.length},
                        buffer + used, capacity - used, &written)) fits = false;
            } else if (buffer != NULL) fits = false;
            if (!base_size_add(used, decoded_length, &used)) return false;
        }
    }
    *out_length = used;
    return fits;
}

const char *document_edit_error_name(document_edit_error error)
{
    switch (error) {
        case DOCUMENT_EDIT_OK: return "ok";
        case DOCUMENT_EDIT_INVALID_ARGUMENT: return "invalid_argument";
        case DOCUMENT_EDIT_STALE_HANDLE: return "stale_handle";
        case DOCUMENT_EDIT_NOT_FOUND: return "not_found";
        case DOCUMENT_EDIT_DUPLICATE_ATTRIBUTE: return "duplicate_attribute";
        case DOCUMENT_EDIT_INVALID_NAME: return "invalid_name";
        case DOCUMENT_EDIT_MALFORMED_XML: return "malformed_xml";
        case DOCUMENT_EDIT_ROOT_DELETE: return "root_delete";
        case DOCUMENT_EDIT_INVALID_OPERATION: return "invalid_operation";
        case DOCUMENT_EDIT_LIMIT: return "limit";
        case DOCUMENT_EDIT_HISTORY_LIMIT: return "history_limit";
        case DOCUMENT_EDIT_OUT_OF_MEMORY: return "out_of_memory";
        case DOCUMENT_EDIT_SAVE_FAILED: return "save_failed";
        default: return "unknown";
    }
}
