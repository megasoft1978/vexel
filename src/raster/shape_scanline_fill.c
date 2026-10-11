#include "shape_scanline_fill.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

#define RASTER_FILL_SCANLINE_PIXEL_RUN_LIMIT (8U * 1024U * 1024U)

static const double sample_y_offsets[RASTER_FILL_SCANLINE_SAMPLE_ROWS] = {
    0.125, 0.375, 0.5, 0.625, 0.875
};

static bool checked_add_size(size_t left, size_t right, size_t *out)
{
    if (right > SIZE_MAX - left) return false;
    *out = left + right;
    return true;
}

static bool checked_mul_size(size_t left, size_t right, size_t *out)
{
    if (left != 0U && right > SIZE_MAX / left) return false;
    *out = left * right;
    return true;
}

static bool shape_is_uniform_filled(const svg_shape_document *document,
                                    size_t shape)
{
    if (document == NULL || shape >= document->shape_count ||
        document->shape_filled == NULL || !document->shape_filled[shape] ||
        document->shape_edge_start == NULL || document->shape_edge_count == NULL ||
        document->colors == NULL || document->strokes == NULL) return false;
    const svg_shape_stroke *stroke = &document->strokes[shape];
    if (stroke->rgba[3] != 0U && stroke->width > 0.0) return false;
    size_t first = document->shape_edge_start[shape];
    size_t count = document->shape_edge_count[shape];
    if (count == 0U || first > document->edge_count ||
        count > document->edge_count - first) return false;
    const uint8_t *color = document->colors + first * 4U;
    for (size_t edge = 1U; edge < count; edge++) {
        if (memcmp(color, document->colors + (first + edge) * 4U, 4U) != 0)
            return false;
    }
    return true;
}

static bool pixel_is_fully_in_view(const svg_shape_document *document,
                                   uint32_t x, uint32_t y,
                                   double scale_x, double scale_y,
                                   double offset_x, double offset_y)
{
    double content_width = document->view_width * scale_x;
    double content_height = document->view_height * scale_y;
    return (double)x >= offset_x && (double)y >= offset_y &&
           (double)x + 1.0 <= offset_x + content_width &&
           (double)y + 1.0 <= offset_y + content_height;
}

static bool crossing_less(const raster_fill_scanline_crossing *left,
                          const raster_fill_scanline_crossing *right)
{
    if (left->x < right->x) return true;
    if (left->x > right->x) return false;
    return left->edge_index < right->edge_index;
}

static void swap_crossings(raster_fill_scanline_crossing *left,
                           raster_fill_scanline_crossing *right)
{
    raster_fill_scanline_crossing temporary = *left;
    *left = *right;
    *right = temporary;
}

static void sift_down(raster_fill_scanline_crossing *values, size_t root,
                      size_t count)
{
    while (root < count / 2U) {
        size_t child = root * 2U + 1U;
        if (child + 1U < count && crossing_less(&values[child],
                                                &values[child + 1U]))
            child++;
        if (!crossing_less(&values[root], &values[child])) return;
        swap_crossings(&values[root], &values[child]);
        root = child;
    }
}

static void sort_crossings(raster_fill_scanline_crossing *values, size_t count)
{
    for (size_t parent = count / 2U; parent != 0U; parent--)
        sift_down(values, parent - 1U, count);
    for (size_t end = count; end > 1U; end--) {
        swap_crossings(&values[0], &values[end - 1U]);
        sift_down(values, 0U, end - 1U);
    }
}

static size_t ceil_log2(size_t value)
{
    size_t levels = 0U;
    size_t power = 1U;
    while (power < value) {
        if (power > SIZE_MAX / 2U) return sizeof(size_t) * 8U;
        power *= 2U;
        levels++;
    }
    return levels;
}

static bool build_intervals(const svg_shape_document *document, size_t shape,
                            double sample_y, const uint32_t *active_edges,
                            size_t active_count,
                            raster_fill_scanline_crossing *crossings,
                            size_t crossing_capacity,
                            raster_fill_scanline_interval *intervals,
                            size_t interval_capacity, size_t *out_interval_count,
                            size_t *out_edge_visits, size_t *out_work_units)
{
    *out_interval_count = 0U;
    if (active_count > crossing_capacity ||
        (active_count != 0U && active_edges == NULL)) return false;
    size_t crossing_count = 0U;
    for (size_t index = 0U; index < active_count; index++) {
        uint32_t edge_index = active_edges[index];
        if ((size_t)edge_index >= document->edge_count) return false;
        geom_edge edge = document->edges[edge_index];
        bool upward = edge.a.y <= sample_y && edge.b.y > sample_y;
        bool downward = edge.b.y <= sample_y && edge.a.y > sample_y;
        if (!upward && !downward) continue;
        double x = edge.a.x + (sample_y - edge.a.y) *
                   (edge.b.x - edge.a.x) / (edge.b.y - edge.a.y);
        if (!isfinite(x)) return false;
        crossings[crossing_count++] = (raster_fill_scanline_crossing){
            x, upward ? 1 : -1, edge_index};
    }
    if (crossing_count > 1U) sort_crossings(crossings, crossing_count);
    int64_t winding = 0;
    uint8_t parity = 0U;
    for (size_t index = 0U; index < crossing_count; index++) {
        winding += crossings[index].winding_delta;
        parity ^= 1U;
    }
    if (crossing_count != 0U &&
        (document->shape_evenodd[shape] ? parity != 0U : winding != 0)) {
        if (interval_capacity == 0U) return false;
        intervals[(*out_interval_count)++] =
            (raster_fill_scanline_interval){-INFINITY, crossings[0].x};
    }
    size_t first = 0U;
    while (first < crossing_count) {
        size_t end = first + 1U;
        int64_t group_winding = crossings[first].winding_delta;
        uint8_t group_parity = 1U;
        while (end < crossing_count && crossings[end].x == crossings[first].x) {
            group_winding += crossings[end].winding_delta;
            group_parity ^= 1U;
            end++;
        }
        winding -= group_winding;
        parity ^= group_parity;
        double right = end < crossing_count ? crossings[end].x : INFINITY;
        bool inside = document->shape_evenodd[shape] ? parity != 0U : winding != 0;
        if (inside && crossings[first].x < right) {
            if (*out_interval_count >= interval_capacity) return false;
            intervals[(*out_interval_count)++] =
                (raster_fill_scanline_interval){crossings[first].x, right};
        }
        first = end;
    }
    *out_edge_visits = active_count;
    size_t sort_levels = ceil_log2(crossing_count > 1U ? crossing_count : 1U);
    size_t per_crossing, sort_work, scan_work;
    if (!checked_mul_size(sort_levels, 2U, &per_crossing) ||
        !checked_add_size(per_crossing, 6U, &per_crossing) ||
        !checked_mul_size(crossing_count, per_crossing, &sort_work) ||
        !checked_mul_size(active_count, 2U, &scan_work) ||
        !checked_add_size(sort_work, scan_work, out_work_units)) return false;
    return true;
}

bool raster_fill_scanline_cache_create(const svg_shape_document *document,
                                       raster_fill_scanline_cache *out_cache,
                                       svg_shape_diagnostic *diagnostic)
{
    if (diagnostic != NULL)
        *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
    if (out_cache == NULL) return false;
    *out_cache = (raster_fill_scanline_cache){0};
    if (document == NULL || (document->shape_count != 0U &&
        document->shape_count > SIZE_MAX / sizeof(*out_cache->eligible_shapes)))
        return false;
    if (document->shape_count == 0U) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_NONE, 0U};
        return true;
    }
    bool *eligible = calloc(document->shape_count, sizeof(*eligible));
    if (eligible == NULL) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_OOM, 0U};
        return false;
    }
    size_t max_edge_count = 0U;
    for (size_t shape = 0U; shape < document->shape_count; shape++) {
        size_t edge_count = document->shape_edge_count[shape];
        if (edge_count > RASTER_FILL_SCANLINE_MAX_EDGES ||
            !shape_is_uniform_filled(document, shape)) continue;
        eligible[shape] = true;
        if (edge_count > max_edge_count) max_edge_count = edge_count;
    }
    out_cache->eligible_shapes = eligible;
    out_cache->shape_count = document->shape_count;
    if (max_edge_count == 0U) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_NONE, 0U};
        return true;
    }
    if ((size_t)document->width >
        RASTER_FILL_SCANLINE_PIXEL_RUN_LIMIT / sizeof(*out_cache->pixel_runs)) {
        memset(eligible, 0, document->shape_count * sizeof(*eligible));
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_NONE, 0U};
        return true;
    }
    size_t interval_stride, interval_count, crossing_bytes, interval_bytes;
    if (!checked_add_size(max_edge_count, 1U, &interval_stride) ||
        !checked_mul_size(interval_stride, RASTER_FILL_SCANLINE_SAMPLE_ROWS,
                          &interval_count) ||
        max_edge_count > SIZE_MAX / sizeof(*out_cache->crossings) ||
        interval_count > SIZE_MAX / sizeof(*out_cache->intervals)) {
        raster_fill_scanline_cache_destroy(out_cache);
        return false;
    }
    crossing_bytes = max_edge_count * sizeof(*out_cache->crossings);
    interval_bytes = interval_count * sizeof(*out_cache->intervals);
    out_cache->crossings = malloc(crossing_bytes);
    out_cache->intervals = malloc(interval_bytes);
    out_cache->pixel_run_capacity = document->width;
    out_cache->pixel_runs = malloc((size_t)document->width *
                                   sizeof(*out_cache->pixel_runs));
    if (out_cache->crossings == NULL || out_cache->intervals == NULL ||
        out_cache->pixel_runs == NULL) {
        raster_fill_scanline_cache_destroy(out_cache);
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_OOM, 0U};
        return false;
    }
    out_cache->max_edge_count = max_edge_count;
    out_cache->interval_stride = interval_stride;
    if (diagnostic != NULL)
        *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_NONE, 0U};
    return true;
}

void raster_fill_scanline_cache_destroy(raster_fill_scanline_cache *cache)
{
    if (cache == NULL) return;
    free(cache->eligible_shapes);
    free(cache->crossings);
    free(cache->intervals);
    free(cache->pixel_runs);
    *cache = (raster_fill_scanline_cache){0};
}

bool raster_fill_scanline_cache_is_eligible(
    const raster_fill_scanline_cache *cache, size_t shape)
{
    return cache != NULL && cache->eligible_shapes != NULL &&
           shape < cache->shape_count && cache->eligible_shapes[shape];
}

const uint8_t *raster_fill_scanline_cache_color(
    const svg_shape_document *document, size_t shape)
{
    if (document == NULL || shape >= document->shape_count ||
        document->shape_edge_start == NULL || document->colors == NULL ||
        document->shape_edge_start[shape] >= document->edge_count) return NULL;
    return document->colors + document->shape_edge_start[shape] * 4U;
}

bool raster_fill_scanline_cache_build_row(
    raster_fill_scanline_cache *cache, const svg_shape_document *document,
    size_t shape, uint32_t pixel_y, double scale_y, double offset_y,
    const size_t *shape_offsets, const uint32_t *active_edges, size_t stride,
    size_t *out_edge_visits, size_t *out_work_units)
{
    if (out_edge_visits != NULL) *out_edge_visits = 0U;
    if (out_work_units != NULL) *out_work_units = 0U;
    if (!raster_fill_scanline_cache_is_eligible(cache, shape) ||
        document == NULL || shape_offsets == NULL || stride <= shape ||
        !isfinite(scale_y) || scale_y <= 0.0 || !isfinite(offset_y) ||
        cache->crossings == NULL || cache->intervals == NULL) return false;
    size_t edge_visits = 0U;
    size_t work_units = 0U;
    for (size_t sample = 0U; sample < RASTER_FILL_SCANLINE_SAMPLE_ROWS;
         sample++) {
        size_t base = sample * stride + shape;
        size_t first = shape_offsets[base];
        size_t end = shape_offsets[base + 1U];
        if (end < first || end - first > cache->max_edge_count) return false;
        size_t count = 0U, visits = 0U, work = 0U;
        double device_y = (double)pixel_y + sample_y_offsets[sample];
        double user_y = document->view_y + (device_y - offset_y) / scale_y;
        const uint32_t *edges = end == first || active_edges == NULL ? NULL :
                                active_edges + first;
        if (!build_intervals(document, shape, user_y, edges, end - first,
                cache->crossings, cache->max_edge_count,
                cache->intervals + sample * cache->interval_stride,
                cache->interval_stride, &count, &visits, &work) ||
            !checked_add_size(edge_visits, visits, &edge_visits) ||
            !checked_add_size(work_units, work, &work_units)) return false;
        cache->interval_counts[sample] = count;
    }
    if (out_edge_visits != NULL) *out_edge_visits = edge_visits;
    if (out_work_units != NULL) *out_work_units = work_units;
    return true;
}

bool raster_fill_scanline_cache_build_pixel_runs(
    raster_fill_scanline_cache *cache, const svg_shape_document *document,
    const raster_pixel_bounds *pixel_bounds, uint32_t pixel_y,
    const raster_boundary_mask *boundary, double scale_x, double scale_y,
    double offset_x, double offset_y, size_t *out_work_units)
{
    if (out_work_units != NULL) *out_work_units = 0U;
    if (cache == NULL || document == NULL || pixel_bounds == NULL ||
        boundary == NULL || boundary->bits == NULL || !pixel_bounds->visible ||
        pixel_y < pixel_bounds->y_first || pixel_y > pixel_bounds->y_last ||
        !isfinite(scale_x) || scale_x <= 0.0 ||
        !isfinite(scale_y) || scale_y <= 0.0 ||
        !isfinite(offset_x) || !isfinite(offset_y) ||
        cache->pixel_runs == NULL) return false;
    cache->pixel_run_count = 0U;
    bool have_run = false;
    bool run_center_only = false;
    size_t run_width = 0U;
    uint32_t run_first = 0U;
    for (uint32_t x = pixel_bounds->x_first; ; x++) {
        size_t pixel = (size_t)pixel_y * document->width + x;
        bool center_only = pixel_is_fully_in_view(document, x, pixel_y,
                scale_x, scale_y, offset_x, offset_y) &&
            !raster_boundary_mask_test(boundary, pixel);
        if (!have_run || center_only != run_center_only) {
            if (have_run) {
                if (cache->pixel_run_count >= cache->pixel_run_capacity) return false;
                cache->pixel_runs[cache->pixel_run_count++] =
                    (raster_fill_pixel_run){run_first, x, run_center_only};
            }
            run_first = x;
            run_center_only = center_only;
            have_run = true;
        }
        run_width++;
        if (x == pixel_bounds->x_last) break;
    }
    if (have_run) {
        uint32_t end_x = pixel_bounds->x_last + 1U;
        if (cache->pixel_run_count >= cache->pixel_run_capacity) return false;
        cache->pixel_runs[cache->pixel_run_count++] =
            (raster_fill_pixel_run){run_first, end_x, run_center_only};
    }
    if (out_work_units != NULL) *out_work_units = run_width;
    return true;
}

const raster_fill_pixel_run *raster_fill_scanline_cache_pixel_runs(
    const raster_fill_scanline_cache *cache, size_t *out_run_count)
{
    if (out_run_count != NULL) *out_run_count = 0U;
    if (cache == NULL || cache->pixel_runs == NULL) return NULL;
    if (out_run_count != NULL) *out_run_count = cache->pixel_run_count;
    return cache->pixel_runs;
}

static size_t first_sample_x_at_least(const svg_shape_document *document,
                                      const raster_pixel_bounds *pixel_bounds,
                                      double scale_x, double offset_x,
                                      double sample_offset, double target,
                                      size_t *comparisons)
{
    size_t low = pixel_bounds->x_first;
    size_t high = (size_t)pixel_bounds->x_last + 1U;
    while (low < high) {
        size_t middle = low + (high - low) / 2U;
        double device_x = (double)middle + sample_offset;
        double user_x = document->view_x + (device_x - offset_x) / scale_x;
        (*comparisons)++;
        if (user_x < target) low = middle + 1U;
        else high = middle;
    }
    return low;
}

bool raster_fill_scanline_interval_pixel_range(
    const svg_shape_document *document, const raster_pixel_bounds *pixel_bounds,
    double scale_x, double offset_x, double sample_offset,
    const raster_fill_scanline_interval *interval,
    size_t *out_first_x, size_t *out_end_x, size_t *out_comparisons)
{
    if (out_first_x != NULL) *out_first_x = 0U;
    if (out_end_x != NULL) *out_end_x = 0U;
    if (out_comparisons != NULL) *out_comparisons = 0U;
    if (document == NULL || pixel_bounds == NULL || interval == NULL ||
        out_first_x == NULL || out_end_x == NULL ||
        !pixel_bounds->visible || !isfinite(scale_x) || scale_x <= 0.0 ||
        !isfinite(offset_x) || !isfinite(sample_offset) ||
        isnan(interval->left) || isnan(interval->right) ||
        interval->right < interval->left) return false;
    size_t comparisons = 0U;
    size_t first = isinf(interval->left) && interval->left < 0.0 ?
        pixel_bounds->x_first : first_sample_x_at_least(document, pixel_bounds,
            scale_x, offset_x, sample_offset, interval->left, &comparisons);
    size_t end = isinf(interval->right) && interval->right > 0.0 ?
        (size_t)pixel_bounds->x_last + 1U :
        first_sample_x_at_least(document, pixel_bounds, scale_x, offset_x,
            sample_offset, interval->right, &comparisons);
    if (end < first) return false;
    *out_first_x = first;
    *out_end_x = end;
    if (out_comparisons != NULL) *out_comparisons = comparisons;
    return true;
}

bool raster_fill_scanline_cache_contains(
    const raster_fill_scanline_cache *cache, size_t sample_row, double x,
    size_t *out_comparisons)
{
    if (out_comparisons != NULL) *out_comparisons = 0U;
    if (cache == NULL || cache->intervals == NULL ||
        sample_row >= RASTER_FILL_SCANLINE_SAMPLE_ROWS || isnan(x)) return false;
    const raster_fill_scanline_interval *intervals =
        cache->intervals + sample_row * cache->interval_stride;
    size_t low = 0U;
    size_t high = cache->interval_counts[sample_row];
    size_t comparisons = 1U;
    while (low < high) {
        size_t middle = low + (high - low) / 2U;
        comparisons++;
        if (intervals[middle].right <= x) low = middle + 1U;
        else high = middle;
    }
    if (out_comparisons != NULL) *out_comparisons = comparisons;
    return low < cache->interval_counts[sample_row] &&
           intervals[low].left <= x && x < intervals[low].right;
}
