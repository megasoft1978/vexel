#ifndef VEXEL_PROJECT_PROJECT_DRAWING_ASSET_H
#define VEXEL_PROJECT_PROJECT_DRAWING_ASSET_H

#include "shape.h"
#include "project_bundle.h"
#include "xml_tokenizer.h"

typedef struct project_drawing_asset_result {
    svg_shape_error error;
    size_t error_offset;
} project_drawing_asset_result;

/* Validates a complete SVG drawing asset using bounded XML visitor passes and
 * the owned shape parser. Source is borrowed; no external resource is fetched
 * and no source bytes are changed. Cancellation stops visitor passes; parsing
 * has no persistent state and the caller retains byte ownership. */
project_drawing_asset_result project_drawing_asset_validate(base_bytes source,
    svg_xml_cancel_check_fn canceled, void *cancel_context, bool *out_canceled);

#endif
