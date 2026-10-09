#include "shape_render.h"

#include <limits.h>
#include <math.h>
#include <stdlib.h>
#include <string.h>

#define RASTER_MAX_TESTS 200000000U
#define RASTER_SAMPLES_PER_AXIS 4U
#define RASTER_SAMPLE_COUNT (RASTER_SAMPLES_PER_AXIS * RASTER_SAMPLES_PER_AXIS)

static bool edge_crosses_right(geom_edge edge, double px, double py);

static bool render_work_is_bounded(const svg_shape_document *doc)
{
    size_t width = doc->width;
    size_t height = doc->height;
    if (height == 0U || width > SIZE_MAX / height) return false;
    size_t pixels = width * height;
    if (doc->edge_count > SIZE_MAX / (RASTER_SAMPLE_COUNT + 1U) ||
        doc->shape_count > SIZE_MAX / RASTER_SAMPLE_COUNT) return false;
    size_t edge_tests = doc->edge_count * (RASTER_SAMPLE_COUNT + 1U);
    size_t shape_tests = doc->shape_count * RASTER_SAMPLE_COUNT;
    if (edge_tests > SIZE_MAX - shape_tests) return false;
    size_t per_pixel = edge_tests + shape_tests;
    if (per_pixel == 0U) per_pixel = 1U;
    return pixels <= (size_t)RASTER_MAX_TESTS / per_pixel;
}

static bool render_sample(const svg_shape_document *doc, double sample_x,
                          double sample_y, double scale, double offset_x,
                          double offset_y, uint8_t *sample_rgba,
                          int32_t *winding, uint8_t *parity,
                          uint8_t *colors, svg_shape_diagnostic *diag)
{
    double content_width = doc->view_width * scale;
    double content_height = doc->view_height * scale;
    if (sample_x < offset_x || sample_y < offset_y ||
        sample_x >= offset_x + content_width ||
        sample_y >= offset_y + content_height) return true;
    if (doc->shape_count != 0U) {
        memset(winding, 0, doc->shape_count * sizeof(*winding));
        memset(parity, 0, doc->shape_count);
        memset(colors, 0, doc->shape_count * 4U);
    }
    double px = doc->view_x + (sample_x - offset_x) / scale;
    double py = doc->view_y + (sample_y - offset_y) / scale;
    for (size_t i = 0; i < doc->edge_count; i++) {
        geom_edge edge = doc->edges[i];
        uint32_t shape = doc->shape_ids[i];
        if ((size_t)shape >= doc->shape_count) {
            if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0};
            return false;
        }
        if (!doc->shape_filled[shape]) continue;
        bool upward = edge.a.y <= py && edge.b.y > py;
        bool downward = edge.b.y <= py && edge.a.y > py;
        if ((!upward && !downward) || !edge_crosses_right(edge, px, py)) continue;
        winding[shape] += upward ? 1 : -1;
        parity[shape] ^= 1U;
        memcpy(colors + (size_t)shape * 4U, doc->colors + i * 4U, 4U);
    }
    for (size_t shape = 0; shape < doc->shape_count; shape++) {
        bool inside = doc->shape_evenodd[shape] ? parity[shape] != 0U : winding[shape] != 0;
        const uint8_t *fill = colors + shape * 4U;
        if (inside && fill[3] != 0U) memcpy(sample_rgba, fill, 4U);
    }
    return true;
}

static bool pixel_may_cross_edge(const svg_shape_document *doc,
                                 double left, double top,
                                 double right, double bottom)
{
    for (size_t i = 0; i < doc->edge_count; i++) {
        geom_edge edge = doc->edges[i];
        bool left_of = edge.a.x < left && edge.b.x < left;
        bool right_of = edge.a.x > right && edge.b.x > right;
        bool above = edge.a.y < top && edge.b.y < top;
        bool below = edge.a.y > bottom && edge.b.y > bottom;
        if (!left_of && !right_of && !above && !below) return true;
    }
    return false;
}

static bool render_pixel(const svg_shape_document *doc, uint32_t x, uint32_t y,
                         double scale, double offset_x, double offset_y,
                         raster_tile *tile, int32_t *winding,
                         uint8_t *parity, uint8_t *colors,
                         svg_shape_diagnostic *diag)
{
    static const double locations[RASTER_SAMPLES_PER_AXIS] = {
        0.125, 0.375, 0.625, 0.875
    };
    double content_width = doc->view_width * scale;
    double content_height = doc->view_height * scale;
    bool fully_in_view = (double)x >= offset_x && (double)y >= offset_y &&
                         (double)x + 1.0 <= offset_x + content_width &&
                         (double)y + 1.0 <= offset_y + content_height;
    if (fully_in_view) {
        double left = doc->view_x + ((double)x - offset_x) / scale;
        double top = doc->view_y + ((double)y - offset_y) / scale;
        double right = doc->view_x + ((double)x + 1.0 - offset_x) / scale;
        double bottom = doc->view_y + ((double)y + 1.0 - offset_y) / scale;
        if (!pixel_may_cross_edge(doc, left, top, right, bottom)) {
            uint8_t center_rgba[4] = {0U, 0U, 0U, 0U};
            if (!render_sample(doc, (double)x + 0.5, (double)y + 0.5,
                               scale, offset_x, offset_y, center_rgba,
                               winding, parity, colors, diag)) return false;
            size_t center_offset = ((size_t)y * doc->width + x) * 4U;
            memcpy(tile->rgba + center_offset, center_rgba, sizeof(center_rgba));
            return true;
        }
    }
    uint32_t red = 0U, green = 0U, blue = 0U, alpha = 0U, covered = 0U;
    for (size_t sy = 0; sy < RASTER_SAMPLES_PER_AXIS; sy++) {
        for (size_t sx = 0; sx < RASTER_SAMPLES_PER_AXIS; sx++) {
            uint8_t sample_rgba[4] = {0U, 0U, 0U, 0U};
            if (!render_sample(doc, (double)x + locations[sx],
                               (double)y + locations[sy], scale, offset_x,
                               offset_y, sample_rgba, winding, parity, colors,
                               diag)) return false;
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
    tile->rgba[offset] = covered == 0U ? 0U : (uint8_t)((red + covered / 2U) / covered);
    tile->rgba[offset + 1U] = covered == 0U ? 0U : (uint8_t)((green + covered / 2U) / covered);
    tile->rgba[offset + 2U] = covered == 0U ? 0U : (uint8_t)((blue + covered / 2U) / covered);
    tile->rgba[offset + 3U] = (uint8_t)((alpha + RASTER_SAMPLE_COUNT / 2U) / RASTER_SAMPLE_COUNT);
    return true;
}

bool raster_shape_render(const svg_shape_document *doc, raster_tile *out,
                         svg_shape_diagnostic *diag)
{
    if (out != NULL) *out = (raster_tile){0};
    if (out == NULL || doc == NULL || doc->width == 0U || doc->height == 0U ||
        doc->edge_count > SVG_SHAPE_MAX_EDGES || doc->shape_count > SVG_SHAPE_MAX_EDGES ||
        (doc->edge_count != 0U && (doc->edges == NULL || doc->colors == NULL ||
                                   doc->shape_ids == NULL)) ||
        (doc->shape_count != 0U &&
         (doc->shape_evenodd == NULL || doc->shape_filled == NULL)) ||
        !isfinite(doc->view_x) || !isfinite(doc->view_y) ||
        !isfinite(doc->view_width) || !isfinite(doc->view_height) ||
        doc->view_width <= 0.0 || doc->view_height <= 0.0 ||
        !render_work_is_bounded(doc)) {
        if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0};
        return false;
    }
    if (doc->shape_count > SIZE_MAX / sizeof(int32_t) ||
        doc->shape_count > SIZE_MAX / 4U) {
        if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0};
        return false;
    }
    for (size_t i = 0; i < doc->edge_count; i++) {
        if ((size_t)doc->shape_ids[i] >= doc->shape_count) {
            if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0};
            return false;
        }
    }
    int32_t *winding = calloc(doc->shape_count, sizeof(*winding));
    uint8_t *parity = calloc(doc->shape_count, sizeof(*parity));
    uint8_t *colors = calloc(doc->shape_count, 4U);
    if (doc->shape_count != 0U && (winding == NULL || parity == NULL || colors == NULL)) {
        free(winding); free(parity); free(colors);
        if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_OOM, 0};
        return false;
    }
    if (!raster_tile_create(doc->width, doc->height, out)) {
        free(winding); free(parity); free(colors);
        if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0};
        return false;
    }
    memset(out->rgba, 0, out->byte_count);
    double sx = (double)doc->width / doc->view_width;
    double sy = (double)doc->height / doc->view_height;
    double scale = fmin(sx, sy);
    double offset_x = ((double)doc->width - doc->view_width * scale) * 0.5;
    double offset_y = ((double)doc->height - doc->view_height * scale) * 0.5;
    for (uint32_t y = 0; y < doc->height; y++) {
        for (uint32_t x = 0; x < doc->width; x++) {
            if (!render_pixel(doc, x, y, scale, offset_x, offset_y, out,
                              winding, parity, colors, diag)) {
                raster_tile_destroy(out); free(winding); free(parity); free(colors);
                return false;
            }
        }
    }
    free(winding); free(parity); free(colors);
    if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_NONE, 0};
    return true;
}

static bool edge_crosses_right(geom_edge edge, double px, double py)
{
    double xcross = edge.a.x + (py - edge.a.y) * (edge.b.x - edge.a.x) /
                                      (edge.b.y - edge.a.y);
    return xcross > px;
}
