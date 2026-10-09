#ifndef VEXEL_SVG_XML_READER_H
#define VEXEL_SVG_XML_READER_H

#include "xml_tokenizer.h"

typedef struct svg_xml_reader svg_xml_reader;

typedef enum svg_xml_reader_status {
    SVG_XML_READER_OK = 0,
    SVG_XML_READER_COMPLETE,
    SVG_XML_READER_INVALID,
    SVG_XML_READER_LIMIT,
    SVG_XML_READER_MALFORMED
} svg_xml_reader_status;

typedef struct svg_xml_reader_tag {
    svg_xml_token_kind kind;
    svg_xml_span source;
    svg_xml_span name;
    size_t attribute_count;
} svg_xml_reader_tag;

typedef struct svg_xml_reader_progress {
    size_t bytes_read;
    size_t source_bytes;
    svg_xml_diagnostic diagnostic;
    bool complete;
} svg_xml_reader_progress;

typedef bool (*svg_xml_reader_tag_fn)(void *context, const svg_xml_reader_tag *tag);

/* Borrows immutable source bytes through destroy; no content is copied. */
svg_xml_reader_status svg_xml_reader_start(base_bytes source, svg_xml_reader **out_reader);
/* Advances at most byte_budget bytes (hard capped at 64 KiB), emitting complete tags. */
svg_xml_reader_status svg_xml_reader_step(svg_xml_reader *reader, size_t byte_budget,
    svg_xml_reader_tag_fn on_tag, void *context);
svg_xml_reader_progress svg_xml_reader_get_progress(const svg_xml_reader *reader);
void svg_xml_reader_destroy(svg_xml_reader *reader);

#endif
