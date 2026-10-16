#include "shape_render_preflight.h"

#include "shape_render_internal.h"
#include "../svg/clip/clip.h"
#include "../svg/paint/paint.h"

#include <math.h>
#include <stdint.h>

/*
 * Work budget of the native-resolution raster preflight, in work units.
 * budget(pixels) = min(CAP, BASE + max(0, pixels - RAMP_START) * PER_PIXEL).
 *   BASE         200,000,000     budget of every canvas up to RAMP_START
 *   RAMP_START   1,048,576 px    1024 x 1024, the largest canvas whose old
 *                                fixed-budget refusals are pinned by tests
 *   PER_PIXEL    24,576          units added per pixel above RAMP_START
 *   CAP          16,000,000,000  absolute ceiling, reached at 1,691,479 px
 * 1600 x 1000 (1,600,000 px) gets 13,751,796,224.
 * Worst case: a document that just fits the cap does 1.6e10 units. Measured
 * on the corpus (Release, Apple Silicon) at about 1.3 s per 1e9 units, so
 * about 21 s. The preflight and render allocate no per-unit memory (measured
 * 77 MB and 382 MB maximum RSS for 1.05e10 and 4.5e8 units), so the bytes
 * bound is the canvas plus the fixed per-shape tables, not the cap.
 * A document over the budget still fails with SVG_SHAPE_ERROR_LIMIT.
 */
#define RASTER_PREFLIGHT_BASE_WORK 200000000ULL
#define RASTER_PREFLIGHT_RAMP_START_PIXELS 1048576ULL
#define RASTER_PREFLIGHT_WORK_PER_PIXEL 24576ULL
#define RASTER_PREFLIGHT_WORK_CAP 16000000000ULL
#define RASTER_PREFLIGHT_SAMPLES_PER_AXIS 4U

static _Thread_local size_t preflight_work_limit = RASTER_PREFLIGHT_BASE_WORK;

size_t raster_preflight_work_budget(uint32_t width, uint32_t height)
{
    unsigned long long pixels = (unsigned long long)width * height;
    if (pixels <= RASTER_PREFLIGHT_RAMP_START_PIXELS)
        return (size_t)RASTER_PREFLIGHT_BASE_WORK;
    unsigned long long extra = pixels - RASTER_PREFLIGHT_RAMP_START_PIXELS;
    unsigned long long room = RASTER_PREFLIGHT_WORK_CAP - RASTER_PREFLIGHT_BASE_WORK;
    if (extra > room / RASTER_PREFLIGHT_WORK_PER_PIXEL)
        return (size_t)RASTER_PREFLIGHT_WORK_CAP;
    return (size_t)(RASTER_PREFLIGHT_BASE_WORK +
                    extra * RASTER_PREFLIGHT_WORK_PER_PIXEL);
}

static const double sample_locations[RASTER_PREFLIGHT_SAMPLES_PER_AXIS] = {
    0.125, 0.375, 0.625, 0.875
};

static bool work_add(size_t *total, size_t amount)
{
    if (*total > preflight_work_limit ||
        amount > preflight_work_limit - *total) return false;
    *total += amount;
    return true;
}

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

static bool sample_is_in_view(const svg_shape_document *document,
                              double sample_x, double sample_y,
                              double scale_x, double scale_y,
                              double offset_x, double offset_y)
{
    double content_width = document->view_width * scale_x;
    double content_height = document->view_height * scale_y;
    return sample_x >= offset_x && sample_y >= offset_y &&
           sample_x < offset_x + content_width &&
           sample_y < offset_y + content_height;
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

static size_t active_count_for(const size_t *shape_offsets, size_t stride,
                               size_t sample, size_t shape)
{
    size_t base = sample * stride;
    return shape_offsets[base + shape + 1U] - shape_offsets[base + shape];
}

static size_t sample_aux_work(const svg_shape_document *document, size_t shape,
                              bool *ok)
{
    size_t work = 0U;
    if (document->has_clips && document->clips != NULL &&
        document->clips[shape] != NULL &&
        !checked_add_size(work, svg_clip_work_units(document->clips[shape]),
                          &work)) {
        *ok = false;
        return 0U;
    }
    const svg_shape_paint *fill_paint =
        svg_shape_paint_for_shape(document, shape);
    if (fill_paint != NULL &&
        fill_paint->kind != SVG_SHAPE_PAINT_SOLID &&
        !checked_add_size(work, fill_paint->stop_count +
            (fill_paint->kind == SVG_SHAPE_PAINT_PATTERN ? 4U : 0U), &work)) {
        *ok = false;
        return 0U;
    }
    const svg_shape_paint *stroke_paint =
        svg_shape_stroke_paint_for_shape(document, shape);
    if (stroke_paint != NULL &&
        stroke_paint->kind != SVG_SHAPE_PAINT_SOLID) {
        size_t stroke_work = stroke_paint->stop_count;
        if (stroke_paint->kind == SVG_SHAPE_PAINT_PATTERN &&
            !checked_add_size(stroke_work, 4U, &stroke_work)) {
            *ok = false;
            return 0U;
        }
        if (!checked_add_size(work, stroke_work, &work)) {
            *ok = false;
            return 0U;
        }
    }
    return work;
}

static bool add_fast_sample(size_t sample_aux, size_t comparisons,
                            size_t *total, size_t *shape_samples,
                            size_t *fill_interval_tests)
{
    size_t sample_work;
    return checked_add_size(*fill_interval_tests, comparisons,
                            fill_interval_tests) &&
           work_add(total, comparisons) &&
           checked_add_size(1U, sample_aux, &sample_work) &&
           checked_add_size(*shape_samples, 1U, shape_samples) &&
           work_add(total, sample_work);
}

static bool count_fast_fill_intervals_for_runs(
    const svg_shape_document *document, const raster_pixel_bounds *box,
    uint32_t y, raster_fill_scanline_cache *fill_cache, size_t sample_row,
    double sample_x_offset, double sample_y, bool center_only,
    const raster_fill_pixel_run *runs, size_t run_count,
    double scale_x, double scale_y, double offset_x, double offset_y,
    size_t sample_aux, size_t *total, size_t *shape_samples,
    size_t *fill_interval_tests)
{
    size_t interval_count = fill_cache->interval_counts[sample_row];
    const raster_fill_scanline_interval *intervals = fill_cache->intervals +
        sample_row * fill_cache->interval_stride;
    for (size_t interval_index = 0U; interval_index < interval_count;
         interval_index++) {
        size_t first_x, end_x, comparisons = 0U;
        if (!raster_fill_scanline_interval_pixel_range(document, box, scale_x,
                offset_x, sample_x_offset, &intervals[interval_index],
                &first_x, &end_x, &comparisons) ||
            !checked_add_size(*fill_interval_tests, comparisons,
                              fill_interval_tests) ||
            !work_add(total, comparisons)) return false;
        for (size_t run_index = 0U; run_index < run_count; run_index++) {
            if (!checked_add_size(*fill_interval_tests, 1U,
                                  fill_interval_tests) ||
                !work_add(total, 1U)) return false;
            if (runs[run_index].center_only != center_only) continue;
            size_t first = first_x > runs[run_index].first_x ? first_x :
                           runs[run_index].first_x;
            size_t end = end_x < runs[run_index].end_x ? end_x :
                         runs[run_index].end_x;
            for (size_t x = first; x < end; x++) {
                if (!checked_add_size(*fill_interval_tests, 1U,
                                      fill_interval_tests) ||
                    !work_add(total, 1U)) return false;
                double sample_x = (double)x + sample_x_offset;
                if (!sample_is_in_view(document, sample_x, sample_y,
                        scale_x, scale_y, offset_x, offset_y)) continue;
                if (!add_fast_sample(sample_aux, 0U, total, shape_samples,
                                     fill_interval_tests)) return false;
            }
        }
    }
    (void)y;
    return true;
}

bool raster_shape_render_work_preflight(
    const svg_shape_document *document,
    const raster_shape_bounds_set *bounds,
    const raster_boundary_mask *boundary,
    raster_scanline_index *scanlines,
    size_t *shape_offsets,
    uint32_t *active_edges,
    size_t active_edge_capacity,
    uint32_t *active_shapes,
    const size_t *stroke_cost,
    const raster_shape_dash_cache *dash_cache,
    raster_fill_scanline_cache *fill_cache,
    double scale_x, double scale_y, double offset_x, double offset_y,
    raster_shape_work_stats *stats)
{
    if (document == NULL || bounds == NULL || boundary == NULL ||
        scanlines == NULL || shape_offsets == NULL ||
        (document->shape_count != 0U &&
         (active_shapes == NULL || stroke_cost == NULL)) ||
        fill_cache == NULL || stats == NULL ||
        (size_t)document->width > SIZE_MAX / (size_t)document->height) return false;
    size_t total = 0U;
    size_t pixels = (size_t)document->width * (size_t)document->height;
    preflight_work_limit =
        raster_preflight_work_budget(document->width, document->height);
    if (!work_add(&total, pixels) || !work_add(&total, bounds->pixel_count) ||
        !work_add(&total, boundary->mark_work) ||
        !work_add(&total, document->edge_count) ||
        !work_add(&total, document->shape_count)) return false;
    size_t levels = 1U;
    size_t tree_size = document->edge_count > document->shape_count ?
                       document->edge_count : document->shape_count;
    for (size_t n = tree_size; n > 1U; n = (n + 1U) / 2U) levels++;
    size_t event_count, event_factor, event_work;
    size_t scanline_levels = 1U;
    for (size_t n = scanlines->scanline_count; n > 1U; n = (n + 1U) / 2U)
        scanline_levels++;
    size_t lookup_factor, lookup_work, stroke_setup_work = 0U;
    for (size_t shape = 0U; shape < document->shape_count; shape++)
        if (!checked_add_size(stroke_setup_work, stroke_cost[shape],
                              &stroke_setup_work)) return false;
    if (dash_cache != NULL &&
        !checked_add_size(stroke_setup_work, dash_cache->build_steps,
                          &stroke_setup_work)) return false;
    if (dash_cache != NULL &&
        (!checked_add_size(stroke_setup_work, dash_cache->grid_cell_count,
                           &stroke_setup_work) ||
         !checked_add_size(stroke_setup_work, dash_cache->grid_ref_count,
                           &stroke_setup_work))) return false;
    if (!checked_add_size(scanlines->event_count, scanlines->shape_event_count,
                          &event_count) ||
        !checked_mul_size(levels, 6U, &event_factor) ||
        !checked_add_size(event_factor, 8U, &event_factor) ||
        !checked_mul_size(event_count, event_factor, &event_work) ||
        !work_add(&total, event_work) ||
        !checked_mul_size(scanline_levels, 2U, &lookup_factor) ||
        !checked_add_size(lookup_factor, 1U, &lookup_factor) ||
        !checked_mul_size(document->edge_count, lookup_factor, &lookup_work) ||
        !work_add(&total, lookup_work) || !work_add(&total, stroke_setup_work))
        return false;

    size_t shape_pixels = 0U, shape_samples = 0U;
    size_t fill_edge_tests = 0U, fill_interval_tests = 0U, stroke_tests = 0U;
    size_t shape_row_visits = 0U, stride;
    if (!checked_add_size(document->shape_count, 1U, &stride)) return false;
    size_t offsets_capacity;
    if (!checked_mul_size(stride, RASTER_SCANLINE_VALUES_PER_ROW,
                          &offsets_capacity)) return false;
    (void)offsets_capacity;
    raster_scanline_index_reset(scanlines);
    for (uint32_t y = 0U; y < document->height; y++) {
        size_t listed_before = scanlines->listed_edge_count;
        size_t active_shape_count = 0U;
        if (!raster_scanline_index_build_row(scanlines, y, shape_offsets,
                active_edges, active_edge_capacity, active_shapes,
                document->shape_count, &active_shape_count) ||
            !work_add(&total, scanlines->listed_edge_count - listed_before))
            return false;
        size_t row_visits;
        if (!checked_add_size(shape_row_visits, active_shape_count,
                              &shape_row_visits) ||
            !checked_mul_size(active_shape_count,
                              RASTER_SCANLINE_VALUES_PER_ROW + 1U, &row_visits) ||
            !work_add(&total, row_visits)) return false;
        for (size_t active = 0U; active < active_shape_count; active++) {
            size_t shape = active_shapes[active];
            const raster_pixel_bounds *box = &bounds->shapes[shape].pixels;
            if (!box->visible || y < box->y_first || y > box->y_last) return false;
            bool aux_ok = true;
            size_t aux_work = sample_aux_work(document, shape, &aux_ok);
            if (!aux_ok) return false;
            size_t edge_visits = 0U, setup_work = 0U;
            bool fast_fill = raster_fill_scanline_cache_is_eligible(fill_cache, shape) &&
                raster_fill_scanline_cache_build_row(fill_cache, document, shape, y,
                    scale_y, offset_y, shape_offsets, active_edges, stride,
                    &edge_visits, &setup_work);
            if (fast_fill) {
                size_t run_work = 0U;
                fast_fill = raster_fill_scanline_cache_build_pixel_runs(
                    fill_cache, document, box, y, boundary, scale_x, scale_y,
                    offset_x, offset_y, &run_work);
                if (fast_fill) {
                    size_t box_width = (size_t)box->x_last - box->x_first + 1U;
                    size_t run_count = 0U;
                    const raster_fill_pixel_run *runs =
                        raster_fill_scanline_cache_pixel_runs(fill_cache,
                                                              &run_count);
                    if (runs == NULL || run_count == 0U ||
                        !checked_add_size(fill_edge_tests, edge_visits,
                                          &fill_edge_tests) ||
                        !work_add(&total, setup_work) ||
                        !work_add(&total, run_work) ||
                        !checked_add_size(shape_pixels, box_width,
                                          &shape_pixels) ||
                        shape_pixels > bounds->pixel_count ||
                        !count_fast_fill_intervals_for_runs(document, box, y,
                            fill_cache, 2U, 0.5, (double)y + 0.5, true,
                            runs, run_count, scale_x, scale_y, offset_x,
                            offset_y, aux_work, &total, &shape_samples,
                            &fill_interval_tests)) return false;
                    for (size_t sy = 0U; sy < RASTER_PREFLIGHT_SAMPLES_PER_AXIS;
                         sy++) {
                        double sample_y = (double)y + sample_locations[sy];
                        size_t slot = sy < 2U ? sy : sy + 1U;
                        for (size_t sx = 0U; sx < RASTER_PREFLIGHT_SAMPLES_PER_AXIS;
                             sx++) {
                            if (!count_fast_fill_intervals_for_runs(document, box,
                                    y, fill_cache, slot, sample_locations[sx],
                                    sample_y, false, runs, run_count,
                                    scale_x, scale_y, offset_x, offset_y,
                                    aux_work, &total, &shape_samples,
                                    &fill_interval_tests)) return false;
                        }
                    }
                    continue;
                }
            }
            for (uint32_t x = box->x_first; x <= box->x_last; x++) {
                if (!checked_add_size(shape_pixels, 1U, &shape_pixels) ||
                    shape_pixels > bounds->pixel_count) return false;
                size_t pixel = (size_t)y * document->width + x;
                bool crisp_edges = document->strokes != NULL &&
                                   document->strokes[shape].crisp_edges;
                bool center_only = pixel_is_fully_in_view(document, x, y,
                    scale_x, scale_y, offset_x, offset_y) &&
                    (crisp_edges || !raster_boundary_mask_test(boundary, pixel));
                if (center_only) {
                    double sample_x = (double)x + 0.5;
                    double sample_y = (double)y + 0.5;
                    size_t edge_count = active_count_for(shape_offsets, stride,
                                                          2U, shape);
                    size_t sample_work;
                    bool stroke_indexed = false, stroke_candidates = false;
                    size_t sample_stroke_work = raster_shape_stroke_sample_work(
                        document, shape, dash_cache,
                        document->view_x + (sample_x - offset_x) / scale_x,
                        document->view_y + (sample_y - offset_y) / scale_y,
                        &stroke_indexed, &stroke_candidates);
                    if (!document->shape_filled[shape] && stroke_indexed &&
                        !stroke_candidates) {
                        if (!checked_add_size(stroke_tests, sample_stroke_work,
                                              &stroke_tests) ||
                            !work_add(&total, sample_stroke_work)) return false;
                        continue;
                    }
                    if (!document->shape_filled[shape] && stroke_indexed &&
                        !checked_add_size(sample_stroke_work, 1U,
                                          &sample_stroke_work)) return false;
                    if (!checked_add_size(1U, edge_count, &sample_work) ||
                        !checked_add_size(sample_work, sample_stroke_work,
                                          &sample_work) ||
                        !checked_add_size(sample_work, aux_work, &sample_work) ||
                        !checked_add_size(shape_samples, 1U, &shape_samples) ||
                        !checked_add_size(fill_edge_tests, edge_count,
                                          &fill_edge_tests) ||
                        !checked_add_size(stroke_tests, sample_stroke_work,
                                          &stroke_tests) ||
                        !work_add(&total, sample_work)) return false;
                    continue;
                }
                for (size_t sy = 0U; sy < RASTER_PREFLIGHT_SAMPLES_PER_AXIS;
                     sy++) {
                    double sample_y = (double)y + sample_locations[sy];
                    size_t active_slot = sy < 2U ? sy : sy + 1U;
                    size_t edge_count = active_count_for(shape_offsets, stride,
                                                          active_slot, shape);
                    for (size_t sx = 0U; sx < RASTER_PREFLIGHT_SAMPLES_PER_AXIS;
                         sx++) {
                        double sample_x = (double)x + sample_locations[sx];
                        if (!sample_is_in_view(document, sample_x, sample_y,
                                scale_x, scale_y, offset_x, offset_y)) continue;
                        size_t sample_work;
                        bool stroke_indexed = false, stroke_candidates = false;
                        size_t sample_stroke_work = raster_shape_stroke_sample_work(
                            document, shape, dash_cache,
                            document->view_x + (sample_x - offset_x) / scale_x,
                            document->view_y + (sample_y - offset_y) / scale_y,
                            &stroke_indexed, &stroke_candidates);
                        if (!document->shape_filled[shape] && stroke_indexed &&
                            !stroke_candidates) {
                            if (!checked_add_size(stroke_tests, sample_stroke_work,
                                                  &stroke_tests) ||
                                !work_add(&total, sample_stroke_work)) return false;
                            continue;
                        }
                        if (!document->shape_filled[shape] && stroke_indexed &&
                            !checked_add_size(sample_stroke_work, 1U,
                                              &sample_stroke_work)) return false;
                        if (!checked_add_size(1U, edge_count, &sample_work) ||
                            !checked_add_size(sample_work, sample_stroke_work,
                                              &sample_work) ||
                            !checked_add_size(sample_work, aux_work, &sample_work) ||
                            !checked_add_size(shape_samples, 1U, &shape_samples) ||
                            !checked_add_size(fill_edge_tests, edge_count,
                                              &fill_edge_tests) ||
                            !checked_add_size(stroke_tests, sample_stroke_work,
                                              &stroke_tests) ||
                            !work_add(&total, sample_work)) return false;
                    }
                }
            }
        }
    }
    if (shape_pixels != bounds->pixel_count) return false;
    stats->shape_pixels = (uint64_t)shape_pixels;
    stats->shape_samples = (uint64_t)shape_samples;
    stats->fill_edge_tests = (uint64_t)fill_edge_tests;
    stats->fill_interval_tests = (uint64_t)fill_interval_tests;
    stats->stroke_tests = (uint64_t)stroke_tests;
    stats->scanline_edge_entries = (uint64_t)scanlines->listed_edge_count;
    stats->shape_row_visits = (uint64_t)shape_row_visits;
    stats->boundary_mark_work = (uint64_t)boundary->mark_work;
    stats->total_work = (uint64_t)total;
    return true;
}
