#include "shape_render_animation_clips.h"

#include "svg/clip/clip.h"

#include <math.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#define ANIMATION_CLIP_COPY_MAX_BYTES (64U * 1024U * 1024U)
#define ANIMATION_CLIP_CHAIN_MAX 128U
#define ANIMATION_CLIP_OWNER_DEPTH_MAX 64U

typedef struct clip_copy_entry {
    const svg_shape_clip *source;
    svg_shape_clip *copy;
} clip_copy_entry;

static bool checked_bytes(size_t count, size_t stride, size_t *out)
{
    if (count != 0U && stride > SIZE_MAX / count) return false;
    *out = count * stride;
    return true;
}

static bool reserve_clone_bytes(size_t bytes, size_t *total_bytes,
                                bool *limit)
{
    if (*total_bytes > ANIMATION_CLIP_COPY_MAX_BYTES ||
        bytes > ANIMATION_CLIP_COPY_MAX_BYTES - *total_bytes) {
        *limit = true;
        return false;
    }
    *total_bytes += bytes;
    return true;
}

static bool hash_capacity(size_t count, size_t *capacity)
{
    if (count > SIZE_MAX / 2U) return false;
    size_t needed = count * 2U;
    size_t result = 8U;
    while (result < needed) {
        if (result > SIZE_MAX / 2U) return false;
        result *= 2U;
    }
    *capacity = result;
    return true;
}

static size_t pointer_hash(const void *pointer)
{
    uintptr_t value = (uintptr_t)pointer;
    value ^= value >> 17U;
    value *= (uintptr_t)0xed5ad4bbU;
    value ^= value >> 11U;
    value *= (uintptr_t)0xac4c1b51U;
    value ^= value >> 15U;
    return (size_t)value;
}

void raster_animation_clips_release(const svg_shape_document *source,
                                    svg_shape_document *evaluated)
{
    if (source == NULL || evaluated == NULL ||
        evaluated->clips == source->clips) return;
    for (size_t shape = 0U; shape < evaluated->shape_count; ++shape)
        svg_clip_destroy(evaluated->clips == NULL ? NULL :
                         evaluated->clips[shape]);
    free(evaluated->clips);
    evaluated->clips = source->clips;
}

bool raster_animation_clips_clone(const svg_shape_document *source,
                                  svg_shape_document *evaluated,
                                  size_t *total_bytes, bool *limit)
{
    if (source == NULL || evaluated == NULL || total_bytes == NULL ||
        limit == NULL || source->clips == NULL || evaluated->clips != source->clips)
        return false;
    *limit = false;
    size_t capacity = 0U, pointer_bytes = 0U, table_bytes = 0U;
    if (!hash_capacity(source->shape_count, &capacity) ||
        !checked_bytes(source->shape_count, sizeof(*evaluated->clips),
                       &pointer_bytes) ||
        !checked_bytes(capacity, sizeof(clip_copy_entry), &table_bytes)) {
        *limit = true;
        return false;
    }
    if (pointer_bytes > SIZE_MAX - table_bytes ||
        !reserve_clone_bytes(pointer_bytes + table_bytes, total_bytes, limit))
        return false;
    svg_shape_clip **copies = source->shape_count == 0U ? NULL :
        calloc(source->shape_count, sizeof(*copies));
    clip_copy_entry *entries = calloc(capacity, sizeof(*entries));
    if ((source->shape_count != 0U && copies == NULL) || entries == NULL) {
        free(copies);
        free(entries);
        *total_bytes -= pointer_bytes + table_bytes;
        return false;
    }
    evaluated->clips = copies;
    size_t mask = capacity - 1U;
    bool ok = true;
    for (size_t shape = 0U; shape < source->shape_count; ++shape) {
        const svg_shape_clip *root = source->clips[shape];
        if (root == NULL) continue;
        size_t slot = pointer_hash(root) & mask;
        while (entries[slot].source != NULL && entries[slot].source != root)
            slot = (slot + 1U) & mask;
        svg_shape_clip *copy = entries[slot].copy;
        if (entries[slot].source == NULL) {
            bool clone_limit = false;
            copy = svg_clip_clone(root, total_bytes, &clone_limit);
            if (copy == NULL) {
                *limit = clone_limit;
                ok = false;
                break;
            }
            entries[slot] = (clip_copy_entry){root, copy};
        } else if (!svg_clip_retain(copy)) {
            ok = false;
            break;
        }
        copies[shape] = copy;
    }
    free(entries);
    if (!ok) {
        raster_animation_clips_release(source, evaluated);
        return false;
    }
    return true;
}

static bool add_work(size_t amount, size_t *work, size_t work_limit)
{
    if (work == NULL || *work >= work_limit || amount > work_limit - *work) {
        if (work != NULL) *work = work_limit;
        return false;
    }
    *work += amount;
    return true;
}

static bool affine_inverse(geom_affine value, geom_affine *out)
{
    double determinant = value.a * value.d - value.b * value.c;
    if (out == NULL || !isfinite(determinant) || determinant == 0.0)
        return false;
    double inverse = 1.0 / determinant;
    *out = (geom_affine){value.d * inverse, -value.b * inverse,
        -value.c * inverse, value.a * inverse,
        (value.c * value.f - value.d * value.e) * inverse,
        (value.b * value.e - value.a * value.f) * inverse};
    return geom_affine_valid(*out);
}

static bool node_product(const svg_shape_document *document, size_t node_index,
                         geom_affine *full, geom_affine *before_suffix,
                         size_t *work, size_t work_limit)
{
    size_t chain[ANIMATION_CLIP_CHAIN_MAX];
    size_t count = 0U;
    size_t current = node_index;
    while (current != SVG_SHAPE_NO_TRANSFORM_NODE) {
        if (current >= document->transform_node_count ||
            count >= ANIMATION_CLIP_CHAIN_MAX ||
            !add_work(1U, work, work_limit)) return false;
        chain[count++] = current;
        current = document->transform_nodes[current].parent;
    }
    geom_affine result = geom_affine_identity();
    geom_affine partial = geom_affine_identity();
    while (count != 0U) {
        size_t index = chain[--count];
        const svg_shape_transform_frame *node = &document->transform_nodes[index];
        geom_affine local;
        if (!geom_affine_compose(node->motion, node->attribute, &local))
            return false;
        if (index == node_index) {
            if (!geom_affine_compose(result, local, &partial)) return false;
        }
        if (!geom_affine_compose(local, node->suffix, &local) ||
            !geom_affine_compose(result, local, &result)) return false;
    }
    *full = result;
    *before_suffix = partial;
    return true;
}

static bool shape_has_node(const svg_shape_document *document, size_t shape,
                           size_t target, size_t *work, size_t work_limit,
                           bool *found)
{
    if (document->shape_sources == NULL || shape >= document->shape_count)
        return false;
    size_t node = document->shape_sources[shape].transform_node;
    size_t visits = 0U;
    *found = false;
    while (node != SVG_SHAPE_NO_TRANSFORM_NODE) {
        if (node >= document->transform_node_count ||
            visits++ >= ANIMATION_CLIP_CHAIN_MAX ||
            !add_work(1U, work, work_limit)) return false;
        if (node == target) {
            *found = true;
            return true;
        }
        node = document->transform_nodes[node].parent;
    }
    return true;
}

static bool bounds_for_node(const svg_shape_document *document, size_t node,
                            geom_affine inverse, double bounds[4],
                            size_t *work, size_t work_limit)
{
    bool have_point = false;
    for (size_t shape = 0U; shape < document->shape_count; ++shape) {
        bool contained = false;
        if (!shape_has_node(document, shape, node, work, work_limit,
                            &contained)) return false;
        if (!contained) continue;
        size_t first = document->shape_edge_start[shape];
        size_t count = document->shape_edge_count[shape];
        if (first > document->edge_count || count > document->edge_count - first)
            return false;
        for (size_t edge = first; edge < first + count; ++edge) {
            if (!add_work(1U, work, work_limit)) return false;
            geom_point points[2] = {document->edges[edge].a,
                                    document->edges[edge].b};
            for (size_t point = 0U; point < 2U; ++point) {
                geom_point local;
                if (!geom_affine_map_point(inverse, points[point], &local))
                    return false;
                if (!have_point) {
                    bounds[0] = bounds[2] = local.x;
                    bounds[1] = bounds[3] = local.y;
                    have_point = true;
                } else {
                    if (local.x < bounds[0]) bounds[0] = local.x;
                    if (local.y < bounds[1]) bounds[1] = local.y;
                    if (local.x > bounds[2]) bounds[2] = local.x;
                    if (local.y > bounds[3]) bounds[3] = local.y;
                }
            }
        }
    }
    return have_point;
}

static bool update_clip_owner(svg_shape_document *document, size_t node,
                              geom_affine full_inverse,
                              geom_affine attribute_inverse,
                              const double *bounds, size_t *work,
                              size_t work_limit)
{
    for (size_t shape = 0U; shape < document->shape_count; ++shape) {
        if (!add_work(ANIMATION_CLIP_OWNER_DEPTH_MAX, work, work_limit))
            return false;
        svg_clip_update_owner(document->clips[shape], node, full_inverse,
                              attribute_inverse, bounds);
    }
    return true;
}

bool raster_animation_clips_update(svg_shape_document *document, size_t shape,
                                   bool update_bounds, size_t *work,
                                   size_t work_limit)
{
    if (document == NULL || document->clips == NULL ||
        document->shape_sources == NULL || shape >= document->shape_count ||
        document->transform_nodes == NULL || work == NULL) return false;
    size_t node = document->shape_sources[shape].transform_node;
    size_t depth = 0U;
    while (node != SVG_SHAPE_NO_TRANSFORM_NODE) {
        if (node >= document->transform_node_count ||
            depth++ >= ANIMATION_CLIP_CHAIN_MAX) return false;
        geom_affine full, attribute, full_inverse, attribute_inverse;
        if (!node_product(document, node, &full, &attribute, work,
                          work_limit) ||
            !geom_affine_valid(full) || !geom_affine_valid(attribute))
            return false;
        double bounds[4];
        const double *updated_bounds = NULL;
        if (update_bounds) {
            if (!affine_inverse(full, &full_inverse) ||
                !affine_inverse(attribute, &attribute_inverse) ||
                !bounds_for_node(document, node, full_inverse, bounds, work,
                                 work_limit)) return false;
            updated_bounds = bounds;
        } else if (!affine_inverse(full, &full_inverse) ||
                   !affine_inverse(attribute, &attribute_inverse)) {
            return false;
        }
        if (!update_clip_owner(document, node, full_inverse,
                               attribute_inverse, updated_bounds, work,
                               work_limit)) return false;
        node = document->transform_nodes[node].parent;
    }
    return true;
}
