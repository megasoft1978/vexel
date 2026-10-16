#ifndef VEXEL_PROJECT_PROJECT_ANIMATION_RENDER_H
#define VEXEL_PROJECT_PROJECT_ANIMATION_RENDER_H

#include "project_timeline.h"
#include "raster/shape_render.h"

#define PROJECT_ANIMATION_RENDER_MAX_COMPOSITE_BYTES ((size_t)536870912U)

typedef enum project_animation_render_error {
    PROJECT_ANIMATION_RENDER_OK = 0,
    PROJECT_ANIMATION_RENDER_INVALID_ARGUMENT,
    PROJECT_ANIMATION_RENDER_INVALID_TIMELINE,
    PROJECT_ANIMATION_RENDER_INVALID_REFERENCE,
    PROJECT_ANIMATION_RENDER_UNSUPPORTED_MEDIA,
    PROJECT_ANIMATION_RENDER_DIMENSION_MISMATCH,
    PROJECT_ANIMATION_RENDER_DECODE_FAILED,
    PROJECT_ANIMATION_RENDER_RASTER_FAILED,
    PROJECT_ANIMATION_RENDER_OUT_OF_MEMORY,
    PROJECT_ANIMATION_RENDER_LIMIT
} project_animation_render_error;

typedef struct project_animation_render_result {
    project_animation_render_error error;
    size_t track_index;
    size_t source_offset;
    svg_shape_diagnostic diagnostic;
} project_animation_render_result;

/* Render the scene plus evaluated property tracks and each visible drawing
 * track's exposure at tick. Loop sampling is not evaluated by this API. The
 * caller must pass a zero-initialized or destroyed out_tile; on success its
 * RGBA buffer is owned by the caller. Existing scene/timeline/bundle storage is
 * borrowed read-only and must remain alive for this call. */
project_animation_render_result project_animation_render_frame(
    const svg_shape_document *scene, const project_bundle_view *bundle,
    const project_timeline_view *timeline, uint32_t tick, raster_tile *out_tile);

const char *project_animation_render_error_name(
    project_animation_render_error error);

#endif
