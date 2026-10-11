#include "shape_render_internal.h"

#include <math.h>
#include <stdlib.h>

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

static bool edge_box_contains(geom_edge edge, double px, double py,
                              double margin)
{
    return px >= fmin(edge.a.x, edge.b.x) - margin &&
           px <= fmax(edge.a.x, edge.b.x) + margin &&
           py >= fmin(edge.a.y, edge.b.y) - margin &&
           py <= fmax(edge.a.y, edge.b.y) + margin;
}

static bool dash_cache_contains(const svg_shape_document *doc, size_t shape,
                                const raster_shape_dash_cache *cache,
                                double px, double py)
{
    const svg_shape_stroke *stroke = &doc->strokes[shape];
    if (cache == NULL || cache->shapes == NULL || shape >= cache->shape_count)
        return false;
    const raster_dash_shape_range *range = &cache->shapes[shape];
    double radius = stroke->width * 0.5;
    const raster_stroke_grid_cell *cell =
        raster_shape_stroke_grid_lookup(range, px, py);
    if (range->grid_cells != NULL) {
        if (cell == NULL) return false;
        for (size_t offset = 0U; offset < cell->count; ++offset) {
            uint32_t encoded = range->grid_refs[cell->first + offset];
            if ((encoded & UINT32_C(0x80000000)) != 0U) {
                size_t index = encoded & ~UINT32_C(0x80000000);
                const raster_dash_join *join = &cache->joins[index];
                double reach = stroke->width * 0.5;
                if (stroke->join == SVG_SHAPE_JOIN_MITER)
                    reach *= stroke->miter_limit;
                if (fabs(px - join->vertex.x) <= reach &&
                    fabs(py - join->vertex.y) <= reach &&
                    join_hit(join->previous, join->vertex, join->next, px, py,
                             radius, stroke)) return true;
            } else {
                const raster_dash_segment *segment = &cache->segments[encoded];
                if (segment_hit_caps(segment->edge.a, segment->edge.b, px, py,
                        radius, segment->start_cap, segment->end_cap)) return true;
            }
        }
        return false;
    }
    for (size_t index = 0U; index < range->segment_count; ++index) {
        const raster_dash_segment *segment =
            &cache->segments[range->segment_first + index];
        double segment_margin = radius;
        if (segment->start_cap == SVG_SHAPE_CAP_SQUARE ||
            segment->end_cap == SVG_SHAPE_CAP_SQUARE)
            segment_margin *= 1.4142135623730951;
        if (edge_box_contains(segment->edge, px, py, segment_margin) &&
            segment_hit_caps(segment->edge.a, segment->edge.b, px, py, radius,
                             segment->start_cap, segment->end_cap)) return true;
    }
    double join_reach = radius;
    if (stroke->join == SVG_SHAPE_JOIN_MITER)
        join_reach *= stroke->miter_limit;
    for (size_t index = 0U; index < range->join_count; ++index) {
        const raster_dash_join *join = &cache->joins[range->join_first + index];
        if (fabs(px - join->vertex.x) <= join_reach &&
            fabs(py - join->vertex.y) <= join_reach &&
            join_hit(join->previous, join->vertex, join->next, px, py, radius,
                     stroke)) return true;
    }
    return false;
}

static bool stroke_range_contains(const svg_shape_document *doc, size_t shape,
                                  size_t first, size_t count, bool closed,
                                  bool implicit_close, bool dash_seam,
                                  const geom_edge *local_edges,
                                  double px, double py)
{
    const svg_shape_stroke *stroke = &doc->strokes[shape];
    if (stroke->rgba[3] == 0U || stroke->width <= 0.0 || count == 0U) return false;
    if (local_edges == NULL) return false;
    geom_affine inverse = doc->shape_stroke_inverse[shape];
    geom_point local_sample;
    if (!geom_affine_map_point(inverse, (geom_point){px, py}, &local_sample)) return false;
    px = local_sample.x;
    py = local_sample.y;
    double radius = stroke->width * 0.5;
    if (!closed && implicit_close && count > 0U) count--;
    if (count == 0U) return false;
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
    bool join_seam = closed && dash_seam;
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
            bool continues = on && end == length &&
                remain - step > RASTER_STROKE_DASH_EPSILON;
            bool crosses_vertex = continues && i + 1U < count;
            if (on) {
                geom_point a={edge.a.x+dx*(at/length),edge.a.y+dy*(at/length)};
                geom_point b={edge.a.x+dx*(end/length),edge.a.y+dy*(end/length)};
                bool starts_at_join = at == 0.0 &&
                    (joins_from_previous || (join_seam && i == 0U));
                bool ends_at_seam = end == length && join_seam && i + 1U == count;
                svg_shape_linecap start_cap = starts_at_join
                    ? SVG_SHAPE_CAP_BUTT : stroke->cap;
                svg_shape_linecap end_cap = (crosses_vertex || ends_at_seam)
                    ? SVG_SHAPE_CAP_BUTT : stroke->cap;
                if (segment_hit_caps(a,b,px,py,radius,start_cap,end_cap)) return true;
            }
            if (crosses_vertex) continues_at_vertex = true;
            at=end;
            remain -= step;
            if (remain <= RASTER_STROKE_DASH_EPSILON) {
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

bool raster_shape_stroke_contains(const svg_shape_document *doc, size_t shape,
                                  const geom_edge *local_edges,
                                  const bool *dash_seams,
                                  const raster_shape_dash_cache *dash_cache,
                                  double px, double py)
{
    const svg_shape_stroke *stroke = &doc->strokes[shape];
    if (stroke->rgba[3] == 0U || stroke->width <= 0.0) return false;
    if (dash_cache != NULL) {
        geom_point local_sample;
        if (!geom_affine_map_point(doc->shape_stroke_inverse[shape],
                                   (geom_point){px, py}, &local_sample))
            return false;
        return dash_cache_contains(doc, shape, dash_cache,
                                   local_sample.x, local_sample.y);
    }
    size_t subpath_count = doc->shape_subpath_count == NULL ? 0U :
                           doc->shape_subpath_count[shape];
    if (subpath_count != 0U) {
        size_t first_subpath = doc->shape_subpath_start[shape];
        for (size_t i = 0U; i < subpath_count; ++i) {
            size_t subpath_index = first_subpath + i;
            const svg_shape_path_subpath *path = &doc->path_subpaths[subpath_index];
            bool seam = dash_seams != NULL &&
                        dash_seams[doc->shape_count + subpath_index];
            if (stroke_range_contains(doc, shape, path->first_edge,
                                      path->edge_count, path->closed, true,
                                      seam, local_edges, px, py)) return true;
        }
        return false;
    }
    bool seam = dash_seams != NULL && dash_seams[shape];
    return stroke_range_contains(doc, shape, doc->shape_edge_start[shape],
        doc->shape_edge_count[shape], doc->shape_stroke_closed[shape],
        doc->shape_filled[shape], seam,
        local_edges, px, py);
}

size_t raster_shape_stroke_sample_work(const svg_shape_document *doc,
                                       size_t shape,
                                       const raster_shape_dash_cache *cache,
                                       double x, double y,
                                       bool *out_indexed,
                                       bool *out_has_candidates)
{
    if (out_indexed != NULL) *out_indexed = false;
    if (out_has_candidates != NULL) *out_has_candidates = false;
    if (doc == NULL || shape >= doc->shape_count || cache == NULL ||
        cache->shapes == NULL || shape >= cache->shape_count) return 0U;
    const svg_shape_stroke *stroke = &doc->strokes[shape];
    if (stroke->rgba[3] == 0U || stroke->width <= 0.0) return 0U;
    const raster_dash_shape_range *range = &cache->shapes[shape];
    if (range->grid_cells == NULL) {
        if (out_has_candidates != NULL)
            *out_has_candidates = range->segment_count != 0U ||
                                  range->join_count != 0U;
        return range->segment_count + range->join_count;
    }
    if (out_indexed != NULL) *out_indexed = true;
    geom_point local_sample;
    if (!geom_affine_map_point(doc->shape_stroke_inverse[shape],
                               (geom_point){x, y}, &local_sample)) return 0U;
    const raster_stroke_grid_cell *cell = raster_shape_stroke_grid_lookup(
        range, local_sample.x, local_sample.y);
    bool has_candidates = cell != NULL && cell->count != 0U;
    if (out_has_candidates != NULL) *out_has_candidates = has_candidates;
    return cell == NULL ? 1U : cell->count + 1U;
}

static bool dash_range_joins_seam(const svg_shape_stroke *stroke,
                                  const geom_edge *edges, size_t first,
                                  size_t count)
{
    if (stroke->dash_count == 0U || count == 0U) return false;
    size_t expanded = stroke->dash_count +
        (stroke->dash_count & 1U ? stroke->dash_count : 0U);
    double period = 0.0;
    for (size_t i = 0U; i < expanded; ++i)
        period += stroke->dash[i % stroke->dash_count];
    if (!isfinite(period) || period <= 0.0) return false;
    double phase = fmod(stroke->dash_offset, period);
    if (phase < 0.0) phase += period;
    size_t pattern = 0U;
    while (phase >= stroke->dash[pattern % stroke->dash_count]) {
        phase -= stroke->dash[pattern % stroke->dash_count];
        pattern = (pattern + 1U) % expanded;
    }
    bool starts_on = (pattern & 1U) == 0U;
    double remain = stroke->dash[pattern % stroke->dash_count] - phase;
    bool ends_on = false;
    for (size_t i = 0U; i < count; ++i) {
        geom_edge edge = edges[first + i];
        double length = hypot(edge.b.x-edge.a.x, edge.b.y-edge.a.y);
        while (length > 0.0) {
            double step = fmin(remain, length);
            ends_on = (pattern & 1U) == 0U;
            length -= step;
            remain -= step;
            if (remain <= RASTER_STROKE_DASH_EPSILON) {
                pattern = (pattern + 1U) % expanded;
                remain = stroke->dash[pattern % stroke->dash_count];
            }
        }
    }
    return starts_on && ends_on;
}

bool raster_shape_dash_subpath_joins_closed_seam(
    const svg_shape_document *doc, size_t shape, size_t subpath,
    const geom_edge *edges)
{
    const svg_shape_stroke *stroke = &doc->strokes[shape];
    if (doc->shape_subpath_count == NULL ||
        subpath >= doc->shape_subpath_count[shape]) return false;
    const svg_shape_path_subpath *path = &doc->path_subpaths[
        doc->shape_subpath_start[shape] + subpath];
    if (!path->closed) return false;
    return dash_range_joins_seam(stroke, edges, path->first_edge,
                                 path->edge_count);
}

bool raster_shape_dash_joins_closed_seam(const svg_shape_document *doc, size_t shape,
                                         const geom_edge *edges)
{
    const svg_shape_stroke *stroke = &doc->strokes[shape];
    size_t subpath_count = doc->shape_subpath_count == NULL ? 0U :
                           doc->shape_subpath_count[shape];
    if (subpath_count != 0U) {
        for (size_t i = 0U; i < subpath_count; ++i)
            if (raster_shape_dash_subpath_joins_closed_seam(doc, shape, i, edges))
                return true;
        return false;
    }
    if (!doc->shape_stroke_closed[shape]) return false;
    return dash_range_joins_seam(stroke, edges, doc->shape_edge_start[shape],
                                 doc->shape_edge_count[shape]);
}

static bool dash_size_add(size_t left, size_t right, size_t *out)
{
    if (right > SIZE_MAX - left) return false;
    *out = left + right;
    return true;
}

static bool dash_size_mul(size_t left, size_t right, size_t *out)
{
    if (left != 0U && right > SIZE_MAX / left) return false;
    *out = left * right;
    return true;
}

static bool dash_cache_bytes_fit(const raster_shape_dash_cache *cache,
                                 size_t segment_capacity,
                                 size_t join_capacity)
{
    size_t range_bytes, segment_bytes, join_bytes, total;
    return cache->shape_count <= SIZE_MAX / sizeof(*cache->shapes) &&
        dash_size_mul(cache->shape_count, sizeof(*cache->shapes), &range_bytes) &&
        dash_size_mul(segment_capacity, sizeof(*cache->segments), &segment_bytes) &&
        dash_size_mul(join_capacity, sizeof(*cache->joins), &join_bytes) &&
        dash_size_add(range_bytes, segment_bytes, &total) &&
        dash_size_add(total, join_bytes, &total) &&
        dash_size_add(total, cache->grid_bytes, &total) &&
        total <= RASTER_DASH_CACHE_BYTE_LIMIT;
}

static bool dash_cache_reserve_segments(raster_shape_dash_cache *cache,
                                        size_t needed,
                                        svg_shape_diagnostic *diagnostic)
{
    if (needed > RASTER_DASH_CACHE_MAX_ENTRIES) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
        return false;
    }
    if (needed <= cache->segment_capacity) return true;
    size_t capacity = cache->segment_capacity == 0U ? 32U :
                      cache->segment_capacity;
    while (capacity < needed && capacity < RASTER_DASH_CACHE_MAX_ENTRIES) {
        capacity = capacity > RASTER_DASH_CACHE_MAX_ENTRIES / 2U ?
                   RASTER_DASH_CACHE_MAX_ENTRIES : capacity * 2U;
    }
    if (!dash_cache_bytes_fit(cache, capacity, cache->join_capacity)) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
        return false;
    }
    raster_dash_segment *segments = realloc(cache->segments,
        capacity * sizeof(*segments));
    if (segments == NULL) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_OOM, 0U};
        return false;
    }
    cache->segments = segments;
    cache->segment_capacity = capacity;
    return true;
}

static bool dash_cache_reserve_joins(raster_shape_dash_cache *cache,
                                     size_t needed,
                                     svg_shape_diagnostic *diagnostic)
{
    if (needed > RASTER_DASH_CACHE_MAX_ENTRIES) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
        return false;
    }
    if (needed <= cache->join_capacity) return true;
    size_t capacity = cache->join_capacity == 0U ? 32U : cache->join_capacity;
    while (capacity < needed && capacity < RASTER_DASH_CACHE_MAX_ENTRIES) {
        capacity = capacity > RASTER_DASH_CACHE_MAX_ENTRIES / 2U ?
                   RASTER_DASH_CACHE_MAX_ENTRIES : capacity * 2U;
    }
    if (!dash_cache_bytes_fit(cache, cache->segment_capacity, capacity)) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
        return false;
    }
    raster_dash_join *joins = realloc(cache->joins, capacity * sizeof(*joins));
    if (joins == NULL) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_OOM, 0U};
        return false;
    }
    cache->joins = joins;
    cache->join_capacity = capacity;
    return true;
}

static bool dash_cache_add_step(raster_shape_dash_cache *cache, size_t amount,
                                svg_shape_diagnostic *diagnostic)
{
    if (amount > RASTER_DASH_CACHE_MAX_STEPS - cache->build_steps) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
        return false;
    }
    cache->build_steps += amount;
    return true;
}

static bool dash_cache_add_segment(raster_shape_dash_cache *cache,
                                   geom_point a, geom_point b,
                                   svg_shape_linecap start_cap,
                                   svg_shape_linecap end_cap,
                                   svg_shape_diagnostic *diagnostic)
{
    size_t needed;
    if (!dash_size_add(cache->segment_count, 1U, &needed) ||
        !dash_cache_reserve_segments(cache, needed, diagnostic)) return false;
    cache->segments[cache->segment_count++] = (raster_dash_segment){
        .edge = {a, b}, .start_cap = start_cap, .end_cap = end_cap};
    return true;
}

static bool dash_cache_add_join(raster_shape_dash_cache *cache,
                                geom_edge previous, geom_edge next,
                                svg_shape_diagnostic *diagnostic)
{
    size_t needed;
    if (!dash_size_add(cache->join_count, 1U, &needed) ||
        !dash_cache_reserve_joins(cache, needed, diagnostic)) return false;
    cache->joins[cache->join_count++] = (raster_dash_join){
        .previous = previous.a, .vertex = previous.b, .next = next.b};
    return true;
}

static bool dash_cache_build_range(raster_shape_dash_cache *cache,
                                   const svg_shape_stroke *stroke,
                                   const geom_edge *edges, size_t first,
                                   size_t count, bool closed,
                                   bool implicit_close, bool dash_seam,
                                   svg_shape_diagnostic *diagnostic)
{
    if (!closed && implicit_close && count != 0U) count--;
    if (count == 0U) return true;
    if (edges == NULL || stroke->dash_count == 0U ||
        stroke->dash_count > SVG_SHAPE_MAX_DASH_VALUES) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
        return false;
    }
    size_t pattern_count = stroke->dash_count;
    size_t expanded_count = pattern_count +
        ((pattern_count & 1U) != 0U ? pattern_count : 0U);
    double period = 0.0;
    if (!dash_cache_add_step(cache, expanded_count, diagnostic)) return false;
    for (size_t index = 0U; index < expanded_count; ++index) {
        double interval = stroke->dash[index % pattern_count];
        if (!isfinite(interval) || interval <= 0.0 ||
            !isfinite(period + interval)) {
            if (diagnostic != NULL)
                *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
            return false;
        }
        period += interval;
    }
    if (period <= 0.0) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
        return false;
    }
    double phase = fmod(stroke->dash_offset, period);
    if (!isfinite(phase)) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
        return false;
    }
    if (phase < 0.0) phase += period;
    size_t pattern = 0U;
    size_t pattern_advances = 0U;
    while (phase >= stroke->dash[pattern % pattern_count]) {
        if (pattern_advances++ >= expanded_count ||
            !dash_cache_add_step(cache, 1U, diagnostic)) {
            if (diagnostic != NULL && diagnostic->category == SVG_SHAPE_ERROR_NONE)
                *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
            return false;
        }
        phase -= stroke->dash[pattern % pattern_count];
        pattern = (pattern + 1U) % expanded_count;
    }
    double remain = stroke->dash[pattern % pattern_count] - phase;
    if (!isfinite(remain) || remain <= 0.0) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
        return false;
    }
    bool joins_from_previous = false;
    for (size_t edge_index = 0U; edge_index < count; ++edge_index) {
        if (!dash_cache_add_step(cache, 1U, diagnostic)) return false;
        geom_edge edge = edges[first + edge_index];
        double dx = edge.b.x - edge.a.x;
        double dy = edge.b.y - edge.a.y;
        double length = hypot(dx, dy);
        if (!isfinite(length)) {
            if (diagnostic != NULL)
                *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
            return false;
        }
        if (length == 0.0) continue;
        double at = 0.0;
        bool continues_at_vertex = false;
        while (at < length) {
            if (!dash_cache_add_step(cache, 1U, diagnostic)) return false;
            double step = fmin(remain, length - at);
            double end = at + step;
            if (!(step > 0.0) || !isfinite(end)) {
                if (diagnostic != NULL)
                    *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
                return false;
            }
            bool on = (pattern & 1U) == 0U;
            bool continues = on && end == length &&
                remain - step > RASTER_STROKE_DASH_EPSILON;
            bool crosses_vertex = continues && edge_index + 1U < count;
            if (on) {
                geom_point a = {edge.a.x + dx * (at / length),
                                edge.a.y + dy * (at / length)};
                geom_point b = {edge.a.x + dx * (end / length),
                                edge.a.y + dy * (end / length)};
                bool starts_at_join = at == 0.0 &&
                    (joins_from_previous || (dash_seam && edge_index == 0U));
                bool ends_at_seam = end == length && dash_seam &&
                    edge_index + 1U == count;
                svg_shape_linecap start_cap = starts_at_join ?
                    SVG_SHAPE_CAP_BUTT : stroke->cap;
                svg_shape_linecap end_cap = (crosses_vertex || ends_at_seam) ?
                    SVG_SHAPE_CAP_BUTT : stroke->cap;
                if (!dash_cache_add_segment(cache, a, b, start_cap, end_cap,
                                            diagnostic)) return false;
            }
            if (crosses_vertex) continues_at_vertex = true;
            at = end;
            remain -= step;
            if (remain <= RASTER_STROKE_DASH_EPSILON) {
                pattern = (pattern + 1U) % expanded_count;
                remain = stroke->dash[pattern % pattern_count];
            }
        }
        if (continues_at_vertex && edge_index + 1U < count &&
            !dash_cache_add_join(cache, edge, edges[first + edge_index + 1U],
                                 diagnostic)) return false;
        joins_from_previous = continues_at_vertex;
    }
    if (dash_seam &&
        !dash_cache_add_join(cache, edges[first + count - 1U], edges[first],
                             diagnostic)) return false;
    return true;
}

static bool dash_cache_build_solid_range(raster_shape_dash_cache *cache,
                                         const svg_shape_stroke *stroke,
                                         const geom_edge *edges, size_t first,
                                         size_t count, bool closed,
                                         bool implicit_close,
                                         svg_shape_diagnostic *diagnostic)
{
    if (!closed && implicit_close && count != 0U) count--;
    if (count == 0U) return true;
    if (edges == NULL) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
        return false;
    }
    for (size_t index = 0U; index < count; ++index) {
        if (!dash_cache_add_step(cache, 1U, diagnostic)) return false;
        svg_shape_linecap cap = (!closed &&
            (index == 0U || index + 1U == count)) ? stroke->cap :
            SVG_SHAPE_CAP_BUTT;
        geom_edge edge = edges[first + index];
        if (!dash_cache_add_segment(cache, edge.a, edge.b, cap, cap, diagnostic))
            return false;
    }
    size_t join_count = closed ? count : count - 1U;
    for (size_t index = 0U; index < join_count; ++index) {
        if (!dash_cache_add_step(cache, 1U, diagnostic)) return false;
        size_t next = closed ? (index + 1U) % count : index + 1U;
        geom_edge previous = edges[first + (next + count - 1U) % count];
        geom_edge following = edges[first + next];
        if (!dash_cache_add_join(cache, previous, following, diagnostic))
            return false;
    }
    return true;
}

void raster_shape_dash_cache_destroy(raster_shape_dash_cache *cache)
{
    if (cache == NULL) return;
    if (cache->shapes != NULL) {
        for (size_t shape = 0U; shape < cache->shape_count; ++shape) {
            free(cache->shapes[shape].grid_cells);
            free(cache->shapes[shape].grid_refs);
        }
    }
    free(cache->segments);
    free(cache->joins);
    free(cache->shapes);
    *cache = (raster_shape_dash_cache){0};
}

bool raster_shape_dash_cache_create(const svg_shape_document *doc,
                                    const geom_edge *local_edges,
                                    const bool *dash_seams,
                                    raster_shape_dash_cache *out_cache,
                                    svg_shape_diagnostic *diagnostic)
{
    if (out_cache == NULL || doc == NULL) return false;
    *out_cache = (raster_shape_dash_cache){0};
    out_cache->shape_count = doc->shape_count;
    if (doc->shape_count == 0U) return true;
    size_t range_bytes;
    if (!dash_size_mul(doc->shape_count, sizeof(*out_cache->shapes),
                       &range_bytes) || range_bytes > RASTER_DASH_CACHE_BYTE_LIMIT) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
        return false;
    }
    out_cache->shapes = calloc(doc->shape_count, sizeof(*out_cache->shapes));
    if (out_cache->shapes == NULL) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_OOM, 0U};
        return false;
    }
    for (size_t shape = 0U; shape < doc->shape_count; ++shape) {
        raster_dash_shape_range *range = &out_cache->shapes[shape];
        range->segment_first = out_cache->segment_count;
        range->join_first = out_cache->join_count;
        const svg_shape_stroke *stroke = &doc->strokes[shape];
        if (stroke->rgba[3] != 0U && stroke->width > 0.0) {
            bool dashed = stroke->dash_count != 0U;
            if (doc->shape_subpath_count[shape] != 0U) {
                size_t first_subpath = doc->shape_subpath_start[shape];
                size_t subpath_count = doc->shape_subpath_count[shape];
                for (size_t index = 0U; index < subpath_count; ++index) {
                    size_t subpath = first_subpath + index;
                    const svg_shape_path_subpath *path = &doc->path_subpaths[subpath];
                    bool seam = dash_seams != NULL &&
                        dash_seams[doc->shape_count + subpath];
                    bool built = dashed ?
                        dash_cache_build_range(out_cache, stroke, local_edges,
                            path->first_edge, path->edge_count, path->closed,
                            true, seam, diagnostic) :
                        dash_cache_build_solid_range(out_cache, stroke, local_edges,
                            path->first_edge, path->edge_count, path->closed,
                            true, diagnostic);
                    if (!built) goto fail;
                }
            } else {
                bool seam = dash_seams != NULL && dash_seams[shape];
                bool built = dashed ?
                    dash_cache_build_range(out_cache, stroke, local_edges,
                        doc->shape_edge_start[shape], doc->shape_edge_count[shape],
                        doc->shape_stroke_closed[shape], doc->shape_filled[shape],
                        seam, diagnostic) :
                    dash_cache_build_solid_range(out_cache, stroke, local_edges,
                        doc->shape_edge_start[shape], doc->shape_edge_count[shape],
                        doc->shape_stroke_closed[shape], doc->shape_filled[shape],
                        diagnostic);
                if (!built) goto fail;
            }
        }
        range->segment_count = out_cache->segment_count - range->segment_first;
        range->join_count = out_cache->join_count - range->join_first;
    }
    if (!raster_shape_stroke_grid_build(doc, out_cache, diagnostic)) goto fail;
    return true;

fail:
    raster_shape_dash_cache_destroy(out_cache);
    return false;
}
