#include "index.h"

#include <stdlib.h>
#include <string.h>

typedef enum scan_mode { SCAN_TEXT, SCAN_TAG, SCAN_COMMENT, SCAN_CDATA, SCAN_PI } scan_mode;
typedef struct open_element { size_t record; size_t name_offset; size_t name_length; } open_element;

struct svg_index_session {
    base_bytes source;
    svg_index_element *elements;
    size_t element_count;
    size_t element_capacity;
    size_t attribute_count;
    size_t bytes_read;
    size_t token_start;
    size_t tag_name_offset;
    size_t tag_name_length;
    size_t root_open_length;
    size_t preview_record;
    size_t depth;
    size_t root_count;
    uint64_t generation;
    scan_mode mode;
    bool closing_tag;
    bool self_closing;
    bool in_quote;
    uint8_t quote;
    bool canceled;
    bool complete;
    bool preview;
    svg_xml_diagnostic diagnostic;
    open_element stack[SVG_INDEX_MAX_DEPTH];
};

static void set_error(svg_index_session *s, svg_xml_error code, size_t at)
{
    if (s->diagnostic.category == SVG_XML_ERROR_NONE) {
        s->diagnostic.category = code;
        s->diagnostic.offset = at;
    }
}

static bool name_byte(uint8_t c)
{
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
           (c >= '0' && c <= '9') || c == '_' || c == '-' || c == '.' || c == ':';
}

static bool equal_name(const svg_index_session *s, size_t at, size_t n, const char *text)
{
    size_t length = strlen(text);
    return n == length && memcmp(s->source.ptr + at, text, n) == 0;
}

static bool reserve_element(svg_index_session *s)
{
    if (s->element_count >= SVG_INDEX_MAX_ELEMENTS) return false;
    if (s->element_count < s->element_capacity) return true;
    size_t cap = s->element_capacity == 0 ? 256U : s->element_capacity * 2U;
    if (cap > SVG_INDEX_MAX_ELEMENTS) cap = SVG_INDEX_MAX_ELEMENTS;
    if (cap > SVG_INDEX_MAX_STORAGE_BYTES / sizeof(*s->elements)) return false;
    void *next = realloc(s->elements, cap * sizeof(*s->elements));
    if (next == NULL) return false;
    s->elements = next;
    s->element_capacity = cap;
    return true;
}

static void finish_open_tag(svg_index_session *s, size_t end)
{
    if (s->closing_tag) {
        if (s->depth == 0) { set_error(s, SVG_XML_ERROR_MISMATCHED_TAG, s->token_start); return; }
        open_element top = s->stack[s->depth - 1U];
        if (top.name_length != s->tag_name_length ||
            memcmp(s->source.ptr + top.name_offset, s->source.ptr + s->tag_name_offset,
                   s->tag_name_length) != 0) {
            set_error(s, SVG_XML_ERROR_MISMATCHED_TAG, s->tag_name_offset); return;
        }
        --s->depth;
        s->elements[top.record].source_length = end - s->elements[top.record].source_offset;
        s->elements[top.record].closed = true;
        if (s->depth == 1U && s->elements[top.record].support != SVG_INDEX_UNSUPPORTED)
            { s->preview = true; s->preview_record = top.record; }
        return;
    }
    if (s->tag_name_length == 0U || s->tag_name_length == 4096U) {
        set_error(s, SVG_XML_ERROR_INVALID_NAME, s->tag_name_offset);
        return;
    }
    if (s->depth >= SVG_INDEX_MAX_DEPTH) { set_error(s, SVG_XML_ERROR_DEPTH_LIMIT, s->token_start); return; }
    if (s->depth == 0 && ++s->root_count > 1U) {
        set_error(s, SVG_XML_ERROR_MULTIPLE_ROOTS, s->token_start); return;
    }
    if (!reserve_element(s)) { set_error(s, SVG_XML_ERROR_NODE_LIMIT, s->token_start); return; }
    size_t i = s->element_count++;
    svg_index_element *e = &s->elements[i];
    e->parent = s->depth == 0 ? SIZE_MAX : s->stack[s->depth - 1U].record;
    e->source_offset = s->token_start;
    e->source_length = end - s->token_start;
    e->name_offset = s->tag_name_offset;
    e->name_length = s->tag_name_length;
    e->depth = s->depth;
    e->support = equal_name(s, e->name_offset, e->name_length, "rect") ? SVG_INDEX_SUPPORTED_RECT :
                 equal_name(s, e->name_offset, e->name_length, "path") ? SVG_INDEX_SUPPORTED_PATH : SVG_INDEX_UNSUPPORTED;
    e->closed = s->self_closing;
    if (s->depth == 0U) s->root_open_length = end - s->token_start;
    if (!s->preview && s->depth == 1U && s->self_closing &&
        e->support != SVG_INDEX_UNSUPPORTED) {
        s->preview = true; s->preview_record = i;
    }
    if (!s->self_closing) {
        s->stack[s->depth].record = i;
        s->stack[s->depth].name_offset = s->tag_name_offset;
        s->stack[s->depth].name_length = s->tag_name_length;
        ++s->depth;
    }
}

static void validate_small_document(svg_index_session *s)
{
    if (s->source.len > 64U * 1024U || s->diagnostic.category != SVG_XML_ERROR_NONE) return;
    size_t capacity = s->source.len + 1U;
    if (capacity > SIZE_MAX / sizeof(svg_xml_token)) {
        set_error(s, SVG_XML_ERROR_OUTPUT_EXHAUSTED, 0U);
        return;
    }
    svg_xml_token *tokens = calloc(capacity, sizeof(*tokens));
    if (tokens == NULL) { set_error(s, SVG_XML_ERROR_OUTPUT_EXHAUSTED, 0U); return; }
    svg_xml_result result = svg_xml_tokenize(s->source, svg_xml_default_options(),
                                              tokens, capacity);
    free(tokens);
    if (result.diagnostic.category != SVG_XML_ERROR_NONE)
        s->diagnostic = result.diagnostic;
    else if (result.element_count != s->element_count) {
        set_error(s, SVG_XML_ERROR_MALFORMED_SYNTAX, 0U);
    }
}

svg_index_status svg_index_start(base_bytes source, uint64_t generation,
                                 svg_index_session **out_session)
{
    if (out_session == NULL) return SVG_INDEX_INVALID;
    *out_session = NULL;
    if (source.len > SVG_INDEX_MAX_SOURCE_BYTES) return SVG_INDEX_LIMIT;
    if (!base_bytes_valid(source)) return SVG_INDEX_INVALID;
    svg_index_session *s = calloc(1, sizeof(*s));
    if (s == NULL) return SVG_INDEX_OOM;
    s->source = source;
    s->generation = generation;
    s->mode = SCAN_TEXT;
    *out_session = s;
    return SVG_INDEX_OK;
}

static void begin_tag(svg_index_session *s, size_t at)
{
    s->mode = SCAN_TAG;
    s->token_start = at;
    s->closing_tag = false;
    s->self_closing = false;
    s->in_quote = false;
    s->tag_name_offset = at + 1U;
    if (s->tag_name_offset < s->source.len && s->source.ptr[s->tag_name_offset] == '/') {
        s->closing_tag = true;
        ++s->tag_name_offset;
    }
    size_t p = s->tag_name_offset;
    while (p < s->source.len && p - s->tag_name_offset < 4096U &&
           name_byte(s->source.ptr[p])) ++p;
    s->tag_name_length = p - s->tag_name_offset;
}

svg_index_status svg_index_step(svg_index_session *s, size_t byte_budget)
{
    if (s == NULL || byte_budget == 0) return SVG_INDEX_INVALID;
    if (s->canceled) return SVG_INDEX_CANCELED;
    if (s->complete) return SVG_INDEX_COMPLETE;
    if (byte_budget > SVG_INDEX_MAX_STEP_BYTES) byte_budget = SVG_INDEX_MAX_STEP_BYTES;
    size_t end = s->bytes_read + byte_budget;
    if (end < s->bytes_read || end > s->source.len) end = s->source.len;
    while (s->bytes_read < end && s->diagnostic.category == SVG_XML_ERROR_NONE) {
        size_t at = s->bytes_read;
        uint8_t c = s->source.ptr[at];
        if (s->mode == SCAN_TEXT) {
            if (c == '<') {
                if (at + 4U <= s->source.len && memcmp(s->source.ptr + at, "<!--", 4U) == 0) {
                    s->mode = SCAN_COMMENT; s->token_start = at;
                }
                if (at + 9U <= s->source.len && memcmp(s->source.ptr + at, "<![CDATA[", 9U) == 0) {
                    s->mode = SCAN_CDATA; s->token_start = at;
                }
                if (at + 2U <= s->source.len && s->source.ptr[at + 1U] == '?') {
                    s->mode = SCAN_PI; s->token_start = at;
                }
                if (s->mode == SCAN_TEXT && at + 2U <= s->source.len && s->source.ptr[at + 1U] == '!') {
                    set_error(s, SVG_XML_ERROR_UNSUPPORTED_DTD, at); break;
                }
                if (s->mode == SCAN_TEXT) begin_tag(s, at);
            }
        } else if (s->mode == SCAN_TAG) {
            if (s->in_quote) {
                if (c == s->quote) s->in_quote = false;
            } else if (c == '\'' || c == '"') {
                s->in_quote = true; s->quote = c;
                if (s->attribute_count >= SVG_INDEX_MAX_ATTRIBUTES)
                    set_error(s, SVG_XML_ERROR_ATTRIBUTE_LIMIT, at);
                else ++s->attribute_count;
            } else if (c == '/' && at + 1U < s->source.len && s->source.ptr[at + 1U] == '>') {
                s->self_closing = true;
            } else if (c == '>') {
                finish_open_tag(s, at + 1U);
                s->mode = SCAN_TEXT;
            } else if ((c == '\'' || c == '"') && s->attribute_count < SVG_INDEX_MAX_ATTRIBUTES) {
                ++s->attribute_count;
            }
        } else if (s->mode == SCAN_COMMENT && c == '>' && at >= 2U &&
                   s->source.ptr[at - 1U] == '-' && s->source.ptr[at - 2U] == '-') {
            s->mode = SCAN_TEXT;
        } else if (s->mode == SCAN_CDATA && c == '>' && at >= 2U &&
                   s->source.ptr[at - 1U] == ']' && s->source.ptr[at - 2U] == ']') {
            s->mode = SCAN_TEXT;
        } else if (s->mode == SCAN_PI && c == '>' && at > 0 && s->source.ptr[at - 1U] == '?') {
            s->mode = SCAN_TEXT;
        }
        ++s->bytes_read;
    }
    if (s->diagnostic.category != SVG_XML_ERROR_NONE) return SVG_INDEX_MALFORMED;
    if (s->bytes_read == s->source.len) {
        if (s->mode != SCAN_TEXT || s->depth != 0 || s->root_count != 1U)
            set_error(s, SVG_XML_ERROR_INCOMPLETE_DOCUMENT, s->source.len);
        else {
            validate_small_document(s);
            s->complete = s->diagnostic.category == SVG_XML_ERROR_NONE;
        }
    }
    return s->diagnostic.category == SVG_XML_ERROR_NONE ?
           (s->complete ? SVG_INDEX_COMPLETE : SVG_INDEX_OK) : SVG_INDEX_MALFORMED;
}

svg_index_progress svg_index_get_progress(const svg_index_session *s)
{
    svg_index_progress p = {0};
    if (s == NULL) return p;
    p.bytes_read = s->bytes_read; p.source_bytes = s->source.len;
    p.element_count = s->element_count; p.attribute_count = s->attribute_count;
    p.generation = s->generation; p.diagnostic = s->diagnostic;
    p.complete = s->complete; p.canceled = s->canceled; p.first_preview_ready = s->preview;
    return p;
}

svg_index_status svg_index_advance_generation(svg_index_session *s, uint64_t generation)
{
    if (s == NULL || generation <= s->generation) return SVG_INDEX_INVALID;
    s->generation = generation; s->preview = false;
    return SVG_INDEX_OK;
}

void svg_index_cancel(svg_index_session *s) { if (s != NULL) s->canceled = true; }

bool svg_index_element_at(const svg_index_session *s, size_t index, svg_index_element *out)
{
    if (s == NULL || out == NULL || index >= s->element_count) return false;
    *out = s->elements[index]; return true;
}

bool svg_index_make_preview_source(const svg_index_session *s, base_mut_bytes *out)
{
    if (out == NULL) return false;
    *out = (base_mut_bytes){NULL, 0U};
    if (s == NULL || !s->preview || s->root_open_length == 0U ||
        s->preview_record >= s->element_count) return false;
    const svg_index_element *root = &s->elements[0];
    const svg_index_element *shape = &s->elements[s->preview_record];
    if (root->name_length != 3U || memcmp(s->source.ptr + root->name_offset, "svg", 3U) != 0 ||
        shape->source_length == 0U) return false;
    size_t total = 0U;
    if (!base_size_add(s->root_open_length, shape->source_length, &total) ||
        !base_size_add(total, 6U, &total) || total > 16U * 1024U * 1024U) return false;
    uint8_t *memory = malloc(total);
    if (memory == NULL) return false;
    memcpy(memory, s->source.ptr + root->source_offset, s->root_open_length);
    memcpy(memory + s->root_open_length, s->source.ptr + shape->source_offset,
           shape->source_length);
    memcpy(memory + total - 6U, "</svg>", 6U);
    *out = (base_mut_bytes){memory, total};
    return true;
}

void svg_index_destroy(svg_index_session *s)
{
    if (s == NULL) return;
    free(s->elements); free(s);
}
