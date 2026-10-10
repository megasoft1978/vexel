#ifndef VEXEL_APP_FRAME_COMPOSE_H
#define VEXEL_APP_FRAME_COMPOSE_H

#include "raster/tile.h"
#include "ui/viewer_controls.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef enum viewer_frame_kind {
    VIEWER_FRAME_LOADING,
    VIEWER_FRAME_DOCUMENT,
    VIEWER_FRAME_ERROR
} viewer_frame_kind;

/* Keep enough of an oversized document visible to recover it with a drag. */
void viewer_navigation_clamp(ui_viewer_navigation *navigation);

/* Writes an opaque RGBA frame into caller-owned storage. Width/height are backing pixels. */
bool viewer_compose_frame(const raster_tile *document,
                          const ui_viewer_navigation *navigation,
                          const ui_viewer_progress *progress,
                          viewer_frame_kind kind, const char *message,
                          uint8_t *out_rgba, size_t output_capacity,
                          uint32_t width, uint32_t height);

#endif
