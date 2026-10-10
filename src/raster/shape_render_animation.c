#include "shape_render.h"

#include "svg/animation/animation.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

static bool checked_bytes(size_t count, size_t stride, size_t *out)
{
    if (count != 0U && stride > SIZE_MAX / count) return false;
    *out = count * stride;
    return true;
}

static bool shape_in_target(const svg_shape_document *document,
                            size_t shape, const svg_anim_track *track)
{
    if (document->shape_sources == NULL || shape >= document->shape_count) return false;
    size_t offset = document->shape_sources[shape].offset;
    if (offset < track->target_offset || track->target_length > SIZE_MAX - track->target_offset)
        return false;
    return offset < track->target_offset + track->target_length;
}

static bool shape_is_direct_target(const svg_shape_document *document,
                                   size_t shape, const svg_anim_track *track)
{
    return document->shape_sources != NULL && shape < document->shape_count &&
           document->shape_sources[shape].offset == track->target_offset;
}

static void scale_unit(double *value)
{
    if (*value < 0.0) *value = 0.0;
    if (*value > 1.0) *value = 1.0;
}

static bool affine_inverse(geom_affine value, geom_affine *out)
{
    double determinant = value.a * value.d - value.b * value.c;
    if (out == NULL || !isfinite(determinant) || fabs(determinant) < 1.0e-15)
        return false;
    double inverse = 1.0 / determinant;
    *out = (geom_affine){value.d * inverse, -value.b * inverse,
        -value.c * inverse, value.a * inverse,
        (value.c * value.f - value.d * value.e) * inverse,
        (value.b * value.e - value.a * value.f) * inverse};
    return geom_affine_valid(*out);
}

static bool transform_matrix(const svg_anim_track *track,
                             const svg_anim_value *value, geom_affine *out)
{
    double x = value->number[0], y = value->number[1];
    if (!isfinite(x) || !isfinite(y) || fabs(x) > 1000000.0 ||
        fabs(y) > 1000000.0) return false;
    geom_affine matrix = geom_affine_identity();
    if (track->transform_type == SVG_ANIM_TRANSFORM_TRANSLATE) {
        matrix.e = x;
        matrix.f = y;
    } else if (track->transform_type == SVG_ANIM_TRANSFORM_SCALE) {
        matrix.a = x;
        matrix.d = value->number[3] >= 2.0 ? y : x;
    } else if (track->transform_type == SVG_ANIM_TRANSFORM_ROTATE) {
        double angle = x * 0.017453292519943295769;
        double cosine = cos(angle), sine = sin(angle);
        if (!isfinite(cosine) || !isfinite(sine)) return false;
        matrix = (geom_affine){cosine, sine, -sine, cosine, 0.0, 0.0};
        if (value->number[3] >= 3.0) {
            geom_affine to_center = geom_affine_identity();
            geom_affine from_center = geom_affine_identity();
            to_center.e = value->number[1];
            to_center.f = value->number[2];
            from_center.e = -value->number[1];
            from_center.f = -value->number[2];
            geom_affine intermediate;
            if (!geom_affine_compose(to_center, matrix, &intermediate) ||
                !geom_affine_compose(intermediate, from_center, &matrix)) return false;
        }
    } else {
        double angle = x * 0.017453292519943295769;
        double tangent = tan(angle);
        if (!isfinite(tangent) || fabs(tangent) > 1000000.0) return false;
        if (track->transform_type == SVG_ANIM_TRANSFORM_SKEW_X) matrix.c = tangent;
        else matrix.b = tangent;
    }
    *out = matrix;
    return geom_affine_valid(matrix);
}

static bool apply_matrix_to_shape(svg_shape_document *document, size_t shape,
                                  geom_affine matrix)
{
    size_t start = document->shape_edge_start[shape];
    size_t count = document->shape_edge_count[shape];
    if (start > document->edge_count || count > document->edge_count - start)
        return false;
    for (size_t index = start; index < start + count; ++index) {
        geom_edge *edge = &document->edges[index];
        geom_point first, second;
        if (!geom_affine_map_point(matrix, edge->a, &first) ||
            !geom_affine_map_point(matrix, edge->b, &second)) return false;
        edge->a = first;
        edge->b = second;
    }
    geom_affine inverse;
    if (document->shape_stroke_inverse != NULL &&
        affine_inverse(matrix, &inverse)) {
        geom_affine composed;
        if (!geom_affine_compose(document->shape_stroke_inverse[shape], inverse,
                                 &composed)) return false;
        document->shape_stroke_inverse[shape] = composed;
    }
    return true;
}

static bool bounds_for_shape(const svg_shape_document *document, size_t shape,
                             double bounds[4])
{
    size_t start = document->shape_edge_start[shape];
    size_t count = document->shape_edge_count[shape];
    if (start > document->edge_count || count > document->edge_count - start ||
        count == 0U) return false;
    bounds[0] = bounds[2] = document->edges[start].a.x;
    bounds[1] = bounds[3] = document->edges[start].a.y;
    for (size_t index = start; index < start + count; ++index) {
        const geom_point points[2] = {document->edges[index].a,
                                      document->edges[index].b};
        for (size_t point = 0U; point < 2U; ++point) {
            double x = points[point].x, y = points[point].y;
            if (!isfinite(x) || !isfinite(y)) return false;
            if (x < bounds[0]) bounds[0] = x;
            if (y < bounds[1]) bounds[1] = y;
            if (x > bounds[2]) bounds[2] = x;
            if (y > bounds[3]) bounds[3] = y;
        }
    }
    return true;
}

static bool matrix_for_numeric(const svg_shape_document *document, size_t shape,
                               const svg_anim_track *track,
                               const svg_anim_value *value, geom_affine *matrix)
{
    if (value->kind != SVG_ANIM_VALUE_NUMBER || !isfinite(value->number[0]) ||
        !isfinite(track->base_value.number[0])) return false;
    double delta = value->number[0] - track->base_value.number[0];
    if (fabs(delta) > 1000000.0) return false;
    *matrix = geom_affine_identity();
    switch (track->attribute) {
    case SVG_ANIM_ATTR_X:
    case SVG_ANIM_ATTR_CX: matrix->e = delta; break;
    case SVG_ANIM_ATTR_Y:
    case SVG_ANIM_ATTR_CY: matrix->f = delta; break;
    case SVG_ANIM_ATTR_WIDTH:
    case SVG_ANIM_ATTR_RX:
    case SVG_ANIM_ATTR_R: {
        double bounds[4];
        if (!(track->base_value.number[0] > 0.0) ||
            !bounds_for_shape(document, shape, bounds)) return false;
        double scale = value->number[0] / track->base_value.number[0];
        if (!isfinite(scale) || scale < 0.0 || scale > 1000000.0) return false;
        double center = track->attribute == SVG_ANIM_ATTR_R ?
                        (bounds[0] + bounds[2]) * 0.5 : bounds[0];
        matrix->a = scale;
        matrix->e = center * (1.0 - scale);
        if (track->attribute == SVG_ANIM_ATTR_R) {
            double center_y = (bounds[1] + bounds[3]) * 0.5;
            matrix->d = scale;
            matrix->f = center_y * (1.0 - scale);
        }
        break;
    }
    case SVG_ANIM_ATTR_HEIGHT:
    case SVG_ANIM_ATTR_RY: {
        double bounds[4];
        if (!(track->base_value.number[0] > 0.0) ||
            !bounds_for_shape(document, shape, bounds)) return false;
        double scale = value->number[0] / track->base_value.number[0];
        if (!isfinite(scale) || scale < 0.0 || scale > 1000000.0) return false;
        double center = bounds[1];
        matrix->d = scale;
        matrix->f = center * (1.0 - scale);
        break;
    }
    default: return false;
    }
    return geom_affine_valid(*matrix);
}

static bool apply_track(svg_shape_document *document,
                        const svg_anim_track *track,
                        const svg_anim_overlay_item *item)
{
    if (!item->active) return true;
    for (size_t shape = 0U; shape < document->shape_count; ++shape) {
        if (!shape_in_target(document, shape, track)) continue;
        if (track->kind == SVG_ANIM_KIND_TRANSFORM) {
            geom_affine matrix;
            if (!transform_matrix(track, &item->value, &matrix) ||
                !apply_matrix_to_shape(document, shape, matrix)) return false;
            continue;
        }
        if (track->kind == SVG_ANIM_KIND_MOTION) {
            if (item->value.kind != SVG_ANIM_VALUE_MOTION) return false;
            double angle = item->value.number[2] * 0.017453292519943295769;
            geom_affine rotate = {cos(angle), sin(angle), -sin(angle),
                                  cos(angle), 0.0, 0.0};
            geom_affine translate = {1.0, 0.0, 0.0, 1.0,
                                     item->value.number[0], item->value.number[1]};
            geom_affine matrix;
            if (!isfinite(angle) || !geom_affine_compose(translate, rotate, &matrix) ||
                !apply_matrix_to_shape(document, shape, matrix)) return false;
            continue;
        }
        if (track->attribute == SVG_ANIM_ATTR_D) return false;
        if (!shape_is_direct_target(document, shape, track)) continue;
        if (track->attribute == SVG_ANIM_ATTR_FILL &&
            item->value.kind == SVG_ANIM_VALUE_COLOR) {
            if (document->colors == NULL) return false;
            size_t start = document->shape_edge_start[shape];
            size_t count = document->shape_edge_count[shape];
            if (start > document->edge_count || count > document->edge_count - start)
                return false;
            for (size_t edge = start; edge < start + count; ++edge)
                memcpy(document->colors + edge * 4U, item->value.rgba, 4U);
        } else if (track->attribute == SVG_ANIM_ATTR_STROKE &&
                   item->value.kind == SVG_ANIM_VALUE_COLOR) {
            if (document->strokes == NULL) return false;
            memcpy(document->strokes[shape].rgba, item->value.rgba, 4U);
        } else if (track->attribute == SVG_ANIM_ATTR_OPACITY ||
                   track->attribute == SVG_ANIM_ATTR_FILL_OPACITY) {
            if (item->value.kind != SVG_ANIM_VALUE_NUMBER || document->colors == NULL)
                return false;
            double opacity = item->value.number[0];
            scale_unit(&opacity);
            size_t start = document->shape_edge_start[shape];
            size_t count = document->shape_edge_count[shape];
            if (start > document->edge_count || count > document->edge_count - start)
                return false;
            for (size_t edge = start; edge < start + count; ++edge) {
                uint8_t *alpha = &document->colors[edge * 4U + 3U];
                *alpha = (uint8_t)floor((double)*alpha * opacity + 0.5);
            }
            if (track->attribute == SVG_ANIM_ATTR_OPACITY && document->strokes != NULL) {
                uint8_t *alpha = &document->strokes[shape].rgba[3];
                *alpha = (uint8_t)floor((double)*alpha * opacity + 0.5);
            }
        } else if (track->attribute == SVG_ANIM_ATTR_STROKE_OPACITY) {
            if (item->value.kind != SVG_ANIM_VALUE_NUMBER || document->strokes == NULL)
                return false;
            double opacity = item->value.number[0];
            scale_unit(&opacity);
            uint8_t *alpha = &document->strokes[shape].rgba[3];
            *alpha = (uint8_t)floor((double)*alpha * opacity + 0.5);
        } else if (track->attribute == SVG_ANIM_ATTR_STROKE_WIDTH ||
                   track->attribute == SVG_ANIM_ATTR_STROKE_DASHOFFSET) {
            if (item->value.kind != SVG_ANIM_VALUE_NUMBER || document->strokes == NULL ||
                (track->attribute == SVG_ANIM_ATTR_STROKE_WIDTH &&
                 item->value.number[0] < 0.0) ||
                fabs(item->value.number[0]) > 1000000.0)
                return false;
            if (track->attribute == SVG_ANIM_ATTR_STROKE_WIDTH)
                document->strokes[shape].width = item->value.number[0];
            else document->strokes[shape].dash_offset = item->value.number[0];
        } else if (track->attribute == SVG_ANIM_ATTR_X ||
                   track->attribute == SVG_ANIM_ATTR_Y ||
                   track->attribute == SVG_ANIM_ATTR_WIDTH ||
                   track->attribute == SVG_ANIM_ATTR_HEIGHT ||
                   track->attribute == SVG_ANIM_ATTR_CX ||
                   track->attribute == SVG_ANIM_ATTR_CY ||
                   track->attribute == SVG_ANIM_ATTR_R ||
                   track->attribute == SVG_ANIM_ATTR_RX ||
                   track->attribute == SVG_ANIM_ATTR_RY) {
            geom_affine matrix;
            if (!matrix_for_numeric(document, shape, track, &item->value, &matrix) ||
                !apply_matrix_to_shape(document, shape, matrix)) return false;
        }
    }
    return true;
}

bool raster_shape_render_at(const svg_shape_document *document,
                            double time_seconds, raster_tile *out_tile,
                            svg_shape_diagnostic *diagnostic)
{
    if (document == NULL || out_tile == NULL || out_tile->rgba != NULL ||
        !isfinite(time_seconds) || time_seconds < 0.0 ||
        time_seconds > SVG_ANIM_MAX_TIMELINE_SECONDS) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U};
        return false;
    }
    svg_anim_overlay overlay = {0};
    if (!svg_anim_eval(&document->animation, time_seconds, &overlay)) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
        return false;
    }
    if (overlay.count == 0U) {
        svg_anim_overlay_destroy(&overlay);
        return raster_shape_render(document, out_tile, diagnostic);
    }
    svg_shape_document evaluated = *document;
    evaluated.edges = NULL;
    evaluated.colors = NULL;
    evaluated.strokes = NULL;
    evaluated.shape_stroke_inverse = NULL;
    size_t edge_bytes = 0U, color_bytes = 0U, stroke_bytes = 0U, inverse_bytes = 0U;
    if (!checked_bytes(document->edge_count, sizeof(*document->edges), &edge_bytes) ||
        !checked_bytes(document->edge_count, 4U, &color_bytes) ||
        !checked_bytes(document->shape_count, sizeof(*document->strokes), &stroke_bytes) ||
        !checked_bytes(document->shape_count, sizeof(*document->shape_stroke_inverse),
                       &inverse_bytes)) {
        svg_anim_overlay_destroy(&overlay);
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
        return false;
    }
    if (edge_bytes != 0U) {
        evaluated.edges = malloc(edge_bytes);
        if (evaluated.edges == NULL) goto oom;
        memcpy(evaluated.edges, document->edges, edge_bytes);
    }
    if (color_bytes != 0U && document->colors != NULL) {
        evaluated.colors = malloc(color_bytes);
        if (evaluated.colors == NULL) goto oom;
        memcpy(evaluated.colors, document->colors, color_bytes);
    }
    if (stroke_bytes != 0U && document->strokes != NULL) {
        evaluated.strokes = malloc(stroke_bytes);
        if (evaluated.strokes == NULL) goto oom;
        memcpy(evaluated.strokes, document->strokes, stroke_bytes);
    }
    if (inverse_bytes != 0U && document->shape_stroke_inverse != NULL) {
        evaluated.shape_stroke_inverse = malloc(inverse_bytes);
        if (evaluated.shape_stroke_inverse == NULL) goto oom;
        memcpy(evaluated.shape_stroke_inverse, document->shape_stroke_inverse,
               inverse_bytes);
    }
    for (size_t index = 0U; index < overlay.count; ++index) {
        size_t track_index = overlay.items[index].track_index;
        if (track_index >= document->animation.track_count ||
            !apply_track(&evaluated, &document->animation.tracks[track_index],
                         &overlay.items[index])) {
            if (diagnostic != NULL)
                *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_UNSUPPORTED,
                    track_index < document->animation.track_count ?
                    document->animation.tracks[track_index].element_offset : 0U};
            goto failed;
        }
    }
    bool result = raster_shape_render(&evaluated, out_tile, diagnostic);
    free(evaluated.edges);
    free(evaluated.colors);
    free(evaluated.strokes);
    free(evaluated.shape_stroke_inverse);
    svg_anim_overlay_destroy(&overlay);
    return result;

oom:
    if (diagnostic != NULL)
        *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_OOM, 0U};
failed:
    free(evaluated.edges);
    free(evaluated.colors);
    free(evaluated.strokes);
    free(evaluated.shape_stroke_inverse);
    svg_anim_overlay_destroy(&overlay);
    return false;
}
