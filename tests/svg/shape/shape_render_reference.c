#include "shape_render_reference.h"
#include "shape_render_internal.h"
#include "../../../src/svg/clip/clip.h"
#include "../../../src/svg/paint/paint.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

#define RASTER_MAX_TESTS 200000000U
#define RASTER_SAMPLES_PER_AXIS 4U
#define RASTER_SAMPLE_COUNT (RASTER_SAMPLES_PER_AXIS * RASTER_SAMPLES_PER_AXIS)

static const double sample_locations[RASTER_SAMPLES_PER_AXIS] = {
    0.125, 0.375, 0.625, 0.875
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

static bool render_work_is_bounded(const svg_shape_document *doc)
{
    size_t width = doc->width, height = doc->height;
    size_t pixels, edge_tests, shape_tests, stroke_per_sample = 0U;
    size_t auxiliary_per_sample = 0U, work_per_pixel, total_work;
    if (height == 0U || !checked_mul_size(width, height, &pixels) ||
        !checked_mul_size(doc->edge_count, RASTER_SAMPLE_COUNT, &edge_tests) ||
        !checked_mul_size(doc->shape_count, RASTER_SAMPLE_COUNT, &shape_tests))
        return false;
    for (size_t shape = 0U; shape < doc->shape_count; shape++) {
        const svg_shape_stroke *stroke = &doc->strokes[shape];
        size_t count = doc->shape_edge_count[shape], cost = 0U;
        if (stroke->rgba[3] != 0U && stroke->width > 0.0) {
            if (!doc->shape_stroke_closed[shape] && doc->shape_filled[shape] &&
                count > 0U) count--;
            if (!checked_mul_size(count, 2U, &cost) ||
                stroke->dash_count > SVG_SHAPE_MAX_DASH_VALUES) return false;
            if (stroke->dash_count > 0U) {
                const double *dash_values =
                    svg_shape_stroke_dash_values_for_shape(doc, shape);
                if (dash_values == NULL) return false;
                size_t expanded = stroke->dash_count +
                    (stroke->dash_count & 1U ? stroke->dash_count : 0U);
                size_t pattern_work;
                if (!checked_mul_size(expanded, 2U, &pattern_work) ||
                    !checked_add_size(cost, pattern_work, &cost)) return false;
                double min_dash = dash_values[0];
                for (size_t d = 0U; d < stroke->dash_count; d++) {
                    if (!isfinite(dash_values[d]) || dash_values[d] <= 0.0)
                        return false;
                    if (dash_values[d] < min_dash) min_dash = dash_values[d];
                }
                size_t first = doc->shape_edge_start[shape];
                for (size_t edge = 0U; edge < count; edge++) {
                    geom_edge local = doc->edges[first + edge];
                    if (!geom_affine_map_point(doc->shape_stroke_inverse[shape],
                                               local.a, &local.a) ||
                        !geom_affine_map_point(doc->shape_stroke_inverse[shape],
                                               local.b, &local.b)) return false;
                    double spans = ceil(hypot(local.b.x - local.a.x,
                                              local.b.y - local.a.y) / min_dash);
                    size_t span_count;
                    if (!isfinite(spans) || spans > (double)RASTER_MAX_TESTS ||
                        spans > (double)SIZE_MAX) return false;
                    span_count = (size_t)spans;
                    if (!checked_add_size(cost, span_count, &cost) ||
                        !checked_add_size(cost, 2U, &cost)) return false;
                }
            }
        }
        if (!checked_add_size(stroke_per_sample, cost, &stroke_per_sample))
            return false;
        if (doc->clips != NULL &&
            !checked_add_size(auxiliary_per_sample,
                              svg_clip_work_units(doc->clips[shape]),
                              &auxiliary_per_sample)) return false;
        const svg_shape_paint *fill_paint =
            svg_shape_paint_for_shape(doc, shape);
        if (fill_paint != NULL &&
            fill_paint->kind != SVG_SHAPE_PAINT_SOLID &&
            !checked_add_size(auxiliary_per_sample,
                              fill_paint->stop_count +
                                  (fill_paint->kind ==
                                       SVG_SHAPE_PAINT_PATTERN ? 4U : 0U),
                              &auxiliary_per_sample)) return false;
        const svg_shape_paint *stroke_paint =
            svg_shape_stroke_paint_for_shape(doc, shape);
        if (stroke_paint != NULL &&
            stroke_paint->kind == SVG_SHAPE_PAINT_PATTERN &&
            !checked_add_size(auxiliary_per_sample, 4U,
                              &auxiliary_per_sample)) return false;
    }
    size_t stroke_tests, auxiliary_tests;
    if (!checked_mul_size(stroke_per_sample, RASTER_SAMPLE_COUNT, &stroke_tests) ||
        !checked_mul_size(auxiliary_per_sample, RASTER_SAMPLE_COUNT,
                          &auxiliary_tests) ||
        !checked_add_size(edge_tests, shape_tests, &work_per_pixel) ||
        !checked_add_size(work_per_pixel, stroke_tests, &work_per_pixel) ||
        !checked_add_size(work_per_pixel, auxiliary_tests, &work_per_pixel))
        return false;
    if (work_per_pixel == 0U) work_per_pixel = 1U;
    return checked_mul_size(pixels, work_per_pixel, &total_work) &&
           total_work <= (size_t)RASTER_MAX_TESTS;
}

static bool render_sample(const svg_shape_document *doc,
                          const geom_edge *local_edges,
                          const bool *dash_seams, double sample_x,
                          double sample_y, double paint_device_x,
                          double paint_device_y, double scale,
                          double offset_x, double offset_y,
                          uint8_t sample_rgba[4])
{
    double content_width = doc->view_width * scale;
    double content_height = doc->view_height * scale;
    if (sample_x < offset_x || sample_y < offset_y ||
        sample_x >= offset_x + content_width ||
        sample_y >= offset_y + content_height) return true;
    double px = doc->view_x + (sample_x - offset_x) / scale;
    double py = doc->view_y + (sample_y - offset_y) / scale;
    double paint_x = doc->view_x + (paint_device_x - offset_x) / scale;
    double paint_y = doc->view_y + (paint_device_y - offset_y) / scale;
    for (size_t shape = 0U; shape < doc->shape_count; shape++) {
        raster_shape_paint_sample(doc, shape, NULL,
                                  doc->shape_edge_count[shape], local_edges,
                                  dash_seams, NULL, px, py, paint_x, paint_y, true,
                                  sample_rgba);
    }
    return true;
}

static bool render_pixel(const svg_shape_document *doc,
                         const geom_edge *local_edges,
                         const bool *dash_seams,
                         uint32_t x, uint32_t y,
                         double scale, double offset_x, double offset_y,
                         raster_tile *tile)
{
    uint32_t red = 0U, green = 0U, blue = 0U, alpha = 0U, covered = 0U;
    double paint_device_x = (double)x + 0.5;
    double paint_device_y = (double)y + 0.5;
    for (size_t sy = 0U; sy < RASTER_SAMPLES_PER_AXIS; sy++) {
        for (size_t sx = 0U; sx < RASTER_SAMPLES_PER_AXIS; sx++) {
            uint8_t sample_rgba[4] = {0U, 0U, 0U, 0U};
            if (!render_sample(doc, local_edges, dash_seams,
                               (double)x + sample_locations[sx],
                               (double)y + sample_locations[sy],
                               paint_device_x, paint_device_y, scale,
                               offset_x, offset_y, sample_rgba)) return false;
            if (sample_rgba[3] != 0U) {
                red += sample_rgba[0];
                green += sample_rgba[1];
                blue += sample_rgba[2];
                covered++;
            }
            alpha += sample_rgba[3];
        }
    }
    size_t offset = ((size_t)y * doc->width + x) * 4U;
    tile->rgba[offset] = covered == 0U ? 0U :
        (uint8_t)((red + covered / 2U) / covered);
    tile->rgba[offset + 1U] = covered == 0U ? 0U :
        (uint8_t)((green + covered / 2U) / covered);
    tile->rgba[offset + 2U] = covered == 0U ? 0U :
        (uint8_t)((blue + covered / 2U) / covered);
    tile->rgba[offset + 3U] =
        (uint8_t)((alpha + RASTER_SAMPLE_COUNT / 2U) / RASTER_SAMPLE_COUNT);
    return true;
}

static bool prepare_local_stroke_edges(const svg_shape_document *doc,
                                       geom_edge **out_edges,
                                       bool **out_dash_seams,
                                       svg_shape_diagnostic *diag)
{
    *out_edges = NULL;
    *out_dash_seams = NULL;
    bool needed = false;
    for (size_t shape = 0U; shape < doc->shape_count; shape++)
        needed |= doc->strokes[shape].rgba[3] != 0U &&
                  doc->strokes[shape].width > 0.0 &&
                  doc->shape_edge_count[shape] != 0U;
    if (!needed || doc->edge_count == 0U) return true;
    if (doc->edge_count > SIZE_MAX / sizeof(**out_edges) ||
        doc->path_subpath_count > SIZE_MAX - doc->shape_count ||
        doc->shape_count + doc->path_subpath_count >
            SIZE_MAX / sizeof(**out_dash_seams)) {
        if (diag != NULL)
            *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0};
        return false;
    }
    geom_edge *edges = malloc(doc->edge_count * sizeof(*edges));
    size_t seam_count = doc->shape_count + doc->path_subpath_count;
    bool *seams = calloc(seam_count, sizeof(*seams));
    if (edges == NULL || seams == NULL) {
        free(edges);
        free(seams);
        if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_OOM, 0};
        return false;
    }
    for (size_t i = 0U; i < doc->edge_count; i++) {
        edges[i] = doc->edges[i];
        size_t shape = doc->shape_ids[i];
        if (doc->strokes[shape].rgba[3] != 0U &&
            doc->strokes[shape].width > 0.0 &&
            (!geom_affine_map_point(doc->shape_stroke_inverse[shape],
                                    edges[i].a, &edges[i].a) ||
             !geom_affine_map_point(doc->shape_stroke_inverse[shape],
                                    edges[i].b, &edges[i].b))) {
            free(edges);
            free(seams);
            if (diag != NULL)
                *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0};
            return false;
        }
    }
    for (size_t shape = 0U; shape < doc->shape_count; shape++) {
        size_t path_count = doc->shape_subpath_count[shape];
        if (path_count != 0U) {
            for (size_t path = 0U; path < path_count; ++path)
                seams[doc->shape_count + doc->shape_subpath_start[shape] + path] =
                    doc->strokes[shape].rgba[3] != 0U &&
                    raster_shape_dash_subpath_joins_closed_seam(
                        doc, shape, path, edges);
        } else {
            seams[shape] = doc->strokes[shape].rgba[3] != 0U &&
                raster_shape_dash_joins_closed_seam(doc, shape, edges);
        }
    }
    *out_edges = edges;
    *out_dash_seams = seams;
    return true;
}

bool raster_shape_render_reference(const svg_shape_document *doc,
                                   raster_tile *out,
                                   svg_shape_diagnostic *diag)
{
    if (out != NULL) *out = (raster_tile){0};
    if (out == NULL || doc == NULL || doc->width == 0U || doc->height == 0U ||
        doc->edge_count > SVG_SHAPE_MAX_EDGES ||
        doc->shape_count > SVG_SHAPE_MAX_EDGES ||
        (doc->edge_count != 0U &&
         (doc->edges == NULL || doc->colors == NULL || doc->shape_ids == NULL)) ||
        (doc->shape_count != 0U &&
         (doc->shape_evenodd == NULL || doc->shape_filled == NULL ||
          doc->shape_stroke_closed == NULL || doc->shape_edge_start == NULL ||
          doc->shape_edge_count == NULL || doc->shape_subpath_start == NULL ||
          doc->shape_subpath_count == NULL || doc->shape_stroke_inverse == NULL ||
          doc->strokes == NULL)) ||
        (doc->path_subpath_count != 0U && doc->path_subpaths == NULL) ||
        !isfinite(doc->view_x) || !isfinite(doc->view_y) ||
        !isfinite(doc->view_width) || !isfinite(doc->view_height) ||
        doc->view_width <= 0.0 || doc->view_height <= 0.0) {
        if (diag != NULL)
            *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0};
        return false;
    }
    for (size_t i = 0U; i < doc->edge_count; i++) {
        if ((size_t)doc->shape_ids[i] >= doc->shape_count) {
            if (diag != NULL)
                *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0};
            return false;
        }
    }
    for (size_t shape = 0U; shape < doc->shape_count; shape++) {
        if (doc->shape_edge_start[shape] > doc->edge_count ||
            doc->shape_edge_count[shape] >
                doc->edge_count - doc->shape_edge_start[shape] ||
            doc->shape_subpath_start[shape] > doc->path_subpath_count ||
            doc->shape_subpath_count[shape] > doc->path_subpath_count -
                doc->shape_subpath_start[shape] ||
            !isfinite(doc->strokes[shape].width) ||
            doc->strokes[shape].width < 0.0 ||
            !isfinite(doc->strokes[shape].miter_limit) ||
            doc->strokes[shape].miter_limit < 1.0 ||
            !isfinite(doc->strokes[shape].dash_offset)) {
            if (diag != NULL)
                *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0};
            return false;
        }
    }
    for (size_t path = 0U; path < doc->path_subpath_count; ++path) {
        if (doc->path_subpaths[path].first_edge > doc->edge_count ||
            doc->path_subpaths[path].edge_count > doc->edge_count -
                doc->path_subpaths[path].first_edge) {
            if (diag != NULL)
                *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0};
            return false;
        }
    }
    if (!render_work_is_bounded(doc)) {
        if (diag != NULL)
            *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0};
        return false;
    }
    geom_edge *local_edges = NULL;
    bool *dash_seams = NULL;
    if (!prepare_local_stroke_edges(doc, &local_edges, &dash_seams, diag))
        return false;
    double scale = fmin((double)doc->width / doc->view_width,
                        (double)doc->height / doc->view_height);
    double offset_x = ((double)doc->width - doc->view_width * scale) * 0.5;
    double offset_y = ((double)doc->height - doc->view_height * scale) * 0.5;
    if (!raster_tile_create(doc->width, doc->height, out)) {
        free(local_edges);
        free(dash_seams);
        if (diag != NULL)
            *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0};
        return false;
    }
    memset(out->rgba, 0, out->byte_count);
    for (uint32_t y = 0U; y < doc->height; y++) {
        for (uint32_t x = 0U; x < doc->width; x++) {
            if (!render_pixel(doc, local_edges, dash_seams, x, y, scale,
                              offset_x, offset_y, out)) {
                raster_tile_destroy(out);
                free(local_edges);
                free(dash_seams);
                if (diag != NULL)
                    *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0};
                return false;
            }
        }
    }
    free(local_edges);
    free(dash_seams);
    if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_NONE, 0};
    return true;
}
