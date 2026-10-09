#ifndef VEXEL_SVG_SHAPE_H
#define VEXEL_SVG_SHAPE_H

#include "base.h"
#include "geometry.h"
#include "xml_tokenizer.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define SVG_SHAPE_MAX_EDGES 262144U
#define SVG_SHAPE_MAX_COMMANDS 65536U

typedef enum svg_shape_error {
    SVG_SHAPE_ERROR_NONE = 0,
    SVG_SHAPE_ERROR_INVALID_ARGUMENT,
    SVG_SHAPE_ERROR_XML,
    SVG_SHAPE_ERROR_UNSUPPORTED,
    SVG_SHAPE_ERROR_NUMBER,
    SVG_SHAPE_ERROR_LIMIT,
    SVG_SHAPE_ERROR_PATH,
    SVG_SHAPE_ERROR_VIEWPORT,
    SVG_SHAPE_ERROR_OOM
} svg_shape_error;

typedef struct svg_shape_diagnostic { svg_shape_error category; size_t offset; } svg_shape_diagnostic;
typedef struct svg_shape_document {
    uint32_t width;
    uint32_t height;
    double view_x, view_y, view_width, view_height;
    size_t edge_count;
    geom_edge *edges;
    uint8_t *colors; /* one owned RGBA color per edge */
    uint32_t *shape_ids; /* edge-to-element map; preserves independent fills */
    size_t shape_count;
    bool *shape_evenodd; /* one fill rule per shape element */
    bool *shape_filled; /* false for open line geometry, which awaits stroke */
    base_bytes source; /* borrowed; caller keeps source alive with document */
} svg_shape_document;

/* out owns decoded arrays and borrows source until document_destroy. Failure leaves out empty. */
svg_shape_error svg_shape_decode(base_bytes source, svg_shape_document *out,
                                 svg_shape_diagnostic *diagnostic);
/* Releases only owned arrays; the borrowed source remains caller-owned. */
void svg_shape_document_destroy(svg_shape_document *document);
const char *svg_shape_error_name(svg_shape_error error);

#endif
