#include "shape_render_internal.h"

#include <math.h>
#include <stdlib.h>

#define RASTER_STROKE_GRID_JOIN_BIT UINT32_C(0x80000000)

static bool add_size(size_t left, size_t right, size_t *out)
{
    if (right > SIZE_MAX - left) return false;
    *out = left + right;
    return true;
}

static bool mul_size(size_t left, size_t right, size_t *out)
{
    if (left != 0U && right > SIZE_MAX / left) return false;
    *out = left * right;
    return true;
}

static bool grid_bytes_fit(const raster_shape_dash_cache *cache, size_t extra)
{
    size_t shapes, segments, joins, used;
    return mul_size(cache->shape_count, sizeof(*cache->shapes), &shapes) &&
        mul_size(cache->segment_capacity, sizeof(*cache->segments), &segments) &&
        mul_size(cache->join_capacity, sizeof(*cache->joins), &joins) &&
        add_size(shapes, segments, &used) && add_size(used, joins, &used) &&
        add_size(used, cache->grid_bytes, &used) && add_size(used, extra, &used) &&
        used <= RASTER_DASH_CACHE_BYTE_LIMIT;
}

static double segment_margin(const raster_dash_segment *segment, double radius)
{
    return segment->start_cap == SVG_SHAPE_CAP_SQUARE ||
           segment->end_cap == SVG_SHAPE_CAP_SQUARE ? radius * 1.4142135623730951 :
           radius;
}

static double join_margin(const svg_shape_stroke *stroke)
{
    double radius = stroke->width * 0.5;
    return stroke->join == SVG_SHAPE_JOIN_MITER ? radius * stroke->miter_limit : radius;
}

static bool grid_axis_cell(double coordinate, double origin, double cell_size,
                           uint16_t count, uint16_t *out)
{
    double offset = (coordinate - origin) / cell_size;
    if (!isfinite(offset) || offset < -1.0) return false;
    if (offset < 0.0) offset = 0.0;
    double cell = floor(offset);
    if (cell >= (double)count) cell = (double)count - 1.0;
    if (cell < 0.0) return false;
    *out = (uint16_t)cell;
    return true;
}

static bool bbox_cells(const raster_dash_shape_range *range,
                       double min_x, double min_y, double max_x, double max_y,
                       uint16_t *x_first, uint16_t *y_first,
                       uint16_t *x_last, uint16_t *y_last)
{
    return grid_axis_cell(min_x, range->grid_min_x, range->grid_cell_size,
                          range->grid_columns, x_first) &&
        grid_axis_cell(min_y, range->grid_min_y, range->grid_cell_size,
                       range->grid_rows, y_first) &&
        grid_axis_cell(max_x, range->grid_min_x, range->grid_cell_size,
                       range->grid_columns, x_last) &&
        grid_axis_cell(max_y, range->grid_min_y, range->grid_cell_size,
                       range->grid_rows, y_last) &&
        *x_first <= *x_last && *y_first <= *y_last;
}

static bool segment_bbox(const raster_dash_segment *segment, double radius,
                         double *min_x, double *min_y,
                         double *max_x, double *max_y)
{
    double margin = segment_margin(segment, radius);
    *min_x = fmin(segment->edge.a.x, segment->edge.b.x) - margin;
    *min_y = fmin(segment->edge.a.y, segment->edge.b.y) - margin;
    *max_x = fmax(segment->edge.a.x, segment->edge.b.x) + margin;
    *max_y = fmax(segment->edge.a.y, segment->edge.b.y) + margin;
    return isfinite(*min_x) && isfinite(*min_y) &&
           isfinite(*max_x) && isfinite(*max_y);
}

static bool join_bbox(const raster_dash_join *join, double margin,
                      double *min_x, double *min_y,
                      double *max_x, double *max_y)
{
    *min_x = join->vertex.x - margin;
    *min_y = join->vertex.y - margin;
    *max_x = join->vertex.x + margin;
    *max_y = join->vertex.y + margin;
    return isfinite(*min_x) && isfinite(*min_y) &&
           isfinite(*max_x) && isfinite(*max_y);
}

static bool add_bbox_references(const raster_dash_shape_range *range,
                                raster_stroke_grid_cell *cells,
                                double min_x, double min_y,
                                double max_x, double max_y,
                                size_t *total)
{
    uint16_t x_first, y_first, x_last, y_last;
    if (!bbox_cells(range, min_x, min_y, max_x, max_y,
                    &x_first, &y_first, &x_last, &y_last)) return false;
    for (uint16_t y = y_first; y <= y_last; ++y) {
        for (uint16_t x = x_first; x <= x_last; ++x) {
            raster_stroke_grid_cell *cell = &cells[(size_t)y * range->grid_columns + x];
            if (cell->count == SIZE_MAX || *total == SIZE_MAX) return false;
            cell->count++;
            (*total)++;
        }
    }
    return true;
}

static bool fill_bbox_references(const raster_dash_shape_range *range,
                                 raster_stroke_grid_cell *cells,
                                 uint32_t *refs, uint32_t encoded,
                                 double min_x, double min_y,
                                 double max_x, double max_y)
{
    uint16_t x_first, y_first, x_last, y_last;
    if (!bbox_cells(range, min_x, min_y, max_x, max_y,
                    &x_first, &y_first, &x_last, &y_last)) return false;
    for (uint16_t y = y_first; y <= y_last; ++y) {
        for (uint16_t x = x_first; x <= x_last; ++x) {
            raster_stroke_grid_cell *cell = &cells[(size_t)y * range->grid_columns + x];
            if (cell->first > range->grid_ref_count ||
                cell->count >= range->grid_ref_count - cell->first) return false;
            refs[cell->first + cell->count++] = encoded;
        }
    }
    return true;
}

static void abandon_grid(raster_shape_dash_cache *cache,
                         raster_dash_shape_range *range,
                         raster_stroke_grid_cell *cells,
                         size_t cell_count)
{
    free(cells);
    range->grid_cells = NULL;
    range->grid_refs = NULL;
    range->grid_columns = 0U;
    range->grid_rows = 0U;
    range->grid_cell_count = 0U;
    range->grid_ref_count = 0U;
    cache->grid_cell_count -= cell_count;
}

static bool build_shape_grid(raster_shape_dash_cache *cache, size_t shape,
                             const svg_shape_stroke *stroke,
                             svg_shape_diagnostic *diagnostic)
{
    raster_dash_shape_range *range = &cache->shapes[shape];
    size_t primitive_count;
    if (!add_size(range->segment_count, range->join_count, &primitive_count) ||
        primitive_count == 0U) return true;
    double radius = stroke->width * 0.5;
    double min_x = INFINITY, min_y = INFINITY, max_x = -INFINITY, max_y = -INFINITY;
    for (size_t index = 0U; index < range->segment_count; ++index) {
        double item_min_x, item_min_y, item_max_x, item_max_y;
        const raster_dash_segment *segment =
            &cache->segments[range->segment_first + index];
        if (!segment_bbox(segment, radius, &item_min_x, &item_min_y,
                          &item_max_x, &item_max_y)) return true;
        min_x = fmin(min_x, item_min_x);
        min_y = fmin(min_y, item_min_y);
        max_x = fmax(max_x, item_max_x);
        max_y = fmax(max_y, item_max_y);
    }
    double join_reach = join_margin(stroke);
    for (size_t index = 0U; index < range->join_count; ++index) {
        double item_min_x, item_min_y, item_max_x, item_max_y;
        const raster_dash_join *join = &cache->joins[range->join_first + index];
        if (!join_bbox(join, join_reach, &item_min_x, &item_min_y,
                       &item_max_x, &item_max_y)) return true;
        min_x = fmin(min_x, item_min_x);
        min_y = fmin(min_y, item_min_y);
        max_x = fmax(max_x, item_max_x);
        max_y = fmax(max_y, item_max_y);
    }
    double span_x = max_x - min_x, span_y = max_y - min_y;
    if (!isfinite(span_x) || !isfinite(span_y) || span_x < 0.0 || span_y < 0.0)
        return true;
    double cell_size = fmax(RASTER_STROKE_GRID_CELL_SIZE,
        fmax(span_x, span_y) / (double)RASTER_STROKE_GRID_MAX_AXIS);
    if (!isfinite(cell_size) || cell_size <= 0.0) return true;
    double columns_value = ceil(span_x / cell_size);
    double rows_value = ceil(span_y / cell_size);
    if (columns_value < 1.0) columns_value = 1.0;
    if (rows_value < 1.0) rows_value = 1.0;
    if (columns_value > RASTER_STROKE_GRID_MAX_AXIS ||
        rows_value > RASTER_STROKE_GRID_MAX_AXIS) return true;
    range->grid_min_x = min_x;
    range->grid_min_y = min_y;
    range->grid_cell_size = cell_size;
    range->grid_columns = (uint16_t)columns_value;
    range->grid_rows = (uint16_t)rows_value;
    size_t cell_count;
    if (!mul_size(range->grid_columns, range->grid_rows, &cell_count) ||
        cell_count > RASTER_STROKE_GRID_MAX_CELLS - cache->grid_cell_count)
        return true;
    size_t cell_bytes;
    if (!mul_size(cell_count, sizeof(raster_stroke_grid_cell), &cell_bytes) ||
        !grid_bytes_fit(cache, cell_bytes)) return true;
    raster_stroke_grid_cell *cells = calloc(cell_count, sizeof(*cells));
    if (cells == NULL) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_OOM, 0U};
        return false;
    }
    cache->grid_cell_count += cell_count;
    range->grid_cell_count = cell_count;
    range->grid_cells = cells;
    size_t reference_count = 0U;
    bool references_fit = true;
    for (size_t index = 0U; index < range->segment_count && references_fit; ++index) {
        double item_min_x, item_min_y, item_max_x, item_max_y;
        const raster_dash_segment *segment =
            &cache->segments[range->segment_first + index];
        references_fit = segment_bbox(segment, radius, &item_min_x, &item_min_y,
                                      &item_max_x, &item_max_y) &&
            add_bbox_references(range, cells, item_min_x, item_min_y,
                                item_max_x, item_max_y, &reference_count);
    }
    for (size_t index = 0U; index < range->join_count && references_fit; ++index) {
        double item_min_x, item_min_y, item_max_x, item_max_y;
        const raster_dash_join *join = &cache->joins[range->join_first + index];
        references_fit = join_bbox(join, join_reach, &item_min_x, &item_min_y,
                                   &item_max_x, &item_max_y) &&
            add_bbox_references(range, cells, item_min_x, item_min_y,
                                item_max_x, item_max_y, &reference_count);
    }
    if (!references_fit ||
        reference_count > RASTER_STROKE_GRID_MAX_REFS - cache->grid_ref_count) {
        abandon_grid(cache, range, cells, cell_count);
        return true;
    }
    size_t ref_bytes, allocation_bytes;
    if (!mul_size(reference_count, sizeof(*range->grid_refs), &ref_bytes) ||
        !add_size(cell_bytes, ref_bytes, &allocation_bytes) ||
        !grid_bytes_fit(cache, allocation_bytes)) {
        abandon_grid(cache, range, cells, cell_count);
        return true;
    }
    uint32_t *refs = malloc(ref_bytes == 0U ? 1U : ref_bytes);
    if (refs == NULL) {
        abandon_grid(cache, range, cells, cell_count);
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_OOM, 0U};
        return false;
    }
    size_t first = 0U;
    for (size_t index = 0U; index < cell_count; ++index) {
        size_t count = cells[index].count;
        cells[index].first = first;
        cells[index].count = 0U;
        if (!add_size(first, count, &first)) {
            free(refs);
            abandon_grid(cache, range, cells, cell_count);
            return true;
        }
    }
    range->grid_refs = refs;
    range->grid_ref_count = reference_count;
    for (size_t index = 0U; index < range->segment_count; ++index) {
        double item_min_x, item_min_y, item_max_x, item_max_y;
        const raster_dash_segment *segment =
            &cache->segments[range->segment_first + index];
        if (!segment_bbox(segment, radius, &item_min_x, &item_min_y,
                          &item_max_x, &item_max_y) ||
            !fill_bbox_references(range, cells, refs,
                (uint32_t)(range->segment_first + index),
                item_min_x, item_min_y, item_max_x, item_max_y)) {
            abandon_grid(cache, range, cells, cell_count);
            free(refs);
            return true;
        }
    }
    for (size_t index = 0U; index < range->join_count; ++index) {
        double item_min_x, item_min_y, item_max_x, item_max_y;
        const raster_dash_join *join = &cache->joins[range->join_first + index];
        if (!join_bbox(join, join_reach, &item_min_x, &item_min_y,
                       &item_max_x, &item_max_y) ||
            !fill_bbox_references(range, cells, refs,
                RASTER_STROKE_GRID_JOIN_BIT |
                    (uint32_t)(range->join_first + index),
                item_min_x, item_min_y, item_max_x, item_max_y)) {
            abandon_grid(cache, range, cells, cell_count);
            free(refs);
            return true;
        }
    }
    size_t new_grid_bytes;
    if (!add_size(cache->grid_bytes, allocation_bytes, &new_grid_bytes)) {
        abandon_grid(cache, range, cells, cell_count);
        free(refs);
        return true;
    }
    range->grid_refs = refs;
    cache->grid_ref_count += reference_count;
    cache->grid_bytes = new_grid_bytes;
    return true;
}

bool raster_shape_stroke_grid_build(const svg_shape_document *doc,
                                    raster_shape_dash_cache *cache,
                                    svg_shape_diagnostic *diagnostic)
{
    if (cache == NULL || doc == NULL || cache->shapes == NULL) return true;
    for (size_t shape = 0U; shape < cache->shape_count; ++shape) {
        const svg_shape_stroke *stroke = &doc->strokes[shape];
        if (stroke->rgba[3] == 0U || stroke->width <= 0.0) continue;
        if (!build_shape_grid(cache, shape, stroke, diagnostic)) return false;
    }
    return true;
}

const raster_stroke_grid_cell *raster_shape_stroke_grid_lookup(
    const raster_dash_shape_range *range, double x, double y)
{
    if (range == NULL || range->grid_cells == NULL ||
        range->grid_columns == 0U || range->grid_rows == 0U) return NULL;
    uint16_t column, row;
    if (!grid_axis_cell(x, range->grid_min_x, range->grid_cell_size,
                        range->grid_columns, &column) ||
        !grid_axis_cell(y, range->grid_min_y, range->grid_cell_size,
                        range->grid_rows, &row)) return NULL;
    return &range->grid_cells[(size_t)row * range->grid_columns + column];
}
