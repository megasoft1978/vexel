#include "xml_tokenizer.h"
#include "xml_internal.h"
#include "xml_tokenizer_private.h"

#include <string.h>

static bool decode_append(uint32_t cp, uint8_t *out, size_t capacity,
                          size_t *length)
{
    uint8_t encoded[4];
    size_t count = 0U;
    if (cp <= 0x7fU) encoded[count++] = (uint8_t)cp;
    else if (cp <= 0x7ffU) {
        encoded[count++] = (uint8_t)(0xc0U | (cp >> 6U));
        encoded[count++] = (uint8_t)(0x80U | (cp & 0x3fU));
    } else if (cp <= 0xffffU && !(cp >= 0xd800U && cp <= 0xdfffU)) {
        encoded[count++] = (uint8_t)(0xe0U | (cp >> 12U));
        encoded[count++] = (uint8_t)(0x80U | ((cp >> 6U) & 0x3fU));
        encoded[count++] = (uint8_t)(0x80U | (cp & 0x3fU));
    } else if (cp <= 0x10ffffU) {
        encoded[count++] = (uint8_t)(0xf0U | (cp >> 18U));
        encoded[count++] = (uint8_t)(0x80U | ((cp >> 12U) & 0x3fU));
        encoded[count++] = (uint8_t)(0x80U | ((cp >> 6U) & 0x3fU));
        encoded[count++] = (uint8_t)(0x80U | (cp & 0x3fU));
    } else return false;
    if (count > capacity - *length) return false;
    memcpy(out + *length, encoded, count);
    *length += count;
    return true;
}

bool svg_xml_decode_attribute_value(base_bytes input, svg_xml_span raw,
    uint8_t *output, size_t output_capacity, size_t *output_length,
    size_t *error_offset)
{
    if (output_length == NULL || error_offset == NULL || !base_bytes_valid(input) ||
        raw.offset > input.len || raw.length > input.len - raw.offset ||
        (output == NULL && output_capacity != 0U)) return false;
    *output_length = 0U;
    *error_offset = 0U;
    size_t cursor = 0U;
    while (cursor < raw.length) {
        if (input.ptr[raw.offset + cursor] != '&') {
            uint32_t cp = 0U;
            size_t next = raw.offset + cursor;
            if (!svg_xml_internal_utf8_next(input.ptr, raw.offset + raw.length,
                                             &next, &cp) ||
                !svg_xml_internal_char(cp)) {
                *error_offset = cursor;
                return false;
            }
            size_t width = next - (raw.offset + cursor);
            if (width > output_capacity - *output_length) {
                *error_offset = cursor;
                return false;
            }
            memcpy(output + *output_length, input.ptr + raw.offset + cursor, width);
            *output_length += width;
            cursor += width;
            continue;
        }
        size_t start = cursor;
        size_t semicolon = start + 1U;
        while (semicolon < raw.length && semicolon - start <= 16U &&
               input.ptr[raw.offset + semicolon] != ';') ++semicolon;
        if (semicolon >= raw.length || semicolon - start > 16U ||
            input.ptr[raw.offset + semicolon] != ';') {
            *error_offset = start;
            return false;
        }
        size_t body = start + 1U;
        size_t body_length = semicolon - body;
        const uint8_t *value = input.ptr + raw.offset + body;
        uint32_t cp = 0U;
        if (body_length == 3U && memcmp(value, "amp", 3U) == 0) cp = '&';
        else if (body_length == 2U && memcmp(value, "lt", 2U) == 0) cp = '<';
        else if (body_length == 2U && memcmp(value, "gt", 2U) == 0) cp = '>';
        else if (body_length == 4U && memcmp(value, "quot", 4U) == 0) cp = '"';
        else if (body_length == 4U && memcmp(value, "apos", 4U) == 0) cp = '\'';
        else if (body_length >= 2U && value[0] == '#') {
            size_t digit = 1U;
            unsigned radix = 10U;
            if (digit < body_length && (value[digit] == 'x' || value[digit] == 'X')) {
                ++digit;
                radix = 16U;
            }
            if (digit == body_length) { *error_offset = start; return false; }
            for (; digit < body_length; ++digit) {
                uint8_t c = value[digit];
                unsigned n;
                if (c >= '0' && c <= '9') n = (unsigned)(c - '0');
                else if (radix == 16U && c >= 'a' && c <= 'f') n = (unsigned)(c - 'a') + 10U;
                else if (radix == 16U && c >= 'A' && c <= 'F') n = (unsigned)(c - 'A') + 10U;
                else { *error_offset = start; return false; }
                if (cp > (0x10ffffU - n) / radix) { *error_offset = start; return false; }
                cp = cp * radix + n;
            }
        } else { *error_offset = start; return false; }
        if (!svg_xml_internal_char(cp) ||
            !decode_append(cp, output, output_capacity, output_length)) {
            *error_offset = start;
            return false;
        }
        cursor = semicolon + 1U;
    }
    return true;
}

svg_xml_options svg_xml_default_options(void)
{
    svg_xml_options options = {{XML_MAX_INPUT_BYTES, XML_MAX_DEPTH, XML_MAX_NODES,
        XML_MAX_ATTRIBUTES_PER_ELEMENT, XML_MAX_TOTAL_ATTRIBUTES,
        XML_MAX_TEXT_RUN_BYTES}};
    return options;
}

static svg_xml_options bounded_options(svg_xml_options options)
{
    const svg_xml_options maximum = svg_xml_default_options();
    if (options.limits.input_bytes > maximum.limits.input_bytes)
        options.limits.input_bytes = maximum.limits.input_bytes;
    if (options.limits.nesting_depth > maximum.limits.nesting_depth)
        options.limits.nesting_depth = maximum.limits.nesting_depth;
    if (options.limits.nodes > maximum.limits.nodes)
        options.limits.nodes = maximum.limits.nodes;
    if (options.limits.attributes_per_element > maximum.limits.attributes_per_element)
        options.limits.attributes_per_element = maximum.limits.attributes_per_element;
    if (options.limits.total_attributes > maximum.limits.total_attributes)
        options.limits.total_attributes = maximum.limits.total_attributes;
    if (options.limits.text_run_bytes > maximum.limits.text_run_bytes)
        options.limits.text_run_bytes = maximum.limits.text_run_bytes;
    return options;
}

svg_xml_result svg_xml_tokenize(base_bytes input, svg_xml_options options,
                                svg_xml_token *tokens, size_t token_capacity)
{
    return svg_xml_tokenizer_run(input, bounded_options(options), tokens,
        token_capacity, NULL, NULL);
}

svg_xml_result svg_xml_tokenize_large(base_bytes input, svg_xml_token *tokens,
                                      size_t token_capacity)
{
    return svg_xml_tokenize_large_cancelable(input, tokens, token_capacity, NULL, NULL);
}

svg_xml_result svg_xml_tokenize_large_cancelable(base_bytes input, svg_xml_token *tokens,
    size_t token_capacity, svg_xml_cancel_check_fn canceled, void *cancel_context)
{
    svg_xml_options options = svg_xml_default_options();
    options.limits.input_bytes = 1024U * 1024U * 1024U;
    return svg_xml_tokenizer_run(input, options, tokens, token_capacity, canceled,
        cancel_context);
}

const char *svg_xml_error_name(svg_xml_error error)
{
    static const char *const names[] = {"none", "invalid_argument", "input_limit",
        "invalid_utf8", "invalid_character", "unsupported_encoding", "invalid_name",
        "malformed_syntax", "duplicate_attribute", "mismatched_tag", "depth_limit",
        "node_limit", "attribute_limit", "text_limit", "output_exhausted",
        "unsupported_dtd", "multiple_roots", "incomplete_document"};
    return (unsigned)error < sizeof(names) / sizeof(names[0]) ?
        names[(unsigned)error] : "unknown";
}
