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
    {"element_tree", "Read a bounded outline with ids, types, attributes, parent links, and geometric bounds.",
     "{\"documentId\":{\"type\":\"string\"},\"parentId\":{\"type\":\"string\"},\"cursor\":{\"type\":\"integer\",\"minimum\":0},\"limit\":{\"type\":\"integer\",\"minimum\":1,\"maximum\":512},\"includeAttributes\":{\"type\":\"boolean\"},\"includeBounds\":{\"type\":\"boolean\"}}",
     "documentId", "documentId:string;parentId:string;cursor:integer;limit:integer;includeAttributes:boolean;includeBounds:boolean"},
    {"element_get", "Read one element, its attributes, source type, parent, and geometric bounds.",
     "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"}}",
     "documentId,elementId", "documentId:string;elementId:string"},
    {"element_query", "Find elements by id, tag, CSS-lite selector, attributes, or transformed bounds hit testing.",
     "{\"documentId\":{\"type\":\"string\"},\"id\":{\"type\":\"string\"},\"tag\":{\"type\":\"string\"},\"selector\":{\"type\":\"string\",\"maxLength\":512},\"attribute\":{\"type\":\"string\"},\"value\":{\"type\":\"string\"},\"hitTest\":{\"type\":\"object\",\"properties\":{\"x\":{\"type\":\"number\"},\"y\":{\"type\":\"number\"}},\"required\":[\"x\",\"y\"]},\"includeAttributes\":{\"type\":\"boolean\"},\"includeBounds\":{\"type\":\"boolean\"},\"cursor\":{\"type\":\"integer\",\"minimum\":0},\"limit\":{\"type\":\"integer\",\"minimum\":1,\"maximum\":512}}",
     "documentId", "documentId:string;id:string;tag:string;selector:string;attribute:string;value:string;hitTest:object;includeAttributes:boolean;includeBounds:boolean;cursor:integer;limit:integer"},
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
    {"element_create", "Create one bounded SVG element under a parent at an optional child index.",
     "{\"documentId\":{\"type\":\"string\"},\"tag\":{\"type\":\"string\",\"enum\":[\"rect\",\"circle\",\"ellipse\",\"line\",\"polyline\",\"polygon\",\"path\",\"text\",\"image\",\"g\",\"use\",\"defs\",\"linearGradient\",\"radialGradient\",\"clipPath\",\"mask\",\"filter\",\"feGaussianBlur\",\"feOffset\",\"feFlood\",\"feBlend\",\"feComposite\",\"feMerge\",\"feMergeNode\",\"feColorMatrix\",\"feDropShadow\"]},\"parentId\":{\"type\":\"string\"},\"index\":{\"type\":\"integer\",\"minimum\":0},\"attributes\":{\"type\":\"object\",\"maxProperties\":32,\"additionalProperties\":{\"type\":\"string\",\"maxLength\":2048}},\"style\":{\"type\":\"object\",\"maxProperties\":16,\"additionalProperties\":{\"type\":\"string\"}},\"text\":{\"type\":\"string\",\"maxLength\":2048}}",
     "documentId,tag,parentId", "documentId:string;tag:string;parentId:string;index:integer;attributes:object;style:object;text:string"},
    {"element_set", "Set bounded attributes and CSS style properties on one element in one undo step.",
     "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"},\"attributes\":{\"type\":\"object\",\"maxProperties\":32,\"additionalProperties\":{\"type\":\"string\",\"maxLength\":2048}},\"style\":{\"type\":\"object\",\"maxProperties\":16,\"additionalProperties\":{\"type\":\"string\"}}}",
     "documentId,elementId", "documentId:string;elementId:string;attributes:object;style:object"},
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
    {"transform_set", "Set a validated SVG transform list on one element.",
     "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"},\"value\":{\"type\":\"string\",\"maxLength\":512}}",
     "documentId,elementId,value", "documentId:string;elementId:string;value:string"},
    {"transform_apply", "Append a bounded translate, scale, rotate-about-point, skew, or matrix transform.",
     "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"},\"operation\":{\"type\":\"string\",\"enum\":[\"translate\",\"scale\",\"rotate\",\"skewX\",\"skewY\",\"matrix\"]},\"x\":{\"type\":\"number\"},\"y\":{\"type\":\"number\"},\"sx\":{\"type\":\"number\"},\"sy\":{\"type\":\"number\"},\"angle\":{\"type\":\"number\"},\"cx\":{\"type\":\"number\"},\"cy\":{\"type\":\"number\"},\"a\":{\"type\":\"number\"},\"b\":{\"type\":\"number\"},\"c\":{\"type\":\"number\"},\"d\":{\"type\":\"number\"},\"e\":{\"type\":\"number\"},\"f\":{\"type\":\"number\"}}",
     "documentId,elementId,operation", "documentId:string;elementId:string;operation:string;x:number;y:number;sx:number;sy:number;angle:number;cx:number;cy:number;a:number;b:number;c:number;d:number;e:number;f:number"},
    {"style_fill", "Set or replace the fill style on one element.",
     "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"},\"value\":{\"type\":\"string\",\"maxLength\":512}}",
     "documentId,elementId,value", "documentId:string;elementId:string;value:string"},
    {"style_stroke", "Set or replace the stroke style on one element.",
     "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"},\"value\":{\"type\":\"string\",\"maxLength\":512}}",
     "documentId,elementId,value", "documentId:string;elementId:string;value:string"},
    {"style_opacity", "Set an opacity value between 0 and 1 on one element.",
     "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"},\"value\":{\"type\":\"string\",\"maxLength\":64}}",
     "documentId,elementId,value", "documentId:string;elementId:string;value:string"},
    {"gradient_create", "Create a local linear or radial gradient definition.",
     "{\"documentId\":{\"type\":\"string\"},\"parentId\":{\"type\":\"string\"},\"kind\":{\"type\":\"string\",\"enum\":[\"linearGradient\",\"radialGradient\"]},\"attributes\":{\"type\":\"object\",\"maxProperties\":16,\"additionalProperties\":{\"type\":\"string\"}},\"index\":{\"type\":\"integer\",\"minimum\":0}}",
     "documentId,parentId,kind", "documentId:string;parentId:string;kind:string;attributes:object;index:integer"},
    {"gradient_stop_add", "Add a bounded stop to a local gradient.",
     "{\"documentId\":{\"type\":\"string\"},\"gradientId\":{\"type\":\"string\"},\"index\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"offset\":{\"type\":\"string\"},\"color\":{\"type\":\"string\"},\"opacity\":{\"type\":\"string\"}}",
     "documentId,gradientId,index,offset,color", "documentId:string;gradientId:string;index:integer;offset:string;color:string;opacity:string"},
    {"gradient_stop_set", "Edit one stop's offset, color, or opacity by its zero-based stop index.",
     "{\"documentId\":{\"type\":\"string\"},\"gradientId\":{\"type\":\"string\"},\"index\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255},\"offset\":{\"type\":\"string\"},\"color\":{\"type\":\"string\"},\"opacity\":{\"type\":\"string\"}}",
     "documentId,gradientId,index", "documentId:string;gradientId:string;index:integer;offset:string;color:string;opacity:string"},
    {"gradient_stop_remove", "Remove one local gradient stop by its zero-based stop index.",
     "{\"documentId\":{\"type\":\"string\"},\"gradientId\":{\"type\":\"string\"},\"index\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":255}}",
     "documentId,gradientId,index", "documentId:string;gradientId:string;index:integer"},
    {"path_from_commands", "Create a path from a validated bounded SVG path command string.",
     "{\"documentId\":{\"type\":\"string\"},\"parentId\":{\"type\":\"string\"},\"commands\":{\"type\":\"string\",\"maxLength\":8192}}",
     "documentId,parentId,commands", "documentId:string;parentId:string;commands:string"},
    {"path_append", "Append validated path commands to an existing path.",
     "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"},\"commands\":{\"type\":\"string\",\"maxLength\":8192}}",
     "documentId,elementId,commands", "documentId:string;elementId:string;commands:string"},
    {"path_edit_node", "Replace one absolute or relative path command node's numeric values.",
     "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"},\"nodeIndex\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":511},\"values\":{\"type\":\"string\",\"maxLength\":256}}",
     "documentId,elementId,nodeIndex,values", "documentId:string;elementId:string;nodeIndex:integer;values:string"},
    {"element_align", "Align two or more bounded elements by their x or y edge or center.",
     "{\"documentId\":{\"type\":\"string\"},\"elementIds\":{\"type\":\"array\",\"minItems\":2,\"maxItems\":64,\"items\":{\"type\":\"string\"}},\"axis\":{\"type\":\"string\",\"enum\":[\"x\",\"y\"]},\"alignment\":{\"type\":\"string\",\"enum\":[\"start\",\"center\",\"end\"]}}",
     "documentId,elementIds,axis,alignment", "documentId:string;elementIds:array;axis:string;alignment:string"},
    {"element_distribute", "Distribute three or more bounded elements with equal gaps along x or y.",
     "{\"documentId\":{\"type\":\"string\"},\"elementIds\":{\"type\":\"array\",\"minItems\":3,\"maxItems\":64,\"items\":{\"type\":\"string\"}},\"axis\":{\"type\":\"string\",\"enum\":[\"x\",\"y\"]}}",
     "documentId,elementIds,axis", "documentId:string;elementIds:array;axis:string"},
    {"batch", "Apply 1 to 32 scene operations atomically as one source-preserving undo step.",
     "{\"documentId\":{\"type\":\"string\"},\"operations\":{\"type\":\"array\",\"minItems\":1,\"maxItems\":32,\"items\":{\"type\":\"object\",\"properties\":{"
     "\"op\":{\"type\":\"string\",\"enum\":[\"element_create\",\"element_set\",\"element_delete\",\"element_duplicate\",\"element_reorder\",\"element_group\",\"element_ungroup\",\"transform_set\",\"transform_apply\",\"path_from_commands\",\"path_append\",\"path_edit_node\",\"style_fill\",\"style_stroke\",\"style_opacity\",\"gradient_create\",\"gradient_stop_add\",\"gradient_stop_set\",\"gradient_stop_remove\",\"element_align\",\"element_distribute\"]},"
     "\"tag\":{\"type\":\"string\"},\"parentId\":{\"type\":\"string\"},\"index\":{\"type\":\"integer\",\"minimum\":0},\"elementId\":{\"type\":\"string\"},\"targetId\":{\"type\":\"string\"},\"before\":{\"type\":\"boolean\"},\"attributes\":{\"type\":\"object\",\"maxProperties\":32,\"additionalProperties\":{\"type\":\"string\"}},\"style\":{\"type\":\"object\",\"maxProperties\":16,\"additionalProperties\":{\"type\":\"string\"}},\"text\":{\"type\":\"string\"},\"value\":{\"type\":\"string\"},\"operation\":{\"type\":\"string\"},\"x\":{\"type\":\"number\"},\"y\":{\"type\":\"number\"},\"sx\":{\"type\":\"number\"},\"sy\":{\"type\":\"number\"},\"angle\":{\"type\":\"number\"},\"cx\":{\"type\":\"number\"},\"cy\":{\"type\":\"number\"},\"a\":{\"type\":\"number\"},\"b\":{\"type\":\"number\"},\"c\":{\"type\":\"number\"},\"d\":{\"type\":\"number\"},\"e\":{\"type\":\"number\"},\"f\":{\"type\":\"number\"},\"commands\":{\"type\":\"string\"},\"nodeIndex\":{\"type\":\"integer\",\"minimum\":0},\"values\":{\"type\":\"string\"},\"kind\":{\"type\":\"string\"},\"gradientId\":{\"type\":\"string\"},\"offset\":{\"type\":\"string\"},\"color\":{\"type\":\"string\"},\"opacity\":{\"type\":\"string\"},\"elementIds\":{\"type\":\"array\",\"maxItems\":64,\"items\":{\"type\":\"string\"}},\"axis\":{\"type\":\"string\"},\"alignment\":{\"type\":\"string\"}},\"required\":[\"op\"],\"additionalProperties\":false}}}",
     "documentId,operations", "documentId:string;operations:array"},
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
     "{\"documentId\":{\"type\":\"string\"},\"region\":{\"type\":\"object\"},\"scale\":{\"type\":\"number\",\"minimum\":0.01,\"maximum\":16},\"time\":{\"type\":\"number\",\"minimum\":0,\"maximum\":3600}}",
     "documentId", "documentId:string;region:object;scale:number;time:number"},
    {"animation_list", "List supported SMIL animation tracks, key values, spans, and diagnostics.",
     "{\"documentId\":{\"type\":\"string\"}}", "documentId", "documentId:string"},
    {"animation_add", "Add one validated SMIL animation element to a target through the edit engine.",
     "{\"documentId\":{\"type\":\"string\"},\"targetId\":{\"type\":\"string\"},\"xml\":{\"type\":\"string\",\"maxLength\":65536}}",
     "documentId,targetId,xml", "documentId:string;targetId:string;xml:string"},
    {"animation_set", "Set or add an attribute on one animation track through the edit engine.",
     "{\"documentId\":{\"type\":\"string\"},\"animationId\":{\"type\":\"string\"},\"name\":{\"type\":\"string\"},\"value\":{\"type\":\"string\"}}",
     "documentId,animationId,name,value", "documentId:string;animationId:string;name:string;value:string"},
    {"animation_remove", "Remove one animation track through the edit engine.",
     "{\"documentId\":{\"type\":\"string\"},\"animationId\":{\"type\":\"string\"}}",
     "documentId,animationId", "documentId:string;animationId:string"},
    {"export_animation", "Export a bounded animation as a GIF or PNG frame sequence inside a granted directory.",
     "{\"documentId\":{\"type\":\"string\"},\"path\":{\"type\":\"string\"},\"format\":{\"type\":\"string\",\"enum\":[\"gif\",\"frames\"]},\"fps\":{\"type\":\"integer\",\"minimum\":1,\"maximum\":30},\"duration\":{\"type\":\"number\",\"minimum\":0.01,\"maximum\":3600}}",
     "documentId,path,format,fps,duration", "documentId:string;path:string;format:string;fps:integer;duration:number"},
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

static bool string_map_valid(const mcp_json_doc *doc, size_t object,
                             size_t maximum, const char **out_error)
{
    if (object >= doc->token_count || doc->tokens[object].type != MCP_JSON_OBJECT ||
        doc->tokens[object].child_count > maximum * 2U) {
        if (out_error != NULL) *out_error = "object exceeds its property limit";
        return false;
    }
    size_t key = doc->tokens[object].first_child;
    size_t count = 0U;
    while (key != MCP_JSON_NO_TOKEN) {
        size_t value = doc->tokens[key].next_sibling;
        if (value == MCP_JSON_NO_TOKEN || doc->tokens[key].type != MCP_JSON_STRING ||
            doc->tokens[value].type != MCP_JSON_STRING || ++count > maximum) {
            if (out_error != NULL) *out_error = "object values must be bounded strings";
            return false;
        }
        key = doc->tokens[value].next_sibling;
    }
    return true;
}

static bool string_array_valid(const mcp_json_doc *doc, size_t array,
                               size_t minimum, size_t maximum,
                               const char **out_error)
{
    if (array >= doc->token_count || doc->tokens[array].type != MCP_JSON_ARRAY ||
        doc->tokens[array].child_count < minimum ||
        doc->tokens[array].child_count > maximum) {
        if (out_error != NULL) *out_error = "array length is outside its supported range";
        return false;
    }
    for (size_t index = 0U; index < doc->tokens[array].child_count; index++) {
        size_t item = mcp_json_array_get(doc, array, index);
        if (item == MCP_JSON_NO_TOKEN || doc->tokens[item].type != MCP_JSON_STRING) {
            if (out_error != NULL) *out_error = "array items must be strings";
            return false;
        }
    }
    return true;
}

static bool batch_operations_valid(const mcp_json_doc *doc, size_t array,
                                   const char **out_error)
{
    if (array >= doc->token_count || doc->tokens[array].type != MCP_JSON_ARRAY ||
        doc->tokens[array].child_count == 0U || doc->tokens[array].child_count > 32U) {
        if (out_error != NULL) *out_error = "batch requires 1 to 32 operations";
        return false;
    }
    for (size_t index = 0U; index < doc->tokens[array].child_count; index++) {
        size_t operation = mcp_json_array_get(doc, array, index);
        size_t op = mcp_json_object_get(doc, operation, "op");
        if (operation == MCP_JSON_NO_TOKEN || doc->tokens[operation].type != MCP_JSON_OBJECT ||
            op == MCP_JSON_NO_TOKEN || doc->tokens[op].type != MCP_JSON_STRING) {
            if (out_error != NULL) *out_error = "batch items must be operation objects with a string op";
            return false;
        }
    }
    return true;
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
            ((strcmp(tool_name, "element_create") == 0 || strcmp(tool_name, "element_set") == 0 ||
              strcmp(tool_name, "gradient_create") == 0) && strcmp(key_buffer, "attributes") == 0) ||
            ((strcmp(tool_name, "element_create") == 0 || strcmp(tool_name, "element_set") == 0) &&
             strcmp(key_buffer, "style") == 0)) {
            size_t maximum = strcmp(key_buffer, "style") == 0 ? 16U : 32U;
            if (!string_map_valid(doc, value, maximum, out_error)) return false;
        } else if (strcmp(tool_name, "element_align") == 0 && strcmp(key_buffer, "elementIds") == 0) {
            if (!string_array_valid(doc, value, 2U, 64U, out_error)) return false;
        } else if (strcmp(tool_name, "element_distribute") == 0 && strcmp(key_buffer, "elementIds") == 0) {
            if (!string_array_valid(doc, value, 3U, 64U, out_error)) return false;
        } else if ((strcmp(tool_name, "batch") == 0 && strcmp(key_buffer, "operations") == 0)) {
            if (!batch_operations_valid(doc, value, out_error)) return false;
        } else if ((strcmp(tool_name, "transaction") == 0 && strcmp(key_buffer, "edits") == 0)) {
            if (doc->tokens[value].child_count == 0U || doc->tokens[value].child_count > 64U) {
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
