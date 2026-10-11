#ifndef VEXEL_APP_FRAME_COMPOSE_H
#define VEXEL_APP_FRAME_COMPOSE_H

#include "raster/tile.h"
#include "viewer_quality.h"
#include "ui/viewer_controls.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef enum viewer_frame_kind {
    VIEWER_FRAME_LOADING,
    VIEWER_FRAME_DOCUMENT,
    VIEWER_FRAME_ERROR,
    VIEWER_FRAME_EMPTY
} viewer_frame_kind;

typedef struct viewer_frame_options {
    bool checkerboard;
    uint8_t background_index;
    bool pixel_grid;
    bool error_hint; /* Interactive viewer retry hint; legacy CLI frames omit it. */
    /* Borrowed for this call only; each tile's RGBA remains cache-owned. */
    const viewer_sharp_tile *sharp_tiles;
    size_t sharp_tile_count;
    const char *status_text;
} viewer_frame_options;

/* Keep enough of an oversized document visible to recover it with a drag. */
void viewer_navigation_clamp(ui_viewer_navigation *navigation);

/* Writes an opaque RGBA frame into caller-owned storage. Width/height are backing pixels. */
bool viewer_compose_frame(const raster_tile *document,
                          const ui_viewer_navigation *navigation,
                          const ui_viewer_progress *progress,
                          viewer_frame_kind kind, const char *message,
                          uint8_t *out_rgba, size_t output_capacity,
                          uint32_t width, uint32_t height);
bool viewer_compose_frame_ex(const raster_tile *document,
                             const ui_viewer_navigation *navigation,
                             const ui_viewer_progress *progress,
                             viewer_frame_kind kind, const char *message,
                             const viewer_frame_options *options,
                             uint8_t *out_rgba, size_t output_capacity,
                             uint32_t width, uint32_t height);

#endif
