#ifndef VEXEL_SVG_SHAPE_ALLOC_H
#define VEXEL_SVG_SHAPE_ALLOC_H

#include <stdbool.h>
#include <stddef.h>

typedef enum svg_shape_alloc_tag {
    SVG_SHAPE_ALLOC_TAG_NONE = 0,
    SVG_SHAPE_ALLOC_TAG_TOKEN_ARRAY,
    SVG_SHAPE_ALLOC_TAG_REFERENCE_INDEX,
    SVG_SHAPE_ALLOC_TAG_CLIP_COPY,
    SVG_SHAPE_ALLOC_TAG_OUTPUT
} svg_shape_alloc_tag;

typedef struct svg_shape_alloc_stats {
    size_t peak_live_bytes;
    size_t live_bytes;
    size_t total_allocated_bytes;
    size_t allocation_count;
    size_t token_array_bytes;
    size_t reference_index_bytes;
    size_t clip_copy_bytes;
    size_t output_bytes;
    bool overflow;
} svg_shape_alloc_stats;

/* The hook is inactive unless a test starts it. It measures allocations routed
 * through the SVG shape/raster modules without changing allocator behavior. */
void svg_shape_alloc_tracking_start(void);
void svg_shape_alloc_tracking_stop(svg_shape_alloc_stats *stats);
void *svg_shape_malloc(size_t size);
void *svg_shape_calloc(size_t count, size_t size);
void *svg_shape_realloc(void *pointer, size_t size);
void svg_shape_free(void *pointer);
bool svg_shape_alloc_set_tag(void *pointer, svg_shape_alloc_tag tag);
void svg_shape_alloc_note_external(void *pointer, size_t size);
void svg_shape_alloc_forget_external(void *pointer);

#endif
