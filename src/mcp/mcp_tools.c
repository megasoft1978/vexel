#include "mcp_protocol.h"

#include <string.h>

typedef struct mcp_tool_schema {
    const char *name;
    const char *description;
    const char *properties;
    const char *required;
    const char *fields;
} mcp_tool_schema;

static const mcp_tool_schema tools[] = {
    {"document_open", "Open a local SVG within a user-granted directory.",
     "{\"path\":{\"type\":\"string\"}}", "path", "path:string"},
    {"document_close", "Close a non-active MCP-opened document after checking for unsaved edits.",
     "{\"documentId\":{\"type\":\"string\"}}", "documentId", "documentId:string"},
    {"document_list", "List SVG documents open in Vexel.", "{}", "", ""},
    {"document_info", "Read the current size, path, and edit state of an open SVG.",
     "{\"documentId\":{\"type\":\"string\"}}", "documentId", "documentId:string"},
    {"tree_get", "Read a bounded page of the document element tree.",
     "{\"documentId\":{\"type\":\"string\"},\"cursor\":{\"type\":\"integer\",\"minimum\":0},\"limit\":{\"type\":\"integer\",\"minimum\":1,\"maximum\":512}}",
     "documentId", "documentId:string;cursor:integer;limit:integer"},
    {"element_get", "Read one element and its attributes by id or returned element handle.",
     "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"}}",
     "documentId,elementId", "documentId:string;elementId:string"},
    {"element_query", "Find elements by id, tag, or exact attribute value.",
     "{\"documentId\":{\"type\":\"string\"},\"id\":{\"type\":\"string\"},\"tag\":{\"type\":\"string\"},\"attribute\":{\"type\":\"string\"},\"value\":{\"type\":\"string\"},\"cursor\":{\"type\":\"integer\",\"minimum\":0},\"limit\":{\"type\":\"integer\",\"minimum\":1,\"maximum\":512}}",
     "documentId", "documentId:string;id:string;tag:string;attribute:string;value:string;cursor:integer;limit:integer"},
    {"attribute_set", "Set an existing XML attribute through the undoable edit engine.",
     "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"},\"name\":{\"type\":\"string\"},\"value\":{\"type\":\"string\"}}",
     "documentId,elementId,name,value", "documentId:string;elementId:string;name:string;value:string"},
    {"attribute_remove", "Remove an XML attribute through the undoable edit engine.",
     "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"},\"name\":{\"type\":\"string\"}}",
     "documentId,elementId,name", "documentId:string;elementId:string;name:string"},
    {"text_set", "Replace an element's text content through the undoable edit engine.",
     "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"},\"text\":{\"type\":\"string\"}}",
     "documentId,elementId,text", "documentId:string;elementId:string;text:string"},
    {"element_add", "Add a basic SVG shape, group, or text template as a child of an element.",
     "{\"documentId\":{\"type\":\"string\"},\"parentId\":{\"type\":\"string\"},\"template\":{\"type\":\"string\",\"enum\":[\"rect\",\"circle\",\"ellipse\",\"line\",\"path\",\"g\",\"text\"]},\"attributes\":{\"type\":\"object\",\"additionalProperties\":{\"type\":\"string\"}}}",
     "documentId,parentId,template", "documentId:string;parentId:string;template:string;attributes:object"},
    {"element_add_fragment", "Add one validated XML element fragment as a child of an element.",
     "{\"documentId\":{\"type\":\"string\"},\"parentId\":{\"type\":\"string\"},\"xml\":{\"type\":\"string\",\"maxLength\":65536}}",
     "documentId,parentId,xml", "documentId:string;parentId:string;xml:string"},
    {"element_delete", "Delete a non-root element through the undoable edit engine.",
     "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"}}",
     "documentId,elementId", "documentId:string;elementId:string"},
    {"element_duplicate", "Duplicate an element as its next sibling.",
     "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"}}",
     "documentId,elementId", "documentId:string;elementId:string"},
    {"element_reorder", "Move an element before or after a sibling element.",
     "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"},\"targetId\":{\"type\":\"string\"},\"before\":{\"type\":\"boolean\"}}",
     "documentId,elementId,targetId,before", "documentId:string;elementId:string;targetId:string;before:boolean"},
    {"element_group", "Wrap an element in an SVG group.",
     "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"}}",
     "documentId,elementId", "documentId:string;elementId:string"},
    {"element_ungroup", "Remove a group while retaining its children.",
     "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"}}",
     "documentId,elementId", "documentId:string;elementId:string"},
    {"transaction", "Apply a bounded edit batch atomically; failed batches are discarded.",
     "{\"documentId\":{\"type\":\"string\"},\"edits\":{\"type\":\"array\",\"minItems\":1,\"maxItems\":64,\"items\":{\"type\":\"object\"}}}",
     "documentId,edits", "documentId:string;edits:array"},
    {"undo", "Undo the latest document edit.", "{\"documentId\":{\"type\":\"string\"}}",
     "documentId", "documentId:string"},
    {"redo", "Redo the next document edit.", "{\"documentId\":{\"type\":\"string\"}}",
     "documentId", "documentId:string"},
    {"save", "Save the current document to its open path.", "{\"documentId\":{\"type\":\"string\"}}",
     "documentId", "documentId:string"},
    {"save_as", "Save the current document atomically inside a granted directory.",
     "{\"documentId\":{\"type\":\"string\"},\"path\":{\"type\":\"string\"}}",
     "documentId,path", "documentId:string;path:string"},
    {"render_png", "Render a bounded image of an open SVG region.",
     "{\"documentId\":{\"type\":\"string\"},\"region\":{\"type\":\"object\"},\"scale\":{\"type\":\"number\",\"minimum\":0.01,\"maximum\":16}}",
     "documentId", "documentId:string;region:object;scale:number"},
    {"selection_get", "Read Vexel's current selection for a document.", "{\"documentId\":{\"type\":\"string\"}}",
     "documentId", "documentId:string"},
    {"selection_set", "Select an element in Vexel by id or returned element handle.",
     "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"}}",
     "documentId,elementId", "documentId:string;elementId:string"},
    {"export_svg", "Return the current source-preserving SVG document text.", "{\"documentId\":{\"type\":\"string\"}}",
     "documentId", "documentId:string"}
};

static bool append_csv_array(mcp_json_buffer *out, const char *csv)
{
    if (!mcp_json_buffer_literal(out, "[")) return false;
    bool first = true;
    const char *cursor = csv;
    while (*cursor != '\0') {
        const char *end = strchr(cursor, ',');
        size_t length = end == NULL ? strlen(cursor) : (size_t)(end - cursor);
        if (!first && !mcp_json_buffer_literal(out, ",")) return false;
        if (!mcp_json_buffer_quoted(out, cursor, length)) return false;
        first = false;
        if (end == NULL) break;
        cursor = end + 1;
    }
    return mcp_json_buffer_literal(out, "]");
}

bool mcp_tools_list_json(mcp_json_buffer *out)
{
    if (out == NULL || !mcp_json_buffer_literal(out, "[")) return false;
    for (size_t index = 0U; index < sizeof(tools) / sizeof(tools[0]); index++) {
        const mcp_tool_schema *tool = &tools[index];
        if (index > 0U && !mcp_json_buffer_literal(out, ",")) return false;
        if (!mcp_json_buffer_literal(out, "{\"name\":" ) ||
            !mcp_json_buffer_quoted(out, tool->name, strlen(tool->name)) ||
            !mcp_json_buffer_literal(out, ",\"description\":" ) ||
            !mcp_json_buffer_quoted(out, tool->description, strlen(tool->description)) ||
            !mcp_json_buffer_literal(out, ",\"inputSchema\":{\"type\":\"object\",\"properties\":" ) ||
            !mcp_json_buffer_literal(out, tool->properties) ||
            !mcp_json_buffer_literal(out, ",\"required\":" ) ||
            !append_csv_array(out, tool->required) ||
            !mcp_json_buffer_literal(out, ",\"additionalProperties\":false}}")) return false;
    }
    return mcp_json_buffer_literal(out, "]");
}

static const mcp_tool_schema *find_tool(const char *name)
{
    for (size_t index = 0U; index < sizeof(tools) / sizeof(tools[0]); index++)
        if (strcmp(name, tools[index].name) == 0) return &tools[index];
    return NULL;
}

static const char *field_type(const char *fields, const char *key)
{
    size_t key_length = strlen(key);
    const char *cursor = fields;
    while (*cursor != '\0') {
        const char *colon = strchr(cursor, ':');
        if (colon == NULL) return NULL;
        const char *end = strchr(colon + 1, ';');
        size_t name_length = (size_t)(colon - cursor);
        if (name_length == key_length && memcmp(cursor, key, key_length) == 0) return colon + 1;
        if (end == NULL) return NULL;
        cursor = end + 1;
    }
    return NULL;
}

static bool type_is(const mcp_json_doc *doc, size_t token, const char *type)
{
    if (token >= doc->token_count) return false;
    mcp_json_type actual = doc->tokens[token].type;
    if (strcmp(type, "string") == 0) return actual == MCP_JSON_STRING;
    if (strcmp(type, "object") == 0) return actual == MCP_JSON_OBJECT;
    if (strcmp(type, "array") == 0) return actual == MCP_JSON_ARRAY;
    if (strcmp(type, "boolean") == 0) return actual == MCP_JSON_TRUE || actual == MCP_JSON_FALSE;
    if (strcmp(type, "number") == 0) return actual == MCP_JSON_NUMBER;
    if (strcmp(type, "integer") == 0) {
        uint64_t number = 0U;
        return actual == MCP_JSON_NUMBER && mcp_json_uint(doc, token, &number);
    }
    return false;
}

bool mcp_tool_validate(const mcp_json_doc *doc, size_t arguments,
                       const char *tool_name, const char **out_error)
{
    if (out_error != NULL) *out_error = "invalid tool arguments";
    if (doc == NULL || arguments >= doc->token_count || tool_name == NULL ||
        doc->tokens[arguments].type != MCP_JSON_OBJECT) return false;
    const mcp_tool_schema *tool = find_tool(tool_name);
    if (tool == NULL) {
        if (out_error != NULL) *out_error = "unknown tool";
        return false;
    }
    size_t key = doc->tokens[arguments].first_child;
    while (key != MCP_JSON_NO_TOKEN) {
        size_t value = doc->tokens[key].next_sibling;
        if (value == MCP_JSON_NO_TOKEN || doc->tokens[key].type != MCP_JSON_STRING) return false;
        char key_buffer[64];
        size_t key_length = 0U;
        if (!mcp_json_string_copy(doc, key, key_buffer, sizeof(key_buffer) - 1U, &key_length)) return false;
        key_buffer[key_length] = '\0';
        const char *expected = field_type(tool->fields, key_buffer);
        if (expected == NULL) {
            if (out_error != NULL) *out_error = "unexpected argument";
            return false;
        }
        size_t expected_length = strcspn(expected, ";");
        char expected_type[16];
        if (expected_length == 0U || expected_length >= sizeof(expected_type)) return false;
        memcpy(expected_type, expected, expected_length);
        expected_type[expected_length] = '\0';
        if (!type_is(doc, value, expected_type)) {
            if (out_error != NULL) *out_error = "argument has the wrong type";
            return false;
        }
        if ((strcmp(tool_name, "element_add") == 0 && strcmp(key_buffer, "attributes") == 0) ||
            (strcmp(tool_name, "transaction") == 0 && strcmp(key_buffer, "edits") == 0)) {
            if (strcmp(key_buffer, "attributes") == 0) {
                size_t child = doc->tokens[value].first_child;
                while (child != MCP_JSON_NO_TOKEN) {
                    size_t child_value = doc->tokens[child].next_sibling;
                    if (child_value == MCP_JSON_NO_TOKEN || doc->tokens[child].type != MCP_JSON_STRING ||
                        doc->tokens[child_value].type != MCP_JSON_STRING) {
                        if (out_error != NULL) *out_error = "template attributes must map strings to strings";
                        return false;
                    }
                    child = doc->tokens[child_value].next_sibling;
                }
            } else if (doc->tokens[value].child_count == 0U || doc->tokens[value].child_count > 64U) {
                if (out_error != NULL) *out_error = "transaction must contain between 1 and 64 edits";
                return false;
            }
        }
        key = doc->tokens[value].next_sibling;
    }
    const char *required = tool->required;
    while (*required != '\0') {
        const char *end = strchr(required, ',');
        size_t length = end == NULL ? strlen(required) : (size_t)(end - required);
        char name[64];
        if (length == 0U || length >= sizeof(name)) return false;
        memcpy(name, required, length);
        name[length] = '\0';
        if (mcp_json_object_get(doc, arguments, name) == MCP_JSON_NO_TOKEN) {
            if (out_error != NULL) *out_error = "required argument missing";
            return false;
        }
        if (end == NULL) break;
        required = end + 1;
    }
    return true;
}
