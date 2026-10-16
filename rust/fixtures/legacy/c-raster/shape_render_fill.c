#include "shape_render_fill.h"
#include "../svg/paint/paint.h"

#include <math.h>

#define RASTER_FILL_SAMPLES_PER_AXIS 4U
#define RASTER_FILL_SAMPLE_COUNT (RASTER_FILL_SAMPLES_PER_AXIS * RASTER_FILL_SAMPLES_PER_AXIS)

static const double sample_locations[RASTER_FILL_SAMPLES_PER_AXIS] = {
    0.125, 0.375, 0.625, 0.875
};

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

bool raster_shape_render_scanline_fill_shape(
    const svg_shape_document *document, size_t shape,
    const raster_pixel_bounds *pixel_bounds, uint32_t pixel_y,
    const raster_boundary_mask *boundary,
    const raster_fill_scanline_cache *fill_cache,
    double scale_x, double scale_y, double offset_x, double offset_y,
    const geom_edge *local_edges, const bool *dash_seams,
    const raster_shape_dash_cache *dash_cache, bool scalar_oracle_alpha,
    uint8_t *center_samples, uint8_t *subpixel_samples)
{
    if (document == NULL || pixel_bounds == NULL || boundary == NULL ||
        fill_cache == NULL || center_samples == NULL || subpixel_samples == NULL ||
        !pixel_bounds->visible || pixel_y < pixel_bounds->y_first ||
        pixel_y > pixel_bounds->y_last || !isfinite(scale_x) || scale_x <= 0.0 ||
        !isfinite(scale_y) || scale_y <= 0.0) return false;
    const uint8_t *fill = raster_fill_scanline_cache_color(document, shape);
    if (fill == NULL || !raster_fill_scanline_cache_is_eligible(fill_cache, shape))
        return false;
    size_t run_count = 0U;
    const raster_fill_pixel_run *runs =
        raster_fill_scanline_cache_pixel_runs(fill_cache, &run_count);
    if (runs == NULL || run_count == 0U) return false;
    const svg_shape_paint *paint = svg_shape_paint_for_shape(document, shape);
    bool gradient = paint != NULL &&
                    paint->kind != SVG_SHAPE_PAINT_SOLID;
    for (size_t interval_index = 0U;
         interval_index < fill_cache->interval_counts[2U]; interval_index++) {
        const raster_fill_scanline_interval *interval = fill_cache->intervals +
            2U * fill_cache->interval_stride + interval_index;
        size_t first_x, end_x;
        if (!raster_fill_scanline_interval_pixel_range(document, pixel_bounds,
                scale_x, offset_x, 0.5, interval, &first_x, &end_x, NULL))
            return false;
        for (size_t run_index = 0U; run_index < run_count; run_index++) {
            if (!runs[run_index].center_only) continue;
            size_t first = first_x > runs[run_index].first_x ? first_x :
                           runs[run_index].first_x;
            size_t end = end_x < runs[run_index].end_x ? end_x :
                         runs[run_index].end_x;
            for (size_t x = first; x < end; x++) {
                double sample_x = (double)x + 0.5;
                double sample_y = (double)pixel_y + 0.5;
                if (!sample_is_in_view(document, sample_x, sample_y, scale_x,
                                       scale_y, offset_x, offset_y)) continue;
                double px = document->view_x + (sample_x - offset_x) / scale_x;
                double py = document->view_y + (sample_y - offset_y) / scale_y;
                raster_shape_paint_sample_inside(document, shape, fill,
                    local_edges, dash_seams, dash_cache, px, py, px, py,
                    scalar_oracle_alpha, center_samples + x * 4U);
            }
        }
    }
    for (size_t sy = 0U; sy < RASTER_FILL_SAMPLES_PER_AXIS; sy++) {
        double sample_y = (double)pixel_y + sample_locations[sy];
        size_t active_slot = sy < 2U ? sy : sy + 1U;
        for (size_t sx = 0U; sx < RASTER_FILL_SAMPLES_PER_AXIS; sx++) {
            for (size_t interval_index = 0U;
                 interval_index < fill_cache->interval_counts[active_slot];
                 interval_index++) {
                const raster_fill_scanline_interval *interval =
                    fill_cache->intervals + active_slot *
                    fill_cache->interval_stride + interval_index;
                size_t first_x, end_x;
                if (!raster_fill_scanline_interval_pixel_range(document,
                        pixel_bounds, scale_x, offset_x, sample_locations[sx],
                        interval, &first_x, &end_x, NULL)) return false;
                for (size_t run_index = 0U; run_index < run_count; run_index++) {
                    if (runs[run_index].center_only) continue;
                    size_t first = first_x > runs[run_index].first_x ? first_x :
                                   runs[run_index].first_x;
                    size_t end = end_x < runs[run_index].end_x ? end_x :
                                 runs[run_index].end_x;
                    for (size_t x = first; x < end; x++) {
                        double sample_x = (double)x + sample_locations[sx];
                        if (!sample_is_in_view(document, sample_x, sample_y,
                                scale_x, scale_y, offset_x, offset_y)) continue;
                        double px = document->view_x +
                                    (sample_x - offset_x) / scale_x;
                        double py = document->view_y +
                                    (sample_y - offset_y) / scale_y;
                        double paint_x = 0.0, paint_y = 0.0;
                        if (gradient) {
                            double center_x = (double)x + 0.5;
                            double center_y = (double)pixel_y + 0.5;
                            paint_x = document->view_x +
                                      (center_x - offset_x) / scale_x;
                            paint_y = document->view_y +
                                      (center_y - offset_y) / scale_y;
                        }
                        size_t sample_index = x * RASTER_FILL_SAMPLE_COUNT +
                            sy * RASTER_FILL_SAMPLES_PER_AXIS + sx;
                        raster_shape_paint_sample_inside(document, shape, fill,
                            local_edges, dash_seams, dash_cache, px, py,
                            paint_x, paint_y, scalar_oracle_alpha,
                            subpixel_samples + sample_index * 4U);
                    }
                }
            }
        }
    }
    return true;
}
