#define _POSIX_C_SOURCE 200809L

#include "mcp_broker_scene_internal.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MCP_SCENE_ATTRIBUTE_CAP 256U
#define MCP_SCENE_ATTRIBUTE_VALUE_CAP 4096U
#define MCP_SCENE_TEXT_CAP 4096U
#define MCP_SCENE_NODE_DEPTH_LIMIT 64U

static bool append_number(mcp_json_buffer *out, double value)
{
    if (!isfinite(value)) return false;
    char text[48];
    int amount = snprintf(text, sizeof(text), "%.9g", value);
    return amount > 0 && (size_t)amount < sizeof(text) &&
        mcp_json_buffer_append(out, text, (size_t)amount);
}

static bool append_size(mcp_json_buffer *out, size_t value)
{
    char text[32];
    int amount = snprintf(text, sizeof(text), "%zu", value);
    return amount > 0 && (size_t)amount < sizeof(text) &&
        mcp_json_buffer_append(out, text, (size_t)amount);
}

bool mcp_scene_string_arg(const mcp_json_doc *request, size_t args,
                          const char *name, char *out, size_t capacity,
                          size_t *out_length)
{
    return mcp_broker_string_arg(request, args, name, out, capacity, out_length);
}

bool mcp_scene_number_arg(const mcp_json_doc *request, size_t object,
                          const char *name, double *out, bool *present)
{
    if (request == NULL || name == NULL || out == NULL || present == NULL) return false;
    size_t token = mcp_json_object_get(request, object, name);
    *present = token != MCP_JSON_NO_TOKEN;
    if (!*present) return true;
    if (token >= request->token_count || request->tokens[token].type != MCP_JSON_NUMBER)
        return false;
    char text[64];
    size_t length = 0U;
    if (!mcp_json_token_copy(request, token, text, sizeof(text) - 1U, &length) ||
        length == 0U || length >= sizeof(text)) return false;
    text[length] = '\0';
    char *end = NULL;
    double value = strtod(text, &end);
    if (end == text || end == NULL || *end != '\0' || !isfinite(value)) return false;
    *out = value;
    return true;
}

bool mcp_scene_attribute_text(mcp_broker_document *document,
                              document_handle element, const char *wanted,
                              char *out, size_t capacity, size_t *out_length,
                              bool *present)
{
    if (document == NULL || document->edit == NULL || wanted == NULL || out == NULL ||
        capacity == 0U || out_length == NULL || present == NULL) return false;
    *present = false;
    *out_length = 0U;
    size_t wanted_length = strlen(wanted);
    size_t count = document_edit_attribute_count(document->edit, element);
    for (size_t index = 0U; index < count; index++) {
        document_edit_attribute attribute;
        base_bytes name = {0};
        if (!document_edit_attribute_at(document->edit, element, index, &attribute) ||
            !document_edit_span_bytes(document->edit, attribute.name, &name)) return false;
        if (name.len != wanted_length || memcmp(name.ptr, wanted, wanted_length) != 0) continue;
        size_t decoded = 0U;
        if (!document_edit_decode_span(document->edit, attribute.value,
                (uint8_t *)out, capacity - 1U, &decoded) || decoded >= capacity ||
            memchr(out, '\0', decoded) != NULL) return false;
        out[decoded] = '\0';
        *out_length = decoded;
        *present = true;
        return true;
    }
    return true;
}

static bool parse_length(const char *text, size_t length, double *out)
{
    if (text == NULL || out == NULL || length == 0U || length >= 64U) return false;
    char buffer[64];
    memcpy(buffer, text, length);
    buffer[length] = '\0';
    char *end = NULL;
    double value = strtod(buffer, &end);
    if (end == buffer || !isfinite(value)) return false;
    while (*end == ' ' || *end == '\t') end++;
    if (strcmp(end, "px") != 0 && *end != '\0') return false;
    *out = value;
    return true;
}

static bool numeric_attr(mcp_broker_document *document, document_handle element,
                         const char *name, double default_value, double *out)
{
    char value[64];
    size_t length = 0U;
    bool present = false;
    if (!mcp_scene_attribute_text(document, element, name, value, sizeof(value),
                                  &length, &present)) return false;
    if (!present) { *out = default_value; return true; }
    return parse_length(value, length, out);
}

static void include_point(mcp_scene_bounds *bounds, double x, double y)
{
    if (!isfinite(x) || !isfinite(y)) return;
    if (!bounds->valid) {
        bounds->min_x = bounds->max_x = x;
        bounds->min_y = bounds->max_y = y;
        bounds->valid = true;
        return;
    }
    if (x < bounds->min_x) bounds->min_x = x;
    if (x > bounds->max_x) bounds->max_x = x;
    if (y < bounds->min_y) bounds->min_y = y;
    if (y > bounds->max_y) bounds->max_y = y;
}

static void union_bounds(mcp_scene_bounds *into, const mcp_scene_bounds *part)
{
    if (part == NULL || !part->valid) return;
    include_point(into, part->min_x, part->min_y);
    include_point(into, part->max_x, part->max_y);
    into->approximate = into->approximate || part->approximate;
}

typedef struct scene_matrix {
    double a, b, c, d, e, f;
} scene_matrix;

static scene_matrix matrix_multiply(scene_matrix left, scene_matrix right)
{
    return (scene_matrix){
        left.a * right.a + left.c * right.b,
        left.b * right.a + left.d * right.b,
        left.a * right.c + left.c * right.d,
        left.b * right.c + left.d * right.d,
        left.a * right.e + left.c * right.f + left.e,
        left.b * right.e + left.d * right.f + left.f
    };
}

static void skip_space(const char **cursor)
{
    while (**cursor == ' ' || **cursor == '\t' || **cursor == '\r' ||
           **cursor == '\n' || **cursor == ',') (*cursor)++;
}

static bool parse_transform_number(const char **cursor, double *out)
{
    skip_space(cursor);
    char *end = NULL;
    double value = strtod(*cursor, &end);
    if (end == *cursor || !isfinite(value)) return false;
    *cursor = end;
    *out = value;
    return true;
}

static bool transform_matrix(const char *text, scene_matrix *out)
{
    scene_matrix result = {1.0, 0.0, 0.0, 1.0, 0.0, 0.0};
    const char *cursor = text;
    size_t operations = 0U;
    while (*cursor != '\0') {
        skip_space(&cursor);
        if (*cursor == '\0') break;
        char name[16];
        size_t used = 0U;
        while (((*cursor >= 'A' && *cursor <= 'Z') ||
                (*cursor >= 'a' && *cursor <= 'z')) && used + 1U < sizeof(name))
            name[used++] = *cursor++;
        if (used == 0U) return false;
        name[used] = '\0';
        skip_space(&cursor);
        if (*cursor++ != '(') return false;
        double values[6] = {0.0, 0.0, 0.0, 0.0, 0.0, 0.0};
        size_t count = 0U;
        while (true) {
            skip_space(&cursor);
            if (*cursor == ')') { cursor++; break; }
            if (count == 6U || !parse_transform_number(&cursor, &values[count++]))
                return false;
        }
        scene_matrix next = {1.0, 0.0, 0.0, 1.0, 0.0, 0.0};
        if (strcmp(name, "matrix") == 0 && count == 6U) {
            next = (scene_matrix){values[0], values[1], values[2], values[3], values[4], values[5]};
        } else if (strcmp(name, "translate") == 0 && (count == 1U || count == 2U)) {
            next.e = values[0]; next.f = count == 2U ? values[1] : 0.0;
        } else if (strcmp(name, "scale") == 0 && (count == 1U || count == 2U)) {
            next.a = values[0]; next.d = count == 2U ? values[1] : values[0];
        } else if (strcmp(name, "rotate") == 0 && (count == 1U || count == 3U)) {
            double angle = values[0] * (3.14159265358979323846 / 180.0);
            double cosine = cos(angle), sine = sin(angle);
            next = (scene_matrix){cosine, sine, -sine, cosine, 0.0, 0.0};
            if (count == 3U) {
                next.e = values[1] - cosine * values[1] + sine * values[2];
                next.f = values[2] - sine * values[1] - cosine * values[2];
            }
        } else if ((strcmp(name, "skewX") == 0 || strcmp(name, "skewY") == 0) && count == 1U) {
            double tangent = tan(values[0] * (3.14159265358979323846 / 180.0));
            if (!isfinite(tangent)) return false;
            if (strcmp(name, "skewX") == 0) next.c = tangent;
            else next.b = tangent;
        } else return false;
        result = matrix_multiply(result, next);
        if (++operations > 16U) return false;
    }
    *out = result;
    return true;
}

static bool transform_bounds(mcp_broker_document *document, document_handle element,
                             mcp_scene_bounds *bounds)
{
    char transform[512];
    size_t length = 0U;
    bool present = false;
    if (!mcp_scene_attribute_text(document, element, "transform", transform,
            sizeof(transform), &length, &present)) return false;
    if (!present || !bounds->valid) return true;
    scene_matrix matrix;
    if (!transform_matrix(transform, &matrix)) {
        bounds->valid = false;
        return true;
    }
    mcp_scene_bounds result = {0};
    include_point(&result, matrix.a * bounds->min_x + matrix.c * bounds->min_y + matrix.e,
                  matrix.b * bounds->min_x + matrix.d * bounds->min_y + matrix.f);
    include_point(&result, matrix.a * bounds->max_x + matrix.c * bounds->min_y + matrix.e,
                  matrix.b * bounds->max_x + matrix.d * bounds->min_y + matrix.f);
    include_point(&result, matrix.a * bounds->min_x + matrix.c * bounds->max_y + matrix.e,
                  matrix.b * bounds->min_x + matrix.d * bounds->max_y + matrix.f);
    include_point(&result, matrix.a * bounds->max_x + matrix.c * bounds->max_y + matrix.e,
                  matrix.b * bounds->max_x + matrix.d * bounds->max_y + matrix.f);
    result.approximate = bounds->approximate;
    *bounds = result;
    return true;
}

static bool element_bounds_depth(mcp_broker_document *document,
                                 document_handle element, size_t depth,
                                 mcp_scene_bounds *out)
{
    if (depth > MCP_SCENE_NODE_DEPTH_LIMIT) return false;
    base_bytes tag = {0};
    if (!document_edit_tag_name(document->edit, element, &tag)) return false;
    *out = (mcp_scene_bounds){0};
    double x = 0.0, y = 0.0, width = 0.0, height = 0.0;
    if (tag.len == 4U && memcmp(tag.ptr, "rect", 4U) == 0) {
        if (!numeric_attr(document, element, "x", 0.0, &x) ||
            !numeric_attr(document, element, "y", 0.0, &y) ||
            !numeric_attr(document, element, "width", 0.0, &width) ||
            !numeric_attr(document, element, "height", 0.0, &height)) return false;
        if (width >= 0.0 && height >= 0.0) {
            include_point(out, x, y); include_point(out, x + width, y + height);
        }
    } else if (tag.len == 6U && memcmp(tag.ptr, "circle", 6U) == 0) {
        double radius = 0.0;
        if (!numeric_attr(document, element, "cx", 0.0, &x) ||
            !numeric_attr(document, element, "cy", 0.0, &y) ||
            !numeric_attr(document, element, "r", 0.0, &radius)) return false;
        if (radius >= 0.0) { include_point(out, x - radius, y - radius); include_point(out, x + radius, y + radius); }
    } else if (tag.len == 7U && memcmp(tag.ptr, "ellipse", 7U) == 0) {
        double rx = 0.0, ry = 0.0;
        if (!numeric_attr(document, element, "cx", 0.0, &x) ||
            !numeric_attr(document, element, "cy", 0.0, &y) ||
            !numeric_attr(document, element, "rx", 0.0, &rx) ||
            !numeric_attr(document, element, "ry", 0.0, &ry)) return false;
        if (rx >= 0.0 && ry >= 0.0) { include_point(out, x - rx, y - ry); include_point(out, x + rx, y + ry); }
    } else if (tag.len == 4U && memcmp(tag.ptr, "line", 4U) == 0) {
        double x2 = 0.0, y2 = 0.0;
        if (!numeric_attr(document, element, "x1", 0.0, &x) ||
            !numeric_attr(document, element, "y1", 0.0, &y) ||
            !numeric_attr(document, element, "x2", 0.0, &x2) ||
            !numeric_attr(document, element, "y2", 0.0, &y2)) return false;
        include_point(out, x, y); include_point(out, x2, y2);
    } else if ((tag.len == 5U && memcmp(tag.ptr, "image", 5U) == 0) ||
               (tag.len == 13U && memcmp(tag.ptr, "foreignObject", 13U) == 0)) {
        if (!numeric_attr(document, element, "x", 0.0, &x) ||
            !numeric_attr(document, element, "y", 0.0, &y) ||
            !numeric_attr(document, element, "width", 0.0, &width) ||
            !numeric_attr(document, element, "height", 0.0, &height)) return false;
        if (width >= 0.0 && height >= 0.0) { include_point(out, x, y); include_point(out, x + width, y + height); }
    } else if (tag.len == 4U && memcmp(tag.ptr, "path", 4U) == 0) {
        char path[8192];
        size_t path_length = 0U;
        bool present = false;
        if (!mcp_scene_attribute_text(document, element, "d", path, sizeof(path),
                &path_length, &present)) return false;
        if (present && !mcp_scene_path_bounds(path, path_length, out)) return false;
    } else if ((tag.len == 8U && memcmp(tag.ptr, "polyline", 8U) == 0) ||
               (tag.len == 7U && memcmp(tag.ptr, "polygon", 7U) == 0)) {
        char points[8192];
        size_t points_length = 0U;
        bool present = false;
        if (!mcp_scene_attribute_text(document, element, "points", points,
                sizeof(points), &points_length, &present)) return false;
        if (present) {
            const char *cursor = points;
            const char *end = points + points_length;
            size_t count = 0U;
            while (cursor < end) {
                while (cursor < end && (*cursor == ' ' || *cursor == '\t' || *cursor == '\r' || *cursor == '\n' || *cursor == ',')) cursor++;
                if (cursor == end) break;
                char *number_end = NULL;
                double px = strtod(cursor, &number_end);
                if (number_end == cursor || number_end > end || !isfinite(px)) return false;
                cursor = number_end;
                while (cursor < end && (*cursor == ' ' || *cursor == '\t' || *cursor == '\r' || *cursor == '\n' || *cursor == ',')) cursor++;
                if (cursor == end) return false;
                double py = strtod(cursor, &number_end);
                if (number_end == cursor || number_end > end || !isfinite(py)) return false;
                cursor = number_end;
                include_point(out, px, py);
                if (++count > 2048U) return false;
            }
        }
    } else if (tag.len == 4U && memcmp(tag.ptr, "text", 4U) == 0) {
        double font_size = 16.0;
        char font_text[64];
        size_t font_length = 0U;
        bool has_font = false;
        if (!numeric_attr(document, element, "x", 0.0, &x) ||
            !numeric_attr(document, element, "y", 0.0, &y) ||
            !mcp_scene_attribute_text(document, element, "font-size", font_text,
                sizeof(font_text), &font_length, &has_font)) return false;
        if (has_font && !parse_length(font_text, font_length, &font_size)) return false;
        uint8_t text[MCP_SCENE_TEXT_CAP];
        size_t text_length = 0U;
        if (!document_edit_text_content(document->edit, element, text,
                sizeof(text), &text_length)) return false;
        include_point(out, x, y - font_size);
        include_point(out, x + (double)text_length * font_size * 0.6, y + font_size * 0.25);
        out->approximate = true;
    } else if ((tag.len == 1U && tag.ptr[0] == 'g') ||
               (tag.len == 3U && memcmp(tag.ptr, "svg", 3U) == 0) ||
               (tag.len == 6U && memcmp(tag.ptr, "symbol", 6U) == 0)) {
        const document_model *model = document_edit_model(document->edit);
        document_handle child;
        if (model == NULL) return false;
        if (document_model_first_child(model, element, &child)) {
            do {
                mcp_scene_bounds child_bounds;
                if (!element_bounds_depth(document, child, depth + 1U, &child_bounds)) return false;
                union_bounds(out, &child_bounds);
            } while (document_model_next_sibling(model, child, &child));
        }
    }
    if (!transform_bounds(document, element, out)) return false;
    return true;
}

bool mcp_scene_element_bounds(mcp_broker_document *document,
                              document_handle element, mcp_scene_bounds *out)
{
    return document != NULL && out != NULL &&
        element_bounds_depth(document, element, 0U, out);
}

static bool append_bounds(mcp_json_buffer *out, const mcp_scene_bounds *bounds)
{
    if (bounds == NULL || !bounds->valid) return mcp_json_buffer_literal(out, "null");
    return mcp_json_buffer_literal(out, "{\"x\":" ) && append_number(out, bounds->min_x) &&
        mcp_json_buffer_literal(out, ",\"y\":" ) && append_number(out, bounds->min_y) &&
        mcp_json_buffer_literal(out, ",\"width\":" ) && append_number(out, bounds->max_x - bounds->min_x) &&
        mcp_json_buffer_literal(out, ",\"height\":" ) && append_number(out, bounds->max_y - bounds->min_y) &&
        mcp_json_buffer_literal(out, bounds->approximate ? ",\"approximate\":true}" : ",\"approximate\":false}");
}

static bool append_attributes(mcp_broker_document *document, document_handle handle,
                              mcp_json_buffer *out)
{
    size_t count = document_edit_attribute_count(document->edit, handle);
    if (count > MCP_SCENE_ATTRIBUTE_CAP || !mcp_json_buffer_literal(out, "[")) return false;
    for (size_t index = 0U; index < count; index++) {
        document_edit_attribute attribute;
        base_bytes name = {0}, value = {0};
        if (!document_edit_attribute_at(document->edit, handle, index, &attribute) ||
            !document_edit_span_bytes(document->edit, attribute.name, &name) ||
            !document_edit_span_bytes(document->edit, attribute.value, &value)) return false;
        if (index > 0U && !mcp_json_buffer_literal(out, ",")) return false;
        if (!mcp_json_buffer_literal(out, "{\"name\":" ) ||
            !mcp_json_buffer_quoted(out, (const char *)name.ptr, name.len) ||
            !mcp_json_buffer_literal(out, ",\"value\":" ) ||
            !mcp_json_buffer_quoted(out, (const char *)value.ptr, value.len) ||
            !mcp_json_buffer_literal(out, "}")) return false;
    }
    return mcp_json_buffer_literal(out, "]");
}

static bool append_element(mcp_broker_document *document, document_handle element,
                           bool include_attributes, bool include_bounds,
                           mcp_json_buffer *out)
{
    base_bytes tag = {0};
    const document_model *model = document_edit_model(document->edit);
    document_handle parent;
    size_t depth = 0U;
    if (model == NULL || !document_edit_tag_name(document->edit, element, &tag) ||
        !document_model_depth(model, element, &depth)) return false;
    if (!mcp_json_buffer_literal(out, "{\"elementId\":" ) ||
        !mcp_broker_element_id(document, element, out) ||
        !mcp_json_buffer_literal(out, ",\"tag\":" ) ||
        !mcp_json_buffer_quoted(out, (const char *)tag.ptr, tag.len) ||
        !mcp_json_buffer_literal(out, ",\"type\":" ) ||
        !mcp_json_buffer_quoted(out, (const char *)tag.ptr, tag.len) ||
        !mcp_json_buffer_literal(out, ",\"depth\":" ) ||
        !append_size(out, depth) ||
        !mcp_json_buffer_literal(out, ",\"parentId\":")) return false;
    if (document_model_parent(model, element, &parent)) {
        if (!mcp_broker_element_id(document, parent, out)) return false;
    } else if (!mcp_json_buffer_literal(out, "null")) return false;
    if (include_attributes && (!mcp_json_buffer_literal(out, ",\"attributes\":" ) ||
        !append_attributes(document, element, out))) return false;
    if (include_bounds) {
        mcp_scene_bounds bounds;
        if (!mcp_scene_element_bounds(document, element, &bounds) ||
            !mcp_json_buffer_literal(out, ",\"bounds\":" ) ||
            !append_bounds(out, &bounds)) return false;
    }
    return mcp_json_buffer_literal(out, "}");
}

static bool append_mode_document(mcp_json_buffer *out, mcp_broker_document *document)
{
    return mcp_json_buffer_literal(out, "{\"documentId\":" ) &&
        mcp_json_buffer_quoted(out, document->id, strlen(document->id)) &&
        mcp_json_buffer_literal(out, ",\"mode\":" ) &&
        mcp_json_buffer_quoted(out, mcp_broker_document_mode(document),
                               strlen(mcp_broker_document_mode(document)));
}

static bool optional_bool_arg(const mcp_json_doc *request, size_t args,
                             const char *name, bool *value)
{
    if (mcp_json_object_get(request, args, name) == MCP_JSON_NO_TOKEN) return true;
    return mcp_broker_bool_arg(request, args, name, value);
}

static bool element_tree(mcp_broker *broker, const mcp_json_doc *request,
                         size_t args, mcp_tool_response *out)
{
    mcp_broker_document *document = mcp_broker_find(broker, request, args, "documentId");
    if (document == NULL) return mcp_broker_set_error(out, "document_not_found", "documentId is not open");
    uint64_t cursor_value = 0U, limit_value = 128U;
    bool present = false, include_attributes = true, include_bounds = true;
    if (!mcp_broker_u64_arg(request, args, "cursor", &cursor_value, &present) ||
        !mcp_broker_u64_arg(request, args, "limit", &limit_value, &present) ||
        cursor_value > SIZE_MAX || limit_value == 0U || limit_value > 512U ||
        !optional_bool_arg(request, args, "includeAttributes", &include_attributes) ||
        !optional_bool_arg(request, args, "includeBounds", &include_bounds))
        return mcp_broker_set_error(out, "invalid_arguments", "element_tree cursor, limit, or flags are invalid");
    size_t parent_token = mcp_json_object_get(request, args, "parentId");
    document_handle parent = {0};
    bool filter_parent = parent_token != MCP_JSON_NO_TOKEN;
    if (filter_parent) {
        char parent_id[256]; size_t parent_length = 0U;
        if (!mcp_scene_string_arg(request, args, "parentId", parent_id,
                sizeof(parent_id), &parent_length) ||
            !mcp_broker_resolve_element(document, parent_id, parent_length, &parent))
            return mcp_broker_set_error(out, "element_not_found", "parentId does not identify a live element");
    }
    const document_model *model = document_edit_model(document->edit);
    if (model == NULL) return mcp_broker_set_error(out, "document_invalid", "document tree is unavailable");
    size_t node_count = document_model_node_count(model);
    if (cursor_value > node_count) return mcp_broker_set_error(out, "invalid_cursor", "element_tree cursor is outside the document");
    if (!append_mode_document(&out->structured_json, document) ||
        !mcp_json_buffer_literal(&out->structured_json, ",\"elements\":[")) return false;
    size_t slot = (size_t)cursor_value, emitted = 0U;
    for (; slot < node_count && emitted < (size_t)limit_value; slot++) {
        document_handle element, element_parent;
        if (!document_model_node_at(model, slot, &element)) continue;
        if (filter_parent && (!document_model_parent(model, element, &element_parent) ||
            element_parent.slot != parent.slot || element_parent.generation != parent.generation)) continue;
        if (emitted > 0U && !mcp_json_buffer_literal(&out->structured_json, ",")) return false;
        if (!append_element(document, element, include_attributes, include_bounds,
                            &out->structured_json)) return false;
        emitted++;
    }
    return mcp_json_buffer_literal(&out->structured_json, "],\"mode\":" ) &&
        mcp_json_buffer_quoted(&out->structured_json,
            mcp_broker_document_mode(document),
            strlen(mcp_broker_document_mode(document))) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"cursor\":" ) &&
        append_size(&out->structured_json, slot) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"complete\":" ) &&
        mcp_json_buffer_literal(&out->structured_json, slot >= node_count ? "true}" : "false}");
}

static bool element_get(mcp_broker *broker, const mcp_json_doc *request,
                        size_t args, mcp_tool_response *out)
{
    mcp_broker_document *document = mcp_broker_find(broker, request, args, "documentId");
    char element_id[256]; size_t element_length = 0U;
    document_handle element;
    if (document == NULL) return mcp_broker_set_error(out, "document_not_found", "documentId is not open");
    if (!mcp_scene_string_arg(request, args, "elementId", element_id,
            sizeof(element_id), &element_length) ||
        !mcp_broker_resolve_element(document, element_id, element_length, &element))
        return mcp_broker_set_error(out, "element_not_found", "elementId does not identify a live element");
    base_bytes tag = {0}; const document_model *model = document_edit_model(document->edit);
    size_t depth = 0U; document_handle parent; mcp_scene_bounds bounds;
    if (model == NULL || !document_edit_tag_name(document->edit, element, &tag) ||
        !document_model_depth(model, element, &depth) ||
        !mcp_scene_element_bounds(document, element, &bounds))
        return mcp_broker_set_error(out, "document_invalid", "element metadata could not be read");
    if (!append_mode_document(&out->structured_json, document) ||
        !mcp_json_buffer_literal(&out->structured_json, ",\"elementId\":" ) ||
        !mcp_broker_element_id(document, element, &out->structured_json) ||
        !mcp_json_buffer_literal(&out->structured_json, ",\"tag\":" ) ||
        !mcp_json_buffer_quoted(&out->structured_json, (const char *)tag.ptr, tag.len) ||
        !mcp_json_buffer_literal(&out->structured_json, ",\"type\":" ) ||
        !mcp_json_buffer_quoted(&out->structured_json, (const char *)tag.ptr, tag.len) ||
        !mcp_json_buffer_literal(&out->structured_json, ",\"depth\":" ) ||
        !append_size(&out->structured_json, depth) ||
        !mcp_json_buffer_literal(&out->structured_json, ",\"parentId\":")) return false;
    if (document_model_parent(model, element, &parent)) {
        if (!mcp_broker_element_id(document, parent, &out->structured_json)) return false;
    } else if (!mcp_json_buffer_literal(&out->structured_json, "null")) return false;
    return mcp_json_buffer_literal(&out->structured_json, ",\"attributes\":" ) &&
        append_attributes(document, element, &out->structured_json) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"bounds\":" ) &&
        append_bounds(&out->structured_json, &bounds) &&
        mcp_json_buffer_literal(&out->structured_json, "}" );
}

typedef struct selector_filter {
    char tag[128];
    size_t tag_length;
    char id[256];
    size_t id_length;
    char attribute[128];
    size_t attribute_length;
    char value[512];
    size_t value_length;
    bool require_value;
} selector_filter;

static bool parse_selector(const char *selector, size_t length, selector_filter *out)
{
    if (selector == NULL || out == NULL || length == 0U || length > 512U) return false;
    *out = (selector_filter){0};
    size_t cursor = 0U;
    if (selector[cursor] == '#') {
        cursor++;
        if (cursor == length) return false;
        out->id_length = length - cursor;
        if (out->id_length >= sizeof(out->id)) return false;
        memcpy(out->id, selector + cursor, out->id_length);
        return true;
    }
    if (selector[cursor] == '[') {
        cursor++;
        size_t start = cursor;
        while (cursor < length && selector[cursor] != '=' && selector[cursor] != ']') cursor++;
        if (cursor == start || cursor - start >= sizeof(out->attribute)) return false;
        out->attribute_length = cursor - start;
        memcpy(out->attribute, selector + start, out->attribute_length);
        if (cursor < length && selector[cursor] == '=') {
            cursor++;
            if (cursor >= length || (selector[cursor] != '\'' && selector[cursor] != '"')) return false;
            char quote = selector[cursor++];
            start = cursor;
            while (cursor < length && selector[cursor] != quote) cursor++;
            if (cursor >= length || cursor + 1U != length || cursor - start >= sizeof(out->value)) return false;
            out->value_length = cursor - start;
            memcpy(out->value, selector + start, out->value_length);
            out->require_value = true;
            return true;
        }
        return cursor + 1U == length && selector[cursor] == ']';
    }
    while (cursor < length && selector[cursor] != '#' && selector[cursor] != '[') cursor++;
    if (cursor > 0U) {
        if (cursor >= sizeof(out->tag)) return false;
        out->tag_length = cursor;
        memcpy(out->tag, selector, cursor);
    }
    if (cursor < length && selector[cursor] == '#') {
        size_t start = ++cursor;
        while (cursor < length && selector[cursor] != '[') cursor++;
        if (cursor == start || cursor - start >= sizeof(out->id)) return false;
        out->id_length = cursor - start;
        memcpy(out->id, selector + start, out->id_length);
    }
    if (cursor < length && selector[cursor] == '[') {
        selector_filter attribute_filter;
        if (!parse_selector(selector + cursor, length - cursor, &attribute_filter) ||
            attribute_filter.tag_length != 0U || attribute_filter.id_length != 0U) return false;
        out->attribute_length = attribute_filter.attribute_length;
        memcpy(out->attribute, attribute_filter.attribute, out->attribute_length);
        out->value_length = attribute_filter.value_length;
        memcpy(out->value, attribute_filter.value, out->value_length);
        out->require_value = attribute_filter.require_value;
        cursor = length;
    }
    return cursor == length && (out->tag_length > 0U || out->id_length > 0U ||
                                out->attribute_length > 0U);
}

static bool query_attribute_match(mcp_broker_document *document,
                                  document_handle element,
                                  const selector_filter *filter)
{
    if (filter->attribute_length == 0U) return true;
    for (size_t index = 0U; index < document_edit_attribute_count(document->edit, element); index++) {
        document_edit_attribute attribute;
        base_bytes name = {0}, value = {0};
        if (!document_edit_attribute_at(document->edit, element, index, &attribute) ||
            !document_edit_span_bytes(document->edit, attribute.name, &name) ||
            name.len != filter->attribute_length ||
            memcmp(name.ptr, filter->attribute, name.len) != 0) continue;
        if (!filter->require_value) return true;
        char decoded[512]; size_t decoded_length = 0U;
        if (document_edit_decode_span(document->edit, attribute.value,
                (uint8_t *)decoded, sizeof(decoded), &decoded_length) &&
            decoded_length == filter->value_length &&
            memcmp(decoded, filter->value, decoded_length) == 0) return true;
        if (document_edit_span_bytes(document->edit, attribute.value, &value) &&
            value.len == filter->value_length &&
            memcmp(value.ptr, filter->value, value.len) == 0) return true;
    }
    return false;
}

static bool element_query(mcp_broker *broker, const mcp_json_doc *request,
                          size_t args, mcp_tool_response *out)
{
    mcp_broker_document *document = mcp_broker_find(broker, request, args, "documentId");
    if (document == NULL) return mcp_broker_set_error(out, "document_not_found", "documentId is not open");
    selector_filter filter = {0};
    char selector[513]; size_t selector_length = 0U;
    size_t selector_token = mcp_json_object_get(request, args, "selector");
    if (selector_token != MCP_JSON_NO_TOKEN) {
        if (!mcp_scene_string_arg(request, args, "selector", selector,
                sizeof(selector), &selector_length) ||
            !parse_selector(selector, selector_length, &filter))
            return mcp_broker_set_error(out, "invalid_selector", "selector supports a tag, #id, [attribute], or [attribute=\"value\"]");
    }
    const char *keys[] = {"id", "tag", "attribute", "value"};
    char values[4][512] = {{0}};
    size_t lengths[4] = {0U, 0U, 0U, 0U};
    for (size_t index = 0U; index < 4U; index++) {
        size_t token = mcp_json_object_get(request, args, keys[index]);
        if (token != MCP_JSON_NO_TOKEN && !mcp_scene_string_arg(request, args,
                keys[index], values[index], sizeof(values[index]), &lengths[index]))
            return mcp_broker_set_error(out, "invalid_arguments", "query filters must be bounded strings");
    }
    if (lengths[0] > 0U) { memcpy(filter.id, values[0], lengths[0]); filter.id_length = lengths[0]; }
    if (lengths[1] > 0U) { memcpy(filter.tag, values[1], lengths[1]); filter.tag_length = lengths[1]; }
    if (lengths[2] > 0U) { memcpy(filter.attribute, values[2], lengths[2]); filter.attribute_length = lengths[2]; }
    if (lengths[3] > 0U) { memcpy(filter.value, values[3], lengths[3]); filter.value_length = lengths[3]; filter.require_value = true; }
    uint64_t cursor_value = 0U, limit_value = 128U;
    bool present = false, include_attributes = false, include_bounds = true;
    if (!mcp_broker_u64_arg(request, args, "cursor", &cursor_value, &present) ||
        !mcp_broker_u64_arg(request, args, "limit", &limit_value, &present) ||
        cursor_value > SIZE_MAX || limit_value == 0U || limit_value > 512U ||
        !optional_bool_arg(request, args, "includeAttributes", &include_attributes) ||
        !optional_bool_arg(request, args, "includeBounds", &include_bounds))
        return mcp_broker_set_error(out, "invalid_arguments", "query cursor, limit, or flags are invalid");
    size_t hit_token = mcp_json_object_get(request, args, "hitTest");
    bool filter_hit = hit_token != MCP_JSON_NO_TOKEN;
    double hit_x = 0.0, hit_y = 0.0;
    if (filter_hit && (request->tokens[hit_token].type != MCP_JSON_OBJECT ||
        !mcp_scene_number_arg(request, hit_token, "x", &hit_x, &present) || !present ||
        !mcp_scene_number_arg(request, hit_token, "y", &hit_y, &present) || !present))
        return mcp_broker_set_error(out, "invalid_arguments", "hitTest requires finite numeric x and y values");
    if (filter.id_length == 0U && filter.tag_length == 0U &&
        filter.attribute_length == 0U && !filter_hit)
        return mcp_broker_set_error(out, "invalid_arguments", "element_query needs an id, tag, selector, attribute filter, or hitTest point");
    document_handle id_match = {0};
    bool id_found = filter.id_length == 0U || mcp_broker_resolve_element(document,
        filter.id, filter.id_length, &id_match);
    const document_model *model = document_edit_model(document->edit);
    if (model == NULL) return mcp_broker_set_error(out, "document_invalid", "document tree is unavailable");
    size_t node_count = document_model_node_count(model);
    if (cursor_value > node_count) return mcp_broker_set_error(out, "invalid_cursor", "query cursor is outside the document");
    if (!append_mode_document(&out->structured_json, document) ||
        !mcp_json_buffer_literal(&out->structured_json, ",\"elements\":[")) return false;
    size_t slot = (size_t)cursor_value, emitted = 0U;
    for (; slot < node_count && emitted < (size_t)limit_value; slot++) {
        document_handle element;
        base_bytes tag = {0};
        if (!document_model_node_at(model, slot, &element) ||
            !document_edit_tag_name(document->edit, element, &tag)) continue;
        if (!id_found || (filter.id_length > 0U &&
            (element.slot != id_match.slot || element.generation != id_match.generation)) ||
            (filter.tag_length > 0U && (tag.len != filter.tag_length ||
             memcmp(tag.ptr, filter.tag, tag.len) != 0)) ||
            !query_attribute_match(document, element, &filter)) continue;
        if (filter_hit) {
            mcp_scene_bounds bounds;
            if (!mcp_scene_element_bounds(document, element, &bounds))
                return mcp_broker_set_error(out, "element_bounds_failed", "element bounds could not be calculated safely");
            if (!bounds.valid || hit_x < bounds.min_x || hit_x > bounds.max_x ||
                hit_y < bounds.min_y || hit_y > bounds.max_y) continue;
        }
        if (emitted > 0U && !mcp_json_buffer_literal(&out->structured_json, ",")) return false;
        if (!append_element(document, element, include_attributes, include_bounds,
                            &out->structured_json)) return false;
        emitted++;
    }
    return mcp_json_buffer_literal(&out->structured_json, "],\"mode\":" ) &&
        mcp_json_buffer_quoted(&out->structured_json,
            mcp_broker_document_mode(document),
            strlen(mcp_broker_document_mode(document))) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"cursor\":" ) &&
        append_size(&out->structured_json, slot) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"complete\":" ) &&
        mcp_json_buffer_literal(&out->structured_json, slot >= node_count ? "true}" : "false}");
}

bool mcp_broker_scene_query_call(mcp_broker *broker, const mcp_json_doc *request,
                                 size_t args, const char *tool_name,
                                 mcp_tool_response *out)
{
    if (strcmp(tool_name, "element_tree") == 0) return element_tree(broker, request, args, out);
    if (strcmp(tool_name, "element_get") == 0) return element_get(broker, request, args, out);
    if (strcmp(tool_name, "element_query") == 0) return element_query(broker, request, args, out);
    return false;
}
