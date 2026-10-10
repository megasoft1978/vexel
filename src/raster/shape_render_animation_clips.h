#ifndef VEXEL_RASTER_SHAPE_RENDER_ANIMATION_CLIPS_H
#define VEXEL_RASTER_SHAPE_RENDER_ANIMATION_CLIPS_H

#include "svg/shape/shape.h"

bool raster_animation_clips_clone(const svg_shape_document *source,
                                 svg_shape_document *evaluated,
                                 size_t *total_bytes, bool *limit);
void raster_animation_clips_release(const svg_shape_document *source,
                                    svg_shape_document *evaluated);
bool raster_animation_clips_update(svg_shape_document *document, size_t shape,
                                   bool update_bounds, size_t *work,
                                   size_t work_limit);

#endif
