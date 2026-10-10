#include "shape_render.h"

#include "svg/animation/animation.h"
#include "svg/paint/paint.h"
#include "shape_render_animation_clips.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

#define ANIMATION_TRANSFORM_CHAIN_MAX 128U
#define ANIMATION_TRANSFORM_WORK_MAX 50000000U

static bool checked_bytes(size_t count, size_t stride, size_t *out)
{
    if (count != 0U && stride > SIZE_MAX / count) return false;
    *out = count * stride;
    return true;
}

static bool consume_work(size_t *work, size_t amount)
{
    if (work == NULL || *work >= ANIMATION_TRANSFORM_WORK_MAX ||
        amount > ANIMATION_TRANSFORM_WORK_MAX - *work) {
        if (work != NULL) *work = ANIMATION_TRANSFORM_WORK_MAX;
        return false;
    }
    *work += amount;
    return true;
}

static bool shape_in_target(const svg_shape_document *document,
                            size_t shape, const svg_anim_track *track,
                            size_t *work)
{
    if (document->shape_sources == NULL || shape >= document->shape_count ||
        track->target_length > SIZE_MAX - track->target_offset) return false;
    size_t target_end = track->target_offset + track->target_length;
    size_t offset = document->shape_sources[shape].offset;
    if (offset >= track->target_offset && offset < target_end) return true;
    size_t node_index = document->shape_sources[shape].transform_node;
    size_t visits = 0U;
    while (node_index != SVG_SHAPE_NO_TRANSFORM_NODE &&
           node_index < document->transform_node_count &&
           visits++ < ANIMATION_TRANSFORM_CHAIN_MAX) {
        if (!consume_work(work, 1U)) return false;
        const svg_shape_transform_frame *node =
            &document->transform_nodes[node_index];
        if (node->source_offset >= track->target_offset &&
            node->source_offset < target_end) return true;
        node_index = node->parent;
    }
    return false;
}

static bool shape_is_direct_target(const svg_shape_document *document,
                                   size_t shape, const svg_anim_track *track,
                                   size_t *work)
{
    if (document->shape_sources == NULL || shape >= document->shape_count)
        return false;
    size_t node_index = document->shape_sources[shape].transform_node;
    if (node_index < document->transform_node_count) {
        if (!consume_work(work, 1U)) return false;
        return document->transform_nodes[node_index].source_offset ==
               track->target_offset;
    }
    return node_index == SVG_SHAPE_NO_TRANSFORM_NODE &&
           document->shape_sources[shape].offset == track->target_offset;
}

static bool shape_target_node(const svg_shape_document *document, size_t shape,
                              const svg_anim_track *track, size_t *out_node,
                              size_t *work)
{
    if (document->shape_sources == NULL || shape >= document->shape_count ||
        document->transform_nodes == NULL || out_node == NULL) return false;
    size_t node_index = document->shape_sources[shape].transform_node;
    size_t visits = 0U;
    while (node_index != SVG_SHAPE_NO_TRANSFORM_NODE &&
           node_index < document->transform_node_count &&
           visits++ < ANIMATION_TRANSFORM_CHAIN_MAX) {
        if (!consume_work(work, 1U)) return false;
        const svg_shape_transform_frame *node =
            &document->transform_nodes[node_index];
        if (node->source_offset == track->target_offset) {
            *out_node = node_index;
            return true;
        }
        node_index = node->parent;
    }
    return false;
}

static bool shape_has_animated_effect_limit(const svg_shape_document *document,
                                            size_t shape, size_t *work,
                                            bool *found)
{
    *found = false;
    for (size_t index = 0U; index < document->opacity_group_count; ++index) {
        if (!consume_work(work, 1U)) return false;
        const svg_shape_opacity_group *group = &document->opacity_groups[index];
        if (shape >= group->first_shape && shape < group->end_shape &&
            (group->filter_index != SVG_SHAPE_NO_FILTER ||
             group->mask_index != SVG_SHAPE_NO_MASK)) {
            *found = true;
            return true;
        }
    }
    return true;
}

static bool shape_transform_product(const svg_shape_document *document,
                                    size_t shape, geom_affine *out,
                                    size_t *work)
{
    if (document->shape_sources == NULL || shape >= document->shape_count ||
        document->transform_nodes == NULL || out == NULL || work == NULL)
        return false;
    size_t chain[ANIMATION_TRANSFORM_CHAIN_MAX];
    size_t count = 0U;
    size_t node_index = document->shape_sources[shape].transform_node;
    while (node_index != SVG_SHAPE_NO_TRANSFORM_NODE) {
        if (node_index >= document->transform_node_count ||
            count == ANIMATION_TRANSFORM_CHAIN_MAX ||
            !consume_work(work, 1U)) return false;
        chain[count++] = node_index;
        node_index = document->transform_nodes[node_index].parent;
    }
    geom_affine result = geom_affine_identity();
    while (count != 0U) {
        const svg_shape_transform_frame *node =
            &document->transform_nodes[chain[--count]];
        geom_affine local, with_motion;
        if (!geom_affine_compose(node->attribute, node->suffix, &local) ||
            !geom_affine_compose(node->motion, local, &with_motion) ||
            !geom_affine_compose(result, with_motion, &result)) return false;
    }
    *out = result;
    return true;
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
                                  geom_affine matrix, size_t *work)
{
    size_t start = document->shape_edge_start[shape];
    size_t count = document->shape_edge_count[shape];
    if (start > document->edge_count || count > document->edge_count - start)
        return false;
    if (!consume_work(work, count)) return false;
    for (size_t index = start; index < start + count; ++index) {
        geom_edge *edge = &document->edges[index];
        geom_point first, second;
        if (!geom_affine_map_point(matrix, edge->a, &first) ||
            !geom_affine_map_point(matrix, edge->b, &second)) return false;
        edge->a = first;
        edge->b = second;
    }
    geom_affine inverse;
    if (document->shape_stroke_inverse != NULL) {
        if (!affine_inverse(matrix, &inverse)) return false;
        geom_affine composed;
        if (!geom_affine_compose(document->shape_stroke_inverse[shape], inverse,
                                 &composed)) return false;
        document->shape_stroke_inverse[shape] = composed;
    }
    if (document->paints != NULL &&
        document->paints[shape].kind != SVG_SHAPE_PAINT_SOLID) {
        geom_affine composed;
        if (!affine_inverse(matrix, &inverse) ||
            !geom_affine_compose(document->paints[shape].user_inverse, inverse,
                                 &composed)) return false;
        document->paints[shape].user_inverse = composed;
    }
    return true;
}

static bool apply_geometry_matrix(svg_shape_document *document, size_t shape,
                                  geom_affine matrix, size_t *work)
{
    size_t start = document->shape_edge_start[shape];
    size_t count = document->shape_edge_count[shape];
    if (start > document->edge_count || count > document->edge_count - start)
        return false;
    if (!consume_work(work, count)) return false;
    for (size_t index = start; index < start + count; ++index) {
        geom_edge *edge = &document->edges[index];
        geom_point first, second;
        if (!geom_affine_map_point(matrix, edge->a, &first) ||
            !geom_affine_map_point(matrix, edge->b, &second)) return false;
        edge->a = first;
        edge->b = second;
    }
    return true;
}

static bool bounds_for_shape(const svg_shape_document *document, size_t shape,
                             geom_affine world_inverse, double bounds[4],
                             size_t *work)
{
    size_t start = document->shape_edge_start[shape];
    size_t count = document->shape_edge_count[shape];
    if (start > document->edge_count || count > document->edge_count - start ||
        count == 0U) return false;
    if (!consume_work(work, count)) return false;
    geom_point first_local;
    if (!geom_affine_map_point(world_inverse, document->edges[start].a,
                               &first_local)) return false;
    bounds[0] = bounds[2] = first_local.x;
    bounds[1] = bounds[3] = first_local.y;
    for (size_t index = start; index < start + count; ++index) {
        const geom_point world_points[2] = {document->edges[index].a,
                                            document->edges[index].b};
        for (size_t point = 0U; point < 2U; ++point) {
            geom_point local;
            if (!geom_affine_map_point(world_inverse, world_points[point],
                                       &local)) return false;
            double x = local.x, y = local.y;
            if (x < bounds[0]) bounds[0] = x;
            if (y < bounds[1]) bounds[1] = y;
            if (x > bounds[2]) bounds[2] = x;
            if (y > bounds[3]) bounds[3] = y;
        }
    }
    return true;
}

static bool refresh_object_paint_bounds(svg_shape_document *document,
                                        size_t shape,
                                        geom_affine world_transform,
                                        size_t *work)
{
    if (document->paints == NULL) return true;
    svg_shape_paint *paint = &document->paints[shape];
    if (!paint->object_bounding_box ||
        (paint->kind != SVG_SHAPE_PAINT_LINEAR &&
         paint->kind != SVG_SHAPE_PAINT_RADIAL)) return true;
    geom_affine inverse;
    double bounds[4];
    if (!affine_inverse(world_transform, &inverse) ||
        !bounds_for_shape(document, shape, inverse, bounds, work) ||
        bounds[2] <= bounds[0] || bounds[3] <= bounds[1]) return false;
    memcpy(paint->bounds, bounds, sizeof(bounds));
    return true;
}

static bool matrix_for_numeric(const svg_shape_document *document, size_t shape,
                               const svg_anim_track *track,
                               const svg_anim_value *value,
                               geom_affine world_transform,
                               geom_affine *world_delta,
                               size_t *work)
{
    if (value->kind != SVG_ANIM_VALUE_NUMBER || !isfinite(value->number[0]) ||
        !isfinite(track->base_value.number[0])) return false;
    double delta = value->number[0] - track->base_value.number[0];
    if (fabs(delta) > 1000000.0) return false;
    geom_affine local_matrix = geom_affine_identity();
    switch (track->attribute) {
    case SVG_ANIM_ATTR_X:
    case SVG_ANIM_ATTR_CX: local_matrix.e = delta; break;
    case SVG_ANIM_ATTR_Y:
    case SVG_ANIM_ATTR_CY: local_matrix.f = delta; break;
    case SVG_ANIM_ATTR_WIDTH:
    case SVG_ANIM_ATTR_RX:
    case SVG_ANIM_ATTR_R: {
        double bounds[4];
        geom_affine inverse;
        if (!(track->base_value.number[0] > 0.0) ||
            !affine_inverse(world_transform, &inverse) ||
            !bounds_for_shape(document, shape, inverse, bounds, work)) return false;
        double scale = value->number[0] / track->base_value.number[0];
        if (!isfinite(scale) || scale < 0.0 || scale > 1000000.0) return false;
        double center = track->attribute == SVG_ANIM_ATTR_R ?
                        (bounds[0] + bounds[2]) * 0.5 : bounds[0];
        local_matrix.a = scale;
        local_matrix.e = center * (1.0 - scale);
        if (track->attribute == SVG_ANIM_ATTR_R) {
            double center_y = (bounds[1] + bounds[3]) * 0.5;
            local_matrix.d = scale;
            local_matrix.f = center_y * (1.0 - scale);
        }
        break;
    }
    case SVG_ANIM_ATTR_HEIGHT:
    case SVG_ANIM_ATTR_RY: {
        double bounds[4];
        geom_affine inverse;
        if (!(track->base_value.number[0] > 0.0) ||
            !affine_inverse(world_transform, &inverse) ||
            !bounds_for_shape(document, shape, inverse, bounds, work)) return false;
        double scale = value->number[0] / track->base_value.number[0];
        if (!isfinite(scale) || scale < 0.0 || scale > 1000000.0) return false;
        double center = bounds[1];
        local_matrix.d = scale;
        local_matrix.f = center * (1.0 - scale);
        break;
    }
    default: return false;
    }
    geom_affine inverse, mapped;
    return geom_affine_valid(local_matrix) &&
        affine_inverse(world_transform, &inverse) &&
        geom_affine_compose(world_transform, local_matrix, &mapped) &&
        geom_affine_compose(mapped, inverse, world_delta);
}

static bool apply_transform_track(svg_shape_document *document,
                                  const svg_anim_track *track,
                                  const svg_anim_overlay_item *item,
                                  bool *transform_seen,
                                  geom_affine *world_transforms,
                                  size_t *work)
{
    if (!item->active) return true;
    geom_affine animation = geom_affine_identity();
    if (track->kind == SVG_ANIM_KIND_TRANSFORM) {
        if (!transform_matrix(track, &item->value, &animation)) return false;
    } else if (track->kind == SVG_ANIM_KIND_MOTION) {
        if (item->value.kind != SVG_ANIM_VALUE_MOTION) return false;
        double angle = item->value.number[2] * 0.017453292519943295769;
        if (!isfinite(angle) || !isfinite(item->value.number[0]) ||
            !isfinite(item->value.number[1])) return false;
        geom_affine rotate = {cos(angle), sin(angle), -sin(angle),
                              cos(angle), 0.0, 0.0};
        geom_affine translate = {1.0, 0.0, 0.0, 1.0,
                                 item->value.number[0], item->value.number[1]};
        if (!geom_affine_compose(translate, rotate, &animation)) return false;
    } else return false;

    bool found = false;
    for (size_t node = 0U; node < document->transform_node_count; ++node) {
        if (!consume_work(work, 1U)) return false;
        svg_shape_transform_frame *target = &document->transform_nodes[node];
        if (target->source_offset != track->target_offset) continue;
        found = true;
        if (track->kind == SVG_ANIM_KIND_TRANSFORM) {
            if (track->additive_sum || transform_seen[node]) {
                if (!geom_affine_compose(target->attribute, animation,
                                         &target->attribute)) return false;
            } else target->attribute = animation;
            transform_seen[node] = true;
        } else if (!geom_affine_compose(target->motion, animation,
                                       &target->motion)) return false;
    }
    if (!found) return true;

    for (size_t shape = 0U; shape < document->shape_count; ++shape) {
        if (!consume_work(work, 1U)) return false;
        size_t target_node = SVG_SHAPE_NO_TRANSFORM_NODE;
        if (!shape_target_node(document, shape, track, &target_node, work)) {
            if (*work >= ANIMATION_TRANSFORM_WORK_MAX) return false;
            continue;
        }
        bool effect_limited = false;
        if (!shape_has_animated_effect_limit(document, shape, work,
                                             &effect_limited) ||
            effect_limited) return false;
        geom_affine next_world, old_inverse, delta;
        if (!shape_transform_product(document, shape, &next_world, work) ||
            !affine_inverse(world_transforms[shape], &old_inverse) ||
            !geom_affine_compose(next_world, old_inverse, &delta) ||
            !apply_matrix_to_shape(document, shape, delta, work)) return false;
        world_transforms[shape] = next_world;
        if (document->has_clips && document->clips != NULL &&
            !raster_animation_clips_update(document, shape, true, work,
                                          ANIMATION_TRANSFORM_WORK_MAX))
            return false;
        (void)target_node;
    }
    return true;
}

static bool apply_track(svg_shape_document *document,
                        const svg_anim_track *track,
                        const svg_anim_overlay_item *item,
                        bool *transform_seen,
                        geom_affine *world_transforms,
                        size_t *work)
{
    if (!item->active) return true;
    if (track->kind == SVG_ANIM_KIND_TRANSFORM ||
        track->kind == SVG_ANIM_KIND_MOTION)
        return apply_transform_track(document, track, item, transform_seen,
                                     world_transforms, work);
    bool descendant_found = false;
    bool direct_found = false;
    for (size_t shape = 0U; shape < document->shape_count; ++shape) {
        if (!consume_work(work, 1U)) return false;
        bool in_target = shape_in_target(document, shape, track, work);
        if (*work >= ANIMATION_TRANSFORM_WORK_MAX) return false;
        if (!in_target) continue;
        descendant_found = true;
        if (track->attribute == SVG_ANIM_ATTR_D) return false;
        bool direct_target = shape_is_direct_target(document, shape, track, work);
        if (*work >= ANIMATION_TRANSFORM_WORK_MAX) return false;
        if (!direct_target) continue;
        direct_found = true;
        bool handled = false;
        if (track->attribute == SVG_ANIM_ATTR_FILL &&
            item->value.kind == SVG_ANIM_VALUE_COLOR) {
            if (document->colors == NULL) return false;
            size_t start = document->shape_edge_start[shape];
            size_t count = document->shape_edge_count[shape];
            if (start > document->edge_count || count > document->edge_count - start)
                return false;
            if (!consume_work(work, count)) return false;
            for (size_t edge = start; edge < start + count; ++edge)
                memcpy(document->colors + edge * 4U, item->value.rgba, 4U);
            handled = true;
        } else if (track->attribute == SVG_ANIM_ATTR_STROKE &&
                   item->value.kind == SVG_ANIM_VALUE_COLOR) {
            if (document->strokes == NULL) return false;
            memcpy(document->strokes[shape].rgba, item->value.rgba, 4U);
            handled = true;
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
            if (!consume_work(work, count)) return false;
            for (size_t edge = start; edge < start + count; ++edge) {
                uint8_t *alpha = &document->colors[edge * 4U + 3U];
                *alpha = (uint8_t)floor((double)*alpha * opacity + 0.5);
            }
            if (track->attribute == SVG_ANIM_ATTR_OPACITY && document->strokes != NULL) {
                uint8_t *alpha = &document->strokes[shape].rgba[3];
                *alpha = (uint8_t)floor((double)*alpha * opacity + 0.5);
            }
            handled = true;
        } else if (track->attribute == SVG_ANIM_ATTR_STROKE_OPACITY) {
            if (item->value.kind != SVG_ANIM_VALUE_NUMBER || document->strokes == NULL)
                return false;
            double opacity = item->value.number[0];
            scale_unit(&opacity);
            uint8_t *alpha = &document->strokes[shape].rgba[3];
            *alpha = (uint8_t)floor((double)*alpha * opacity + 0.5);
            handled = true;
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
            handled = true;
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
            bool effect_limited = false;
            if (!shape_has_animated_effect_limit(document, shape, work,
                                                 &effect_limited) ||
                effect_limited) return false;
            if (!matrix_for_numeric(document, shape, track, &item->value,
                                    world_transforms[shape], &matrix, work) ||
                (document->paints != NULL &&
                 document->paints[shape].kind == SVG_SHAPE_PAINT_PATTERN) ||
                !apply_geometry_matrix(document, shape, matrix, work) ||
                !refresh_object_paint_bounds(document, shape,
                                             world_transforms[shape], work) ||
                (document->has_clips && document->clips != NULL &&
                 !raster_animation_clips_update(document, shape, true, work,
                                                ANIMATION_TRANSFORM_WORK_MAX)))
                return false;
            handled = true;
        }
        if (!handled) return false;
    }
    return !descendant_found || direct_found;
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
    evaluated.paints = NULL;
    evaluated.stroke_paints = NULL;
    evaluated.transform_nodes = NULL;
    evaluated.clips = document->clips;
    geom_affine *world_transforms = NULL;
    bool *transform_seen = NULL;
    bool clips_cloned = false;
    bool needs_clip_copy = false;
    size_t edge_bytes = 0U, color_bytes = 0U, stroke_bytes = 0U;
    size_t inverse_bytes = 0U, paint_bytes = 0U, stroke_paint_bytes = 0U;
    size_t transform_bytes = 0U, transform_seen_bytes = 0U, world_bytes = 0U;
    if (!checked_bytes(document->edge_count, sizeof(*document->edges), &edge_bytes) ||
        !checked_bytes(document->edge_count, 4U, &color_bytes) ||
        !checked_bytes(document->shape_count, sizeof(*document->strokes), &stroke_bytes) ||
        !checked_bytes(document->shape_count, sizeof(*document->shape_stroke_inverse),
                       &inverse_bytes) ||
        !checked_bytes(document->shape_count, sizeof(*document->paints), &paint_bytes) ||
        !checked_bytes(document->shape_count, sizeof(*document->stroke_paints),
                       &stroke_paint_bytes) ||
        !checked_bytes(document->transform_node_count,
                       sizeof(*document->transform_nodes), &transform_bytes) ||
        !checked_bytes(document->transform_node_count, sizeof(*transform_seen),
                       &transform_seen_bytes) ||
        !checked_bytes(document->shape_count, sizeof(geom_affine), &world_bytes)) {
        svg_anim_overlay_destroy(&overlay);
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
        return false;
    }
    if (document->has_clips && document->clips != NULL) {
        for (size_t index = 0U; index < overlay.count; ++index) {
            const svg_anim_overlay_item *item = &overlay.items[index];
            if (!item->active ||
                item->track_index >= document->animation.track_count) continue;
            const svg_anim_track *track =
                &document->animation.tracks[item->track_index];
            bool geometry = (track->kind == SVG_ANIM_KIND_ANIMATE ||
                             track->kind == SVG_ANIM_KIND_SET) &&
                (track->attribute == SVG_ANIM_ATTR_X ||
                 track->attribute == SVG_ANIM_ATTR_Y ||
                 track->attribute == SVG_ANIM_ATTR_WIDTH ||
                 track->attribute == SVG_ANIM_ATTR_HEIGHT ||
                 track->attribute == SVG_ANIM_ATTR_CX ||
                 track->attribute == SVG_ANIM_ATTR_CY ||
                 track->attribute == SVG_ANIM_ATTR_R ||
                 track->attribute == SVG_ANIM_ATTR_RX ||
                 track->attribute == SVG_ANIM_ATTR_RY);
            if (track->kind == SVG_ANIM_KIND_TRANSFORM ||
                track->kind == SVG_ANIM_KIND_MOTION || geometry) {
                needs_clip_copy = true;
                break;
            }
        }
    }
    if (needs_clip_copy) {
        size_t clone_bytes = 0U;
        bool clone_limit = false;
        if (!raster_animation_clips_clone(document, &evaluated, &clone_bytes,
                                          &clone_limit)) {
            if (diagnostic != NULL)
                *diagnostic = (svg_shape_diagnostic){
                    clone_limit ? SVG_SHAPE_ERROR_LIMIT : SVG_SHAPE_ERROR_OOM,
                    0U};
            goto failed;
        }
        clips_cloned = true;
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
    if (paint_bytes != 0U && document->paints != NULL) {
        evaluated.paints = malloc(paint_bytes);
        if (evaluated.paints == NULL) goto oom;
        memcpy(evaluated.paints, document->paints, paint_bytes);
    }
    if (stroke_paint_bytes != 0U && document->stroke_paints != NULL) {
        evaluated.stroke_paints = malloc(stroke_paint_bytes);
        if (evaluated.stroke_paints == NULL) goto oom;
        memcpy(evaluated.stroke_paints, document->stroke_paints,
               stroke_paint_bytes);
    }
    if (transform_bytes != 0U && document->transform_nodes != NULL) {
        evaluated.transform_nodes = malloc(transform_bytes);
        if (evaluated.transform_nodes == NULL) goto oom;
        memcpy(evaluated.transform_nodes, document->transform_nodes,
               transform_bytes);
    }
    if (transform_seen_bytes != 0U) {
        transform_seen = calloc(document->transform_node_count,
                                sizeof(*transform_seen));
        if (transform_seen == NULL) goto oom;
    }
    if (world_bytes != 0U) {
        world_transforms = malloc(world_bytes);
        if (world_transforms == NULL) goto oom;
        for (size_t shape = 0U; shape < document->shape_count; ++shape)
            world_transforms[shape] = document->shape_sources[shape].transform;
    }
    size_t work = 0U;
    for (size_t index = 0U; index < overlay.count; ++index) {
        size_t track_index = overlay.items[index].track_index;
        if (track_index >= document->animation.track_count ||
            !apply_track(&evaluated, &document->animation.tracks[track_index],
                         &overlay.items[index], transform_seen,
                         world_transforms, &work)) {
            if (diagnostic != NULL)
                *diagnostic = (svg_shape_diagnostic){
                    work >= ANIMATION_TRANSFORM_WORK_MAX ? SVG_SHAPE_ERROR_LIMIT :
                                                          SVG_SHAPE_ERROR_UNSUPPORTED,
                    track_index < document->animation.track_count ?
                    document->animation.tracks[track_index].element_offset : 0U};
            goto failed;
        }
    }
    bool result = raster_shape_render(&evaluated, out_tile, diagnostic);
    free(transform_seen);
    free(world_transforms);
    free(evaluated.edges);
    free(evaluated.colors);
    free(evaluated.strokes);
    free(evaluated.shape_stroke_inverse);
    free(evaluated.paints);
    free(evaluated.stroke_paints);
    free(evaluated.transform_nodes);
    if (clips_cloned)
        raster_animation_clips_release(document, &evaluated);
    svg_anim_overlay_destroy(&overlay);
    return result;

oom:
    if (diagnostic != NULL)
        *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_OOM, 0U};
failed:
    free(transform_seen);
    free(world_transforms);
    free(evaluated.edges);
    free(evaluated.colors);
    free(evaluated.strokes);
    free(evaluated.shape_stroke_inverse);
    free(evaluated.paints);
    free(evaluated.stroke_paints);
    free(evaluated.transform_nodes);
    if (clips_cloned)
        raster_animation_clips_release(document, &evaluated);
    svg_anim_overlay_destroy(&overlay);
    return false;
}
