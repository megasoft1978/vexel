#include "shape_render.h"
#include "shape_bounds.h"
#include "shape_render_internal.h"
#include "shape_scanline.h"
#include "../svg/clip/clip.h"
#include "../svg/paint/paint.h"
#include "../svg/shape/shape_alloc.h"

#include <limits.h>
#include <math.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

#define RASTER_MAX_TESTS 200000000U
#define RASTER_SAMPLES_PER_AXIS 4U
#define RASTER_SAMPLE_COUNT (RASTER_SAMPLES_PER_AXIS * RASTER_SAMPLES_PER_AXIS)

static bool edge_crosses_right(geom_edge edge, double px, double py);

static void composite_source_over(uint8_t destination[4], const uint8_t source[4])
{
    uint32_t source_alpha = source[3];
    if (source_alpha == 0U) return;
    uint32_t destination_alpha = destination[3];
    uint32_t inverse_source_alpha = 255U - source_alpha;
    uint32_t output_alpha_numerator = source_alpha * 255U +
        destination_alpha * inverse_source_alpha;
    for (size_t channel = 0U; channel < 3U; ++channel) {
        uint32_t numerator = (uint32_t)source[channel] * source_alpha * 255U +
            (uint32_t)destination[channel] * destination_alpha *
                inverse_source_alpha;
        destination[channel] = (uint8_t)((numerator +
            output_alpha_numerator / 2U) / output_alpha_numerator);
    }
    destination[3] = (uint8_t)((output_alpha_numerator + 127U) / 255U);
}

static bool segment_hit_caps(geom_point a, geom_point b, double px, double py,
                             double radius, svg_shape_linecap start_cap,
                             svg_shape_linecap end_cap)
{
    double dx = b.x-a.x, dy = b.y-a.y, length = hypot(dx,dy);
    if (length == 0.0) {
        if (start_cap == SVG_SHAPE_CAP_ROUND || end_cap == SVG_SHAPE_CAP_ROUND)
            return hypot(px-a.x,py-a.y) <= radius;
        if (start_cap == SVG_SHAPE_CAP_SQUARE || end_cap == SVG_SHAPE_CAP_SQUARE)
            return fabs(px-a.x)<=radius && fabs(py-a.y)<=radius;
        return false;
    }
    double along = ((px-a.x)*dx + (py-a.y)*dy) / length;
    double across = fabs((px-a.x)*dy - (py-a.y)*dx) / length;
    double low = 0.0, high = length;
    if (start_cap == SVG_SHAPE_CAP_SQUARE) low -= radius;
    if (end_cap == SVG_SHAPE_CAP_SQUARE) high += radius;
    if (along >= low && along <= high && across <= radius) return true;
    if (start_cap == SVG_SHAPE_CAP_ROUND && hypot(px-a.x,py-a.y) <= radius) return true;
    if (end_cap == SVG_SHAPE_CAP_ROUND && hypot(px-b.x,py-b.y) <= radius) return true;
    return false;
}

static bool segment_hit(geom_point a, geom_point b, double px, double py,
                        double radius, svg_shape_linecap cap)
{
    return segment_hit_caps(a, b, px, py, radius, cap, cap);
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
                            const geom_edge *local_edges, const bool *dash_seams,
                            double px, double py)
{
    const svg_shape_stroke *stroke = &doc->strokes[shape];
    size_t first = doc->shape_edge_start[shape], count = doc->shape_edge_count[shape];
    if (stroke->rgba[3] == 0U || stroke->width <= 0.0 || count == 0U) return false;
    if (local_edges == NULL) return false;
    geom_affine inverse = doc->shape_stroke_inverse[shape];
    geom_point local_sample;
    if (!geom_affine_map_point(inverse, (geom_point){px, py}, &local_sample)) return false;
    px = local_sample.x;
    py = local_sample.y;
    double radius = stroke->width * 0.5;
    bool closed = doc->shape_stroke_closed[shape];
    if (!closed && doc->shape_filled[shape] && count > 0U) count--;
    size_t pattern_count = stroke->dash_count;
    if (pattern_count == 0U) {
        for (size_t i = 0; i < count; i++) {
            geom_edge e = local_edges[first+i];
            svg_shape_linecap cap = (!closed && (i==0U||i+1U==count)) ? stroke->cap : SVG_SHAPE_CAP_BUTT;
            if (segment_hit(e.a,e.b,px,py,radius,cap)) return true;
        }
        size_t joins = closed ? count : (count > 0U ? count-1U : 0U);
        for (size_t i=0; i<joins; i++) {
            size_t next_index=closed?i:(i+1U);
            geom_edge prev=local_edges[first+(next_index+count-1U)%count];
            geom_edge next=local_edges[first+next_index%count];
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
    bool join_seam = closed && dash_seams != NULL && dash_seams[shape];
    bool joins_from_previous = false;
    for (size_t i = 0; i < count; i++) {
        geom_edge edge = local_edges[first+i];
        double dx=edge.b.x-edge.a.x, dy=edge.b.y-edge.a.y, length=hypot(dx,dy), at=0.0;
        if (length == 0.0) continue;
        bool continues_at_vertex = false;
        while (at < length) {
            double step = fmin(remain, length - at);
            double end = at + step;
            bool on = (pattern & 1U) == 0U;
            bool continues = on && end == length && remain - step > 1e-12;
            if (on) {
                geom_point a={edge.a.x+dx*(at/length),edge.a.y+dy*(at/length)};
                geom_point b={edge.a.x+dx*(end/length),edge.a.y+dy*(end/length)};
                bool starts_at_join = at == 0.0 &&
                    (joins_from_previous || (join_seam && i == 0U));
                bool ends_at_seam = end == length && join_seam && i + 1U == count;
                svg_shape_linecap start_cap = starts_at_join
                    ? SVG_SHAPE_CAP_BUTT : stroke->cap;
                svg_shape_linecap end_cap = (continues || ends_at_seam)
                    ? SVG_SHAPE_CAP_BUTT : stroke->cap;
                if (segment_hit_caps(a,b,px,py,radius,start_cap,end_cap)) return true;
            }
            if (continues) continues_at_vertex = true;
            at=end;
            remain -= step;
            if (remain <= 1e-12) {
                pattern=(pattern+1U)%expanded_count;
                remain=stroke->dash[pattern % pattern_count];
            }
        }
        if (continues_at_vertex && i + 1U < count) {
            geom_edge next = local_edges[first+i+1U];
            if (join_hit(edge.a, edge.b, next.b, px, py, radius, stroke)) return true;
        }
        joins_from_previous = continues_at_vertex;
    }
    if (join_seam) {
        geom_edge previous = local_edges[first+count-1U];
        geom_edge next = local_edges[first];
        if (join_hit(previous.a, previous.b, next.b, px, py, radius, stroke)) return true;
    }
    return false;
}

static bool dash_joins_closed_seam(const svg_shape_document *doc, size_t shape,
                                   const geom_edge *edges)
{
    const svg_shape_stroke *stroke = &doc->strokes[shape];
    size_t count = stroke->dash_count;
    if (count == 0U || !doc->shape_stroke_closed[shape]) return false;
    size_t expanded = count + (count & 1U ? count : 0U);
    double period = 0.0;
    for (size_t i = 0U; i < expanded; i++) period += stroke->dash[i % count];
    if (!isfinite(period) || period <= 0.0) return false;
    double phase = fmod(stroke->dash_offset, period);
    if (phase < 0.0) phase += period;
    size_t pattern = 0U;
    while (phase >= stroke->dash[pattern % count]) {
        phase -= stroke->dash[pattern % count];
        pattern = (pattern + 1U) % expanded;
    }
    bool starts_on = (pattern & 1U) == 0U;
    double remain = stroke->dash[pattern % count] - phase;
    bool ends_on = false;
    size_t first = doc->shape_edge_start[shape];
    for (size_t i = 0U; i < doc->shape_edge_count[shape]; i++) {
        geom_edge edge = edges[first+i];
        double length = hypot(edge.b.x-edge.a.x, edge.b.y-edge.a.y);
        while (length > 0.0) {
            double step = fmin(remain, length);
            ends_on = (pattern & 1U) == 0U;
            length -= step;
            remain -= step;
            if (remain <= 1e-12) {
                pattern = (pattern + 1U) % expanded;
                remain = stroke->dash[pattern % count];
            }
        }
    }
    return starts_on && ends_on;
}

static const double sample_locations[RASTER_SAMPLES_PER_AXIS] = {
    0.125, 0.375, 0.625, 0.875
};

static bool work_add(size_t *total, size_t amount)
{
    if (amount > (size_t)RASTER_MAX_TESTS - *total) return false;
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

static bool compute_stroke_work(const svg_shape_document *doc,
                                const geom_edge *local_edges,
                                size_t *per_sample)
{
    for (size_t shape = 0U; shape < doc->shape_count; shape++) {
        per_sample[shape] = 0U;
        const svg_shape_stroke *stroke = &doc->strokes[shape];
        if (stroke->rgba[3] == 0U || stroke->width <= 0.0) continue;
        if (stroke->dash_count > SVG_SHAPE_MAX_DASH_VALUES) return false;
        size_t count = doc->shape_edge_count[shape];
        if (!doc->shape_stroke_closed[shape] && doc->shape_filled[shape] && count > 0U)
            count--;
        if (count > SIZE_MAX / 2U) return false;
        size_t cost = count * 2U;
        if (stroke->dash_count != 0U) {
            size_t expanded_count = stroke->dash_count +
                (stroke->dash_count & 1U ? stroke->dash_count : 0U);
            size_t pattern_work;
            if (count != 0U &&
                (!checked_mul_size(expanded_count, 2U, &pattern_work) ||
                 !checked_add_size(cost, pattern_work, &cost))) return false;
            double min_dash = stroke->dash[0];
            for (size_t d = 0U; d < stroke->dash_count; d++) {
                if (!isfinite(stroke->dash[d]) || stroke->dash[d] <= 0.0) return false;
                if (stroke->dash[d] < min_dash) min_dash = stroke->dash[d];
            }
            size_t first = doc->shape_edge_start[shape];
            for (size_t edge = 0U; edge < count; edge++) {
                geom_edge local = local_edges[first + edge];
                double spans = ceil(hypot(local.b.x-local.a.x,
                                          local.b.y-local.a.y) / min_dash);
                if (!isfinite(spans) || spans > (double)RASTER_MAX_TESTS ||
                    spans > (double)(SIZE_MAX - cost - 2U)) return false;
                size_t span_count = (size_t)spans;
                if (span_count > SIZE_MAX - cost - 2U) return false;
                cost += span_count + 2U;
            }
        }
        per_sample[shape] = cost;
    }
    return true;
}

static bool sample_is_in_view(const svg_shape_document *doc, double sample_x,
                              double sample_y, double scale, double offset_x,
                              double offset_y)
{
    double content_width = doc->view_width * scale;
    double content_height = doc->view_height * scale;
    return sample_x >= offset_x && sample_y >= offset_y &&
           sample_x < offset_x + content_width &&
           sample_y < offset_y + content_height;
}

static bool pixel_is_fully_in_view(const svg_shape_document *doc, uint32_t x,
                                   uint32_t y, double scale, double offset_x,
                                   double offset_y)
{
    double content_width = doc->view_width * scale;
    double content_height = doc->view_height * scale;
    return (double)x >= offset_x && (double)y >= offset_y &&
           (double)x + 1.0 <= offset_x + content_width &&
           (double)y + 1.0 <= offset_y + content_height;
}

static bool document_has_clips(const svg_shape_document *doc)
{
    if (doc->clips == NULL) return false;
    for (size_t shape = 0U; shape < doc->shape_count; shape++)
        if (doc->clips[shape] != NULL) return true;
    return false;
}

static bool pixel_may_cross_edge(const svg_shape_document *doc,
                                 double left, double top,
                                 double right, double bottom)
{
    for (size_t i = 0U; i < doc->edge_count; i++) {
        geom_edge edge = doc->edges[i];
        size_t shape = doc->shape_ids[i];
        double radius = 0.0;
        const svg_shape_stroke *stroke = &doc->strokes[shape];
        if (stroke->rgba[3] != 0U && stroke->width > 0.0) {
            geom_affine inverse = doc->shape_stroke_inverse[shape];
            double determinant = inverse.a * inverse.d - inverse.b * inverse.c;
            double a = inverse.d / determinant, b = -inverse.b / determinant;
            double c = -inverse.c / determinant, d = inverse.a / determinant;
            double scale_bound = sqrt(a*a + b*b + c*c + d*d);
            double join_bound = stroke->join == SVG_SHAPE_JOIN_MITER ?
                                stroke->miter_limit : 1.0;
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

static bool refine_clip_boundary_mask(const svg_shape_document *doc,
                                      double scale, double offset_x,
                                      double offset_y,
                                      raster_boundary_mask *boundary)
{
    if (!document_has_clips(doc) || boundary->marked_pixel_count == 0U)
        return true;
    if (doc->edge_count != 0U &&
        boundary->marked_pixel_count > SIZE_MAX / doc->edge_count) return false;
    size_t refine_work = boundary->marked_pixel_count * doc->edge_count;
    if (refine_work > (size_t)RASTER_MAX_TESTS ||
        boundary->mark_work > SIZE_MAX - refine_work) return false;
    boundary->mark_work += refine_work;
    for (size_t pixel = 0U; pixel < boundary->bit_count; pixel++) {
        size_t byte = pixel >> 3U;
        uint8_t bit = (uint8_t)(1U << (pixel & 7U));
        if ((boundary->bits[byte] & bit) == 0U) continue;
        uint32_t x = (uint32_t)(pixel % doc->width);
        uint32_t y = (uint32_t)(pixel / doc->width);
        if (!pixel_is_fully_in_view(doc, x, y, scale, offset_x, offset_y))
            continue;
        double left = doc->view_x + ((double)x - offset_x) / scale;
        double top = doc->view_y + ((double)y - offset_y) / scale;
        double right = doc->view_x + ((double)x + 1.0 - offset_x) / scale;
        double bottom = doc->view_y + ((double)y + 1.0 - offset_y) / scale;
        if (!pixel_may_cross_edge(doc, left, top, right, bottom)) {
            boundary->bits[byte] &= (uint8_t)~bit;
            boundary->marked_pixel_count--;
        }
    }
    return true;
}

static void paint_shape_sample(const svg_shape_document *doc, size_t shape,
                               const uint32_t *active_edges, size_t active_count,
                               const geom_edge *local_edges,
                               const bool *dash_seams, double px, double py,
                               bool scalar_oracle_alpha,
                               uint8_t sample_rgba[4])
{
    int32_t winding = 0;
    uint8_t parity = 0U;
    uint8_t fill[4] = {0U, 0U, 0U, 0U};
    for (size_t i = 0U; i < active_count; i++) {
        size_t edge_index = active_edges[i];
        geom_edge edge = doc->edges[edge_index];
        bool upward = edge.a.y <= py && edge.b.y > py;
        bool downward = edge.b.y <= py && edge.a.y > py;
        if ((!upward && !downward) || !edge_crosses_right(edge, px, py)) continue;
        winding += upward ? 1 : -1;
        parity ^= 1U;
        memcpy(fill, doc->colors + edge_index * 4U, sizeof(fill));
    }
    bool inside = doc->shape_evenodd[shape] ? parity != 0U : winding != 0;
    bool allowed = doc->clips == NULL || svg_clip_contains(doc->clips[shape], px, py);
    uint8_t sampled_fill[4];
    if (inside && allowed && fill[3] != 0U) {
        memcpy(sampled_fill, fill, sizeof(sampled_fill));
        bool gradient = doc->paints != NULL &&
            doc->paints[shape].kind != SVG_SHAPE_PAINT_SOLID;
        if (gradient) {
            svg_paint_sample(&doc->paints[shape], doc->paint_stops, px, py,
                             sampled_fill);
        }
        if (scalar_oracle_alpha && !gradient) {
            /* The test-only alpha mode preserves the frozen scalar oracle. */
            memcpy(sample_rgba, sampled_fill, sizeof(sampled_fill));
        } else composite_source_over(sample_rgba, sampled_fill);
    }
    if (allowed && stroke_contains(doc, shape, local_edges, dash_seams, px, py)) {
        if (scalar_oracle_alpha)
            memcpy(sample_rgba, doc->strokes[shape].rgba, 4U);
        else composite_source_over(sample_rgba, doc->strokes[shape].rgba);
    }
}

static size_t active_count_for(const size_t *shape_offsets, size_t stride,
                              size_t sample, size_t shape)
{
    size_t base = sample * stride;
    return shape_offsets[base + shape + 1U] - shape_offsets[base + shape];
}

static const uint32_t *active_edges_for(const size_t *shape_offsets,
                                       const uint32_t *active_edges,
                                       size_t stride, size_t sample,
                                       size_t shape)
{
    if (active_edges == NULL) return NULL;
    return active_edges + shape_offsets[sample * stride + shape];
}

static bool render_work_preflight(const svg_shape_document *doc,
                                  const raster_shape_bounds_set *bounds,
                                  const raster_boundary_mask *boundary,
                                  raster_scanline_index *scanlines,
                                  size_t *shape_offsets,
                                  uint32_t *active_edges,
                                  size_t active_edge_capacity,
                                  uint32_t *active_shapes,
                                  const size_t *stroke_cost,
                                  double scale, double offset_x,
                                  double offset_y,
                                  raster_shape_work_stats *stats)
{
    if ((size_t)doc->width > SIZE_MAX / (size_t)doc->height) return false;
    size_t total = 0U;
    size_t pixels = (size_t)doc->width * (size_t)doc->height;
    if (!work_add(&total, pixels) || !work_add(&total, bounds->pixel_count) ||
        !work_add(&total, boundary->mark_work) ||
        !work_add(&total, doc->edge_count) || !work_add(&total, doc->shape_count)) return false;
    size_t levels = 1U;
    size_t tree_size = doc->edge_count > doc->shape_count ?
                       doc->edge_count : doc->shape_count;
    for (size_t n = tree_size; n > 1U; n = (n + 1U) / 2U) levels++;
    size_t event_count;
    size_t event_factor;
    size_t event_work;
    size_t scanline_levels = 1U;
    for (size_t n = scanlines->scanline_count; n > 1U; n = (n + 1U) / 2U)
        scanline_levels++;
    size_t lookup_factor;
    size_t lookup_work;
    size_t stroke_setup_work = 0U;
    for (size_t shape = 0U; shape < doc->shape_count; shape++)
        if (!checked_add_size(stroke_setup_work, stroke_cost[shape],
                              &stroke_setup_work)) return false;
    if (!checked_add_size(scanlines->event_count, scanlines->shape_event_count,
                          &event_count) ||
        !checked_mul_size(levels, 6U, &event_factor) ||
        !checked_add_size(event_factor, 8U, &event_factor) ||
        !checked_mul_size(event_count, event_factor, &event_work) ||
        !work_add(&total, event_work) ||
        !checked_mul_size(scanline_levels, 2U, &lookup_factor) ||
        !checked_add_size(lookup_factor, 1U, &lookup_factor) ||
        !checked_mul_size(doc->edge_count, lookup_factor, &lookup_work) ||
        !work_add(&total, lookup_work) ||
        !work_add(&total, stroke_setup_work)) return false;
    size_t shape_pixels = 0U;
    size_t shape_samples = 0U;
    size_t fill_edge_tests = 0U;
    size_t stroke_tests = 0U;
    size_t shape_row_visits = 0U;
    size_t stride = doc->shape_count + 1U;
    size_t offsets_capacity;
    if (!checked_mul_size(stride, RASTER_SCANLINE_VALUES_PER_ROW, &offsets_capacity))
        return false;
    (void)offsets_capacity;
    raster_scanline_index_reset(scanlines);
    for (uint32_t y = 0U; y < doc->height; y++) {
        size_t listed_before = scanlines->listed_edge_count;
        size_t active_shape_count = 0U;
        if (!raster_scanline_index_build_row(scanlines, y, shape_offsets,
                                             active_edges, active_edge_capacity,
                                             active_shapes, doc->shape_count,
                                             &active_shape_count)) return false;
        if (!work_add(&total, scanlines->listed_edge_count - listed_before)) return false;
        size_t row_visits;
        if (!checked_add_size(shape_row_visits, active_shape_count, &shape_row_visits) ||
            !checked_mul_size(active_shape_count,
                              RASTER_SCANLINE_VALUES_PER_ROW + 1U, &row_visits) ||
            !work_add(&total, row_visits)) return false;
        for (size_t active = 0U; active < active_shape_count; active++) {
            size_t shape = active_shapes[active];
            const raster_pixel_bounds *box = &bounds->shapes[shape].pixels;
            if (!box->visible || y < box->y_first || y > box->y_last) return false;
            size_t sample_aux_work = 0U;
            if (doc->clips != NULL &&
                !checked_add_size(sample_aux_work,
                                  svg_clip_work_units(doc->clips[shape]),
                                  &sample_aux_work)) return false;
            if (doc->paints != NULL &&
                doc->paints[shape].kind != SVG_SHAPE_PAINT_SOLID &&
                !checked_add_size(sample_aux_work,
                                  doc->paints[shape].stop_count,
                                  &sample_aux_work)) return false;
            for (uint32_t x = box->x_first; x <= box->x_last; x++) {
                if (!checked_add_size(shape_pixels, 1U, &shape_pixels) ||
                    shape_pixels > bounds->pixel_count) return false;
                size_t pixel = (size_t)y * doc->width + x;
                bool center_only = pixel_is_fully_in_view(doc, x, y, scale,
                                                          offset_x, offset_y) &&
                                   !raster_boundary_mask_test(boundary, pixel);
                if (center_only) {
                    size_t edge_count = active_count_for(shape_offsets, stride, 2U, shape);
                    size_t sample_work;
                    if (!checked_add_size(1U, edge_count, &sample_work) ||
                        !checked_add_size(sample_work, stroke_cost[shape], &sample_work) ||
                        !checked_add_size(sample_work, sample_aux_work, &sample_work) ||
                        !checked_add_size(shape_samples, 1U, &shape_samples) ||
                        !checked_add_size(fill_edge_tests, edge_count, &fill_edge_tests) ||
                        !checked_add_size(stroke_tests, stroke_cost[shape], &stroke_tests) ||
                        !work_add(&total, sample_work)) return false;
                    continue;
                }
                for (size_t sy = 0U; sy < RASTER_SAMPLES_PER_AXIS; sy++) {
                    double sample_y = (double)y + sample_locations[sy];
                    size_t active_slot = sy < 2U ? sy : sy + 1U;
                    size_t edge_count = active_count_for(shape_offsets, stride,
                                                         active_slot, shape);
                    for (size_t sx = 0U; sx < RASTER_SAMPLES_PER_AXIS; sx++) {
                        double sample_x = (double)x + sample_locations[sx];
                        if (!sample_is_in_view(doc, sample_x, sample_y, scale,
                                               offset_x, offset_y)) continue;
                        size_t sample_work;
                        if (!checked_add_size(1U, edge_count, &sample_work) ||
                            !checked_add_size(sample_work, stroke_cost[shape], &sample_work) ||
                            !checked_add_size(sample_work, sample_aux_work, &sample_work) ||
                            !checked_add_size(shape_samples, 1U, &shape_samples) ||
                            !checked_add_size(fill_edge_tests, edge_count,
                                              &fill_edge_tests) ||
                            !checked_add_size(stroke_tests, stroke_cost[shape],
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
    stats->stroke_tests = (uint64_t)stroke_tests;
    stats->scanline_edge_entries = (uint64_t)scanlines->listed_edge_count;
    stats->shape_row_visits = (uint64_t)shape_row_visits;
    stats->boundary_pixel_marks = (uint64_t)boundary->mark_work;
    stats->total_work = (uint64_t)total;
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
    for (size_t i = 0; i < doc->shape_count; i++)
        needed |= doc->strokes[i].rgba[3] != 0U && doc->strokes[i].width > 0.0 &&
                  doc->shape_edge_count[i] != 0U;
    if (!needed || doc->edge_count == 0U) return true;
    if (doc->edge_count > SIZE_MAX / sizeof(**out_edges)) {
        if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0};
        return false;
    }
    geom_edge *edges = malloc(doc->edge_count * sizeof(*edges));
    bool *seams = calloc(doc->shape_count, sizeof(*seams));
    if (edges == NULL || seams == NULL) {
        free(edges); free(seams);
        if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_OOM, 0};
        return false;
    }
    for (size_t i = 0; i < doc->edge_count; i++) {
        edges[i] = doc->edges[i];
        size_t shape = doc->shape_ids[i];
        if (doc->strokes[shape].rgba[3] != 0U && doc->strokes[shape].width > 0.0 &&
            (!geom_affine_map_point(doc->shape_stroke_inverse[shape], edges[i].a, &edges[i].a) ||
             !geom_affine_map_point(doc->shape_stroke_inverse[shape], edges[i].b, &edges[i].b))) {
            free(edges); free(seams);
            if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0};
            return false;
        }
    }
    for (size_t i = 0U; i < doc->shape_count; i++)
        seams[i] = doc->strokes[i].rgba[3] != 0U &&
                   dash_joins_closed_seam(doc, i, edges);
    *out_edges = edges;
    *out_dash_seams = seams;
    return true;
}

static bool raster_shape_render_internal(const svg_shape_document *doc,
                                         raster_tile *out,
                                         svg_shape_diagnostic *diag,
                                         raster_shape_work_stats *work,
                                         bool scalar_oracle_alpha)
{
    if (out != NULL) *out = (raster_tile){0};
    if (work != NULL) *work = (raster_shape_work_stats){0};
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
    for (size_t i = 0U; i < doc->edge_count; i++) {
        if ((size_t)doc->shape_ids[i] >= doc->shape_count) {
            if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0};
            return false;
        }
    }
    for (size_t i = 0U; i < doc->shape_count; i++) {
        if (doc->shape_edge_start[i] > doc->edge_count ||
            doc->shape_edge_count[i] > doc->edge_count-doc->shape_edge_start[i] ||
            !isfinite(doc->strokes[i].width) || doc->strokes[i].width < 0.0 ||
            !isfinite(doc->strokes[i].miter_limit) || doc->strokes[i].miter_limit < 1.0 ||
            !isfinite(doc->strokes[i].dash_offset)) {
            if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0};
            return false;
        }
    }
    if ((size_t)doc->width > SIZE_MAX / (size_t)doc->height) {
        if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0};
        return false;
    }
    double scale = fmin((double)doc->width / doc->view_width,
                        (double)doc->height / doc->view_height);
    double offset_x = ((double)doc->width - doc->view_width * scale) * 0.5;
    double offset_y = ((double)doc->height - doc->view_height * scale) * 0.5;
    if (!isfinite(scale) || scale <= 0.0 || !isfinite(offset_x) || !isfinite(offset_y)) {
        if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0};
        return false;
    }

    bool success = false;
    geom_edge *local_edges = NULL;
    bool *dash_seams = NULL;
    size_t *stroke_cost = NULL;
    size_t *shape_offsets = NULL;
    uint32_t *active_edges = NULL;
    uint32_t *active_shapes = NULL;
    uint8_t *center_samples = NULL;
    uint8_t *subpixel_samples = NULL;
    raster_shape_bounds_set bounds = {0};
    raster_boundary_mask boundary = {0};
    raster_scanline_index scanlines = {0};

    if (!prepare_local_stroke_edges(doc, &local_edges, &dash_seams, diag)) goto cleanup;
    if (!raster_shape_bounds_build(doc, scale, offset_x, offset_y, &bounds, diag) ||
        !raster_shape_bounds_mark_boundaries(doc, scale, offset_x, offset_y,
                                             &boundary, diag) ||
        !raster_scanline_index_create(doc, &bounds, scale, offset_y, &scanlines, diag))
        goto cleanup;
    if (!refine_clip_boundary_mask(doc, scale, offset_x, offset_y, &boundary)) {
        if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0};
        goto cleanup;
    }
    if (doc->shape_count != 0U) {
        stroke_cost = calloc(doc->shape_count, sizeof(*stroke_cost));
        if (stroke_cost == NULL) {
            if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_OOM, 0};
            goto cleanup;
        }
    }
    if (!compute_stroke_work(doc, local_edges, stroke_cost)) {
        if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0};
        goto cleanup;
    }
    size_t offset_count;
    size_t edge_slot_count;
    size_t row_sample_bytes;
    size_t row_center_bytes;
    size_t stride;
    if (!checked_add_size(doc->shape_count, 1U, &stride) ||
        !checked_mul_size(stride, RASTER_SCANLINE_VALUES_PER_ROW, &offset_count) ||
        !checked_mul_size(doc->edge_count, RASTER_SCANLINE_VALUES_PER_ROW,
                          &edge_slot_count) ||
        !checked_mul_size((size_t)doc->width, RASTER_SAMPLE_COUNT * 4U,
                          &row_sample_bytes) ||
        !checked_mul_size((size_t)doc->width, 4U, &row_center_bytes) ||
        offset_count > SIZE_MAX / sizeof(*shape_offsets) ||
        edge_slot_count > SIZE_MAX / sizeof(*active_edges) ||
        doc->shape_count > SIZE_MAX / sizeof(*active_shapes)) {
        if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0};
        goto cleanup;
    }
    shape_offsets = calloc(offset_count, sizeof(*shape_offsets));
    if (edge_slot_count != 0U) active_edges = malloc(edge_slot_count * sizeof(*active_edges));
    if (doc->shape_count != 0U)
        active_shapes = malloc(doc->shape_count * sizeof(*active_shapes));
    center_samples = calloc(row_center_bytes, 1U);
    subpixel_samples = calloc(row_sample_bytes, 1U);
    if (shape_offsets == NULL || (edge_slot_count != 0U && active_edges == NULL) ||
        (doc->shape_count != 0U && active_shapes == NULL) ||
        center_samples == NULL || subpixel_samples == NULL) {
        if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_OOM, 0};
        goto cleanup;
    }
    raster_shape_work_stats measured = {0};
    if (!render_work_preflight(doc, &bounds, &boundary, &scanlines,
                               shape_offsets, active_edges, edge_slot_count,
                               active_shapes, stroke_cost, scale, offset_x, offset_y,
                               &measured)) {
        if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0};
        goto cleanup;
    }
    if (work != NULL) *work = measured;
    raster_scanline_index_reset(&scanlines);
    if (!raster_tile_create(doc->width, doc->height, out)) {
        if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0};
        goto cleanup;
    }
    svg_shape_alloc_note_external(out->rgba, out->byte_count);
    (void)svg_shape_alloc_set_tag(out->rgba, SVG_SHAPE_ALLOC_TAG_OUTPUT);
    memset(out->rgba, 0, out->byte_count);
    for (uint32_t y = 0U; y < doc->height; y++) {
        for (uint32_t x = 0U; x < doc->width; x++) {
            size_t pixel = (size_t)y * doc->width + x;
            bool center_only = pixel_is_fully_in_view(doc, x, y, scale,
                                                      offset_x, offset_y) &&
                               !raster_boundary_mask_test(&boundary, pixel);
            if (center_only) {
                memset(center_samples + (size_t)x * 4U, 0, 4U);
            } else {
                size_t sample_offset = (size_t)x * RASTER_SAMPLE_COUNT * 4U;
                memset(subpixel_samples + sample_offset, 0,
                       RASTER_SAMPLE_COUNT * 4U);
            }
        }
        size_t active_shape_count = 0U;
        if (!raster_scanline_index_build_row(&scanlines, y, shape_offsets,
                                             active_edges, edge_slot_count,
                                             active_shapes, doc->shape_count,
                                             &active_shape_count)) {
            if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0};
            goto cleanup;
        }
        for (size_t active = 0U; active < active_shape_count; active++) {
            size_t shape = active_shapes[active];
            const raster_pixel_bounds *box = &bounds.shapes[shape].pixels;
            for (uint32_t x = box->x_first; x <= box->x_last; x++) {
                size_t pixel_index = (size_t)y * doc->width + x;
                bool center_only = pixel_is_fully_in_view(doc, x, y, scale,
                                                          offset_x, offset_y) &&
                                   !raster_boundary_mask_test(&boundary, pixel_index);
                if (center_only) {
                    double sample_x = (double)x + 0.5;
                    double sample_y = (double)y + 0.5;
                    if (!sample_is_in_view(doc, sample_x, sample_y, scale,
                                           offset_x, offset_y)) continue;
                    double px = doc->view_x + (sample_x - offset_x) / scale;
                    double py = doc->view_y + (sample_y - offset_y) / scale;
                    size_t count = active_count_for(shape_offsets, stride, 2U, shape);
                    const uint32_t *edges = active_edges_for(shape_offsets, active_edges,
                                                              stride, 2U, shape);
                    paint_shape_sample(doc, shape, edges, count, local_edges,
                                       dash_seams, px, py, scalar_oracle_alpha,
                                       center_samples + (size_t)x * 4U);
                    continue;
                }
                for (size_t sy = 0U; sy < RASTER_SAMPLES_PER_AXIS; sy++) {
                    double sample_y = (double)y + sample_locations[sy];
                    size_t active_slot = sy < 2U ? sy : sy + 1U;
                    size_t count = active_count_for(shape_offsets, stride,
                                                    active_slot, shape);
                    const uint32_t *edges = active_edges_for(shape_offsets,
                        active_edges, stride, active_slot, shape);
                    for (size_t sx = 0U; sx < RASTER_SAMPLES_PER_AXIS; sx++) {
                        double sample_x = (double)x + sample_locations[sx];
                        if (!sample_is_in_view(doc, sample_x, sample_y, scale,
                                               offset_x, offset_y)) continue;
                        double px = doc->view_x + (sample_x - offset_x) / scale;
                        double py = doc->view_y + (sample_y - offset_y) / scale;
                        size_t sample_index = (size_t)x * RASTER_SAMPLE_COUNT +
                                              sy * RASTER_SAMPLES_PER_AXIS + sx;
                        paint_shape_sample(doc, shape, edges, count, local_edges,
                            dash_seams, px, py, scalar_oracle_alpha,
                            subpixel_samples + sample_index * 4U);
                    }
                }
            }
        }
        for (uint32_t x = 0U; x < doc->width; x++) {
            size_t pixel_index = (size_t)y * doc->width + x;
            size_t destination = pixel_index * 4U;
            if (pixel_is_fully_in_view(doc, x, y, scale, offset_x, offset_y) &&
                !raster_boundary_mask_test(&boundary, pixel_index)) {
                memcpy(out->rgba + destination, center_samples + (size_t)x * 4U, 4U);
                continue;
            }
            uint32_t red = 0U, green = 0U, blue = 0U, alpha = 0U, covered = 0U;
            for (size_t sample = 0U; sample < RASTER_SAMPLE_COUNT; sample++) {
                const uint8_t *rgba = subpixel_samples +
                    ((size_t)x * RASTER_SAMPLE_COUNT + sample) * 4U;
                if (rgba[3] != 0U) {
                    red += rgba[0];
                    green += rgba[1];
                    blue += rgba[2];
                    covered++;
                }
                alpha += rgba[3];
            }
            out->rgba[destination] = covered == 0U ? 0U :
                (uint8_t)((red + covered / 2U) / covered);
            out->rgba[destination + 1U] = covered == 0U ? 0U :
                (uint8_t)((green + covered / 2U) / covered);
            out->rgba[destination + 2U] = covered == 0U ? 0U :
                (uint8_t)((blue + covered / 2U) / covered);
            out->rgba[destination + 3U] =
                (uint8_t)((alpha + RASTER_SAMPLE_COUNT / 2U) / RASTER_SAMPLE_COUNT);
        }
    }
    if (diag != NULL) *diag = (svg_shape_diagnostic){SVG_SHAPE_ERROR_NONE, 0};
    success = true;

cleanup:
    if (!success && out != NULL && out->rgba != NULL) {
        svg_shape_alloc_forget_external(out->rgba);
        raster_tile_destroy(out);
    }
    free(local_edges);
    free(dash_seams);
    free(stroke_cost);
    free(shape_offsets);
    free(active_edges);
    free(active_shapes);
    free(center_samples);
    free(subpixel_samples);
    raster_shape_bounds_destroy(&bounds);
    raster_boundary_mask_destroy(&boundary);
    raster_scanline_index_destroy(&scanlines);
    return success;
}

bool raster_shape_render_with_stats(const svg_shape_document *document,
                                    raster_tile *out_tile,
                                    svg_shape_diagnostic *diagnostic,
                                    raster_shape_work_stats *work)
{
    return raster_shape_render_internal(document, out_tile, diagnostic, work,
                                        false);
}

bool raster_shape_render_with_stats_scalar_oracle_alpha(
    const svg_shape_document *document, raster_tile *out_tile,
    svg_shape_diagnostic *diagnostic, raster_shape_work_stats *work)
{
    return raster_shape_render_internal(document, out_tile, diagnostic, work,
                                        true);
}

bool raster_shape_render(const svg_shape_document *document, raster_tile *out_tile,
                         svg_shape_diagnostic *diagnostic)
{
    return raster_shape_render_internal(document, out_tile, diagnostic, NULL,
                                        false);
}

static bool edge_crosses_right(geom_edge edge, double px, double py)
{
    double xcross = edge.a.x + (py - edge.a.y) * (edge.b.x - edge.a.x) /
                                      (edge.b.y - edge.a.y);
    return xcross > px;
}
