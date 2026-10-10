#include "shape_internal.h"

#include "../clip/clip.h"

#include <math.h>
#include <string.h>

bool svg_shape_source_assign(svg_shape_document *document, size_t first_shape,
                             size_t end_shape, size_t source_offset,
                             size_t source_length)
{
    if (document == NULL || document->shape_sources == NULL ||
        first_shape > end_shape || end_shape > document->shape_count ||
        source_offset > document->source.len ||
        source_length > document->source.len - source_offset) return false;
    for (size_t shape = first_shape; shape < end_shape; ++shape) {
        document->shape_sources[shape].offset = source_offset;
        document->shape_sources[shape].length = source_length;
    }
    return true;
}

static bool view_transform(const svg_shape_document *document,
                           double *scale_x, double *scale_y,
                           double *offset_x, double *offset_y)
{
    if (document == NULL || document->width == 0U || document->height == 0U ||
        !isfinite(document->view_width) || !isfinite(document->view_height) ||
        document->view_width <= 0.0 || document->view_height <= 0.0)
        return false;
    *scale_x = (double)document->width / document->view_width;
    *scale_y = (double)document->height / document->view_height;
    *offset_x = 0.0;
    *offset_y = 0.0;
    svg_shape_aspect aspect = document->preserve_aspect;
    if (!aspect.set) aspect = (svg_shape_aspect){0.5, 0.5, false, false, true};
    if (!aspect.none) {
        double scale = aspect.slice ? fmax(*scale_x, *scale_y) :
                                     fmin(*scale_x, *scale_y);
        *scale_x = scale;
        *scale_y = scale;
        *offset_x = ((double)document->width - document->view_width * scale) *
                    aspect.align_x;
        *offset_y = ((double)document->height - document->view_height * scale) *
                    aspect.align_y;
    }
    return isfinite(*scale_x) && isfinite(*scale_y) &&
           *scale_x > 0.0 && *scale_y > 0.0 &&
           isfinite(*offset_x) && isfinite(*offset_y);
}

bool svg_shape_bounds_query(const svg_shape_document *document, size_t shape,
                            svg_shape_selection *out)
{
    if (document == NULL || out == NULL || shape >= document->shape_count ||
        document->shape_edge_start == NULL ||
        document->shape_edge_count == NULL || document->shape_sources == NULL ||
        document->edges == NULL) return false;
    size_t first = document->shape_edge_start[shape];
    size_t count = document->shape_edge_count[shape];
    if (first > document->edge_count || count > document->edge_count - first)
        return false;
    double min_x = 0.0, min_y = 0.0, max_x = 0.0, max_y = 0.0;
    if (count != 0U) {
        geom_point point = document->edges[first].a;
        min_x = max_x = point.x;
        min_y = max_y = point.y;
        for (size_t i = first; i < first + count; ++i) {
            const geom_point points[2] = {document->edges[i].a,
                                           document->edges[i].b};
            for (size_t p = 0U; p < 2U; ++p) {
                if (!geom_finite_point(points[p])) return false;
                if (points[p].x < min_x) min_x = points[p].x;
                if (points[p].x > max_x) max_x = points[p].x;
                if (points[p].y < min_y) min_y = points[p].y;
                if (points[p].y > max_y) max_y = points[p].y;
            }
        }
    }
    double sx, sy, ox, oy;
    if (!view_transform(document, &sx, &sy, &ox, &oy)) return false;
    double device_x0 = ox + (min_x - document->view_x) * sx;
    double device_y0 = oy + (min_y - document->view_y) * sy;
    double device_x1 = ox + (max_x - document->view_x) * sx;
    double device_y1 = oy + (max_y - document->view_y) * sy;
    if (!isfinite(device_x0) || !isfinite(device_y0) ||
        !isfinite(device_x1) || !isfinite(device_y1)) return false;
    const svg_shape_source_map *source = &document->shape_sources[shape];
    *out = (svg_shape_selection){
        .shape_index = shape,
        .source_offset = source->offset,
        .source_length = source->length,
        .user_bounds = {min_x, min_y, max_x - min_x, max_y - min_y},
        .device_bounds = {device_x0, device_y0,
                          device_x1 - device_x0, device_y1 - device_y0},
        .transform = source->transform
    };
    return true;
}

static double segment_distance(geom_point point, geom_edge edge)
{
    double dx = edge.b.x - edge.a.x;
    double dy = edge.b.y - edge.a.y;
    double length2 = dx * dx + dy * dy;
    if (length2 == 0.0) return hypot(point.x - edge.a.x,
                                     point.y - edge.a.y);
    double t = ((point.x - edge.a.x) * dx + (point.y - edge.a.y) * dy) /
               length2;
    if (t < 0.0) t = 0.0;
    if (t > 1.0) t = 1.0;
    return hypot(point.x - (edge.a.x + t * dx),
                 point.y - (edge.a.y + t * dy));
}

static bool fill_contains(const svg_shape_document *document, size_t shape,
                          double x, double y)
{
    size_t first = document->shape_edge_start[shape];
    size_t count = document->shape_edge_count[shape];
    int winding = 0;
    bool parity = false;
    bool visible = document->paints != NULL &&
        document->paints[shape].kind == SVG_SHAPE_PAINT_IMAGE;
    for (size_t i = first; i < first + count; ++i) {
        if (document->colors[i * 4U + 3U] != 0U) visible = true;
        geom_edge edge = document->edges[i];
        bool upward = edge.a.y <= y && edge.b.y > y;
        bool downward = edge.b.y <= y && edge.a.y > y;
        if (!upward && !downward) continue;
        double crossing = edge.a.x + (y - edge.a.y) *
                          (edge.b.x - edge.a.x) / (edge.b.y - edge.a.y);
        if (crossing <= x) continue;
        winding += upward ? 1 : -1;
        parity = !parity;
    }
    if (!visible || !document->shape_filled[shape]) return false;
    return document->shape_evenodd[shape] ? parity : winding != 0;
}

static bool stroke_contains(const svg_shape_document *document, size_t shape,
                            geom_point point, double tolerance)
{
    const svg_shape_stroke *stroke = &document->strokes[shape];
    if (stroke->rgba[3] == 0U || stroke->width <= 0.0) return false;
    geom_affine inverse = document->shape_stroke_inverse[shape];
    geom_point local;
    if (!geom_affine_map_point(inverse, point, &local)) return false;
    double inverse_norm = sqrt(inverse.a * inverse.a + inverse.b * inverse.b +
                               inverse.c * inverse.c + inverse.d * inverse.d);
    double radius = stroke->width * 0.5 + tolerance * inverse_norm;
    size_t first = document->shape_edge_start[shape];
    size_t count = document->shape_edge_count[shape];
    if (document->shape_subpath_count != NULL &&
        document->shape_subpath_count[shape] != 0U &&
        document->path_subpaths != NULL) {
        size_t subpath_first = document->shape_subpath_start[shape];
        size_t subpath_count = document->shape_subpath_count[shape];
        for (size_t i = subpath_first; i < subpath_first + subpath_count; ++i) {
            const svg_shape_path_subpath *subpath =
                &document->path_subpaths[i];
            size_t edges = subpath->edge_count;
            if (!subpath->closed && edges != 0U) edges--;
            for (size_t e = 0U; e < edges; ++e) {
                geom_edge edge = document->edges[subpath->first_edge + e];
                if (!geom_affine_map_point(inverse, edge.a, &edge.a) ||
                    !geom_affine_map_point(inverse, edge.b, &edge.b)) return false;
                if (segment_distance(local, edge) <= radius) return true;
            }
        }
        return false;
    }
    if (count > 0U && !document->shape_stroke_closed[shape]) count--;
    for (size_t i = 0U; i < count; ++i) {
        geom_edge edge = document->edges[first + i];
        if (!geom_affine_map_point(inverse, edge.a, &edge.a) ||
            !geom_affine_map_point(inverse, edge.b, &edge.b)) return false;
        if (segment_distance(local, edge) <= radius) return true;
    }
    return false;
}

bool svg_shape_hit_test(const svg_shape_document *document, double x, double y,
                        double tolerance, svg_shape_selection *out)
{
    if (document == NULL || out == NULL || !isfinite(x) || !isfinite(y) ||
        !isfinite(tolerance) || tolerance < 0.0 ||
        tolerance > 1000000.0 || document->shape_sources == NULL ||
        document->shape_edge_start == NULL ||
        document->shape_edge_count == NULL || document->shape_filled == NULL ||
        document->shape_evenodd == NULL || document->strokes == NULL ||
        document->shape_stroke_inverse == NULL) return false;
    for (size_t reverse = document->shape_count; reverse != 0U; --reverse) {
        size_t shape = reverse - 1U;
        bool allowed = svg_clip_contains(document->clips == NULL ? NULL :
                                         document->clips[shape], x, y);
        if (!allowed) continue;
        bool inside = fill_contains(document, shape, x, y);
        if (!inside && !stroke_contains(document, shape,
                (geom_point){x, y}, tolerance)) continue;
        return svg_shape_bounds_query(document, shape, out);
    }
    return false;
}

bool svg_shape_element_at_offset(const svg_shape_document *document,
                                 size_t source_offset,
                                 svg_shape_selection *out)
{
    if (document == NULL || out == NULL || document->shape_sources == NULL)
        return false;
    size_t best = SIZE_MAX;
    size_t best_length = SIZE_MAX;
    for (size_t shape = 0U; shape < document->shape_count; ++shape) {
        const svg_shape_source_map *source = &document->shape_sources[shape];
        if (source->length == 0U || source_offset < source->offset ||
            source_offset - source->offset >= source->length) continue;
        if (source->length < best_length ||
            (source->length == best_length && shape > best)) {
            best = shape;
            best_length = source->length;
        }
    }
    return best != SIZE_MAX && svg_shape_bounds_query(document, best, out);
}
