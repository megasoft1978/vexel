#include "xml_reader.h"

#include <stdlib.h>
#include <string.h>

#define XML_READER_MAX_SOURCE_BYTES (1024U * 1024U * 1024U)
#define XML_READER_MAX_STEP_BYTES (64U * 1024U)

typedef enum reader_mode { MODE_TEXT, MODE_PROBE, MODE_TAG, MODE_COMMENT, MODE_CDATA, MODE_PI } reader_mode;
typedef enum probe_kind { PROBE_NONE, PROBE_COMMENT, PROBE_CDATA } probe_kind;

struct svg_xml_reader {
    base_bytes source;
    size_t bytes_read;
    size_t token_start;
    size_t name_offset;
    size_t name_length;
    size_t attribute_count;
    size_t probe_position;
    reader_mode mode;
    probe_kind probe;
    bool closing;
    bool name_complete;
    bool in_quote;
    bool pending_slash;
    bool self_closing;
    uint8_t quote;
    bool complete;
    svg_xml_diagnostic diagnostic;
};

static bool is_name_byte(uint8_t c)
{
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
           (c >= '0' && c <= '9') || c >= 0x80U || c == '_' || c == '-' ||
           c == '.' || c == ':';
}

static void reader_fail(svg_xml_reader *r, svg_xml_error error, size_t offset)
{
    if (r->diagnostic.category == SVG_XML_ERROR_NONE) {
        r->diagnostic.category = error;
        r->diagnostic.offset = offset <= r->source.len ? offset : r->source.len;
    }
}

static bool emit_tag(svg_xml_reader *r, size_t end, svg_xml_reader_tag_fn callback,
                     void *context)
{
    if (r->name_length == 0U || r->name_length > 4096U) {
        reader_fail(r, SVG_XML_ERROR_INVALID_NAME, r->name_offset);
        return false;
    }
    if (callback != NULL) {
        svg_xml_reader_tag tag = {
            r->closing ? SVG_XML_TOKEN_END_ELEMENT :
            r->self_closing ? SVG_XML_TOKEN_EMPTY_ELEMENT : SVG_XML_TOKEN_START_ELEMENT,
            {r->token_start, end - r->token_start},
            {r->name_offset, r->name_length}, r->attribute_count
        };
        if (!callback(context, &tag)) {
            reader_fail(r, SVG_XML_ERROR_OUTPUT_EXHAUSTED, r->token_start);
            return false;
        }
    }
    r->mode = MODE_TEXT;
    return true;
}

svg_xml_reader_status svg_xml_reader_start(base_bytes source, svg_xml_reader **out)
{
    if (out == NULL) return SVG_XML_READER_INVALID;
    *out = NULL;
    if (source.len > XML_READER_MAX_SOURCE_BYTES) return SVG_XML_READER_LIMIT;
    if (!base_bytes_valid(source)) return SVG_XML_READER_INVALID;
    svg_xml_reader *r = calloc(1U, sizeof(*r));
    if (r == NULL) return SVG_XML_READER_INVALID;
    r->source = source;
    r->mode = MODE_TEXT;
    *out = r;
    return SVG_XML_READER_OK;
}

svg_xml_reader_status svg_xml_reader_step(svg_xml_reader *r, size_t byte_budget,
    svg_xml_reader_tag_fn callback, void *context)
{
    if (r == NULL || byte_budget == 0U) return SVG_XML_READER_INVALID;
    if (r->diagnostic.category != SVG_XML_ERROR_NONE) return SVG_XML_READER_MALFORMED;
    if (r->complete) return SVG_XML_READER_COMPLETE;
    if (byte_budget > XML_READER_MAX_STEP_BYTES) byte_budget = XML_READER_MAX_STEP_BYTES;
    size_t end = r->bytes_read + byte_budget;
    if (end < r->bytes_read || end > r->source.len) end = r->source.len;
    while (r->bytes_read < end && r->diagnostic.category == SVG_XML_ERROR_NONE) {
        size_t at = r->bytes_read;
        uint8_t c = r->source.ptr[at];
        if (r->mode == MODE_TEXT) {
            if (c == '<') {
                r->mode = MODE_PROBE;
                r->token_start = at;
                r->probe = PROBE_NONE;
                r->probe_position = 0U;
            }
        } else if (r->mode == MODE_PROBE) {
            if (r->probe_position == 0U) {
                if (c == '?') r->mode = MODE_PI;
                else if (c == '/') {
                    r->mode = MODE_TAG; r->closing = true;
                    r->name_offset = at + 1U; r->name_length = 0U;
                    r->attribute_count = 0U;
                    r->name_complete = false; r->pending_slash = false;
                } else if (c == '!') r->probe_position = 1U;
                else if (is_name_byte(c)) {
                    r->mode = MODE_TAG; r->closing = false;
                    r->name_offset = at; r->name_length = 1U;
                    r->attribute_count = 0U;
                    r->name_complete = false; r->pending_slash = false;
                } else reader_fail(r, SVG_XML_ERROR_INVALID_NAME, at);
            } else if (r->probe == PROBE_NONE) {
                if (c == '-') { r->probe = PROBE_COMMENT; r->probe_position = 1U; }
                else if (c == '[') { r->probe = PROBE_CDATA; r->probe_position = 1U; }
                else reader_fail(r, SVG_XML_ERROR_UNSUPPORTED_DTD, r->token_start);
            } else if (r->probe == PROBE_COMMENT) {
                if (c != '-') reader_fail(r, SVG_XML_ERROR_MALFORMED_SYNTAX, r->token_start);
                else r->mode = MODE_COMMENT;
            } else {
                static const char tail[] = "CDATA[";
                if (r->probe_position > sizeof(tail) - 1U ||
                    c != (uint8_t)tail[r->probe_position - 1U])
                    reader_fail(r, SVG_XML_ERROR_MALFORMED_SYNTAX, r->token_start);
                else if (r->probe_position == sizeof(tail) - 1U) r->mode = MODE_CDATA;
                else ++r->probe_position;
            }
        } else if (r->mode == MODE_TAG) {
            if (!r->name_complete) {
                if (is_name_byte(c)) {
                    if (r->name_length == 4096U) reader_fail(r, SVG_XML_ERROR_INVALID_NAME, r->name_offset);
                    else ++r->name_length;
                    ++r->bytes_read;
                    continue;
                }
                if (r->name_length == 0U) reader_fail(r, SVG_XML_ERROR_INVALID_NAME, r->name_offset);
                else r->name_complete = true;
            }
            if (r->in_quote) {
                if (c == r->quote) r->in_quote = false;
            } else if (c == '\'' || c == '"') {
                r->in_quote = true; r->quote = c;
                ++r->attribute_count;
            } else if (c == '/') {
                r->pending_slash = true;
            } else if (c == '>') {
                r->self_closing = r->pending_slash;
                if (!emit_tag(r, at + 1U, callback, context)) break;
            } else r->pending_slash = false;
        } else if (r->mode == MODE_COMMENT && c == '>' && at >= 2U &&
                   r->source.ptr[at - 1U] == '-' && r->source.ptr[at - 2U] == '-') {
            r->mode = MODE_TEXT;
        } else if (r->mode == MODE_CDATA && c == '>' && at >= 2U &&
                   r->source.ptr[at - 1U] == ']' && r->source.ptr[at - 2U] == ']') {
            r->mode = MODE_TEXT;
        } else if (r->mode == MODE_PI && c == '>' && at > 0U && r->source.ptr[at - 1U] == '?') {
            r->mode = MODE_TEXT;
        }
        ++r->bytes_read;
    }
    if (r->diagnostic.category != SVG_XML_ERROR_NONE) return SVG_XML_READER_MALFORMED;
    if (r->bytes_read == r->source.len) {
        if (r->mode != MODE_TEXT) reader_fail(r, SVG_XML_ERROR_INCOMPLETE_DOCUMENT, r->source.len);
        else r->complete = true;
    }
    return r->diagnostic.category == SVG_XML_ERROR_NONE ?
        (r->complete ? SVG_XML_READER_COMPLETE : SVG_XML_READER_OK) : SVG_XML_READER_MALFORMED;
}

svg_xml_reader_progress svg_xml_reader_get_progress(const svg_xml_reader *r)
{
    svg_xml_reader_progress p = {0};
    if (r == NULL) return p;
    p.bytes_read = r->bytes_read; p.source_bytes = r->source.len;
    p.diagnostic = r->diagnostic; p.complete = r->complete;
    return p;
}

void svg_xml_reader_destroy(svg_xml_reader *r) { free(r); }
