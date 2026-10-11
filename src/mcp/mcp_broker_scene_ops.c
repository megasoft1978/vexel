#include "mcp_broker_scene_internal.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MCP_SCENE_BATCH_LIMIT 32U
#define MCP_SCENE_SOURCE_LIMIT (64U * 1024U * 1024U)
#define MCP_SCENE_CHANGE_SPAN_LIMIT DOCUMENT_EDIT_XML_FRAGMENT_LIMIT
#define MCP_SCENE_ATTRIBUTE_LIMIT 32U
#define MCP_SCENE_NAME_CAP 128U
#define MCP_SCENE_VALUE_CAP 2048U
#define MCP_SCENE_PATH_REF_CAP 2048U

typedef struct scene_ref {
    char requested[256];
    size_t requested_length;
    char path[MCP_SCENE_PATH_REF_CAP];
    size_t path_length;
} scene_ref;

static bool string_equals(const char *value, size_t length, const char *literal)
{
    return strlen(literal) == length && memcmp(value, literal, length) == 0;
}

static bool safe_reference_value(const char *value, size_t length)
{
    for (size_t index = 0U; index + 11U <= length; index++) {
        if ((value[index] == 'j' || value[index] == 'J') &&
            (value[index + 1U] == 'a' || value[index + 1U] == 'A') &&
            (value[index + 2U] == 'v' || value[index + 2U] == 'V') &&
            (value[index + 3U] == 'a' || value[index + 3U] == 'A') &&
            (value[index + 4U] == 's' || value[index + 4U] == 'S') &&
            (value[index + 5U] == 'c' || value[index + 5U] == 'C') &&
            (value[index + 6U] == 'r' || value[index + 6U] == 'R') &&
            (value[index + 7U] == 'i' || value[index + 7U] == 'I') &&
            (value[index + 8U] == 'p' || value[index + 8U] == 'P') &&
            (value[index + 9U] == 't' || value[index + 9U] == 'T') &&
            value[index + 10U] == ':') return false;
    }
    for (size_t index = 0U; index + 4U <= length; index++) {
        if ((value[index] == 'u' || value[index] == 'U') &&
            (value[index + 1U] == 'r' || value[index + 1U] == 'R') &&
            (value[index + 2U] == 'l' || value[index + 2U] == 'L') &&
            value[index + 3U] == '(') {
            size_t cursor = index + 4U;
            while (cursor < length && (value[cursor] == ' ' || value[cursor] == '\t' ||
                value[cursor] == '\'' || value[cursor] == '"')) cursor++;
            if (cursor >= length || value[cursor] != '#') return false;
            while (cursor < length && value[cursor] != ')') cursor++;
            if (cursor == length) return false;
            index = cursor;
        }
    }
    return true;
}

static int base64_value(char value)
{
    if (value >= 'A' && value <= 'Z') return value - 'A';
    if (value >= 'a' && value <= 'z') return value - 'a' + 26;
    if (value >= '0' && value <= '9') return value - '0' + 52;
    if (value == '+') return 62;
    if (value == '/') return 63;
    return -1;
}

static bool safe_image_data_uri(const char *value, size_t length)
{
    static const char png[] = "data:image/png;base64,";
    static const char jpeg[] = "data:image/jpeg;base64,";
    size_t png_length = sizeof(png) - 1U;
    size_t jpeg_length = sizeof(jpeg) - 1U;
    size_t prefix_length = 0U;
    if (length > png_length && memcmp(value, png, png_length) == 0)
        prefix_length = png_length;
    else if (length > jpeg_length && memcmp(value, jpeg, jpeg_length) == 0)
        prefix_length = jpeg_length;
    else return false;
    size_t payload_length = length - prefix_length;
    if (payload_length == 0U || payload_length % 4U != 0U) return false;
    size_t padding = value[length - 1U] == '=' ? 1U : 0U;
    if (payload_length > 1U && value[length - 2U] == '=') padding++;
    if (padding > 2U) return false;
    for (size_t index = 0U; index < payload_length; index++) {
        char byte = value[prefix_length + index];
        if (byte == '=') {
            if (index < payload_length - padding) return false;
        } else if (base64_value(byte) < 0) return false;
    }
    if (padding == 2U &&
        (base64_value(value[length - 3U]) < 0 ||
         (base64_value(value[length - 3U]) & 15) != 0)) return false;
    if (padding == 1U &&
        (base64_value(value[length - 2U]) < 0 ||
         (base64_value(value[length - 2U]) & 3) != 0)) return false;
    return true;
}

static bool safe_attribute_name(const char *name, size_t length)
{
    if (name == NULL || length == 0U || length >= MCP_SCENE_NAME_CAP) return false;
    if (!((name[0] >= 'A' && name[0] <= 'Z') || (name[0] >= 'a' && name[0] <= 'z') || name[0] == '_')) return false;
    for (size_t index = 1U; index < length; index++) {
        char c = name[index];
        if (!((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') ||
              (c >= '0' && c <= '9') || c == '_' || c == '-' || c == ':' || c == '.')) return false;
    }
    if (length >= 2U && (name[0] == 'o' || name[0] == 'O') &&
        (name[1] == 'n' || name[1] == 'N')) return false;
    if (string_equals(name, length, "style") || string_equals(name, length, "src") ||
        string_equals(name, length, "data") || string_equals(name, length, "xmlns")) return false;
    return true;
}

static bool safe_attribute_value(const char *tag, size_t tag_length,
                                 const char *name, size_t name_length,
                                 const char *value, size_t value_length)
{
    if (!safe_reference_value(value, value_length)) return false;
    if (string_equals(name, name_length, "href") ||
        string_equals(name, name_length, "xlink:href")) {
        if (value_length > 1U && value[0] == '#') return true;
        return string_equals(tag, tag_length, "image") &&
            safe_image_data_uri(value, value_length);
    }
    return true;
}

static bool capture_ref(mcp_broker_document *document, const char *id,
                        size_t id_length, scene_ref *out, document_handle *handle)
{
    if (document == NULL || id == NULL || out == NULL || handle == NULL ||
        id_length == 0U || id_length >= sizeof(out->requested)) return false;
    if (!mcp_broker_resolve_element(document, id, id_length, handle)) return false;
    *out = (scene_ref){0};
    memcpy(out->requested, id, id_length);
    out->requested_length = id_length;
    if (id[0] == '@') {
        if (!document_edit_element_path(document->edit, *handle,
                (uint8_t *)out->path, sizeof(out->path) - 1U, &out->path_length) ||
            out->path_length >= sizeof(out->path)) return false;
        out->path[out->path_length] = '\0';
    }
    return true;
}

bool mcp_scene_resolve_element_path(mcp_broker_document *document,
                                    const char *path, size_t path_length,
                                    document_handle *out)
{
    if (document == NULL || path == NULL || out == NULL || path_length == 0U ||
        path_length >= MCP_SCENE_PATH_REF_CAP) return false;
    const document_model *model = document_edit_model(document->edit);
    document_handle current;
    if (model == NULL || !document_model_root(model, &current)) return false;
    size_t cursor = 0U;
    bool first_segment = true;
    while (cursor < path_length) {
        if (path[cursor++] != '/') return false;
        size_t start = cursor;
        while (cursor < path_length && path[cursor] != '[') cursor++;
        if (cursor == start || cursor >= path_length || cursor - start >= MCP_SCENE_NAME_CAP) return false;
        size_t tag_length = cursor - start;
        cursor++;
        size_t index = 0U;
        if (cursor >= path_length || path[cursor] < '0' || path[cursor] > '9') return false;
        while (cursor < path_length && path[cursor] >= '0' && path[cursor] <= '9') {
            size_t digit = (size_t)(path[cursor++] - '0');
            if (index > (SIZE_MAX - digit) / 10U) return false;
            index = index * 10U + digit;
        }
        if (cursor >= path_length || path[cursor++] != ']' || index == 0U) return false;
        if (first_segment) {
            base_bytes current_tag = {0};
            if (index != 1U || !document_model_tag_name(model, current, &current_tag) ||
                current_tag.len != tag_length ||
                memcmp(current_tag.ptr, path + start, tag_length) != 0) return false;
            first_segment = false;
        } else {
            document_handle child;
            size_t same_tag_index = 0U;
            bool found = false;
            if (!document_model_first_child(model, current, &child)) return false;
            do {
                base_bytes child_tag = {0};
                if (document_model_tag_name(model, child, &child_tag) &&
                    child_tag.len == tag_length && memcmp(child_tag.ptr, path + start, tag_length) == 0) {
                    same_tag_index++;
                    if (same_tag_index == index) { current = child; found = true; break; }
                }
                document_handle next;
                if (!document_model_next_sibling(model, child, &next)) break;
                child = next;
            } while (true);
            if (!found) return false;
        }
        if (cursor == path_length) {
            *out = current;
            return true;
        }
    }
    return false;
}

static bool resolve_ref(mcp_broker_document *document, const scene_ref *reference,
                        document_handle *out)
{
    if (reference == NULL || out == NULL) return false;
    if (reference->requested[0] != '@' && mcp_broker_resolve_element(document,
            reference->requested, reference->requested_length, out)) return true;
    return reference->path_length > 0U && mcp_scene_resolve_element_path(document,
        reference->path, reference->path_length, out);
}

static bool read_ref_arg(mcp_broker_document *document, const mcp_json_doc *request,
                         size_t args, const char *field, scene_ref *out,
                         document_handle *handle)
{
    char id[256]; size_t length = 0U;
    return mcp_scene_string_arg(request, args, field, id, sizeof(id), &length) &&
        capture_ref(document, id, length, out, handle);
}

static bool generate_id(mcp_broker_document *document, char out[64])
{
    for (uint32_t index = 0U; index < 10000U; index++) {
        int amount = snprintf(out, 64U, "mcp-scene-%u-%u", document->serial, index);
        if (amount < 0 || amount >= 64) return false;
        document_handle found;
        if (!document_edit_find_id(document->edit,
                (base_bytes){(const uint8_t *)out, (size_t)amount}, &found)) return true;
    }
    return false;
}

static document_edit_error set_attribute_value(mcp_broker_document *document,
    const scene_ref *reference, const char *name, size_t name_length,
    const char *value, size_t value_length)
{
    if (!safe_attribute_name(name, name_length))
        return DOCUMENT_EDIT_INVALID_NAME;
    document_handle element;
    if (!resolve_ref(document, reference, &element)) return DOCUMENT_EDIT_STALE_HANDLE;
    base_bytes tag = {0};
    if (!document_edit_tag_name(document->edit, element, &tag) ||
        !safe_attribute_value((const char *)tag.ptr, tag.len, name, name_length,
                             value, value_length)) return DOCUMENT_EDIT_INVALID_NAME;
    for (size_t index = 0U; index < document_edit_attribute_count(document->edit, element); index++) {
        document_edit_attribute attribute;
        base_bytes current_name = {0};
        if (!document_edit_attribute_at(document->edit, element, index, &attribute) ||
            !document_edit_span_bytes(document->edit, attribute.name, &current_name))
            return DOCUMENT_EDIT_INVALID_OPERATION;
        if (current_name.len == name_length && memcmp(current_name.ptr, name, name_length) == 0)
            return document_edit_set_attribute(document->edit, element,
                (base_bytes){(const uint8_t *)name, name_length},
                (base_bytes){(const uint8_t *)value, value_length});
    }
    return document_edit_add_attribute(document->edit, element,
        (base_bytes){(const uint8_t *)name, name_length},
        (base_bytes){(const uint8_t *)value, value_length});
}

static bool read_string_value(const mcp_json_doc *request, size_t token,
                              char *out, size_t capacity, size_t *out_length)
{
    if (token >= request->token_count || request->tokens[token].type != MCP_JSON_STRING)
        return false;
    return mcp_json_string_copy(request, token, out, capacity - 1U, out_length) &&
        *out_length < capacity && memchr(out, '\0', *out_length) == NULL;
}

static bool collect_attributes(mcp_broker_document *document,
    const mcp_json_doc *request, size_t object, const char *field,
    const char *tag, size_t tag_length,
    document_edit_template_attribute attributes[MCP_SCENE_ATTRIBUTE_LIMIT],
    char names[MCP_SCENE_ATTRIBUTE_LIMIT][MCP_SCENE_NAME_CAP],
    char values[MCP_SCENE_ATTRIBUTE_LIMIT][MCP_SCENE_VALUE_CAP], size_t *out_count)
{
    *out_count = 0U;
    size_t token = mcp_json_object_get(request, object, field);
    if (token == MCP_JSON_NO_TOKEN) return true;
    if (request->tokens[token].type != MCP_JSON_OBJECT ||
        request->tokens[token].child_count > MCP_SCENE_ATTRIBUTE_LIMIT * 2U) return false;
    size_t key = request->tokens[token].first_child;
    while (key != MCP_JSON_NO_TOKEN) {
        size_t value = request->tokens[key].next_sibling;
        size_t name_length = 0U, value_length = 0U;
        if (value == MCP_JSON_NO_TOKEN || *out_count >= MCP_SCENE_ATTRIBUTE_LIMIT ||
            !read_string_value(request, key, names[*out_count], sizeof(names[0]), &name_length) ||
            !read_string_value(request, value, values[*out_count], sizeof(values[0]), &value_length) ||
            !safe_attribute_name(names[*out_count], name_length) ||
            !safe_attribute_value(tag, tag_length, names[*out_count], name_length,
                                  values[*out_count], value_length)) return false;
        names[*out_count][name_length] = '\0';
        values[*out_count][value_length] = '\0';
        if (strcmp(names[*out_count], "id") == 0 &&
            document_edit_find_id(document->edit,
                (base_bytes){(const uint8_t *)values[*out_count], value_length},
                &(document_handle){0})) return false;
        attributes[*out_count] = (document_edit_template_attribute){
            {(const uint8_t *)names[*out_count], name_length},
            {(const uint8_t *)values[*out_count], value_length}};
        (*out_count)++;
        key = request->tokens[value].next_sibling;
    }
    return true;
}

static bool xml_append_escaped(mcp_json_buffer *out, const char *value,
                               size_t length, bool attribute)
{
    size_t start = 0U;
    for (size_t index = 0U; index < length; index++) {
        const char *replacement = NULL;
        if (value[index] == '&') replacement = "&amp;";
        else if (value[index] == '<') replacement = "&lt;";
        else if (value[index] == '>') replacement = "&gt;";
        else if (attribute && value[index] == '"') replacement = "&quot;";
        else if (attribute && value[index] == '\'') replacement = "&apos;";
        if (replacement != NULL) {
            if (index > start && !mcp_json_buffer_append(out, value + start, index - start)) return false;
            if (!mcp_json_buffer_literal(out, replacement)) return false;
            start = index + 1U;
        }
    }
    return length == start || mcp_json_buffer_append(out, value + start, length - start);
}

static bool element_tag_allowed(const char *tag, size_t length)
{
    static const char *const allowed[] = {
        "rect", "circle", "ellipse", "line", "polyline", "polygon", "path",
        "text", "image", "g", "use", "defs", "linearGradient", "radialGradient",
        "clipPath", "mask", "filter", "feGaussianBlur", "feOffset", "feFlood",
        "feBlend", "feComposite", "feMerge", "feMergeNode", "feColorMatrix",
        "feDropShadow"
    };
    for (size_t index = 0U; index < sizeof(allowed) / sizeof(allowed[0]); index++)
        if (string_equals(tag, length, allowed[index])) return true;
    return false;
}

static bool style_object_string(const mcp_json_doc *request, size_t object,
                                char out[MCP_SCENE_VALUE_CAP], size_t *out_length)
{
    size_t token = mcp_json_object_get(request, object, "style");
    *out_length = 0U;
    if (token == MCP_JSON_NO_TOKEN) return true;
    static const char *const properties[] = {"fill", "stroke", "stroke-width", "opacity",
        "fill-opacity", "stroke-opacity", "stroke-linecap", "stroke-linejoin",
        "stroke-dasharray", "stop-color", "stop-opacity", "font-size", "font-family",
        "font-weight", "text-anchor", "dominant-baseline"};
    if (request->tokens[token].type != MCP_JSON_OBJECT || request->tokens[token].child_count > 32U) return false;
    size_t key = request->tokens[token].first_child;
    while (key != MCP_JSON_NO_TOKEN) {
        size_t value = request->tokens[key].next_sibling;
        char name[64], text[MCP_SCENE_VALUE_CAP];
        size_t name_length = 0U, value_length = 0U;
        if (value == MCP_JSON_NO_TOKEN ||
            !read_string_value(request, key, name, sizeof(name), &name_length) ||
            !read_string_value(request, value, text, sizeof(text), &value_length) ||
            !safe_reference_value(text, value_length)) return false;
        bool allowed = false;
        for (size_t index = 0U; index < sizeof(properties) / sizeof(properties[0]); index++)
            if (string_equals(name, name_length, properties[index])) { allowed = true; break; }
        if (!allowed || *out_length + name_length + value_length + 2U >= MCP_SCENE_VALUE_CAP) return false;
        if (*out_length > 0U) out[(*out_length)++] = ';';
        memcpy(out + *out_length, name, name_length); *out_length += name_length;
        out[(*out_length)++] = ':';
        memcpy(out + *out_length, text, value_length); *out_length += value_length;
        key = request->tokens[value].next_sibling;
    }
    out[*out_length] = '\0';
    return true;
}

static bool capture_index_sibling(mcp_broker_document *document,
    document_handle parent, size_t index, scene_ref *out_ref, bool *out_found,
    size_t *out_child_count)
{
    const document_model *model = document_edit_model(document->edit);
    document_handle child;
    *out_found = false;
    *out_child_count = 0U;
    if (model == NULL) return false;
    if (!document_model_first_child(model, parent, &child)) return true;
    size_t ordinal = 0U;
    do {
        if (ordinal++ == index) {
            uint8_t path[MCP_SCENE_PATH_REF_CAP]; size_t path_length = 0U;
            if (!document_edit_element_path(document->edit, child, path,
                    sizeof(path) - 1U, &path_length) || path_length >= sizeof(path)) return false;
            memset(out_ref, 0, sizeof(*out_ref));
            memcpy(out_ref->path, path, path_length);
            out_ref->path[path_length] = '\0';
            out_ref->path_length = path_length;
            memcpy(out_ref->requested, "@path", 5U);
            out_ref->requested_length = 5U;
            *out_found = true;
            return true;
        }
        document_handle next;
        if (!document_model_next_sibling(model, child, &next)) break;
        child = next;
    } while (true);
    *out_child_count = ordinal;
    return true;
}

static document_edit_error create_element(mcp_broker_document *document,
    const mcp_json_doc *request, size_t args, document_handle *out_element)
{
    char tag[32], parent_id[256]; size_t tag_length = 0U, parent_length = 0U;
    if (!mcp_scene_string_arg(request, args, "tag", tag, sizeof(tag), &tag_length) ||
        !element_tag_allowed(tag, tag_length) ||
        !mcp_scene_string_arg(request, args, "parentId", parent_id,
            sizeof(parent_id), &parent_length)) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    document_handle parent;
    if (!mcp_broker_resolve_element(document, parent_id, parent_length, &parent)) return DOCUMENT_EDIT_NOT_FOUND;
    document_edit_template_attribute attributes[MCP_SCENE_ATTRIBUTE_LIMIT];
    char names[MCP_SCENE_ATTRIBUTE_LIMIT][MCP_SCENE_NAME_CAP];
    char values[MCP_SCENE_ATTRIBUTE_LIMIT][MCP_SCENE_VALUE_CAP];
    size_t count = 0U;
    if (!collect_attributes(document, request, args, "attributes", tag, tag_length,
            attributes, names, values, &count))
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    bool has_id = false;
    for (size_t index = 0U; index < count; index++)
        if (strcmp(names[index], "id") == 0) has_id = true;
    if (!has_id) {
        char id[64];
        if (!generate_id(document, id)) return DOCUMENT_EDIT_LIMIT;
        size_t id_length = strlen(id);
        memcpy(names[count], "id", 3U); memcpy(values[count], id, id_length + 1U);
        attributes[count] = (document_edit_template_attribute){
            {(const uint8_t *)names[count], 2U}, {(const uint8_t *)values[count], id_length}};
        count++;
    }
    char style[MCP_SCENE_VALUE_CAP]; size_t style_length = 0U;
    if (!style_object_string(request, args, style, &style_length)) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    if (style_length > 0U) {
        if (count >= MCP_SCENE_ATTRIBUTE_LIMIT) return DOCUMENT_EDIT_LIMIT;
        memcpy(names[count], "style", 6U); memcpy(values[count], style, style_length + 1U);
        attributes[count] = (document_edit_template_attribute){
            {(const uint8_t *)names[count], 5U}, {(const uint8_t *)values[count], style_length}};
        count++;
    }
    char text[MCP_SCENE_VALUE_CAP]; size_t text_length = 0U;
    size_t text_token = mcp_json_object_get(request, args, "text");
    if (text_token != MCP_JSON_NO_TOKEN &&
        (!string_equals(tag, tag_length, "text") ||
         !read_string_value(request, text_token, text, sizeof(text), &text_length)))
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    uint64_t index_value = UINT64_MAX; bool has_index = false;
    if (!mcp_broker_u64_arg(request, args, "index", &index_value, &has_index) ||
        (has_index && index_value > SIZE_MAX)) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    scene_ref sibling_ref = {0}; bool has_sibling = false;
    if (has_index) {
        size_t child_count = 0U;
        if (!capture_index_sibling(document, parent, (size_t)index_value,
                &sibling_ref, &has_sibling, &child_count)) return DOCUMENT_EDIT_INVALID_OPERATION;
        if (!has_sibling && (size_t)index_value > child_count) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    }
    document_edit_error error;
    if (string_equals(tag, tag_length, "text") && text_token != MCP_JSON_NO_TOKEN) {
        error = document_edit_insert_text_element(document->edit, parent, attributes,
            count, (base_bytes){(const uint8_t *)text, text_length}, out_element);
    } else {
        mcp_json_buffer fragment = {0};
        if (!mcp_json_buffer_init(&fragment, 128U, DOCUMENT_EDIT_XML_FRAGMENT_LIMIT))
            return DOCUMENT_EDIT_OUT_OF_MEMORY;
        bool okay = mcp_json_buffer_literal(&fragment, "<") &&
            mcp_json_buffer_append(&fragment, tag, tag_length);
        for (size_t index = 0U; okay && index < count; index++) {
            if (strcmp(names[index], "style") == 0) {
                /* style is synthesized by the helper and is separately allowlisted. */
            } else if (!safe_attribute_name(names[index], strlen(names[index])) ||
                !safe_attribute_value(tag, tag_length, names[index], strlen(names[index]),
                    values[index], strlen(values[index]))) {
                okay = false; break;
            }
            okay = mcp_json_buffer_literal(&fragment, " ") &&
                mcp_json_buffer_append(&fragment, names[index], strlen(names[index])) &&
                mcp_json_buffer_literal(&fragment, "=\"") &&
                xml_append_escaped(&fragment, values[index], strlen(values[index]), true) &&
                mcp_json_buffer_literal(&fragment, "\"");
        }
        if (okay && string_equals(tag, tag_length, "text") && text_token != MCP_JSON_NO_TOKEN)
            okay = mcp_json_buffer_literal(&fragment, ">") &&
                xml_append_escaped(&fragment, text, text_length, false) &&
                mcp_json_buffer_literal(&fragment, "</text>");
        else if (okay) okay = mcp_json_buffer_literal(&fragment, "/>");
        if (!okay) { mcp_json_buffer_destroy(&fragment); return DOCUMENT_EDIT_LIMIT; }
        error = document_edit_insert_xml_fragment(document->edit, parent,
            (base_bytes){(const uint8_t *)fragment.data, fragment.length}, out_element);
        mcp_json_buffer_destroy(&fragment);
    }
    if (error != DOCUMENT_EDIT_OK) return error;
    char created_id[64];
    size_t id_length = 0U; bool has_created_id = false;
    if (!mcp_scene_attribute_text(document, *out_element, "id", created_id,
            sizeof(created_id), &id_length, &has_created_id)) return DOCUMENT_EDIT_INVALID_OPERATION;
    if (!has_created_id) return DOCUMENT_EDIT_INVALID_OPERATION;
    if (has_sibling) {
        document_handle sibling;
        if (!resolve_ref(document, &sibling_ref, &sibling) ||
            !document_edit_find_id(document->edit,
                (base_bytes){(const uint8_t *)created_id, id_length}, out_element))
            return DOCUMENT_EDIT_STALE_HANDLE;
        error = document_edit_reorder_sibling(document->edit, *out_element, sibling, true);
        if (error != DOCUMENT_EDIT_OK) return error;
        if (!document_edit_find_id(document->edit,
                (base_bytes){(const uint8_t *)created_id, id_length}, out_element))
            return DOCUMENT_EDIT_STALE_HANDLE;
    }
    return DOCUMENT_EDIT_OK;
}

static document_edit_error set_element(mcp_broker_document *document,
    const mcp_json_doc *request, size_t args, document_handle *out_element)
{
    size_t attributes = mcp_json_object_get(request, args, "attributes");
    size_t style = mcp_json_object_get(request, args, "style");
    if (attributes == MCP_JSON_NO_TOKEN && style == MCP_JSON_NO_TOKEN)
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    if (attributes != MCP_JSON_NO_TOKEN && style == MCP_JSON_NO_TOKEN &&
        request->tokens[attributes].child_count == 0U) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    scene_ref reference; document_handle element;
    if (!read_ref_arg(document, request, args, "elementId", &reference, &element))
        return DOCUMENT_EDIT_NOT_FOUND;
    if (attributes != MCP_JSON_NO_TOKEN) {
        if (request->tokens[attributes].type != MCP_JSON_OBJECT ||
            request->tokens[attributes].child_count > MCP_SCENE_ATTRIBUTE_LIMIT * 2U)
            return DOCUMENT_EDIT_LIMIT;
        size_t key = request->tokens[attributes].first_child;
        while (key != MCP_JSON_NO_TOKEN) {
            size_t value_token = request->tokens[key].next_sibling;
            char name[MCP_SCENE_NAME_CAP], value[MCP_SCENE_VALUE_CAP];
            size_t name_length = 0U, value_length = 0U;
            if (value_token == MCP_JSON_NO_TOKEN ||
                !read_string_value(request, key, name, sizeof(name), &name_length) ||
                !read_string_value(request, value_token, value, sizeof(value), &value_length))
                return DOCUMENT_EDIT_INVALID_ARGUMENT;
            document_edit_error error = set_attribute_value(document, &reference,
                name, name_length, value, value_length);
            if (error != DOCUMENT_EDIT_OK) return error;
            key = request->tokens[value_token].next_sibling;
        }
    }
    if (style != MCP_JSON_NO_TOKEN) {
        char style_text[MCP_SCENE_VALUE_CAP]; size_t style_length = 0U;
        if (!style_object_string(request, args, style_text, &style_length) || style_length == 0U)
            return DOCUMENT_EDIT_INVALID_ARGUMENT;
        document_edit_error error = set_attribute_value(document, &reference,
            "style", 5U, style_text, style_length);
        if (error != DOCUMENT_EDIT_OK) return error;
    }
    return resolve_ref(document, &reference, out_element) ? DOCUMENT_EDIT_OK : DOCUMENT_EDIT_STALE_HANDLE;
}

static document_edit_error apply_structural(mcp_broker_document *document,
    const mcp_json_doc *request, size_t args, const char *tool_name,
    document_handle *out_element)
{
    scene_ref reference; document_handle element;
    if (!read_ref_arg(document, request, args, "elementId", &reference, &element))
        return DOCUMENT_EDIT_NOT_FOUND;
    if (strcmp(tool_name, "element_delete") == 0) {
        const document_model *model = document_edit_model(document->edit);
        if (model == NULL || !document_model_parent(model, element, out_element))
            return DOCUMENT_EDIT_ROOT_DELETE;
        return document_edit_delete_element(document->edit, element);
    }
    if (strcmp(tool_name, "element_duplicate") == 0) {
        document_handle duplicate;
        document_edit_error error = document_edit_duplicate_element(document->edit, element, &duplicate);
        if (error != DOCUMENT_EDIT_OK) return error;
        return mcp_broker_scene_duplicate(document, duplicate, out_element);
    }
    if (strcmp(tool_name, "element_group") == 0) {
        document_handle group;
        document_edit_error error = document_edit_wrap_in_group(document->edit, element, &group);
        if (error != DOCUMENT_EDIT_OK) return error;
        char new_id[64];
        if (!generate_id(document, new_id)) return DOCUMENT_EDIT_LIMIT;
        error = document_edit_set_attribute(document->edit, group,
            (base_bytes){(const uint8_t *)"id", 2U},
            (base_bytes){(const uint8_t *)new_id, strlen(new_id)});
        if (error == DOCUMENT_EDIT_OK && !document_edit_find_id(document->edit,
                (base_bytes){(const uint8_t *)new_id, strlen(new_id)}, out_element))
            return DOCUMENT_EDIT_INVALID_OPERATION;
        return error;
    }
    if (strcmp(tool_name, "element_ungroup") == 0) {
        const document_model *model = document_edit_model(document->edit);
        if (model == NULL || !document_model_parent(model, element, out_element))
            return DOCUMENT_EDIT_INVALID_OPERATION;
        return document_edit_ungroup(document->edit, element);
    }
    if (strcmp(tool_name, "element_reorder") == 0) {
        scene_ref target_ref; document_handle target;
        if (!read_ref_arg(document, request, args, "targetId", &target_ref, &target))
            return DOCUMENT_EDIT_NOT_FOUND;
        bool before = false;
        if (!mcp_broker_bool_arg(request, args, "before", &before)) return DOCUMENT_EDIT_INVALID_ARGUMENT;
        document_edit_error error = document_edit_reorder_sibling(document->edit,
            element, target, before);
        if (error == DOCUMENT_EDIT_OK && !resolve_ref(document, &reference, out_element))
            (void)document_edit_root(document->edit, out_element);
        return error;
    }
    return DOCUMENT_EDIT_INVALID_OPERATION;
}

static bool parse_transform(const char *text, size_t length)
{
    if (length == 0U || length > 512U) return false;
    char buffer[513]; memcpy(buffer, text, length); buffer[length] = '\0';
    const char *cursor = buffer;
    size_t count = 0U;
    while (*cursor != '\0') {
        while (*cursor == ' ' || *cursor == '\t' || *cursor == '\r' || *cursor == '\n' || *cursor == ',') cursor++;
        if (*cursor == '\0') break;
        char name[16]; size_t used = 0U;
        while (((*cursor >= 'A' && *cursor <= 'Z') || (*cursor >= 'a' && *cursor <= 'z')) && used + 1U < sizeof(name)) name[used++] = *cursor++;
        if (used == 0U) return false;
        name[used] = '\0';
        while (*cursor == ' ' || *cursor == '\t') cursor++;
        if (*cursor++ != '(') return false;
        double values[6]; size_t value_count = 0U;
        while (true) {
            while (*cursor == ' ' || *cursor == '\t' || *cursor == ',' || *cursor == '\r' || *cursor == '\n') cursor++;
            if (*cursor == ')') { cursor++; break; }
            if (value_count == 6U) return false;
            char *end = NULL; double value = strtod(cursor, &end);
            if (end == cursor || end == NULL || !isfinite(value)) return false;
            values[value_count++] = value; cursor = end;
        }
        bool valid = (strcmp(name, "matrix") == 0 && value_count == 6U) ||
            (strcmp(name, "translate") == 0 && (value_count == 1U || value_count == 2U)) ||
            (strcmp(name, "scale") == 0 && (value_count == 1U || value_count == 2U)) ||
            (strcmp(name, "rotate") == 0 && (value_count == 1U || value_count == 3U)) ||
            ((strcmp(name, "skewX") == 0 || strcmp(name, "skewY") == 0) && value_count == 1U);
        if (!valid || ++count > 16U) return false;
    }
    return count > 0U;
}

static document_edit_error transform_apply(mcp_broker_document *document,
    const mcp_json_doc *request, size_t args, const char *tool_name,
    document_handle *out_element)
{
    scene_ref reference; document_handle element;
    if (!read_ref_arg(document, request, args, "elementId", &reference, &element)) return DOCUMENT_EDIT_NOT_FOUND;
    char transform[513]; size_t transform_length = 0U;
    if (strcmp(tool_name, "transform_set") == 0) {
        if (!mcp_scene_string_arg(request, args, "value", transform,
                sizeof(transform), &transform_length) || !parse_transform(transform, transform_length))
            return DOCUMENT_EDIT_INVALID_ARGUMENT;
        document_edit_error error = set_attribute_value(document, &reference,
            "transform", 9U, transform, transform_length);
        if (error != DOCUMENT_EDIT_OK) return error;
        return resolve_ref(document, &reference, out_element) ? DOCUMENT_EDIT_OK : DOCUMENT_EDIT_STALE_HANDLE;
    }
    char operation[16]; size_t operation_length = 0U;
    if (!mcp_scene_string_arg(request, args, "operation", operation,
            sizeof(operation), &operation_length)) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    double x = 0.0, y = 0.0, sx = 1.0, sy = 1.0, angle = 0.0;
    double a = 1.0, b = 0.0, c = 0.0, d = 1.0, e = 0.0, f = 0.0;
    double cx = 0.0, cy = 0.0;
    bool x_present = false, y_present = false, sx_present = false, sy_present = false;
    bool angle_present = false, cx_present = false, cy_present = false;
    bool a_present = false, b_present = false, c_present = false;
    bool d_present = false, e_present = false, f_present = false;
    if (!mcp_scene_number_arg(request, args, "x", &x, &x_present) ||
        !mcp_scene_number_arg(request, args, "y", &y, &y_present) ||
        !mcp_scene_number_arg(request, args, "sx", &sx, &sx_present) ||
        !mcp_scene_number_arg(request, args, "sy", &sy, &sy_present) ||
        !mcp_scene_number_arg(request, args, "angle", &angle, &angle_present) ||
        !mcp_scene_number_arg(request, args, "cx", &cx, &cx_present) ||
        !mcp_scene_number_arg(request, args, "cy", &cy, &cy_present) ||
        !mcp_scene_number_arg(request, args, "a", &a, &a_present) ||
        !mcp_scene_number_arg(request, args, "b", &b, &b_present) ||
        !mcp_scene_number_arg(request, args, "c", &c, &c_present) ||
        !mcp_scene_number_arg(request, args, "d", &d, &d_present) ||
        !mcp_scene_number_arg(request, args, "e", &e, &e_present) ||
        !mcp_scene_number_arg(request, args, "f", &f, &f_present)) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    if (cx_present != cy_present ||
        (strcmp(operation, "translate") == 0 && cx_present))
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    int amount = -1;
    if (strcmp(operation, "translate") == 0) {
        if (!x_present && !y_present) return DOCUMENT_EDIT_INVALID_ARGUMENT;
        amount = snprintf(transform, sizeof(transform), "translate(%.9g %.9g)", x, y);
    }
    else if (strcmp(operation, "scale") == 0) {
        if (!sx_present) return DOCUMENT_EDIT_INVALID_ARGUMENT;
        if (!sy_present) sy = sx;
        amount = snprintf(transform, sizeof(transform), "scale(%.9g %.9g)", sx, sy);
    } else if (strcmp(operation, "rotate") == 0) {
        if (!angle_present || cx_present != cy_present) return DOCUMENT_EDIT_INVALID_ARGUMENT;
        amount = cx_present ? snprintf(transform, sizeof(transform), "rotate(%.9g %.9g %.9g)", angle, cx, cy) :
            snprintf(transform, sizeof(transform), "rotate(%.9g)", angle);
    } else if (strcmp(operation, "skewX") == 0 && angle_present)
        amount = snprintf(transform, sizeof(transform), "skewX(%.9g)", angle);
    else if (strcmp(operation, "skewY") == 0 && angle_present)
        amount = snprintf(transform, sizeof(transform), "skewY(%.9g)", angle);
    else if (strcmp(operation, "matrix") == 0) {
        if (!a_present || !b_present || !c_present || !d_present || !e_present || !f_present)
            return DOCUMENT_EDIT_INVALID_ARGUMENT;
        amount = snprintf(transform, sizeof(transform), "matrix(%.9g %.9g %.9g %.9g %.9g %.9g)", a, b, c, d, e, f);
    }
    if (amount <= 0 || (size_t)amount >= sizeof(transform)) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    transform_length = (size_t)amount;
    if (cx_present && strcmp(operation, "rotate") != 0) {
        char centered[513];
        int centered_length = snprintf(centered, sizeof(centered),
            "translate(%.9g %.9g) %.*s translate(%.9g %.9g)",
            cx, cy, (int)transform_length, transform, -cx, -cy);
        if (centered_length <= 0 || (size_t)centered_length >= sizeof(centered))
            return DOCUMENT_EDIT_LIMIT;
        memcpy(transform, centered, (size_t)centered_length + 1U);
        transform_length = (size_t)centered_length;
    }
    if (!parse_transform(transform, transform_length)) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    char existing[513]; size_t existing_length = 0U; bool has_existing = false;
    if (!mcp_scene_attribute_text(document, element, "transform", existing,
            sizeof(existing), &existing_length, &has_existing)) return DOCUMENT_EDIT_INVALID_OPERATION;
    if (has_existing) {
        if (existing_length + 1U + transform_length >= sizeof(transform)) return DOCUMENT_EDIT_LIMIT;
        memmove(transform + existing_length + 1U, transform, transform_length);
        memcpy(transform, existing, existing_length);
        transform[existing_length] = ' ';
        transform_length += existing_length + 1U;
        transform[transform_length] = '\0';
    }
    document_edit_error error = set_attribute_value(document, &reference,
        "transform", 9U, transform, transform_length);
    if (error != DOCUMENT_EDIT_OK) return error;
    return resolve_ref(document, &reference, out_element) ? DOCUMENT_EDIT_OK : DOCUMENT_EDIT_STALE_HANDLE;
}

document_edit_error mcp_broker_scene_apply_operation(
    mcp_broker_document *document, const mcp_json_doc *request, size_t args,
    const char *tool_name, document_handle *out_element, bool *out_created)
{
    if (document == NULL || request == NULL || tool_name == NULL || out_element == NULL || out_created == NULL)
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    *out_created = false;
    if (strcmp(tool_name, "element_create") == 0) {
        document_edit_error error = create_element(document, request, args, out_element);
        *out_created = error == DOCUMENT_EDIT_OK;
        return error;
    }
    if (strcmp(tool_name, "element_set") == 0) return set_element(document, request, args, out_element);
    if (strcmp(tool_name, "element_delete") == 0 || strcmp(tool_name, "element_duplicate") == 0 ||
        strcmp(tool_name, "element_reorder") == 0 || strcmp(tool_name, "element_group") == 0 ||
        strcmp(tool_name, "element_ungroup") == 0) {
        document_edit_error error = apply_structural(document, request, args,
            tool_name, out_element);
        if (error == DOCUMENT_EDIT_OK && strcmp(tool_name, "element_duplicate") == 0)
            *out_created = true;
        return error;
    }
    if (strcmp(tool_name, "transform_set") == 0 || strcmp(tool_name, "transform_apply") == 0)
        return transform_apply(document, request, args, tool_name, out_element);
    if (strcmp(tool_name, "path_from_commands") == 0 || strcmp(tool_name, "path_append") == 0 ||
        strcmp(tool_name, "path_edit_node") == 0)
        return mcp_broker_scene_path_apply(document, request, args, tool_name, out_element);
    if (strcmp(tool_name, "style_fill") == 0 || strcmp(tool_name, "style_stroke") == 0 ||
        strcmp(tool_name, "style_opacity") == 0 || strcmp(tool_name, "gradient_create") == 0 ||
        strcmp(tool_name, "gradient_stop_add") == 0 || strcmp(tool_name, "gradient_stop_set") == 0 ||
        strcmp(tool_name, "gradient_stop_remove") == 0)
        return mcp_broker_scene_style_apply(document, request, args, tool_name, out_element);
    if (strcmp(tool_name, "element_align") == 0 || strcmp(tool_name, "element_distribute") == 0)
        return mcp_broker_scene_align_apply(document, request, args, tool_name, out_element);
    return DOCUMENT_EDIT_INVALID_OPERATION;
}
