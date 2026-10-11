#include "mcp_broker_scene_internal.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MCP_SCENE_STYLE_CAP 4096U
#define MCP_SCENE_ALIGN_LIMIT 64U

typedef struct style_ref {
    char id[256];
    size_t id_length;
    char path[2048];
    size_t path_length;
} style_ref;

typedef struct align_item {
    style_ref reference;
    mcp_scene_bounds bounds;
    double start;
    double size;
} align_item;

static bool same(const char *left, size_t length, const char *right)
{
    return strlen(right) == length && memcmp(left, right, length) == 0;
}

static bool read_value(const mcp_json_doc *request, size_t token,
                       char *out, size_t capacity, size_t *out_length)
{
    if (capacity == 0U || token >= request->token_count ||
        request->tokens[token].type != MCP_JSON_STRING ||
        !mcp_json_string_copy(request, token, out, capacity - 1U, out_length) ||
        *out_length >= capacity || memchr(out, '\0', *out_length) != NULL) return false;
    out[*out_length] = '\0';
    return true;
}

static bool resolve_target(mcp_broker_document *document,
                           const mcp_json_doc *request, size_t args,
                           style_ref *out, document_handle *element)
{
    char id[256]; size_t id_length = 0U;
    if (!mcp_scene_string_arg(request, args, "elementId", id, sizeof(id), &id_length) ||
        !mcp_broker_resolve_element(document, id, id_length, element)) return false;
    memset(out, 0, sizeof(*out));
    memcpy(out->id, id, id_length); out->id_length = id_length;
    if (id[0] == '@') {
        if (!document_edit_element_path(document->edit, *element,
                (uint8_t *)out->path, sizeof(out->path) - 1U, &out->path_length) ||
            out->path_length >= sizeof(out->path)) return false;
        out->path[out->path_length] = '\0';
    }
    return true;
}

static bool resolve_path(mcp_broker_document *document, const style_ref *reference,
                         document_handle *out)
{
    return reference->path_length > 0U && mcp_scene_resolve_element_path(document,
        reference->path, reference->path_length, out);
}

static bool resolve_ref(mcp_broker_document *document, const style_ref *reference,
                        document_handle *out)
{
    if (reference->id[0] != '@' && mcp_broker_resolve_element(document,
            reference->id, reference->id_length, out)) return true;
    return resolve_path(document, reference, out);
}

static document_edit_error set_attr(mcp_broker_document *document,
                                    document_handle element, const char *name,
                                    const char *value)
{
    size_t name_length = strlen(name), value_length = strlen(value);
    for (size_t index = 0U; index < document_edit_attribute_count(document->edit, element); index++) {
        document_edit_attribute attribute; base_bytes attr_name = {0};
        if (!document_edit_attribute_at(document->edit, element, index, &attribute) ||
            !document_edit_span_bytes(document->edit, attribute.name, &attr_name)) return DOCUMENT_EDIT_INVALID_OPERATION;
        if (attr_name.len == name_length && memcmp(attr_name.ptr, name, name_length) == 0)
            return document_edit_set_attribute(document->edit, element,
                (base_bytes){(const uint8_t *)name, name_length},
                (base_bytes){(const uint8_t *)value, value_length});
    }
    return document_edit_add_attribute(document->edit, element,
        (base_bytes){(const uint8_t *)name, name_length},
        (base_bytes){(const uint8_t *)value, value_length});
}

static bool style_value_safe(const char *value, size_t length)
{
    for (size_t index = 0U; index < length; index++) {
        if (index + 11U <= length && (value[index] == 'j' || value[index] == 'J') &&
            (value[index + 1U] == 'a' || value[index + 1U] == 'A') &&
            (value[index + 2U] == 'v' || value[index + 2U] == 'V') &&
            (value[index + 3U] == 'a' || value[index + 3U] == 'A') &&
            (value[index + 4U] == 's' || value[index + 4U] == 'S') &&
            (value[index + 5U] == 'c' || value[index + 5U] == 'C') &&
            (value[index + 6U] == 'r' || value[index + 6U] == 'R') &&
            (value[index + 7U] == 'i' || value[index + 7U] == 'I') &&
            (value[index + 8U] == 'p' || value[index + 8U] == 'P') &&
            (value[index + 9U] == 't' || value[index + 9U] == 'T') && value[index + 10U] == ':') return false;
        if (index + 4U <= length && (value[index] == 'u' || value[index] == 'U') &&
            (value[index + 1U] == 'r' || value[index + 1U] == 'R') &&
            (value[index + 2U] == 'l' || value[index + 2U] == 'L') && value[index + 3U] == '(') {
            size_t cursor = index + 4U;
            while (cursor < length && (value[cursor] == ' ' || value[cursor] == '\'' || value[cursor] == '"')) cursor++;
            if (cursor >= length || value[cursor] != '#') return false;
            while (cursor < length && value[cursor] != ')') cursor++;
            if (cursor == length) return false;
            index = cursor;
        }
    }
    return true;
}

static bool style_property_allowed(const char *property)
{
    static const char *const allowed[] = {"fill", "stroke", "opacity", "fill-opacity",
        "stroke-opacity", "stroke-width", "stroke-linecap", "stroke-linejoin",
        "stroke-dasharray", "stop-color", "stop-opacity", "font-size", "font-family",
        "font-weight", "text-anchor", "dominant-baseline"};
    for (size_t index = 0U; index < sizeof(allowed) / sizeof(allowed[0]); index++)
        if (strcmp(property, allowed[index]) == 0) return true;
    return false;
}

static document_edit_error update_style(mcp_broker_document *document,
    const style_ref *reference, const char *property, const char *value,
    size_t value_length)
{
    if (!style_property_allowed(property) || !style_value_safe(value, value_length))
        return DOCUMENT_EDIT_INVALID_NAME;
    char old_style[MCP_SCENE_STYLE_CAP]; size_t old_length = 0U; bool present = false;
    document_handle element;
    if (!resolve_ref(document, reference, &element) ||
        !mcp_scene_attribute_text(document, element, "style", old_style,
            sizeof(old_style), &old_length, &present)) return DOCUMENT_EDIT_STALE_HANDLE;
    mcp_json_buffer result = {0};
    if (!mcp_json_buffer_init(&result, old_length + value_length + 32U,
            MCP_SCENE_STYLE_CAP - 1U)) return DOCUMENT_EDIT_OUT_OF_MEMORY;
    bool replaced = false;
    size_t cursor = 0U;
    while (present && cursor < old_length) {
        size_t end = cursor;
        while (end < old_length && old_style[end] != ';') end++;
        size_t colon = cursor;
        while (colon < end && old_style[colon] != ':') colon++;
        size_t name_start = cursor, name_end = colon;
        while (name_start < name_end && (old_style[name_start] == ' ' || old_style[name_start] == '\t')) name_start++;
        while (name_end > name_start && (old_style[name_end - 1U] == ' ' || old_style[name_end - 1U] == '\t')) name_end--;
        bool matches = colon < end && name_end - name_start == strlen(property) &&
            memcmp(old_style + name_start, property, name_end - name_start) == 0;
        if (result.length > 0U && !mcp_json_buffer_literal(&result, ";")) break;
        if (matches) {
            replaced = true;
            if (!mcp_json_buffer_literal(&result, property) || !mcp_json_buffer_literal(&result, ":") ||
                !mcp_json_buffer_append(&result, value, value_length)) break;
        } else if (!mcp_json_buffer_append(&result, old_style + cursor, end - cursor)) break;
        cursor = end < old_length ? end + 1U : end;
    }
    bool okay = true;
    if (!replaced) {
        if (result.length > 0U) okay = mcp_json_buffer_literal(&result, ";");
        okay = okay && mcp_json_buffer_literal(&result, property) &&
            mcp_json_buffer_literal(&result, ":") &&
            mcp_json_buffer_append(&result, value, value_length);
    }
    if (!okay) { mcp_json_buffer_destroy(&result); return DOCUMENT_EDIT_LIMIT; }
    element = (document_handle){0};
    if (!resolve_ref(document, reference, &element)) { mcp_json_buffer_destroy(&result); return DOCUMENT_EDIT_STALE_HANDLE; }
    document_edit_error error = set_attr(document, element, "style", result.data);
    mcp_json_buffer_destroy(&result);
    return error;
}

static bool append_xml_value(mcp_json_buffer *out, const char *value, size_t length)
{
    size_t start = 0U;
    for (size_t index = 0U; index < length; index++) {
        const char *replacement = value[index] == '&' ? "&amp;" :
            value[index] == '<' ? "&lt;" : value[index] == '"' ? "&quot;" : NULL;
        if (replacement == NULL) continue;
        if (index > start && !mcp_json_buffer_append(out, value + start, index - start)) return false;
        if (!mcp_json_buffer_literal(out, replacement)) return false;
        start = index + 1U;
    }
    return start == length || mcp_json_buffer_append(out, value + start, length - start);
}

static bool gradient_kind_valid(const char *kind, size_t length)
{
    return same(kind, length, "linearGradient") || same(kind, length, "radialGradient");
}

static bool gradient_attribute_allowed(const char *name, size_t length,
                                       const char *kind)
{
    static const char *const linear[] = {"x1", "y1", "x2", "y2", "gradientUnits",
        "gradientTransform", "spreadMethod"};
    static const char *const radial[] = {"cx", "cy", "r", "fx", "fy", "fr",
        "gradientUnits", "gradientTransform", "spreadMethod"};
    if (length >= 2U && (name[0] == 'o' || name[0] == 'O') &&
        (name[1] == 'n' || name[1] == 'N')) return false;
    const char *const *allowed = same(kind, strlen(kind), "linearGradient") ? linear : radial;
    size_t count = same(kind, strlen(kind), "linearGradient") ?
        sizeof(linear) / sizeof(linear[0]) : sizeof(radial) / sizeof(radial[0]);
    for (size_t index = 0U; index < count; index++)
        if (same(name, length, allowed[index])) return true;
    return false;
}

static bool unique_id(mcp_broker_document *document, char output[64])
{
    for (uint32_t serial = 0U; serial < 10000U; serial++) {
        int amount = snprintf(output, 64U, "mcp-gradient-%u-%u", document->serial, serial);
        if (amount < 0 || amount >= 64) return false;
        document_handle existing;
        if (!document_edit_find_id(document->edit,
            (base_bytes){(const uint8_t *)output, (size_t)amount}, &existing)) return true;
    }
    return false;
}

static bool capture_path_ref(mcp_broker_document *document,
                             document_handle element, style_ref *out)
{
    if (!document_edit_element_path(document->edit, element,
            (uint8_t *)out->path, sizeof(out->path) - 1U, &out->path_length) ||
        out->path_length >= sizeof(out->path)) return false;
    out->path[out->path_length] = '\0';
    out->id[0] = '@'; out->id[1] = '\0'; out->id_length = 1U;
    return true;
}

static bool capture_child_at(mcp_broker_document *document,
    document_handle parent, size_t index, style_ref *out_ref, bool *out_found,
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
            if (!capture_path_ref(document, child, out_ref)) return false;
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

static document_edit_error gradient_create(mcp_broker_document *document,
    const mcp_json_doc *request, size_t args, document_handle *out_element)
{
    char parent_id[256], kind[32]; size_t parent_length = 0U, kind_length = 0U;
    if (!mcp_scene_string_arg(request, args, "parentId", parent_id,
            sizeof(parent_id), &parent_length) ||
        !mcp_scene_string_arg(request, args, "kind", kind, sizeof(kind), &kind_length) ||
        !gradient_kind_valid(kind, kind_length)) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    document_handle parent;
    if (!mcp_broker_resolve_element(document, parent_id, parent_length, &parent)) return DOCUMENT_EDIT_NOT_FOUND;
    char id[64];
    if (!unique_id(document, id)) return DOCUMENT_EDIT_LIMIT;
    mcp_json_buffer fragment = {0};
    if (!mcp_json_buffer_init(&fragment, 128U, DOCUMENT_EDIT_XML_FRAGMENT_LIMIT)) return DOCUMENT_EDIT_OUT_OF_MEMORY;
    bool okay = mcp_json_buffer_literal(&fragment, "<") &&
        mcp_json_buffer_append(&fragment, kind, kind_length) &&
        mcp_json_buffer_literal(&fragment, " id=\"") &&
        append_xml_value(&fragment, id, strlen(id)) &&
        mcp_json_buffer_literal(&fragment, "\"");
    size_t attrs = mcp_json_object_get(request, args, "attributes");
    if (attrs != MCP_JSON_NO_TOKEN) {
        if (request->tokens[attrs].type != MCP_JSON_OBJECT || request->tokens[attrs].child_count > 32U) okay = false;
        size_t key = request->tokens[attrs].first_child;
        while (okay && key != MCP_JSON_NO_TOKEN) {
            size_t value_token = request->tokens[key].next_sibling;
            char name[128], value[MCP_SCENE_STYLE_CAP]; size_t name_length = 0U, value_length = 0U;
            if (value_token == MCP_JSON_NO_TOKEN || !read_value(request, key, name, sizeof(name), &name_length) ||
                !read_value(request, value_token, value, sizeof(value), &value_length) ||
                name_length == 0U || !gradient_attribute_allowed(name, name_length, kind) ||
                !style_value_safe(value, value_length) ||
                same(name, name_length, "id") || same(name, name_length, "href") ||
                same(name, name_length, "xlink:href") || same(name, name_length, "style")) {
                okay = false; break;
            }
            for (size_t i = 0U; i < name_length; i++) {
                char c = name[i];
                if (!((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') ||
                    (c >= '0' && c <= '9') || c == '_' || c == '-' || c == ':')) { okay = false; break; }
            }
            if (okay) okay = mcp_json_buffer_literal(&fragment, " ") &&
                mcp_json_buffer_append(&fragment, name, name_length) &&
                mcp_json_buffer_literal(&fragment, "=\"") &&
                append_xml_value(&fragment, value, value_length) &&
                mcp_json_buffer_literal(&fragment, "\"");
            key = request->tokens[value_token].next_sibling;
        }
    }
    okay = okay && mcp_json_buffer_literal(&fragment, "/>");
    if (!okay) { mcp_json_buffer_destroy(&fragment); return DOCUMENT_EDIT_INVALID_ARGUMENT; }
    uint64_t index_value = UINT64_MAX; bool has_index = false;
    if (!mcp_broker_u64_arg(request, args, "index", &index_value, &has_index) ||
        (has_index && index_value > SIZE_MAX)) {
        mcp_json_buffer_destroy(&fragment);
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    }
    style_ref target_ref = {0}; bool has_target = false;
    if (has_index) {
        size_t child_count = 0U;
        if (!capture_child_at(document, parent, (size_t)index_value,
                &target_ref, &has_target, &child_count)) {
            mcp_json_buffer_destroy(&fragment);
            return DOCUMENT_EDIT_INVALID_OPERATION;
        }
        if (!has_target && (size_t)index_value > child_count) {
            mcp_json_buffer_destroy(&fragment);
            return DOCUMENT_EDIT_INVALID_ARGUMENT;
        }
    }
    document_edit_error error = document_edit_insert_xml_fragment(document->edit,
        parent, (base_bytes){(const uint8_t *)fragment.data, fragment.length}, out_element);
    mcp_json_buffer_destroy(&fragment);
    if (error == DOCUMENT_EDIT_OK && has_target) {
        document_handle target;
        if (!resolve_ref(document, &target_ref, &target)) return DOCUMENT_EDIT_STALE_HANDLE;
        error = document_edit_reorder_sibling(document->edit, *out_element, target, true);
    }
    if (error == DOCUMENT_EDIT_OK && !document_edit_find_id(document->edit,
            (base_bytes){(const uint8_t *)id, strlen(id)}, out_element))
        return DOCUMENT_EDIT_INVALID_OPERATION;
    return error;
}

static bool find_stop(mcp_broker_document *document, const char *gradient_id,
                      size_t gradient_length, size_t stop_index,
                      document_handle *out_gradient, document_handle *out_stop)
{
    if (!document_edit_find_id(document->edit,
            (base_bytes){(const uint8_t *)gradient_id, gradient_length}, out_gradient)) return false;
    base_bytes tag = {0};
    if (!document_edit_tag_name(document->edit, *out_gradient, &tag) ||
        ((tag.len != 14U || memcmp(tag.ptr, "linearGradient", 14U) != 0) &&
         (tag.len != 14U || memcmp(tag.ptr, "radialGradient", 14U) != 0))) return false;
    const document_model *model = document_edit_model(document->edit);
    document_handle child;
    if (model == NULL || !document_model_first_child(model, *out_gradient, &child)) return false;
    size_t index = 0U;
    do {
        base_bytes child_tag = {0};
        if (document_model_tag_name(model, child, &child_tag) && child_tag.len == 4U &&
            memcmp(child_tag.ptr, "stop", 4U) == 0) {
            if (index == stop_index) { *out_stop = child; return true; }
            index++;
        }
        document_handle next;
        if (!document_model_next_sibling(model, child, &next)) break;
        child = next;
    } while (true);
    return false;
}

static bool finite_offset(const char *text, size_t length)
{
    char buffer[64];
    if (length == 0U || length >= sizeof(buffer)) return false;
    memcpy(buffer, text, length); buffer[length] = '\0';
    char *end = NULL; double value = strtod(buffer, &end);
    if (end == buffer || end == NULL || !isfinite(value)) return false;
    if (*end == '%') return end[1] == '\0' && value >= 0.0 && value <= 100.0;
    return *end == '\0' && value >= 0.0 && value <= 1.0;
}

static bool finite_unit_interval(const char *text, size_t length)
{
    char buffer[64];
    if (length == 0U || length >= sizeof(buffer)) return false;
    memcpy(buffer, text, length); buffer[length] = '\0';
    char *end = NULL; double value = strtod(buffer, &end);
    return end != buffer && end != NULL && *end == '\0' &&
        isfinite(value) && value >= 0.0 && value <= 1.0;
}

static document_edit_error gradient_stop_apply(mcp_broker_document *document,
    const mcp_json_doc *request, size_t args, const char *tool_name,
    document_handle *out_element)
{
    char gradient_id[256]; size_t gradient_length = 0U;
    if (!mcp_scene_string_arg(request, args, "gradientId", gradient_id,
            sizeof(gradient_id), &gradient_length)) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    uint64_t index_value = 0U; bool present = false;
    if (!mcp_broker_u64_arg(request, args, "index", &index_value, &present) || !present || index_value > 255U)
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    if (strcmp(tool_name, "gradient_stop_add") == 0) {
        char offset[64], color[128], opacity[64]; size_t offset_length = 0U, color_length = 0U, opacity_length = 0U;
        if (!mcp_scene_string_arg(request, args, "offset", offset, sizeof(offset), &offset_length) ||
            !mcp_scene_string_arg(request, args, "color", color, sizeof(color), &color_length) ||
            !finite_offset(offset, offset_length) || !style_value_safe(color, color_length))
            return DOCUMENT_EDIT_INVALID_ARGUMENT;
        size_t opacity_token = mcp_json_object_get(request, args, "opacity");
        if (opacity_token != MCP_JSON_NO_TOKEN &&
            (!mcp_scene_string_arg(request, args, "opacity", opacity, sizeof(opacity), &opacity_length) ||
             !finite_offset(opacity, opacity_length))) return DOCUMENT_EDIT_INVALID_ARGUMENT;
        document_handle gradient;
        if (!document_edit_find_id(document->edit,
                (base_bytes){(const uint8_t *)gradient_id, gradient_length}, &gradient)) return DOCUMENT_EDIT_NOT_FOUND;
        base_bytes tag = {0};
        if (!document_edit_tag_name(document->edit, gradient, &tag) ||
            !((tag.len == 14U && memcmp(tag.ptr, "linearGradient", 14U) == 0) ||
              (tag.len == 14U && memcmp(tag.ptr, "radialGradient", 14U) == 0))) return DOCUMENT_EDIT_INVALID_OPERATION;
        const document_model *model = document_edit_model(document->edit);
        document_handle child; size_t stop_count = 0U;
        style_ref target_ref = {0}; bool has_target = false;
        if (model != NULL && document_model_first_child(model, gradient, &child)) {
            do {
                base_bytes child_tag = {0};
                if (document_model_tag_name(model, child, &child_tag) &&
                    child_tag.len == 4U && memcmp(child_tag.ptr, "stop", 4U) == 0) {
                    if (stop_count == (size_t)index_value) {
                        if (!capture_path_ref(document, child, &target_ref)) return DOCUMENT_EDIT_INVALID_OPERATION;
                        has_target = true;
                    }
                    stop_count++;
                }
                document_handle next;
                if (!document_model_next_sibling(model, child, &next)) break;
                child = next;
            } while (true);
        }
        if ((size_t)index_value > stop_count) return DOCUMENT_EDIT_INVALID_ARGUMENT;
        char stop_id[64];
        if (!unique_id(document, stop_id)) return DOCUMENT_EDIT_LIMIT;
        mcp_json_buffer fragment = {0};
        if (!mcp_json_buffer_init(&fragment, 96U, 1024U)) return DOCUMENT_EDIT_OUT_OF_MEMORY;
        bool okay = mcp_json_buffer_literal(&fragment, "<stop id=\"") &&
            append_xml_value(&fragment, stop_id, strlen(stop_id)) &&
            mcp_json_buffer_literal(&fragment, "\" offset=\"") &&
            append_xml_value(&fragment, offset, offset_length) &&
            mcp_json_buffer_literal(&fragment, "\" stop-color=\"") &&
            append_xml_value(&fragment, color, color_length) &&
            mcp_json_buffer_literal(&fragment, "\"");
        if (okay && opacity_token != MCP_JSON_NO_TOKEN) okay = mcp_json_buffer_literal(&fragment, " stop-opacity=\"") &&
            append_xml_value(&fragment, opacity, opacity_length) && mcp_json_buffer_literal(&fragment, "\"");
        okay = okay && mcp_json_buffer_literal(&fragment, "/>");
        if (!okay) { mcp_json_buffer_destroy(&fragment); return DOCUMENT_EDIT_LIMIT; }
        document_edit_error error = document_edit_insert_xml_fragment(document->edit,
            gradient, (base_bytes){(const uint8_t *)fragment.data, fragment.length}, out_element);
        mcp_json_buffer_destroy(&fragment);
        if (error == DOCUMENT_EDIT_OK && has_target) {
            document_handle target;
            if (!resolve_ref(document, &target_ref, &target)) return DOCUMENT_EDIT_STALE_HANDLE;
            error = document_edit_reorder_sibling(document->edit, *out_element, target, true);
        }
        if (error == DOCUMENT_EDIT_OK && !document_edit_find_id(document->edit,
                (base_bytes){(const uint8_t *)stop_id, strlen(stop_id)}, out_element))
            return DOCUMENT_EDIT_INVALID_OPERATION;
        return error;
    }
    document_handle gradient, stop;
    if (!find_stop(document, gradient_id, gradient_length, (size_t)index_value,
            &gradient, &stop)) return DOCUMENT_EDIT_NOT_FOUND;
    if (strcmp(tool_name, "gradient_stop_remove") == 0) {
        *out_element = gradient;
        return document_edit_delete_element(document->edit, stop);
    }
    if (mcp_json_object_get(request, args, "offset") == MCP_JSON_NO_TOKEN &&
        mcp_json_object_get(request, args, "color") == MCP_JSON_NO_TOKEN &&
        mcp_json_object_get(request, args, "opacity") == MCP_JSON_NO_TOKEN)
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    document_edit_error error = DOCUMENT_EDIT_OK;
    const char *fields[] = {"offset", "color", "opacity"};
    const char *names[] = {"offset", "stop-color", "stop-opacity"};
    for (size_t field = 0U; field < 3U; field++) {
        size_t token = mcp_json_object_get(request, args, fields[field]);
        if (token == MCP_JSON_NO_TOKEN) continue;
        char value[128]; size_t length = 0U;
        if (!read_value(request, token, value, sizeof(value), &length) ||
            (field == 0U && !finite_offset(value, length)) ||
            (field > 0U && !style_value_safe(value, length))) return DOCUMENT_EDIT_INVALID_ARGUMENT;
        if (field == 2U && !finite_offset(value, length)) return DOCUMENT_EDIT_INVALID_ARGUMENT;
        if (!find_stop(document, gradient_id, gradient_length, (size_t)index_value,
                &gradient, &stop)) return DOCUMENT_EDIT_STALE_HANDLE;
        error = set_attr(document, stop, names[field], value);
        if (error != DOCUMENT_EDIT_OK) return error;
    }
    *out_element = gradient;
    return error;
}

document_edit_error mcp_broker_scene_style_apply(
    mcp_broker_document *document, const mcp_json_doc *request, size_t args,
    const char *tool_name, document_handle *out_element)
{
    if (strcmp(tool_name, "gradient_create") == 0)
        return gradient_create(document, request, args, out_element);
    if (strcmp(tool_name, "gradient_stop_add") == 0 ||
        strcmp(tool_name, "gradient_stop_set") == 0 ||
        strcmp(tool_name, "gradient_stop_remove") == 0)
        return gradient_stop_apply(document, request, args, tool_name, out_element);
    style_ref reference; document_handle element;
    if (!resolve_target(document, request, args, &reference, &element)) return DOCUMENT_EDIT_NOT_FOUND;
    char value[512]; size_t length = 0U;
    if (!mcp_scene_string_arg(request, args, "value", value, sizeof(value), &length))
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    if (length == 0U) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    const char *property = strcmp(tool_name, "style_fill") == 0 ? "fill" :
        strcmp(tool_name, "style_stroke") == 0 ? "stroke" : "opacity";
    if (strcmp(property, "opacity") == 0 && !finite_unit_interval(value, length))
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    document_edit_error error = update_style(document, &reference, property, value, length);
    if (error != DOCUMENT_EDIT_OK) return error;
    return resolve_ref(document, &reference, out_element) ? DOCUMENT_EDIT_OK : DOCUMENT_EDIT_STALE_HANDLE;
}

static bool align_ref(mcp_broker_document *document, const mcp_json_doc *request,
    size_t token, style_ref *out, document_handle *element)
{
    char id[256]; size_t length = 0U;
    if (!read_value(request, token, id, sizeof(id), &length) ||
        !mcp_broker_resolve_element(document, id, length, element)) return false;
    memset(out, 0, sizeof(*out)); memcpy(out->id, id, length); out->id_length = length;
    if (id[0] == '@') {
        if (!document_edit_element_path(document->edit, *element,
                (uint8_t *)out->path, sizeof(out->path) - 1U, &out->path_length) ||
            out->path_length >= sizeof(out->path)) return false;
        out->path[out->path_length] = '\0';
    }
    return true;
}

static document_edit_error append_translation(mcp_broker_document *document,
    const style_ref *reference, double dx, double dy, document_handle *out_element)
{
    document_handle element;
    if (!resolve_ref(document, reference, &element)) return DOCUMENT_EDIT_STALE_HANDLE;
    char transform[512];
    int amount = snprintf(transform, sizeof(transform), "translate(%.9g %.9g)", dx, dy);
    if (amount <= 0 || (size_t)amount >= sizeof(transform)) return DOCUMENT_EDIT_LIMIT;
    char old[512]; size_t old_length = 0U; bool has_old = false;
    if (!mcp_scene_attribute_text(document, element, "transform", old,
            sizeof(old), &old_length, &has_old)) return DOCUMENT_EDIT_INVALID_OPERATION;
    if (has_old) {
        if (old_length + 1U + (size_t)amount >= sizeof(transform)) return DOCUMENT_EDIT_LIMIT;
        memmove(transform + old_length + 1U, transform, (size_t)amount);
        memcpy(transform, old, old_length); transform[old_length] = ' ';
        amount += (int)old_length + 1;
        transform[(size_t)amount] = '\0';
    }
    document_edit_error error = set_attr(document, element, "transform", transform);
    if (error != DOCUMENT_EDIT_OK) return error;
    return resolve_ref(document, reference, out_element) ?
        DOCUMENT_EDIT_OK : DOCUMENT_EDIT_STALE_HANDLE;
}

document_edit_error mcp_broker_scene_align_apply(
    mcp_broker_document *document, const mcp_json_doc *request, size_t args,
    const char *tool_name, document_handle *out_element)
{
    size_t elements = mcp_json_object_get(request, args, "elementIds");
    if (elements == MCP_JSON_NO_TOKEN || request->tokens[elements].type != MCP_JSON_ARRAY ||
        request->tokens[elements].child_count < 2U || request->tokens[elements].child_count > MCP_SCENE_ALIGN_LIMIT)
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    char axis[8]; size_t axis_length = 0U;
    if (!mcp_scene_string_arg(request, args, "axis", axis, sizeof(axis), &axis_length) ||
        !(same(axis, axis_length, "x") || same(axis, axis_length, "y"))) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    bool horizontal = axis[0] == 'x';
    align_item items[MCP_SCENE_ALIGN_LIMIT];
    size_t count = request->tokens[elements].child_count;
    size_t token = request->tokens[elements].first_child;
    for (size_t index = 0U; index < count; index++) {
        document_handle element;
        if (token == MCP_JSON_NO_TOKEN || !align_ref(document, request, token,
                &items[index].reference, &element) ||
            !mcp_scene_element_bounds(document, element, &items[index].bounds) ||
            !items[index].bounds.valid) return DOCUMENT_EDIT_INVALID_OPERATION;
        if (index == 0U) *out_element = element;
        items[index].start = horizontal ? items[index].bounds.min_x : items[index].bounds.min_y;
        items[index].size = horizontal ? items[index].bounds.max_x - items[index].bounds.min_x :
                                         items[index].bounds.max_y - items[index].bounds.min_y;
        token = request->tokens[token].next_sibling;
    }
    double low = items[0].start, high = items[0].start + items[0].size;
    for (size_t index = 1U; index < count; index++) {
        if (items[index].start < low) low = items[index].start;
        if (items[index].start + items[index].size > high) high = items[index].start + items[index].size;
    }
    if (strcmp(tool_name, "element_distribute") == 0) {
        if (count < 3U) return DOCUMENT_EDIT_INVALID_ARGUMENT;
        for (size_t index = 0U; index < count; index++)
            for (size_t next = index + 1U; next < count; next++)
                if (items[next].start < items[index].start) {
                    align_item swap = items[index]; items[index] = items[next]; items[next] = swap;
                }
        double occupied = 0.0;
        for (size_t index = 0U; index < count; index++) occupied += items[index].size;
        double gap = ((high - low) - occupied) / (double)(count - 1U);
        double cursor = low;
        for (size_t index = 0U; index < count; index++) {
            double delta = cursor - items[index].start;
            if (fabs(delta) > 1e-9) {
                document_edit_error error = append_translation(document,
                    &items[index].reference, horizontal ? delta : 0.0,
                    horizontal ? 0.0 : delta, out_element);
                if (error != DOCUMENT_EDIT_OK) return error;
            }
            cursor += items[index].size + gap;
        }
        return DOCUMENT_EDIT_OK;
    }
    char alignment[16]; size_t alignment_length = 0U;
    if (!mcp_scene_string_arg(request, args, "alignment", alignment,
            sizeof(alignment), &alignment_length)) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    bool start = same(alignment, alignment_length, "start");
    bool center = same(alignment, alignment_length, "center");
    bool end = same(alignment, alignment_length, "end");
    if (!start && !center && !end) return DOCUMENT_EDIT_INVALID_ARGUMENT;
    double center_value = (low + high) * 0.5;
    for (size_t index = 0U; index < count; index++) {
        double target = start ? low : end ? high - items[index].size : center_value - items[index].size * 0.5;
        double delta = target - items[index].start;
        if (fabs(delta) <= 1e-9) continue;
        document_edit_error error = append_translation(document,
            &items[index].reference, horizontal ? delta : 0.0,
            horizontal ? 0.0 : delta, out_element);
        if (error != DOCUMENT_EDIT_OK) return error;
    }
    return DOCUMENT_EDIT_OK;
}
