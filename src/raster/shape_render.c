#include "shape_render.h"

#include <limits.h>
#include <math.h>
#include <stdlib.h>
#include <string.h>

#define RASTER_MAX_TESTS 200000000U

static bool edge_crosses_right(geom_edge edge, double px, double py);

static bool render_work_is_bounded(const svg_shape_document *doc)
{
    size_t width = doc->width;
    size_t height = doc->height;
    if (height == 0U || width > SIZE_MAX / height) return false;
    size_t pixels = width * height;
    if (doc->edge_count > SIZE_MAX - doc->shape_count) return false;
    size_t per_pixel = doc->edge_count + doc->shape_count;
    if (per_pixel == 0U) return true;
    return pixels <= (size_t)RASTER_MAX_TESTS / per_pixel;
}

static bool render_pixel(const svg_shape_document *doc, uint32_t x, uint32_t y,
                         double sx, double sy, raster_tile *tile, int32_t *winding,
                         uint8_t *parity, uint8_t *colors,
                         svg_shape_diagnostic *diag)
{
    if (doc->shape_count != 0U) {
        memset(winding, 0, doc->shape_count * sizeof(*winding));
        memset(parity, 0, doc->shape_count);
        memset(colors, 0, doc->shape_count * 4U);
    }
    double px = doc->view_x + ((double)x + 0.5) / sx;
    double py = doc->view_y + ((double)y + 0.5) / sy;
    for (size_t i = 0; i < doc->edge_count; i++) {
        geom_edge edge = doc->edges[i];
        bool upward = edge.a.y <= py && edge.b.y > py;
        bool downward = edge.b.y <= py && edge.a.y > py;
        if ((!upward && !downward) || !edge_crosses_right(edge, px, py)) continue;
        uint32_t shape = doc->shape_ids[i];
        if ((size_t)shape >= doc->shape_count) {
            if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0};
            return false;
        }
        winding[shape] += upward ? 1 : -1;
        parity[shape] ^= 1U;
        memcpy(colors + (size_t)shape * 4U, doc->colors + i * 4U, 4U);
    }
    size_t offset = ((size_t)y * doc->width + x) * 4U;
    for (size_t shape = 0; shape < doc->shape_count; shape++) {
        bool inside = doc->shape_evenodd[shape] ? parity[shape] != 0U : winding[shape] != 0;
        const uint8_t *fill = colors + shape * 4U;
        if (inside && fill[3] != 0U) memcpy(tile->rgba + offset, fill, 4U);
    }
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
        (doc->shape_count != 0U && doc->shape_evenodd == NULL) ||
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
    for (uint32_t y = 0; y < doc->height; y++) {
        for (uint32_t x = 0; x < doc->width; x++) {
            if (!render_pixel(doc, x, y, sx, sy, out, winding, parity, colors, diag)) {
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
