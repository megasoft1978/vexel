#include "document_edit_internal.h"

#include <stdlib.h>
#include <string.h>

#define DOCUMENT_EDIT_TEXT_ATTRIBUTE_LIMIT 128U

static bool text_fragment_append(uint8_t *output, size_t capacity, size_t *used,
    const uint8_t *bytes, size_t length)
{
    if (output == NULL || used == NULL || (length > 0U && bytes == NULL) ||
        *used > capacity || length > capacity - *used) return false;
    if (length > 0U) memcpy(output + *used, bytes, length);
    *used += length;
    return true;
}

document_edit_error document_edit_insert_text_element(document_edit *edit,
    document_handle parent, const document_edit_template_attribute *attributes,
    size_t attribute_count, base_bytes text, document_handle *out_inserted)
{
    if (edit == NULL || out_inserted == NULL || !base_bytes_valid(text) ||
        (attribute_count > 0U && attributes == NULL))
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    if (attribute_count > DOCUMENT_EDIT_TEXT_ATTRIBUTE_LIMIT)
        return DOCUMENT_EDIT_LIMIT;
    uint8_t *fragment = malloc(DOCUMENT_EDIT_XML_FRAGMENT_LIMIT);
    if (fragment == NULL) return DOCUMENT_EDIT_OUT_OF_MEMORY;
    size_t used = 0U;
    static const uint8_t start[] = "<text";
    bool okay = text_fragment_append(fragment, DOCUMENT_EDIT_XML_FRAGMENT_LIMIT,
        &used, start, sizeof(start) - 1U);
    for (size_t index = 0U; okay && index < attribute_count; ++index) {
        base_bytes name = attributes[index].name;
        base_bytes value = attributes[index].value;
        if (!document_edit_name_valid(name) || !base_bytes_valid(value)) {
            free(fragment);
            return DOCUMENT_EDIT_INVALID_NAME;
        }
        for (size_t prior = 0U; prior < index; ++prior) {
            if (attributes[prior].name.len == name.len &&
                memcmp(attributes[prior].name.ptr, name.ptr, name.len) == 0) {
                free(fragment);
                return DOCUMENT_EDIT_DUPLICATE_ATTRIBUTE;
            }
        }
        uint8_t *encoded = NULL;
        size_t encoded_length = 0U;
        if (!document_edit_encode_attribute(value, '"', &encoded, &encoded_length)) {
            free(fragment);
            return DOCUMENT_EDIT_OUT_OF_MEMORY;
        }
        static const uint8_t equals[] = "=\"";
        static const uint8_t quote[] = "\"";
        okay = text_fragment_append(fragment, DOCUMENT_EDIT_XML_FRAGMENT_LIMIT,
                &used, (const uint8_t *)" ", 1U) &&
            text_fragment_append(fragment, DOCUMENT_EDIT_XML_FRAGMENT_LIMIT,
                &used, name.ptr, name.len) &&
            text_fragment_append(fragment, DOCUMENT_EDIT_XML_FRAGMENT_LIMIT,
                &used, equals, sizeof(equals) - 1U) &&
            text_fragment_append(fragment, DOCUMENT_EDIT_XML_FRAGMENT_LIMIT,
                &used, encoded, encoded_length) &&
            text_fragment_append(fragment, DOCUMENT_EDIT_XML_FRAGMENT_LIMIT,
                &used, quote, sizeof(quote) - 1U);
        free(encoded);
    }
    static const uint8_t open_end[] = ">";
    static const uint8_t close[] = "</text>";
    uint8_t *encoded_text = NULL;
    size_t encoded_text_length = 0U;
    if (okay && !document_edit_encode_text(text, &encoded_text,
            &encoded_text_length)) okay = false;
    if (okay) okay = text_fragment_append(fragment,
            DOCUMENT_EDIT_XML_FRAGMENT_LIMIT, &used, open_end, 1U) &&
        text_fragment_append(fragment, DOCUMENT_EDIT_XML_FRAGMENT_LIMIT,
            &used, encoded_text, encoded_text_length) &&
        text_fragment_append(fragment, DOCUMENT_EDIT_XML_FRAGMENT_LIMIT,
            &used, close, sizeof(close) - 1U);
    free(encoded_text);
    if (!okay) {
        free(fragment);
        return DOCUMENT_EDIT_LIMIT;
    }
    document_edit_error result = document_edit_insert_xml_fragment(edit, parent,
        (base_bytes){fragment, used}, out_inserted);
    free(fragment);
    if (result == DOCUMENT_EDIT_OK)
        document_edit_history_relabel_latest(edit, DOCUMENT_EDIT_HISTORY_ACTION_ADD_TEXT);
    return result;
}
