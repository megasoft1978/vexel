#include "traverse_internal.h"

#include <stdint.h>

static uint8_t ascii_lower(uint8_t value)
{
    return value >= 'A' && value <= 'Z' ?
        (uint8_t)(value + ('a' - 'A')) : value;
}

static bool language_range_matches(base_bytes source, svg_xml_span range,
                                   base_bytes language)
{
    if (range.length == 1U && source.ptr[range.offset] == '*') return true;
    if (range.length > language.len || range.length == 0U) return false;
    for (size_t i = 0U; i < range.length; ++i)
        if (ascii_lower(source.ptr[range.offset + i]) !=
            ascii_lower(language.ptr[i])) return false;
    return range.length == language.len ||
           language.ptr[range.length] == '-';
}

static bool switch_child_matches(shape_traversal *traversal, size_t element,
                                 bool *matches)
{
    *matches = true;
    svg_xml_span ignored = {0};
    if (svg_shape_traverse_attr(traversal->source, traversal->tokens,
            traversal->token_count, element, "requiredFeatures", NULL,
            &ignored) ||
        svg_shape_traverse_attr(traversal->source, traversal->tokens,
            traversal->token_count, element, "requiredExtensions", NULL,
            &ignored)) {
        svg_shape_traverse_fail(traversal->diagnostic,
            SVG_SHAPE_ERROR_UNSUPPORTED, ignored.offset);
        return false;
    }
    svg_xml_span languages = {0};
    if (!svg_shape_traverse_attr(traversal->source, traversal->tokens,
            traversal->token_count, element, "systemLanguage", NULL,
            &languages)) return true;
    if (languages.length == 0U || languages.offset > traversal->source.len ||
        languages.length > traversal->source.len - languages.offset) {
        svg_shape_traverse_fail(traversal->diagnostic,
            SVG_SHAPE_ERROR_UNSUPPORTED, languages.offset);
        return false;
    }
    size_t cursor = 0U;
    bool any_match = false;
    while (cursor < languages.length) {
        while (cursor < languages.length &&
               (traversal->source.ptr[languages.offset + cursor] == ' ' ||
                traversal->source.ptr[languages.offset + cursor] == '\t' ||
                traversal->source.ptr[languages.offset + cursor] == '\r' ||
                traversal->source.ptr[languages.offset + cursor] == '\n'))
            cursor++;
        size_t first = cursor;
        while (cursor < languages.length &&
               traversal->source.ptr[languages.offset + cursor] != ',')
            cursor++;
        size_t last = cursor;
        while (last > first &&
               (traversal->source.ptr[languages.offset + last - 1U] == ' ' ||
                traversal->source.ptr[languages.offset + last - 1U] == '\t' ||
                traversal->source.ptr[languages.offset + last - 1U] == '\r' ||
                traversal->source.ptr[languages.offset + last - 1U] == '\n'))
            last--;
        if (last == first) {
            svg_shape_traverse_fail(traversal->diagnostic,
                SVG_SHAPE_ERROR_UNSUPPORTED, languages.offset + first);
            return false;
        }
        svg_xml_span range = {languages.offset + first, last - first};
        for (size_t i = 0U; i < range.length; ++i) {
            uint8_t byte = traversal->source.ptr[range.offset + i];
            if (!((byte >= 'a' && byte <= 'z') ||
                  (byte >= 'A' && byte <= 'Z') ||
                  (byte >= '0' && byte <= '9') || byte == '-' || byte == '*')) {
                svg_shape_traverse_fail(traversal->diagnostic,
                    SVG_SHAPE_ERROR_UNSUPPORTED, range.offset + i);
                return false;
            }
        }
        if (language_range_matches(traversal->source, range,
                                   traversal->system_language))
            any_match = true;
        if (cursor < languages.length) {
            cursor++;
            if (cursor == languages.length) {
                svg_shape_traverse_fail(traversal->diagnostic,
                    SVG_SHAPE_ERROR_UNSUPPORTED, languages.offset + cursor - 1U);
                return false;
            }
        }
    }
    *matches = any_match;
    return true;
}

bool svg_shape_traverse_select_switch_child(shape_traversal *traversal, size_t element,
                                size_t end, size_t *child_first,
                                size_t *child_limit, bool *found)
{
    *found = false;
    for (size_t i = element + 1U; i < end;) {
        const svg_xml_token *token = &traversal->tokens[i];
        if ((token->kind != SVG_XML_TOKEN_START_ELEMENT &&
             token->kind != SVG_XML_TOKEN_EMPTY_ELEMENT) ||
            token->depth != traversal->tokens[element].depth + 1U) {
            i++;
            continue;
        }
        size_t child_end = svg_shape_traverse_matching_end(traversal, i);
        if (child_end == SIZE_MAX || child_end >= end) {
            svg_shape_traverse_fail(traversal->diagnostic,
                SVG_SHAPE_ERROR_XML, token->name.offset);
            return false;
        }
        bool matches = false;
        if (!switch_child_matches(traversal, i, &matches)) return false;
        if (matches) {
            *child_first = i;
            *child_limit = child_end + 1U;
            *found = true;
            return true;
        }
        i = child_end + 1U;
    }
    return true;
}

