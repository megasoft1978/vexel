#ifndef VEXEL_RASTER_SHAPE_SCANLINE_H
#define VEXEL_RASTER_SHAPE_SCANLINE_H

#include "shape.h"
#include "shape_bounds.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define RASTER_SCANLINE_VALUES_PER_ROW 5U

typedef struct raster_scanline_event {
    size_t slot;
    uint32_t edge;
    bool add;
} raster_scanline_event;

typedef struct raster_shape_row_event {
    uint32_t row;
    uint32_t shape;
    bool add;
} raster_shape_row_event;

typedef struct raster_scanline_node {
    int32_t left;
    int32_t right;
    int32_t height;
} raster_scanline_node;

typedef struct raster_scanline_index {
    const svg_shape_document *document;
    raster_scanline_event *events;
    raster_scanline_node *nodes;
    int32_t *roots;
    raster_shape_row_event *shape_events;
    raster_scanline_node *shape_nodes;
    int32_t shape_root;
    size_t event_count;
    size_t event_cursor;
    size_t shape_event_count;
    size_t shape_event_cursor;
    size_t shape_count;
    size_t edge_count;
    size_t scanline_count;
    size_t listed_edge_count;
} raster_scanline_index;

bool raster_scanline_index_create(const svg_shape_document *document,
                                  const raster_shape_bounds_set *bounds,
                                  double scale, double offset_y,
                                  raster_scanline_index *out,
                                  svg_shape_diagnostic *diagnostic);
void raster_scanline_index_reset(raster_scanline_index *index);
void raster_scanline_index_destroy(raster_scanline_index *index);
bool raster_scanline_index_build_row(raster_scanline_index *index,
                                     uint32_t pixel_y,
                                     size_t *shape_offsets,
                                     uint32_t *active_edges,
                                     size_t active_edge_capacity,
                                     uint32_t *active_shapes,
                                     size_t active_shape_capacity,
                                     size_t *active_shape_count);

#endif
