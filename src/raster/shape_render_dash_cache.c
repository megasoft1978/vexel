#include "shape_render_internal.h"

#include <math.h>
#include <stdlib.h>

static bool dash_range_joins_seam(const svg_shape_stroke *stroke,
                                  const double *dash_values,
                                  const geom_edge *edges, size_t first,
                                  size_t count)
{
    if (stroke->dash_count == 0U || count == 0U || dash_values == NULL)
        return false;
    size_t expanded = stroke->dash_count +
        (stroke->dash_count & 1U ? stroke->dash_count : 0U);
    double period = 0.0;
    for (size_t i = 0U; i < expanded; ++i)
        period += dash_values[i % stroke->dash_count];
    if (!isfinite(period) || period <= 0.0) return false;
    double phase = fmod(stroke->dash_offset, period);
    if (phase < 0.0) phase += period;
    size_t pattern = 0U;
    while (phase >= dash_values[pattern % stroke->dash_count]) {
        phase -= dash_values[pattern % stroke->dash_count];
        pattern = (pattern + 1U) % expanded;
    }
    bool starts_on = (pattern & 1U) == 0U;
    double remain = dash_values[pattern % stroke->dash_count] - phase;
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
                remain = dash_values[pattern % stroke->dash_count];
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
    return dash_range_joins_seam(stroke,
                                 svg_shape_stroke_dash_values_for_shape(
                                     doc, shape), edges, path->first_edge,
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
    return dash_range_joins_seam(stroke,
                                 svg_shape_stroke_dash_values_for_shape(
                                     doc, shape), edges,
                                 doc->shape_edge_start[shape],
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

static bool dash_cache_capacity(size_t current, size_t needed,
                               size_t *out_capacity)
{
    size_t capacity = current == 0U ? 32U : current;
    while (capacity < needed) {
        size_t growth = capacity / 8U;
        if (growth == 0U) growth = 1U;
        if (growth > SIZE_MAX - capacity) return false;
        capacity += growth;
    }
    *out_capacity = capacity;
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
    if (needed <= cache->segment_capacity) return true;
    size_t capacity;
    if (!dash_cache_capacity(cache->segment_capacity, needed, &capacity)) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
        return false;
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
    if (needed <= cache->join_capacity) return true;
    size_t capacity;
    if (!dash_cache_capacity(cache->join_capacity, needed, &capacity)) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
        return false;
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
                                   const double *dash_values,
                                   const geom_edge *edges, size_t first,
                                   size_t count, bool closed,
                                   bool implicit_close, bool dash_seam,
                                   svg_shape_diagnostic *diagnostic)
{
    if (!closed && implicit_close && count != 0U) count--;
    if (count == 0U) return true;
    if (edges == NULL || dash_values == NULL || stroke->dash_count == 0U ||
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
        double interval = dash_values[index % pattern_count];
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
    while (phase >= dash_values[pattern % pattern_count]) {
        if (pattern_advances++ >= expanded_count ||
            !dash_cache_add_step(cache, 1U, diagnostic)) {
            if (diagnostic != NULL && diagnostic->category == SVG_SHAPE_ERROR_NONE)
                *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
            return false;
        }
        phase -= dash_values[pattern % pattern_count];
        pattern = (pattern + 1U) % expanded_count;
    }
    double remain = dash_values[pattern % pattern_count] - phase;
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
                remain = dash_values[pattern % pattern_count];
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
            const double *dash_values =
                svg_shape_stroke_dash_values_for_shape(doc, shape);
            if (dashed && dash_values == NULL) {
                if (diagnostic != NULL)
                    *diagnostic = (svg_shape_diagnostic){
                        SVG_SHAPE_ERROR_LIMIT, 0U};
                goto fail;
            }
            if (doc->shape_subpath_count[shape] != 0U) {
                size_t first_subpath = doc->shape_subpath_start[shape];
                size_t subpath_count = doc->shape_subpath_count[shape];
                for (size_t index = 0U; index < subpath_count; ++index) {
                    size_t subpath = first_subpath + index;
                    const svg_shape_path_subpath *path = &doc->path_subpaths[subpath];
                    bool seam = dash_seams != NULL &&
                        dash_seams[doc->shape_count + subpath];
                    bool built = dashed ?
                        dash_cache_build_range(out_cache, stroke, dash_values,
                            local_edges,
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
                    dash_cache_build_range(out_cache, stroke, dash_values,
                        local_edges,
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
