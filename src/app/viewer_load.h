#ifndef VEXEL_APP_VIEWER_LOAD_H
#define VEXEL_APP_VIEWER_LOAD_H

#include "platform/platform_task.h"
#include "raster/tile.h"
#include "shape.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef enum viewer_load_status {
    VIEWER_LOAD_OK,
    VIEWER_LOAD_FAILED,
    VIEWER_LOAD_CANCELED
} viewer_load_status;

typedef struct viewer_loaded_document {
    uint8_t *source;
    size_t source_length;
    svg_shape_document shape;
    raster_tile tile;
    svg_shape_error shape_error;
    size_t error_offset;
    char load_error[96];
} viewer_loaded_document;

/* On success, `out` owns the source, decoded shape, and raster tile. */
viewer_load_status viewer_load_document(const char *path,
                                        const platform_task_cancel_token *token,
                                        viewer_loaded_document *out);
/* Takes ownership of `source` and decodes it on the caller's worker thread. */
viewer_load_status viewer_decode_owned_source(uint8_t *source, size_t source_length,
    const platform_task_cancel_token *token, viewer_loaded_document *out);
void viewer_loaded_document_destroy(viewer_loaded_document *document);

#endif
