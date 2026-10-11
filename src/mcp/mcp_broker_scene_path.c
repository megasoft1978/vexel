#include "mcp_broker_scene_internal.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MCP_SCENE_PATH_CAP 8192U
#define MCP_SCENE_PATH_NODE_LIMIT 512U

typedef struct path_node {
    char command;
    bool implicit_command;
    size_t start;
    size_t end;
    double values[7];
    size_t value_count;
} path_node;

typedef struct path_document {
    path_node nodes[MCP_SCENE_PATH_NODE_LIMIT];
    size_t count;
} path_document;

static void skip_separators(const char *text, size_t length, size_t *cursor)
{
    while (*cursor < length && (text[*cursor] == ' ' || text[*cursor] == '\t' ||
        text[*cursor] == '\r' || text[*cursor] == '\n' || text[*cursor] == ','))
        (*cursor)++;
}

static bool command_arity(char command, size_t *out_arity)
{
    switch (command) {
        case 'M': case 'm': case 'L': case 'l': case 'T': case 't': *out_arity = 2U; return true;
        case 'H': case 'h': case 'V': case 'v': *out_arity = 1U; return true;
        case 'C': case 'c': *out_arity = 6U; return true;
        case 'S': case 's': case 'Q': case 'q': *out_arity = 4U; return true;
        case 'A': case 'a': *out_arity = 7U; return true;
        case 'Z': case 'z': *out_arity = 0U; return true;
        default: return false;
    }
}

static bool path_parse(const char *text, size_t length, path_document *out)
{
    if (text == NULL || out == NULL || length > MCP_SCENE_PATH_CAP) return false;
    *out = (path_document){0};
    size_t cursor = 0U;
    char active_command = '\0';
    bool first_move_group = false;
    while (cursor < length) {
        skip_separators(text, length, &cursor);
        if (cursor == length) break;
        bool implicit_command = true;
        if (((text[cursor] >= 'A' && text[cursor] <= 'Z') ||
             (text[cursor] >= 'a' && text[cursor] <= 'z'))) {
            active_command = text[cursor++];
            first_move_group = active_command == 'M' || active_command == 'm';
            implicit_command = false;
        } else if (active_command == '\0' || active_command == 'Z' || active_command == 'z') {
            return false;
        }
        size_t arity = 0U;
        if (!command_arity(active_command, &arity) || out->count >= MCP_SCENE_PATH_NODE_LIMIT)
            return false;
        path_node *node = &out->nodes[out->count];
        node->command = active_command;
        node->implicit_command = implicit_command;
        node->start = cursor;
        if (arity == 0U) {
            node->end = cursor;
            out->count++;
            active_command = '\0';
            continue;
        }
        for (size_t index = 0U; index < arity; index++) {
            skip_separators(text, length, &cursor);
            if (cursor >= length || ((text[cursor] >= 'A' && text[cursor] <= 'Z') ||
                (text[cursor] >= 'a' && text[cursor] <= 'z'))) return false;
            char *end = NULL;
            double value = strtod(text + cursor, &end);
            if (end == text + cursor || !isfinite(value) || end > text + length) return false;
            cursor = (size_t)(end - text);
            node->values[index] = value;
            node->value_count++;
        }
        if ((active_command == 'A' || active_command == 'a') &&
            (node->values[0] < 0.0 || node->values[1] < 0.0 ||
             (node->values[3] != 0.0 && node->values[3] != 1.0) ||
             (node->values[4] != 0.0 && node->values[4] != 1.0))) return false;
        node->end = cursor;
        out->count++;
        if (first_move_group) {
            active_command = active_command == 'M' ? 'L' : 'l';
            first_move_group = false;
        }
    }
    return out->count > 0U;
}

static void include_point(mcp_scene_bounds *bounds, double x, double y)
{
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

bool mcp_scene_path_bounds(const char *path_data, size_t path_length,
                           mcp_scene_bounds *out)
{
    if (out == NULL) return false;
    *out = (mcp_scene_bounds){0};
    path_document path;
    if (!path_parse(path_data, path_length, &path)) return false;
    double x = 0.0, y = 0.0, start_x = 0.0, start_y = 0.0;
    for (size_t index = 0U; index < path.count; index++) {
        const path_node *node = &path.nodes[index];
        bool relative = node->command >= 'a' && node->command <= 'z';
        char command = relative ? (char)(node->command - ('a' - 'A')) : node->command;
        const double *v = node->values;
        if (command == 'M' || command == 'L' || command == 'T') {
            double nx = v[0] + (relative ? x : 0.0);
            double ny = v[1] + (relative ? y : 0.0);
            include_point(out, nx, ny); x = nx; y = ny;
            if (command == 'M') { start_x = x; start_y = y; }
        } else if (command == 'H') {
            x = v[0] + (relative ? x : 0.0); include_point(out, x, y);
        } else if (command == 'V') {
            y = v[0] + (relative ? y : 0.0); include_point(out, x, y);
        } else if (command == 'C' || command == 'S' || command == 'Q') {
            size_t pairs = command == 'C' ? 3U : (command == 'S' || command == 'Q' ? 2U : 0U);
            for (size_t pair = 0U; pair < pairs; pair++)
                include_point(out, v[pair * 2U] + (relative ? x : 0.0),
                              v[pair * 2U + 1U] + (relative ? y : 0.0));
            x = v[node->value_count - 2U] + (relative ? x : 0.0);
            y = v[node->value_count - 1U] + (relative ? y : 0.0);
            include_point(out, x, y); out->approximate = true;
        } else if (command == 'A') {
            x = v[5] + (relative ? x : 0.0);
            y = v[6] + (relative ? y : 0.0);
            double radius = fmax(fabs(v[0]), fabs(v[1]));
            include_point(out, x - radius, y - radius);
            include_point(out, x + radius, y + radius);
            out->approximate = true;
        } else if (command == 'Z') {
            x = start_x; y = start_y; include_point(out, x, y);
        }
    }
    return true;
}

static bool set_path_data(mcp_broker_document *document, document_handle element,
                          const char *path, size_t path_length)
{
    return document_edit_set_attribute(document->edit, element,
        (base_bytes){(const uint8_t *)"d", 1U},
        (base_bytes){(const uint8_t *)path, path_length}) == DOCUMENT_EDIT_OK;
}

static bool generate_path_id(mcp_broker_document *document, char out[64])
{
    for (uint32_t index = 0U; index < 10000U; index++) {
        int amount = snprintf(out, 64U, "mcp-path-%u-%u", document->serial, index);
        if (amount < 0 || amount >= 64) return false;
        document_handle found;
        if (!document_edit_find_id(document->edit,
                (base_bytes){(const uint8_t *)out, (size_t)amount}, &found)) return true;
    }
    return false;
}

document_edit_error mcp_broker_scene_path_apply(
    mcp_broker_document *document, const mcp_json_doc *request, size_t args,
    const char *tool_name, document_handle *out_element)
{
    if (document == NULL || request == NULL || tool_name == NULL || out_element == NULL)
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    if (strcmp(tool_name, "path_from_commands") == 0) {
        char parent_id[256], path[MCP_SCENE_PATH_CAP + 1U];
        size_t parent_length = 0U, path_length = 0U;
        if (!mcp_scene_string_arg(request, args, "parentId", parent_id,
                sizeof(parent_id), &parent_length) ||
            !mcp_scene_string_arg(request, args, "commands", path,
                sizeof(path), &path_length)) return DOCUMENT_EDIT_INVALID_ARGUMENT;
        path_document parsed;
        if (!path_parse(path, path_length, &parsed)) return DOCUMENT_EDIT_INVALID_OPERATION;
        document_handle parent;
        if (!mcp_broker_resolve_element(document, parent_id, parent_length, &parent))
            return DOCUMENT_EDIT_NOT_FOUND;
        document_edit_template_attribute attribute = {
            {(const uint8_t *)"d", 1U}, {(const uint8_t *)path, path_length}
        };
        document_edit_error error = document_edit_insert_template(document->edit, parent,
            DOCUMENT_EDIT_TEMPLATE_PATH, &attribute, 1U, out_element);
        if (error != DOCUMENT_EDIT_OK) return error;
        char generated_id[64];
        if (!generate_path_id(document, generated_id)) return DOCUMENT_EDIT_LIMIT;
        error = document_edit_add_attribute(document->edit, *out_element,
            (base_bytes){(const uint8_t *)"id", 2U},
            (base_bytes){(const uint8_t *)generated_id, strlen(generated_id)});
        if (error != DOCUMENT_EDIT_OK) return error;
        return document_edit_find_id(document->edit,
            (base_bytes){(const uint8_t *)generated_id, strlen(generated_id)}, out_element) ?
            DOCUMENT_EDIT_OK : DOCUMENT_EDIT_INVALID_OPERATION;
    }
    char element_id[256]; size_t element_length = 0U;
    if (!mcp_scene_string_arg(request, args, "elementId", element_id,
            sizeof(element_id), &element_length) ||
        !mcp_broker_resolve_element(document, element_id, element_length, out_element))
        return DOCUMENT_EDIT_NOT_FOUND;
    char path[MCP_SCENE_PATH_CAP + 1U]; size_t path_length = 0U; bool has_path = false;
    if (!mcp_scene_attribute_text(document, *out_element, "d", path,
            sizeof(path), &path_length, &has_path)) return DOCUMENT_EDIT_INVALID_OPERATION;
    if (strcmp(tool_name, "path_append") == 0) {
        char suffix[MCP_SCENE_PATH_CAP + 1U]; size_t suffix_length = 0U;
        if (!mcp_scene_string_arg(request, args, "commands", suffix,
                sizeof(suffix), &suffix_length)) return DOCUMENT_EDIT_INVALID_ARGUMENT;
        path_document parsed_suffix;
        if (!path_parse(suffix, suffix_length, &parsed_suffix)) return DOCUMENT_EDIT_INVALID_OPERATION;
        size_t total = path_length;
        if (has_path && total > 0U) {
            if (total + 1U > MCP_SCENE_PATH_CAP) return DOCUMENT_EDIT_LIMIT;
            path[total++] = ' ';
        }
        if (suffix_length > MCP_SCENE_PATH_CAP - total) return DOCUMENT_EDIT_LIMIT;
        memcpy(path + total, suffix, suffix_length);
        total += suffix_length;
        path[total] = '\0';
        path_document parsed;
        if (!path_parse(path, total, &parsed)) return DOCUMENT_EDIT_INVALID_OPERATION;
        return set_path_data(document, *out_element, path, total) ?
            DOCUMENT_EDIT_OK : DOCUMENT_EDIT_INVALID_OPERATION;
    }
    if (strcmp(tool_name, "path_edit_node") == 0) {
        if (!has_path) return DOCUMENT_EDIT_NOT_FOUND;
        uint64_t node_index = 0U; bool present = false;
        if (!mcp_broker_u64_arg(request, args, "nodeIndex", &node_index, &present) ||
            !present || node_index > SIZE_MAX) return DOCUMENT_EDIT_INVALID_ARGUMENT;
        char values[MCP_SCENE_PATH_CAP + 1U]; size_t values_length = 0U;
        if (!mcp_scene_string_arg(request, args, "values", values,
                sizeof(values), &values_length)) return DOCUMENT_EDIT_INVALID_ARGUMENT;
        path_document parsed;
        if (!path_parse(path, path_length, &parsed) || node_index >= parsed.count)
            return DOCUMENT_EDIT_NOT_FOUND;
        const path_node *node = &parsed.nodes[(size_t)node_index];
        if (node->value_count == 0U) return DOCUMENT_EDIT_INVALID_OPERATION;
        char replacement[MCP_SCENE_PATH_CAP + 1U];
        size_t replacement_length = 0U;
        if (node->implicit_command) {
            replacement[replacement_length++] = node->command;
            replacement[replacement_length++] = ' ';
        } else replacement[replacement_length++] = ' ';
        if (values_length > MCP_SCENE_PATH_CAP - replacement_length) return DOCUMENT_EDIT_LIMIT;
        memcpy(replacement + replacement_length, values, values_length);
        replacement_length += values_length;
        char validation_text[MCP_SCENE_PATH_CAP + 1U];
        if (values_length > MCP_SCENE_PATH_CAP - 2U) return DOCUMENT_EDIT_LIMIT;
        validation_text[0] = node->command;
        validation_text[1] = ' ';
        memcpy(validation_text + 2U, values, values_length);
        size_t validation_length = values_length + 2U;
        path_document validate;
        if (!path_parse(validation_text, validation_length, &validate) || validate.count != 1U ||
            validate.nodes[0].command != node->command) return DOCUMENT_EDIT_INVALID_OPERATION;
        size_t new_length = path_length - (node->end - node->start);
        if (replacement_length > MCP_SCENE_PATH_CAP - new_length) return DOCUMENT_EDIT_LIMIT;
        size_t tail_length = path_length - node->end;
        memmove(path + node->start + replacement_length, path + node->end, tail_length);
        memcpy(path + node->start, replacement, replacement_length);
        new_length += replacement_length;
        path[new_length] = '\0';
        return set_path_data(document, *out_element, path, new_length) ?
            DOCUMENT_EDIT_OK : DOCUMENT_EDIT_INVALID_OPERATION;
    }
    return DOCUMENT_EDIT_INVALID_OPERATION;
}

bool mcp_broker_scene_path_call(mcp_broker *broker,
                                const mcp_json_doc *request, size_t args,
                                const char *tool_name,
                                mcp_tool_response *out)
{
    (void)broker; (void)request; (void)args; (void)tool_name; (void)out;
    return false;
}
