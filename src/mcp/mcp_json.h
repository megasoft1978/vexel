#ifndef VEXEL_MCP_JSON_H
#define VEXEL_MCP_JSON_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define MCP_JSON_MAX_BYTES (2U * 1024U * 1024U)
#define MCP_JSON_MAX_DEPTH 32U
#define MCP_JSON_MAX_TOKENS 65536U
#define MCP_JSON_NO_TOKEN SIZE_MAX

typedef enum mcp_json_type {
    MCP_JSON_OBJECT,
    MCP_JSON_ARRAY,
    MCP_JSON_STRING,
    MCP_JSON_NUMBER,
    MCP_JSON_TRUE,
    MCP_JSON_FALSE,
    MCP_JSON_NULL
} mcp_json_type;

typedef struct mcp_json_token {
    mcp_json_type type;
    size_t start;
    size_t end;
    size_t first_child;
    size_t last_child;
    size_t next_sibling;
    size_t child_count;
} mcp_json_token;

typedef struct mcp_json_doc {
    const uint8_t *source;
    size_t source_length;
    mcp_json_token *tokens;
    size_t token_count;
    size_t token_capacity;
    size_t root;
} mcp_json_doc;

typedef struct mcp_json_buffer {
    char *data;
    size_t length;
    size_t capacity;
    size_t limit;
} mcp_json_buffer;

bool mcp_json_parse(const uint8_t *source, size_t length, mcp_json_doc *out);
void mcp_json_doc_destroy(mcp_json_doc *doc);
size_t mcp_json_object_get(const mcp_json_doc *doc, size_t object, const char *key);
size_t mcp_json_array_get(const mcp_json_doc *doc, size_t array, size_t index);
bool mcp_json_string_equals(const mcp_json_doc *doc, size_t token, const char *literal);
bool mcp_json_string_copy(const mcp_json_doc *doc, size_t token, char *out,
                          size_t capacity, size_t *out_length);
bool mcp_json_token_copy(const mcp_json_doc *doc, size_t token, char *out,
                         size_t capacity, size_t *out_length);
bool mcp_json_uint(const mcp_json_doc *doc, size_t token, uint64_t *out);
bool mcp_json_bool(const mcp_json_doc *doc, size_t token, bool *out);
bool mcp_json_buffer_init(mcp_json_buffer *buffer, size_t initial, size_t limit);
void mcp_json_buffer_destroy(mcp_json_buffer *buffer);
bool mcp_json_buffer_append(mcp_json_buffer *buffer, const char *data, size_t length);
bool mcp_json_buffer_literal(mcp_json_buffer *buffer, const char *literal);
bool mcp_json_buffer_quoted(mcp_json_buffer *buffer, const char *data, size_t length);
bool mcp_json_buffer_raw_token(mcp_json_buffer *buffer, const mcp_json_doc *doc,
                               size_t token);

#endif
