#ifndef VEXEL_SVG_INDEX_H
#define VEXEL_SVG_INDEX_H

#include "base.h"
#include "xml_tokenizer.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define SVG_INDEX_MAX_SOURCE_BYTES (1024U * 1024U * 1024U)
#define SVG_INDEX_MAX_STEP_BYTES (64U * 1024U)
#define SVG_INDEX_MAX_ELEMENTS 1000000U
#define SVG_INDEX_MAX_ATTRIBUTES 1000000U
#define SVG_INDEX_MAX_STORAGE_BYTES (256U * 1024U * 1024U)
#define SVG_INDEX_MAX_DEPTH 256U

typedef struct svg_index_session svg_index_session;

typedef enum svg_index_status {
    SVG_INDEX_OK = 0,
    SVG_INDEX_INVALID,
    SVG_INDEX_LIMIT,
    SVG_INDEX_OOM,
    SVG_INDEX_MALFORMED,
    SVG_INDEX_CANCELED,
    SVG_INDEX_COMPLETE
} svg_index_status;

typedef enum svg_index_support {
    SVG_INDEX_UNSUPPORTED = 0,
    SVG_INDEX_SUPPORTED_RECT,
    SVG_INDEX_SUPPORTED_PATH
} svg_index_support;

typedef struct svg_index_element {
    size_t parent;
    size_t source_offset;
    size_t source_length;
    size_t name_offset;
    size_t name_length;
    size_t depth;
    svg_index_support support;
    bool closed;
} svg_index_element;

typedef struct svg_index_progress {
    size_t bytes_read;
    size_t source_bytes;
    size_t element_count;
    size_t attribute_count;
    uint64_t generation;
    svg_xml_diagnostic diagnostic;
    bool complete;
    bool canceled;
    bool first_preview_ready;
} svg_index_progress;

svg_index_status svg_index_start(base_bytes source, uint64_t generation,
                                 svg_index_session **out_session);
svg_index_status svg_index_step(svg_index_session *session, size_t byte_budget);
svg_index_progress svg_index_get_progress(const svg_index_session *session);
svg_index_status svg_index_advance_generation(svg_index_session *session,
                                               uint64_t generation);
void svg_index_cancel(svg_index_session *session);
bool svg_index_element_at(const svg_index_session *session, size_t index,
                          svg_index_element *out_element);
/* On a completed supported child, returns an owned synthetic preview SVG. */
bool svg_index_make_preview_source(const svg_index_session *session,
                                  base_mut_bytes *out_source);
void svg_index_destroy(svg_index_session *session);

#endif
