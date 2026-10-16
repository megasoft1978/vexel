#ifndef VEXEL_RASTER_SHAPE_RENDER_INTERNAL_H
#define VEXEL_RASTER_SHAPE_RENDER_INTERNAL_H

#include "shape_render.h"

#include <stddef.h>
#include <stdint.h>

#define RASTER_STROKE_DASH_EPSILON 1e-12
#define RASTER_DASH_CACHE_MAX_STEPS 1000000U
#define RASTER_STROKE_GRID_MAX_CELLS 65536U
#define RASTER_STROKE_GRID_MAX_REFS 1000000U
#define RASTER_STROKE_GRID_MAX_AXIS 128U
#define RASTER_STROKE_GRID_CELL_SIZE 1.0
#define RASTER_DASH_CACHE_BYTE_LIMIT (16U * 1024U * 1024U)

typedef struct raster_stroke_grid_cell {
    size_t first;
    size_t count;
} raster_stroke_grid_cell;

typedef struct raster_dash_segment {
    geom_edge edge;
    svg_shape_linecap start_cap;
    svg_shape_linecap end_cap;
} raster_dash_segment;

typedef struct raster_dash_join {
    geom_point previous;
    geom_point vertex;
    geom_point next;
} raster_dash_join;

typedef struct raster_dash_shape_range {
    size_t segment_first;
    size_t segment_count;
    size_t join_first;
    size_t join_count;
    raster_stroke_grid_cell *grid_cells;
    uint32_t *grid_refs;
    double grid_min_x;
    double grid_min_y;
    double grid_cell_size;
    uint16_t grid_columns;
    uint16_t grid_rows;
    size_t grid_cell_count;
    size_t grid_ref_count;
} raster_dash_shape_range;

typedef struct raster_shape_dash_cache {
    raster_dash_segment *segments;
    size_t segment_count;
    size_t segment_capacity;
    raster_dash_join *joins;
    size_t join_count;
    size_t join_capacity;
    raster_dash_shape_range *shapes;
    size_t shape_count;
    size_t build_steps;
    size_t grid_cell_count;
    size_t grid_ref_count;
    size_t grid_bytes;
} raster_shape_dash_cache;

typedef struct raster_shape_work_stats {
    uint64_t shape_pixels;
    uint64_t shape_samples;
    uint64_t fill_edge_tests;
    uint64_t fill_interval_tests;
    uint64_t stroke_tests;
    uint64_t scanline_edge_entries;
    uint64_t shape_row_visits;
    uint64_t boundary_mark_work;
    uint64_t total_work;
} raster_shape_work_stats;

/* Private raster helpers shared by the render orchestration and sample modules. */
void raster_shape_paint_sample(const svg_shape_document *doc, size_t shape,
                               const uint32_t *active_edges, size_t active_count,
                               const geom_edge *local_edges,
                               const bool *dash_seams,
                               const raster_shape_dash_cache *dash_cache,
                               double px, double py,
                               double paint_x, double paint_y,
                               bool scalar_oracle_alpha,
                               uint8_t sample_rgba[4]);
void raster_shape_paint_sample_inside(
    const svg_shape_document *doc, size_t shape, const uint8_t fill_rgba[4],
    const geom_edge *local_edges, const bool *dash_seams,
    const raster_shape_dash_cache *dash_cache, double px, double py,
    double paint_x, double paint_y, bool scalar_oracle_alpha,
    uint8_t sample_rgba[4]);
bool raster_shape_stroke_contains(const svg_shape_document *doc, size_t shape,
                                  const geom_edge *local_edges,
                                  const bool *dash_seams,
                                  const raster_shape_dash_cache *dash_cache,
                                  double px, double py);
bool raster_shape_dash_cache_create(const svg_shape_document *doc,
                                    const geom_edge *local_edges,
                                    const bool *dash_seams,
                                    raster_shape_dash_cache *out_cache,
                                    svg_shape_diagnostic *diagnostic);
void raster_shape_dash_cache_destroy(raster_shape_dash_cache *cache);
bool raster_shape_stroke_grid_build(const svg_shape_document *doc,
                                    raster_shape_dash_cache *cache,
                                    svg_shape_diagnostic *diagnostic);
const raster_stroke_grid_cell *raster_shape_stroke_grid_lookup(
    const raster_dash_shape_range *range, double x, double y);
size_t raster_shape_stroke_sample_work(const svg_shape_document *doc,
                                       size_t shape,
                                       const raster_shape_dash_cache *cache,
                                       double x, double y,
                                       bool *out_indexed,
                                       bool *out_has_candidates);
bool raster_shape_dash_joins_closed_seam(const svg_shape_document *doc,
                                         size_t shape, const geom_edge *edges);
bool raster_shape_dash_subpath_joins_closed_seam(
    const svg_shape_document *doc, size_t shape, size_t subpath,
    const geom_edge *edges);
bool raster_shape_render_flat(const svg_shape_document *document,
                              raster_tile *out_tile,
                              svg_shape_diagnostic *diagnostic,
                              raster_shape_work_stats *work,
                              bool scalar_oracle_alpha);
bool raster_shape_render_grouped(const svg_shape_document *document,
                                 raster_tile *out_tile,
                                 svg_shape_diagnostic *diagnostic,
                                 raster_shape_work_stats *work,
                                 bool scalar_oracle_alpha);

/* Internal measurement entry point used by deterministic tests and benchmarks. */
bool raster_shape_render_with_stats(const svg_shape_document *document,
                                    raster_tile *out_tile,
                                    svg_shape_diagnostic *diagnostic,
                                    raster_shape_work_stats *work);

/* Test adapter for synthetic-alpha comparisons with the frozen scalar oracle.
 * Production rendering always uses source-over compositing. */
bool raster_shape_render_with_stats_scalar_oracle_alpha(
    const svg_shape_document *document, raster_tile *out_tile,
    svg_shape_diagnostic *diagnostic, raster_shape_work_stats *work);

#endif
