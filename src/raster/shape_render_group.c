#include "shape_render_internal.h"
#include "../svg/paint/paint.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

#define GROUP_SCRATCH_LIMIT (64U * 1024U * 1024U)

typedef struct group_render_context {
    const svg_shape_document *document;
    size_t scratch_bytes;
    size_t tile_bytes;
    raster_shape_work_stats *work;
    bool scalar_oracle_alpha;
    svg_shape_diagnostic *diagnostic;
} group_render_context;

typedef struct subset_storage {
    uint32_t *shape_ids;
    size_t *shape_edge_start;
    size_t *shape_subpath_start;
    svg_shape_path_subpath *path_subpaths;
} subset_storage;

static bool reserve_scratch(group_render_context *context)
{
    if (context->tile_bytes > GROUP_SCRATCH_LIMIT - context->scratch_bytes)
        return false;
    context->scratch_bytes += context->tile_bytes;
    return true;
}

static void release_scratch(group_render_context *context)
{
    if (context->scratch_bytes >= context->tile_bytes)
        context->scratch_bytes -= context->tile_bytes;
}

static bool add_stat(uint64_t *target, uint64_t value)
{
    if (value > UINT64_MAX - *target) return false;
    *target += value;
    return true;
}

static bool add_stats(raster_shape_work_stats *target,
                      const raster_shape_work_stats *value)
{
    return add_stat(&target->shape_pixels, value->shape_pixels) &&
        add_stat(&target->shape_samples, value->shape_samples) &&
        add_stat(&target->fill_edge_tests, value->fill_edge_tests) &&
        add_stat(&target->stroke_tests, value->stroke_tests) &&
        add_stat(&target->scanline_edge_entries, value->scanline_edge_entries) &&
        add_stat(&target->shape_row_visits, value->shape_row_visits) &&
        add_stat(&target->boundary_mark_work, value->boundary_mark_work) &&
        add_stat(&target->total_work, value->total_work);
}

static void source_over(uint8_t destination[4], const uint8_t source[4])
{
    uint32_t source_alpha = source[3];
    if (source_alpha == 0U) return;
    uint32_t destination_alpha = destination[3];
    uint32_t inverse_source = 255U - source_alpha;
    uint32_t output_alpha = source_alpha * 255U + destination_alpha * inverse_source;
    for (size_t channel = 0U; channel < 3U; ++channel) {
        uint32_t numerator = (uint32_t)source[channel] * source_alpha * 255U +
            (uint32_t)destination[channel] * destination_alpha * inverse_source;
        destination[channel] = (uint8_t)((numerator + output_alpha / 2U) /
                                          output_alpha);
    }
    destination[3] = (uint8_t)((output_alpha + 127U) / 255U);
}

static void composite_tile(raster_tile *destination, const raster_tile *source,
                           double opacity)
{
    for (size_t offset = 0U; offset < source->byte_count; offset += 4U) {
        uint8_t rgba[4];
        memcpy(rgba, source->rgba + offset, sizeof(rgba));
        rgba[3] = (uint8_t)lround((double)rgba[3] * opacity);
        source_over(destination->rgba + offset, rgba);
    }
}

static void subset_destroy(subset_storage *storage)
{
    free(storage->shape_ids);
    free(storage->shape_edge_start);
    free(storage->shape_subpath_start);
    free(storage->path_subpaths);
    *storage = (subset_storage){0};
}

static bool make_subset(const svg_shape_document *source, size_t first_shape,
                        size_t end_shape, svg_shape_document *view,
                        subset_storage *storage)
{
    if (first_shape >= end_shape || end_shape > source->shape_count ||
        source->shape_edge_start == NULL || source->shape_edge_count == NULL ||
        source->shape_subpath_start == NULL ||
        source->shape_subpath_count == NULL) return false;
    size_t first_edge = source->shape_edge_start[first_shape];
    size_t last_shape = end_shape - 1U;
    size_t edge_end = source->shape_edge_start[last_shape] +
                      source->shape_edge_count[last_shape];
    if (first_edge > edge_end || edge_end > source->edge_count) return false;
    size_t edge_count = edge_end - first_edge;
    size_t first_subpath = source->shape_subpath_start[first_shape];
    size_t last_subpath = source->shape_subpath_start[last_shape] +
                          source->shape_subpath_count[last_shape];
    if (first_subpath > last_subpath ||
        last_subpath > source->path_subpath_count) return false;
    size_t shape_count = end_shape - first_shape;
    size_t subpath_count = last_subpath - first_subpath;
    if (shape_count > SIZE_MAX / sizeof(*storage->shape_edge_start) ||
        shape_count > SIZE_MAX / sizeof(*storage->shape_subpath_start) ||
        edge_count > SIZE_MAX / sizeof(*storage->shape_ids) ||
        subpath_count > SIZE_MAX / sizeof(*storage->path_subpaths)) return false;

    storage->shape_edge_start = malloc(shape_count * sizeof(*storage->shape_edge_start));
    storage->shape_subpath_start = malloc(
        shape_count * sizeof(*storage->shape_subpath_start));
    if (edge_count != 0U)
        storage->shape_ids = malloc(edge_count * sizeof(*storage->shape_ids));
    if (subpath_count != 0U)
        storage->path_subpaths = malloc(
            subpath_count * sizeof(*storage->path_subpaths));
    if (storage->shape_edge_start == NULL ||
        storage->shape_subpath_start == NULL ||
        (edge_count != 0U && storage->shape_ids == NULL) ||
        (subpath_count != 0U && storage->path_subpaths == NULL)) return false;

    *view = *source;
    view->shape_count = shape_count;
    view->edge_count = edge_count;
    view->edges = edge_count == 0U ? NULL : source->edges + first_edge;
    view->colors = edge_count == 0U ? NULL : source->colors + first_edge * 4U;
    view->shape_ids = storage->shape_ids;
    view->shape_edge_start = storage->shape_edge_start;
    view->shape_edge_count = source->shape_edge_count + first_shape;
    view->shape_subpath_start = storage->shape_subpath_start;
    view->shape_subpath_count = source->shape_subpath_count + first_shape;
    view->path_subpath_count = subpath_count;
    view->path_subpaths = storage->path_subpaths;
    view->shape_evenodd = source->shape_evenodd + first_shape;
    view->shape_filled = source->shape_filled + first_shape;
    view->shape_stroke_closed = source->shape_stroke_closed + first_shape;
    view->shape_stroke_inverse = source->shape_stroke_inverse + first_shape;
    view->strokes = source->strokes + first_shape;
    view->paints = source->paints == NULL ? NULL : source->paints + first_shape;
    view->clips = source->clips == NULL ? NULL : source->clips + first_shape;
    view->opacity_groups = NULL;
    view->opacity_group_count = 0U;

    for (size_t shape = 0U; shape < shape_count; ++shape) {
        size_t original = first_shape + shape;
        if (source->shape_edge_start[original] < first_edge ||
            source->shape_subpath_start[original] < first_subpath) return false;
        storage->shape_edge_start[shape] =
            source->shape_edge_start[original] - first_edge;
        storage->shape_subpath_start[shape] =
            source->shape_subpath_start[original] - first_subpath;
    }
    for (size_t edge = 0U; edge < edge_count; ++edge) {
        size_t shape = source->shape_ids[first_edge + edge];
        if (shape < first_shape || shape >= end_shape) return false;
        storage->shape_ids[edge] = (uint32_t)(shape - first_shape);
    }
    for (size_t path = 0U; path < subpath_count; ++path) {
        svg_shape_path_subpath copy = source->path_subpaths[first_subpath + path];
        if (copy.first_edge < first_edge ||
            copy.first_edge > edge_end || copy.edge_count > edge_end - copy.first_edge)
            return false;
        copy.first_edge -= first_edge;
        storage->path_subpaths[path] = copy;
    }
    return true;
}

static bool render_flat_range(group_render_context *context, size_t first_shape,
                              size_t end_shape, raster_tile *destination)
{
    if (first_shape == end_shape) return true;
    if (!reserve_scratch(context)) {
        if (context->diagnostic != NULL)
            *context->diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
        return false;
    }
    svg_shape_document view = {0};
    subset_storage storage = {0};
    raster_tile layer = {0};
    raster_shape_work_stats measured = {0};
    bool ok = make_subset(context->document, first_shape, end_shape,
                          &view, &storage);
    if (ok) ok = raster_shape_render_flat(&view, &layer, context->diagnostic,
                                           &measured,
                                           context->scalar_oracle_alpha);
    if (ok && context->work != NULL && !add_stats(context->work, &measured)) {
        if (context->diagnostic != NULL)
            *context->diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
        ok = false;
    }
    if (ok) composite_tile(destination, &layer, 1.0);
    if (layer.rgba != NULL) raster_tile_destroy(&layer);
    subset_destroy(&storage);
    release_scratch(context);
    return ok;
}

static bool render_group_range(group_render_context *context, size_t first_shape,
                               size_t end_shape, size_t parent_group,
                               raster_tile *destination, size_t recursion)
{
    if (recursion > 64U) return false;
    size_t cursor = first_shape;
    const svg_shape_document *doc = context->document;
    for (size_t index = 0U; index < doc->opacity_group_count; ++index) {
        const svg_shape_opacity_group *group = &doc->opacity_groups[index];
        if (group->parent != parent_group) continue;
        if (group->first_shape < cursor || group->end_shape < group->first_shape ||
            group->end_shape > end_shape) {
            if (context->diagnostic != NULL)
                *context->diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U};
            return false;
        }
        if (!render_flat_range(context, cursor, group->first_shape, destination))
            return false;
        cursor = group->end_shape;
        if (group->first_shape == group->end_shape) continue;
        if (!reserve_scratch(context)) {
            if (context->diagnostic != NULL)
                *context->diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
            return false;
        }
        raster_tile layer = {0};
        if (!raster_tile_create(doc->width, doc->height, &layer)) {
            release_scratch(context);
            if (context->diagnostic != NULL)
                *context->diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
            return false;
        }
        memset(layer.rgba, 0, layer.byte_count);
        bool ok = render_group_range(context, group->first_shape,
                                     group->end_shape, index, &layer,
                                     recursion + 1U);
        if (ok) composite_tile(destination, &layer, group->opacity);
        raster_tile_destroy(&layer);
        release_scratch(context);
        if (!ok) return false;
    }
    return render_flat_range(context, cursor, end_shape, destination);
}

bool raster_shape_render_grouped(const svg_shape_document *document,
                                 raster_tile *out_tile,
                                 svg_shape_diagnostic *diagnostic,
                                 raster_shape_work_stats *work,
                                 bool scalar_oracle_alpha)
{
    if (out_tile != NULL) *out_tile = (raster_tile){0};
    if (work != NULL) *work = (raster_shape_work_stats){0};
    if (document == NULL || out_tile == NULL || document->opacity_groups == NULL ||
        document->opacity_group_count > SVG_SHAPE_MAX_EDGES ||
        document->width == 0U || document->height == 0U ||
        (size_t)document->width > SIZE_MAX / (size_t)document->height ||
        (size_t)document->width * (size_t)document->height > SIZE_MAX / 4U) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U};
        return false;
    }
    for (size_t i = 0U; i < document->opacity_group_count; ++i) {
        const svg_shape_opacity_group *group = &document->opacity_groups[i];
        if (group->first_shape > group->end_shape ||
            group->end_shape > document->shape_count ||
            !isfinite(group->opacity) || group->opacity < 0.0 ||
            group->opacity >= 1.0 ||
            (group->parent != SVG_SHAPE_NO_GROUP && group->parent >= i)) {
            if (diagnostic != NULL)
                *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U};
            return false;
        }
    }
    size_t pixels = (size_t)document->width * (size_t)document->height;
    if (pixels > SIZE_MAX / 4U) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
        return false;
    }
    group_render_context context = {
        .document = document,
        .tile_bytes = pixels * 4U,
        .work = work,
        .scalar_oracle_alpha = scalar_oracle_alpha,
        .diagnostic = diagnostic
    };
    if (!raster_tile_create(document->width, document->height, out_tile)) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, 0U};
        return false;
    }
    memset(out_tile->rgba, 0, out_tile->byte_count);
    if (!render_group_range(&context, 0U, document->shape_count,
                            SVG_SHAPE_NO_GROUP, out_tile, 0U)) {
        raster_tile_destroy(out_tile);
        return false;
    }
    if (diagnostic != NULL)
        *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_NONE, 0U};
    return true;
}
