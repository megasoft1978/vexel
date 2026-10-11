#include "mcp_json.h"

#include <stdlib.h>
#include <string.h>

typedef struct json_parser {
    mcp_json_doc *doc;
    size_t cursor;
} json_parser;

static void skip_space(json_parser *parser)
{
    while (parser->cursor < parser->doc->source_length) {
        uint8_t byte = parser->doc->source[parser->cursor];
        if (byte != ' ' && byte != '\t' && byte != '\r' && byte != '\n') break;
        parser->cursor++;
    }
}

static bool hex_digit(uint8_t byte, uint32_t *out)
{
    if (byte >= '0' && byte <= '9') *out = (uint32_t)(byte - '0');
    else if (byte >= 'a' && byte <= 'f') *out = (uint32_t)(byte - 'a') + 10U;
    else if (byte >= 'A' && byte <= 'F') *out = (uint32_t)(byte - 'A') + 10U;
    else return false;
    return true;
}

static bool read_hex4(json_parser *parser, uint32_t *out)
{
    if (parser->doc->source_length - parser->cursor < 4U) return false;
    uint32_t value = 0U;
    for (size_t index = 0U; index < 4U; index++) {
        uint32_t digit = 0U;
        if (!hex_digit(parser->doc->source[parser->cursor++], &digit)) return false;
        value = (value << 4U) | digit;
    }
    *out = value;
    return true;
}

static bool utf8_width(const uint8_t *source, size_t length, size_t offset,
                       size_t *out_width)
{
    uint8_t first = source[offset];
    if (first < 0x80U) { *out_width = 1U; return true; }
    size_t width = first >= 0xc2U && first <= 0xdfU ? 2U :
        first >= 0xe0U && first <= 0xefU ? 3U :
        first >= 0xf0U && first <= 0xf4U ? 4U : 0U;
    if (width == 0U || width > length - offset) return false;
    uint8_t second = source[offset + 1U];
    if ((second & 0xc0U) != 0x80U ||
        (first == 0xe0U && second < 0xa0U) ||
        (first == 0xedU && second >= 0xa0U) ||
        (first == 0xf0U && second < 0x90U) ||
        (first == 0xf4U && second >= 0x90U)) return false;
    for (size_t index = 2U; index < width; index++)
        if ((source[offset + index] & 0xc0U) != 0x80U) return false;
    *out_width = width;
    return true;
}

static size_t add_token(mcp_json_doc *doc, mcp_json_type type, size_t start)
{
    if (doc->token_count >= doc->token_capacity) return MCP_JSON_NO_TOKEN;
    size_t index = doc->token_count++;
    doc->tokens[index] = (mcp_json_token){type, start, start,
        MCP_JSON_NO_TOKEN, MCP_JSON_NO_TOKEN, MCP_JSON_NO_TOKEN, 0U};
    return index;
}

static bool append_child(mcp_json_doc *doc, size_t parent, size_t child)
{
    if (parent >= doc->token_count || child >= doc->token_count) return false;
    mcp_json_token *parent_token = &doc->tokens[parent];
    if (parent_token->first_child == MCP_JSON_NO_TOKEN) {
        parent_token->first_child = child;
    } else {
        doc->tokens[parent_token->last_child].next_sibling = child;
    }
    parent_token->last_child = child;
    parent_token->child_count++;
    return true;
}

static bool parse_string(json_parser *parser, size_t parent, size_t *out_token)
{
    const size_t start = parser->cursor;
    if (parser->doc->source[parser->cursor++] != '"') return false;
    while (parser->cursor < parser->doc->source_length) {
        uint8_t byte = parser->doc->source[parser->cursor];
        if (byte == '"') {
            parser->cursor++;
            size_t token = add_token(parser->doc, MCP_JSON_STRING, start);
            if (token == MCP_JSON_NO_TOKEN) return false;
            parser->doc->tokens[token].end = parser->cursor;
            if (parent != MCP_JSON_NO_TOKEN && !append_child(parser->doc, parent, token)) return false;
            *out_token = token;
            return true;
        }
        if (byte < 0x20U) return false;
        if (byte == '\\') {
            parser->cursor++;
            if (parser->cursor >= parser->doc->source_length) return false;
            uint8_t escape = parser->doc->source[parser->cursor++];
            if (escape == '"' || escape == '\\' || escape == '/' || escape == 'b' ||
                escape == 'f' || escape == 'n' || escape == 'r' || escape == 't') continue;
            if (escape != 'u') return false;
            uint32_t codepoint = 0U;
            if (!read_hex4(parser, &codepoint)) return false;
            if (codepoint >= 0xd800U && codepoint <= 0xdbffU) {
                if (parser->doc->source_length - parser->cursor < 6U ||
                    parser->doc->source[parser->cursor] != '\\' ||
                    parser->doc->source[parser->cursor + 1U] != 'u') return false;
                parser->cursor += 2U;
                uint32_t low = 0U;
                if (!read_hex4(parser, &low) || low < 0xdc00U || low > 0xdfffU) return false;
            } else if (codepoint >= 0xdc00U && codepoint <= 0xdfffU) return false;
            continue;
        }
        if (byte >= 0x80U) {
            size_t width = 0U;
            if (!utf8_width(parser->doc->source, parser->doc->source_length,
                            parser->cursor, &width)) return false;
            parser->cursor += width;
        } else parser->cursor++;
    }
    return false;
}

static bool parse_value(json_parser *parser, size_t parent, size_t depth, size_t *out_token);

static bool parse_container(json_parser *parser, size_t parent, size_t depth,
                            bool object, size_t *out_token)
{
    if (depth >= MCP_JSON_MAX_DEPTH) return false;
    const uint8_t opening = object ? '{' : '[';
    const uint8_t closing = object ? '}' : ']';
    if (parser->doc->source[parser->cursor++] != opening) return false;
    size_t container = add_token(parser->doc,
        object ? MCP_JSON_OBJECT : MCP_JSON_ARRAY, parser->cursor - 1U);
    if (container == MCP_JSON_NO_TOKEN) return false;
    if (parent != MCP_JSON_NO_TOKEN && !append_child(parser->doc, parent, container)) return false;
    skip_space(parser);
    if (parser->cursor < parser->doc->source_length &&
        parser->doc->source[parser->cursor] == closing) {
        parser->doc->tokens[container].end = ++parser->cursor;
        *out_token = container;
        return true;
    }
    for (;;) {
        skip_space(parser);
        if (parser->cursor >= parser->doc->source_length) return false;
        if (object) {
            size_t key = MCP_JSON_NO_TOKEN;
            if (parser->doc->source[parser->cursor] != '"' ||
                !parse_string(parser, container, &key)) return false;
            skip_space(parser);
            if (parser->cursor >= parser->doc->source_length ||
                parser->doc->source[parser->cursor++] != ':') return false;
            skip_space(parser);
        }
        size_t child = MCP_JSON_NO_TOKEN;
        if (!parse_value(parser, container, depth + 1U, &child)) return false;
        skip_space(parser);
        if (parser->cursor >= parser->doc->source_length) return false;
        uint8_t delimiter = parser->doc->source[parser->cursor++];
        if (delimiter == closing) {
            parser->doc->tokens[container].end = parser->cursor;
            *out_token = container;
            return true;
        }
        if (delimiter != ',') return false;
        skip_space(parser);
        if (parser->cursor >= parser->doc->source_length ||
            parser->doc->source[parser->cursor] == closing) return false;
    }
}

static bool parse_number(json_parser *parser, size_t parent, size_t *out_token)
{
    size_t start = parser->cursor;
    if (parser->doc->source[parser->cursor] == '-') parser->cursor++;
    if (parser->cursor >= parser->doc->source_length) return false;
    if (parser->doc->source[parser->cursor] == '0') parser->cursor++;
    else {
        if (parser->doc->source[parser->cursor] < '1' || parser->doc->source[parser->cursor] > '9') return false;
        while (parser->cursor < parser->doc->source_length &&
               parser->doc->source[parser->cursor] >= '0' &&
               parser->doc->source[parser->cursor] <= '9') parser->cursor++;
    }
    if (parser->cursor < parser->doc->source_length && parser->doc->source[parser->cursor] == '.') {
        parser->cursor++;
        size_t first_fraction = parser->cursor;
        while (parser->cursor < parser->doc->source_length &&
               parser->doc->source[parser->cursor] >= '0' &&
               parser->doc->source[parser->cursor] <= '9') parser->cursor++;
        if (parser->cursor == first_fraction) return false;
    }
    if (parser->cursor < parser->doc->source_length &&
        (parser->doc->source[parser->cursor] == 'e' || parser->doc->source[parser->cursor] == 'E')) {
        parser->cursor++;
        if (parser->cursor < parser->doc->source_length &&
            (parser->doc->source[parser->cursor] == '+' || parser->doc->source[parser->cursor] == '-')) parser->cursor++;
        size_t first_exponent = parser->cursor;
        while (parser->cursor < parser->doc->source_length &&
               parser->doc->source[parser->cursor] >= '0' &&
               parser->doc->source[parser->cursor] <= '9') parser->cursor++;
        if (parser->cursor == first_exponent) return false;
    }
    size_t token = add_token(parser->doc, MCP_JSON_NUMBER, start);
    if (token == MCP_JSON_NO_TOKEN) return false;
    parser->doc->tokens[token].end = parser->cursor;
    if (parent != MCP_JSON_NO_TOKEN && !append_child(parser->doc, parent, token)) return false;
    *out_token = token;
    return true;
}

static bool parse_value(json_parser *parser, size_t parent, size_t depth, size_t *out_token)
{
    skip_space(parser);
    if (parser->cursor >= parser->doc->source_length) return false;
    uint8_t byte = parser->doc->source[parser->cursor];
    if (byte == '{') return parse_container(parser, parent, depth, true, out_token);
    if (byte == '[') return parse_container(parser, parent, depth, false, out_token);
    if (byte == '"') return parse_string(parser, parent, out_token);
    if (byte == '-' || (byte >= '0' && byte <= '9')) return parse_number(parser, parent, out_token);
    const char *literal = byte == 't' ? "true" : byte == 'f' ? "false" : byte == 'n' ? "null" : NULL;
    if (literal == NULL) return false;
    size_t length = strlen(literal);
    if (length > parser->doc->source_length - parser->cursor ||
        memcmp(parser->doc->source + parser->cursor, literal, length) != 0) return false;
    mcp_json_type type = byte == 't' ? MCP_JSON_TRUE : byte == 'f' ? MCP_JSON_FALSE : MCP_JSON_NULL;
    size_t start = parser->cursor;
    parser->cursor += length;
    size_t token = add_token(parser->doc, type, start);
    if (token == MCP_JSON_NO_TOKEN) return false;
    parser->doc->tokens[token].end = parser->cursor;
    if (parent != MCP_JSON_NO_TOKEN && !append_child(parser->doc, parent, token)) return false;
    *out_token = token;
    return true;
}

bool mcp_json_parse(const uint8_t *source, size_t length, mcp_json_doc *out)
{
    if (out == NULL || source == NULL || length == 0U || length > MCP_JSON_MAX_BYTES) return false;
    *out = (mcp_json_doc){0};
    size_t capacity = length + 1U;
    if (capacity > MCP_JSON_MAX_TOKENS) capacity = MCP_JSON_MAX_TOKENS;
    out->tokens = calloc(capacity, sizeof(*out->tokens));
    if (out->tokens == NULL) return false;
    out->source = source;
    out->source_length = length;
    out->token_capacity = capacity;
    json_parser parser = {out, 0U};
    size_t root = MCP_JSON_NO_TOKEN;
    bool okay = parse_value(&parser, MCP_JSON_NO_TOKEN, 0U, &root);
    skip_space(&parser);
    if (!okay || parser.cursor != length || root == MCP_JSON_NO_TOKEN) {
        mcp_json_doc_destroy(out);
        return false;
    }
    out->root = root;
    return true;
}

void mcp_json_doc_destroy(mcp_json_doc *doc)
{
    if (doc == NULL) return;
    free(doc->tokens);
    *doc = (mcp_json_doc){0};
}

size_t mcp_json_object_get(const mcp_json_doc *doc, size_t object, const char *key)
{
    if (doc == NULL || key == NULL || object >= doc->token_count ||
        doc->tokens[object].type != MCP_JSON_OBJECT) return MCP_JSON_NO_TOKEN;
    size_t token = doc->tokens[object].first_child;
    while (token != MCP_JSON_NO_TOKEN) {
        size_t value = doc->tokens[token].next_sibling;
        if (value == MCP_JSON_NO_TOKEN) return MCP_JSON_NO_TOKEN;
        if (mcp_json_string_equals(doc, token, key)) return value;
        token = doc->tokens[value].next_sibling;
    }
    return MCP_JSON_NO_TOKEN;
}

size_t mcp_json_array_get(const mcp_json_doc *doc, size_t array, size_t index)
{
    if (doc == NULL || array >= doc->token_count || doc->tokens[array].type != MCP_JSON_ARRAY)
        return MCP_JSON_NO_TOKEN;
    size_t token = doc->tokens[array].first_child;
    while (token != MCP_JSON_NO_TOKEN && index > 0U) {
        token = doc->tokens[token].next_sibling;
        index--;
    }
    return token;
}

static bool copy_codepoint(uint32_t codepoint, char *out, size_t capacity, size_t *cursor)
{
    uint8_t encoded[4];
    size_t count;
    if (codepoint <= 0x7fU) { encoded[0] = (uint8_t)codepoint; count = 1U; }
    else if (codepoint <= 0x7ffU) {
        encoded[0] = (uint8_t)(0xc0U | (codepoint >> 6U));
        encoded[1] = (uint8_t)(0x80U | (codepoint & 0x3fU)); count = 2U;
    } else if (codepoint <= 0xffffU) {
        encoded[0] = (uint8_t)(0xe0U | (codepoint >> 12U));
        encoded[1] = (uint8_t)(0x80U | ((codepoint >> 6U) & 0x3fU));
        encoded[2] = (uint8_t)(0x80U | (codepoint & 0x3fU)); count = 3U;
    } else if (codepoint <= 0x10ffffU) {
        encoded[0] = (uint8_t)(0xf0U | (codepoint >> 18U));
        encoded[1] = (uint8_t)(0x80U | ((codepoint >> 12U) & 0x3fU));
        encoded[2] = (uint8_t)(0x80U | ((codepoint >> 6U) & 0x3fU));
        encoded[3] = (uint8_t)(0x80U | (codepoint & 0x3fU)); count = 4U;
    } else return false;
    if (count > capacity - *cursor) return false;
    memcpy(out + *cursor, encoded, count);
    *cursor += count;
    return true;
}

bool mcp_json_string_copy(const mcp_json_doc *doc, size_t token, char *out,
                          size_t capacity, size_t *out_length)
{
    if (doc == NULL || out == NULL || token >= doc->token_count ||
        doc->tokens[token].type != MCP_JSON_STRING || out_length == NULL) return false;
    size_t cursor = doc->tokens[token].start + 1U;
    size_t end = doc->tokens[token].end - 1U;
    size_t written = 0U;
    while (cursor < end) {
        uint8_t byte = doc->source[cursor++];
        if (byte != '\\') {
            if (written >= capacity) return false;
            out[written++] = (char)byte;
            continue;
        }
        if (cursor >= end) return false;
        uint8_t escape = doc->source[cursor++];
        char simple = escape == '"' ? '"' : escape == '\\' ? '\\' : escape == '/' ? '/' :
            escape == 'b' ? '\b' : escape == 'f' ? '\f' : escape == 'n' ? '\n' :
            escape == 'r' ? '\r' : escape == 't' ? '\t' : '\0';
        if (simple != '\0') {
            if (written >= capacity) return false;
            out[written++] = simple;
            continue;
        }
        if (escape != 'u' || end - cursor < 4U) return false;
        uint32_t codepoint = 0U;
        for (size_t index = 0U; index < 4U; index++) {
            uint32_t digit = 0U;
            if (!hex_digit(doc->source[cursor++], &digit)) return false;
            codepoint = (codepoint << 4U) | digit;
        }
        if (codepoint >= 0xd800U && codepoint <= 0xdbffU) {
            if (end - cursor < 6U || doc->source[cursor] != '\\' || doc->source[cursor + 1U] != 'u') return false;
            cursor += 2U;
            uint32_t low = 0U;
            for (size_t index = 0U; index < 4U; index++) {
                uint32_t digit = 0U;
                if (!hex_digit(doc->source[cursor++], &digit)) return false;
                low = (low << 4U) | digit;
            }
            if (low < 0xdc00U || low > 0xdfffU) return false;
            codepoint = 0x10000U + ((codepoint - 0xd800U) << 10U) + (low - 0xdc00U);
        }
        if (!copy_codepoint(codepoint, out, capacity, &written)) return false;
    }
    *out_length = written;
    return true;
}

bool mcp_json_string_equals(const mcp_json_doc *doc, size_t token, const char *literal)
{
    if (doc == NULL || literal == NULL || token >= doc->token_count ||
        doc->tokens[token].type != MCP_JSON_STRING) return false;
    char decoded[128];
    size_t length = 0U;
    size_t expected = strlen(literal);
    return expected < sizeof(decoded) &&
        mcp_json_string_copy(doc, token, decoded, sizeof(decoded), &length) &&
        length == expected && memcmp(decoded, literal, expected) == 0;
}

bool mcp_json_token_copy(const mcp_json_doc *doc, size_t token, char *out,
                         size_t capacity, size_t *out_length)
{
    if (doc == NULL || out == NULL || out_length == NULL || token >= doc->token_count) return false;
    const mcp_json_token *item = &doc->tokens[token];
    size_t length = item->end - item->start;
    if (length > capacity) return false;
    memcpy(out, doc->source + item->start, length);
    *out_length = length;
    return true;
}

bool mcp_json_uint(const mcp_json_doc *doc, size_t token, uint64_t *out)
{
    if (doc == NULL || out == NULL || token >= doc->token_count ||
        doc->tokens[token].type != MCP_JSON_NUMBER) return false;
    const mcp_json_token *item = &doc->tokens[token];
    uint64_t value = 0U;
    if (item->start == item->end) return false;
    for (size_t index = item->start; index < item->end; index++) {
        uint8_t byte = doc->source[index];
        if (byte < '0' || byte > '9' || value > (UINT64_MAX - (uint64_t)(byte - '0')) / 10U) return false;
        value = value * 10U + (uint64_t)(byte - '0');
    }
    *out = value;
    return true;
}

bool mcp_json_bool(const mcp_json_doc *doc, size_t token, bool *out)
{
    if (doc == NULL || out == NULL || token >= doc->token_count) return false;
    if (doc->tokens[token].type == MCP_JSON_TRUE) { *out = true; return true; }
    if (doc->tokens[token].type == MCP_JSON_FALSE) { *out = false; return true; }
    return false;
}

bool mcp_json_buffer_init(mcp_json_buffer *buffer, size_t initial, size_t limit)
{
    if (buffer == NULL || limit == 0U || initial > limit) return false;
    *buffer = (mcp_json_buffer){0};
    buffer->limit = limit;
    if (initial == 0U) return true;
    if (initial == SIZE_MAX) return false;
    buffer->data = malloc(initial + 1U);
    if (buffer->data == NULL) return false;
    buffer->capacity = initial;
    buffer->data[0] = '\0';
    return true;
}

void mcp_json_buffer_destroy(mcp_json_buffer *buffer)
{
    if (buffer == NULL) return;
    free(buffer->data);
    *buffer = (mcp_json_buffer){0};
}

bool mcp_json_buffer_append(mcp_json_buffer *buffer, const char *data, size_t length)
{
    if (buffer == NULL || (length > 0U && data == NULL) || length > buffer->limit - buffer->length) return false;
    size_t needed = buffer->length + length;
    if (needed > buffer->capacity) {
        size_t capacity = buffer->capacity == 0U ? 256U : buffer->capacity;
        while (capacity < needed) {
            if (capacity > buffer->limit / 2U) { capacity = buffer->limit; break; }
            capacity *= 2U;
        }
        if (capacity == SIZE_MAX) return false;
        /* The source may alias our own storage: remember its offset across realloc. */
        bool aliased = length > 0U && buffer->data != NULL && data >= buffer->data &&
                       data < buffer->data + buffer->capacity + 1U;
        size_t source_offset = aliased ? (size_t)(data - buffer->data) : 0U;
        char *grown = realloc(buffer->data, capacity + 1U);
        if (grown == NULL) return false;
        buffer->data = grown;
        buffer->capacity = capacity;
        if (aliased) data = buffer->data + source_offset;
    }
    if (length > 0U) memmove(buffer->data + buffer->length, data, length);
    buffer->length = needed;
    if (buffer->data != NULL) buffer->data[buffer->length] = '\0';
    return true;
}

bool mcp_json_buffer_literal(mcp_json_buffer *buffer, const char *literal)
{
    return literal != NULL && mcp_json_buffer_append(buffer, literal, strlen(literal));
}

bool mcp_json_buffer_quoted(mcp_json_buffer *buffer, const char *data, size_t length)
{
    if (buffer == NULL || (length > 0U && data == NULL) || !mcp_json_buffer_literal(buffer, "\"")) return false;
    static const char hex[] = "0123456789abcdef";
    for (size_t index = 0U; index < length; index++) {
        uint8_t byte = (uint8_t)data[index];
        char escape[6];
        size_t amount = 0U;
        if (byte == '"' || byte == '\\') { escape[0] = '\\'; escape[1] = (char)byte; amount = 2U; }
        else if (byte == '\b') { escape[0] = '\\'; escape[1] = 'b'; amount = 2U; }
        else if (byte == '\f') { escape[0] = '\\'; escape[1] = 'f'; amount = 2U; }
        else if (byte == '\n') { escape[0] = '\\'; escape[1] = 'n'; amount = 2U; }
        else if (byte == '\r') { escape[0] = '\\'; escape[1] = 'r'; amount = 2U; }
        else if (byte == '\t') { escape[0] = '\\'; escape[1] = 't'; amount = 2U; }
        else if (byte < 0x20U) {
            escape[0] = '\\'; escape[1] = 'u'; escape[2] = '0'; escape[3] = '0';
            escape[4] = hex[byte >> 4U]; escape[5] = hex[byte & 0x0fU]; amount = 6U;
        } else { escape[0] = (char)byte; amount = 1U; }
        if (!mcp_json_buffer_append(buffer, escape, amount)) return false;
    }
    return mcp_json_buffer_literal(buffer, "\"");
}

bool mcp_json_buffer_raw_token(mcp_json_buffer *buffer, const mcp_json_doc *doc,
                               size_t token)
{
    if (doc == NULL || token >= doc->token_count) return false;
    const mcp_json_token *item = &doc->tokens[token];
    return item->end >= item->start && mcp_json_buffer_append(buffer,
        (const char *)doc->source + item->start, item->end - item->start);
}
