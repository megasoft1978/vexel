#include "shape_render.h"

#include <limits.h>
#include <math.h>
#include <stdlib.h>
#include <string.h>

#define RASTER_MAX_TESTS 200000000U
#define RASTER_SAMPLES_PER_AXIS 4U
#define RASTER_SAMPLE_COUNT (RASTER_SAMPLES_PER_AXIS * RASTER_SAMPLES_PER_AXIS)

static bool edge_crosses_right(geom_edge edge, double px, double py);

static bool segment_hit(geom_point a, geom_point b, double px, double py,
                        double radius, svg_shape_linecap cap)
{
    double dx = b.x-a.x, dy = b.y-a.y, length = hypot(dx,dy);
    if (length == 0.0) {
        if (cap == SVG_SHAPE_CAP_ROUND) return hypot(px-a.x,py-a.y) <= radius;
        if (cap == SVG_SHAPE_CAP_SQUARE)
            return fabs(px-a.x)<=radius && fabs(py-a.y)<=radius;
        return false;
    }
    double along = ((px-a.x)*dx + (py-a.y)*dy) / length;
    double across = fabs((px-a.x)*dy - (py-a.y)*dx) / length;
    double low = 0.0, high = length;
    if (cap == SVG_SHAPE_CAP_SQUARE) { low -= radius; high += radius; }
    if (along >= low && along <= high && across <= radius) return true;
    if (cap == SVG_SHAPE_CAP_ROUND &&
        (hypot(px-a.x,py-a.y) <= radius || hypot(px-b.x,py-b.y) <= radius)) return true;
    return false;
}

static bool in_triangle(double px, double py, geom_point a, geom_point b, geom_point c)
{
    double d1=(px-b.x)*(a.y-b.y)-(a.x-b.x)*(py-b.y);
    double d2=(px-c.x)*(b.y-c.y)-(b.x-c.x)*(py-c.y);
    double d3=(px-a.x)*(c.y-a.y)-(c.x-a.x)*(py-a.y);
    bool neg=d1<0.0||d2<0.0||d3<0.0, pos=d1>0.0||d2>0.0||d3>0.0;
    return !(neg&&pos);
}

static bool join_hit(geom_point previous, geom_point vertex, geom_point next,
                     double px, double py, double radius,
                     const svg_shape_stroke *stroke)
{
    if (stroke->join == SVG_SHAPE_JOIN_ROUND)
        return hypot(px-vertex.x,py-vertex.y) <= radius;
    double ax=vertex.x-previous.x, ay=vertex.y-previous.y;
    double bx=next.x-vertex.x, by=next.y-vertex.y;
    double al=hypot(ax,ay), bl=hypot(bx,by);
    if (al==0.0||bl==0.0) return false;
    double cross=ax*by-ay*bx;
    if (fabs(cross)<1e-12) return false;
    double side=cross>0.0?-1.0:1.0;
    geom_point p1={vertex.x+side*(-ay/al)*radius,vertex.y+side*(ax/al)*radius};
    geom_point p2={vertex.x+side*(-by/bl)*radius,vertex.y+side*(bx/bl)*radius};
    if (stroke->join==SVG_SHAPE_JOIN_BEVEL) return in_triangle(px,py,vertex,p1,p2);
    double denom=ax*by-ay*bx;
    double qx=p2.x-p1.x, qy=p2.y-p1.y;
    double t=(qx*by-qy*bx)/denom;
    geom_point miter={p1.x+t*ax,p1.y+t*ay};
    double ratio=hypot(miter.x-vertex.x,miter.y-vertex.y)/radius;
    if (!isfinite(ratio)||ratio>stroke->miter_limit)
        return in_triangle(px,py,vertex,p1,p2);
    return in_triangle(px,py,vertex,p1,miter)||in_triangle(px,py,vertex,miter,p2);
}

static bool stroke_contains(const svg_shape_document *doc, size_t shape,
                            double px, double py)
{
    const svg_shape_stroke *stroke = &doc->strokes[shape];
    geom_affine inverse = doc->shape_stroke_inverse[shape];
    geom_point local_sample;
    if (!geom_affine_map_point(inverse, (geom_point){px, py}, &local_sample)) return false;
    px = local_sample.x;
    py = local_sample.y;
    size_t first = doc->shape_edge_start[shape], count = doc->shape_edge_count[shape];
    if (stroke->rgba[3] == 0U || stroke->width <= 0.0 || count == 0U) return false;
    double radius = stroke->width * 0.5;
    bool closed = doc->shape_stroke_closed[shape];
    if (!closed && doc->shape_filled[shape] && count > 0U) count--;
    size_t pattern_count = stroke->dash_count;
    if (pattern_count == 0U) {
        for (size_t i = 0; i < count; i++) {
            geom_edge e = doc->edges[first+i];
            if (!geom_affine_map_point(inverse, e.a, &e.a) ||
                !geom_affine_map_point(inverse, e.b, &e.b)) return false;
            svg_shape_linecap cap = (!closed && (i==0U||i+1U==count)) ? stroke->cap : SVG_SHAPE_CAP_BUTT;
            if (segment_hit(e.a,e.b,px,py,radius,cap)) return true;
        }
        size_t joins = closed ? count : (count > 0U ? count-1U : 0U);
        for (size_t i=0; i<joins; i++) {
            size_t next_index=closed?i:(i+1U);
            geom_edge prev=doc->edges[first+(next_index+count-1U)%count];
            geom_edge next=doc->edges[first+next_index%count];
            if (!geom_affine_map_point(inverse, prev.a, &prev.a) ||
                !geom_affine_map_point(inverse, prev.b, &prev.b) ||
                !geom_affine_map_point(inverse, next.a, &next.a) ||
                !geom_affine_map_point(inverse, next.b, &next.b)) return false;
            if (join_hit(prev.a,prev.b,next.b,px,py,radius,stroke)) return true;
        }
        return false;
    }
    size_t expanded_count = pattern_count + (pattern_count & 1U ? pattern_count : 0U);
    double period = 0.0;
    for (size_t i=0; i<expanded_count; i++) period += stroke->dash[i % pattern_count];
    if (!isfinite(period) || period <= 0.0) return false;
    double phase = fmod(stroke->dash_offset, period);
    if (phase < 0.0) phase += period;
    size_t pattern = 0U;
    while (phase >= stroke->dash[pattern % pattern_count]) {
        phase -= stroke->dash[pattern % pattern_count];
        pattern = (pattern + 1U) % expanded_count;
    }
    double remain = stroke->dash[pattern % pattern_count] - phase;
    for (size_t i = 0; i < count; i++) {
        geom_edge edge = doc->edges[first+i];
        if (!geom_affine_map_point(inverse, edge.a, &edge.a) ||
            !geom_affine_map_point(inverse, edge.b, &edge.b)) return false;
        double dx=edge.b.x-edge.a.x, dy=edge.b.y-edge.a.y, length=hypot(dx,dy), at=0.0;
        if (length == 0.0) continue;
        while (at < length) {
            double step = fmin(remain, length - at);
            double end = at + step;
            if ((pattern & 1U) == 0U) {
                geom_point a={edge.a.x+dx*(at/length),edge.a.y+dy*(at/length)};
                geom_point b={edge.a.x+dx*(end/length),edge.a.y+dy*(end/length)};
                if (segment_hit(a,b,px,py,radius,stroke->cap)) return true;
            }
            at=end;
            remain -= step;
            if (remain <= 1e-12) {
                pattern=(pattern+1U)%expanded_count;
                remain=stroke->dash[pattern % pattern_count];
            }
        }
    }
    return false;
}

static bool render_work_is_bounded(const svg_shape_document *doc)
{
    size_t width = doc->width;
    size_t height = doc->height;
    if (height == 0U || width > SIZE_MAX / height) return false;
    size_t pixels = width * height;
    if (doc->edge_count > SIZE_MAX / (RASTER_SAMPLE_COUNT + 1U) ||
        doc->shape_count > SIZE_MAX / RASTER_SAMPLE_COUNT) return false;
    size_t edge_tests = doc->edge_count * (RASTER_SAMPLE_COUNT + 1U);
    size_t stroke_tests = 0U;
    if (doc->strokes != NULL && doc->shape_edge_count != NULL) {
        for (size_t i=0; i<doc->shape_count; i++) {
            if (doc->strokes[i].rgba[3] != 0U) {
                size_t count=doc->shape_edge_count[i];
                if (!doc->shape_stroke_closed[i] && doc->shape_filled[i] && count>0U) count--;
                size_t cost=count*2U;
                const svg_shape_stroke *stroke=&doc->strokes[i];
                if (stroke->dash_count>SVG_SHAPE_MAX_DASH_VALUES) return false;
                if (stroke->dash_count>0U) {
                    double min_dash=stroke->dash[0];
                    for (size_t d=0; d<stroke->dash_count; d++) {
                        if (!isfinite(stroke->dash[d]) || stroke->dash[d]<=0.0) return false;
                        if (stroke->dash[d]<min_dash) min_dash=stroke->dash[d];
                    }
                    size_t first=doc->shape_edge_start[i];
                    for (size_t edge=0; edge<count; edge++) {
                        geom_edge e=doc->edges[first+edge];
                        if (!geom_affine_map_point(doc->shape_stroke_inverse[i], e.a, &e.a) ||
                            !geom_affine_map_point(doc->shape_stroke_inverse[i], e.b, &e.b)) return false;
                        double spans=ceil(hypot(e.b.x-e.a.x,e.b.y-e.a.y)/min_dash);
                        if (!isfinite(spans) || spans>(double)(RASTER_MAX_TESTS/RASTER_SAMPLE_COUNT)) return false;
                        if ((size_t)spans>SIZE_MAX-cost-2U) return false;
                        cost+=(size_t)spans+2U;
                    }
                }
                if (cost>SIZE_MAX-stroke_tests) return false;
                stroke_tests+=cost;
            }
        }
        if (stroke_tests > SIZE_MAX / RASTER_SAMPLE_COUNT) return false;
        stroke_tests *= RASTER_SAMPLE_COUNT;
    }
    if (edge_tests > SIZE_MAX - stroke_tests) return false;
    edge_tests += stroke_tests;
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
        if (stroke_contains(doc, shape, px, py))
            memcpy(sample_rgba, doc->strokes[shape].rgba, 4U);
    }
    return true;
}

static bool pixel_may_cross_edge(const svg_shape_document *doc,
                                 double left, double top,
                                 double right, double bottom)
{
    for (size_t i = 0; i < doc->edge_count; i++) {
        geom_edge edge = doc->edges[i];
        uint32_t shape = doc->shape_ids[i];
        double radius = 0.0;
        const svg_shape_stroke *stroke = &doc->strokes[shape];
        if (stroke->rgba[3] != 0U && stroke->width > 0.0) {
            geom_affine inverse = doc->shape_stroke_inverse[shape];
            double determinant = inverse.a * inverse.d - inverse.b * inverse.c;
            double a = inverse.d / determinant, b = -inverse.b / determinant;
            double c = -inverse.c / determinant, d = inverse.a / determinant;
            double scale_bound = sqrt(a*a + b*b + c*c + d*d);
            double join_bound = stroke->join == SVG_SHAPE_JOIN_MITER ? stroke->miter_limit : 1.0;
            radius = stroke->width * 0.5 * scale_bound * join_bound;
            if (!isfinite(radius)) return true;
        }
        bool left_of = edge.a.x < left-radius && edge.b.x < left-radius;
        bool right_of = edge.a.x > right+radius && edge.b.x > right+radius;
        bool above = edge.a.y < top-radius && edge.b.y < top-radius;
        bool below = edge.a.y > bottom+radius && edge.b.y > bottom+radius;
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
         (doc->shape_evenodd == NULL || doc->shape_filled == NULL ||
          doc->shape_stroke_closed == NULL || doc->shape_edge_start == NULL ||
          doc->shape_edge_count == NULL || doc->shape_stroke_inverse == NULL ||
          doc->strokes == NULL)) ||
        !isfinite(doc->view_x) || !isfinite(doc->view_y) ||
        !isfinite(doc->view_width) || !isfinite(doc->view_height) ||
        doc->view_width <= 0.0 || doc->view_height <= 0.0) {
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
    for (size_t i=0; i<doc->shape_count; i++) {
        if (doc->shape_edge_start[i] > doc->edge_count ||
            doc->shape_edge_count[i] > doc->edge_count-doc->shape_edge_start[i] ||
            !isfinite(doc->strokes[i].width) || doc->strokes[i].width<0.0 ||
            !isfinite(doc->strokes[i].miter_limit) || doc->strokes[i].miter_limit<1.0 ||
            !isfinite(doc->strokes[i].dash_offset)) {
            if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0};
            return false;
        }
    }
    if (!render_work_is_bounded(doc)) {
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
