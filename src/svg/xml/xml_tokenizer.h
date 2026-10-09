#ifndef VEXEL_XML_TOKENIZER_H
#define VEXEL_XML_TOKENIZER_H

#include "base.h"

#include <stddef.h>
#include <stdint.h>

typedef enum svg_xml_token_kind {
    SVG_XML_TOKEN_DECLARATION = 1,
    SVG_XML_TOKEN_PROCESSING_INSTRUCTION,
    SVG_XML_TOKEN_COMMENT,
    SVG_XML_TOKEN_START_ELEMENT,
    SVG_XML_TOKEN_END_ELEMENT,
    SVG_XML_TOKEN_EMPTY_ELEMENT,
    SVG_XML_TOKEN_ATTRIBUTE,
    SVG_XML_TOKEN_TEXT,
    SVG_XML_TOKEN_CDATA
} svg_xml_token_kind;

typedef enum svg_xml_error {
    SVG_XML_ERROR_NONE = 0,
    SVG_XML_ERROR_INVALID_ARGUMENT,
    SVG_XML_ERROR_INPUT_LIMIT,
    SVG_XML_ERROR_INVALID_UTF8,
    SVG_XML_ERROR_INVALID_CHARACTER,
    SVG_XML_ERROR_UNSUPPORTED_ENCODING,
    SVG_XML_ERROR_INVALID_NAME,
    SVG_XML_ERROR_MALFORMED_SYNTAX,
    SVG_XML_ERROR_DUPLICATE_ATTRIBUTE,
    SVG_XML_ERROR_MISMATCHED_TAG,
    SVG_XML_ERROR_DEPTH_LIMIT,
    SVG_XML_ERROR_NODE_LIMIT,
    SVG_XML_ERROR_ATTRIBUTE_LIMIT,
    SVG_XML_ERROR_TEXT_LIMIT,
    SVG_XML_ERROR_OUTPUT_EXHAUSTED,
    SVG_XML_ERROR_UNSUPPORTED_DTD,
    SVG_XML_ERROR_MULTIPLE_ROOTS,
    SVG_XML_ERROR_INCOMPLETE_DOCUMENT
} svg_xml_error;

typedef struct svg_xml_span {
    size_t offset;
    size_t length;
} svg_xml_span;

typedef struct svg_xml_token {
    svg_xml_token_kind kind;
    svg_xml_span source;
    svg_xml_span name;
    svg_xml_span value;
    svg_xml_span prefix;
    svg_xml_span local_name;
    size_t depth;
} svg_xml_token;

typedef struct svg_xml_limits {
    size_t input_bytes;
    size_t nesting_depth;
    size_t nodes;
    size_t attributes_per_element;
    size_t total_attributes;
    size_t text_run_bytes;
} svg_xml_limits;

typedef struct svg_xml_options {
    svg_xml_limits limits;
} svg_xml_options;

typedef struct svg_xml_diagnostic {
    svg_xml_error category;
    size_t offset;
} svg_xml_diagnostic;

typedef struct svg_xml_result {
    size_t token_count;
    size_t element_count;
    size_t attribute_count;
    size_t node_count;
    svg_xml_diagnostic diagnostic;
} svg_xml_result;

typedef bool (*svg_xml_cancel_check_fn)(void *context);

typedef struct svg_xml_token_stream svg_xml_token_stream;
typedef enum svg_xml_stream_status {
    SVG_XML_STREAM_OK = 0,
    SVG_XML_STREAM_COMPLETE,
    SVG_XML_STREAM_FAILED,
    SVG_XML_STREAM_CANCELED
} svg_xml_stream_status;

svg_xml_options svg_xml_default_options(void);
svg_xml_result svg_xml_tokenize(base_bytes input, svg_xml_options options,
                                svg_xml_token *tokens, size_t token_capacity);
/* Same grammar and non-input limits as the default API; allows up to 1 GiB input. */
svg_xml_result svg_xml_tokenize_large(base_bytes input, svg_xml_token *tokens,
                                      size_t token_capacity);
svg_xml_result svg_xml_tokenize_large_cancelable(base_bytes input, svg_xml_token *tokens,
    size_t token_capacity, svg_xml_cancel_check_fn canceled, void *cancel_context);
/* Borrows immutable input until destroy; max_storage_bytes bounds its token vector. */
svg_xml_stream_status svg_xml_token_stream_start(base_bytes input, size_t max_storage_bytes,
    svg_xml_cancel_check_fn canceled, void *cancel_context, svg_xml_token_stream **out_stream);
/* Resumes character validation/token emission; budget is capped at 64 KiB. */
svg_xml_stream_status svg_xml_token_stream_step(svg_xml_token_stream *stream, size_t byte_budget);
svg_xml_result svg_xml_token_stream_result(const svg_xml_token_stream *stream);
/* Grammar-validated source prefix; safe to query between worker steps. */
size_t svg_xml_token_stream_bytes_read(const svg_xml_token_stream *stream);
/* Transfers the completed vector to the caller; destroy frees it otherwise. */
bool svg_xml_token_stream_take(svg_xml_token_stream *stream, svg_xml_token **out_tokens,
                               size_t *out_token_count);
void svg_xml_token_stream_destroy(svg_xml_token_stream *stream);
const char *svg_xml_error_name(svg_xml_error error);

#endif
