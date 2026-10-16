#include "shape_render_internal.h"
#include "../svg/paint/paint.h"
#include "shape_filter.h"

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
    size_t mask_depth;
    svg_shape_diagnostic *diagnostic;
} group_render_context;

typedef struct subset_storage {
    uint32_t *shape_ids;
    size_t *shape_edge_start;
    size_t *shape_subpath_start;
    svg_shape_path_subpath *path_subpaths;
} subset_storage;

static bool render_group_range(group_render_context *context,
                               size_t first_shape, size_t end_shape,
                               size_t parent_group,
                               raster_tile *destination, size_t recursion);

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

static double blend_channel(uint8_t source, uint8_t backdrop, uint8_t mode)
{
    double foreground = (double)source / 255.0;
    double background = (double)backdrop / 255.0;
    switch (mode) {
    case 1U: return foreground * background;
    case 2U: return foreground + background - foreground * background;
    case 3U: return foreground < background ? foreground : background;
    case 4U: return foreground > background ? foreground : background;
    default: return foreground;
    }
}

static void composite_tile(raster_tile *destination, const raster_tile *source,
                           double opacity, uint8_t blend_mode)
{
    for (size_t offset = 0U; offset < source->byte_count; offset += 4U) {
        const uint8_t *foreground = source->rgba + offset;
        uint8_t *backdrop = destination->rgba + offset;
        double source_alpha = (double)foreground[3] / 255.0 * opacity;
        double backdrop_alpha = (double)backdrop[3] / 255.0;
        if (source_alpha <= 0.0) continue;
        if (source_alpha > 1.0) source_alpha = 1.0;
        if (blend_mode == 0U || backdrop_alpha <= 0.0) {
            uint8_t rgba[4];
            memcpy(rgba, foreground, sizeof(rgba));
            rgba[3] = (uint8_t)lround(source_alpha * 255.0);
            source_over(backdrop, rgba);
            continue;
        }
        double output_alpha = source_alpha + backdrop_alpha -
                              source_alpha * backdrop_alpha;
        for (size_t channel = 0U; channel < 3U; ++channel) {
            double source_color = (double)foreground[channel] / 255.0;
            double backdrop_color = (double)backdrop[channel] / 255.0;
            double blended = blend_channel(foreground[channel],
                                           backdrop[channel], blend_mode);
            double premultiplied = (1.0 - source_alpha) * backdrop_color *
                                   backdrop_alpha +
                (1.0 - backdrop_alpha) * source_color * source_alpha +
                source_alpha * backdrop_alpha * blended;
            double output = output_alpha == 0.0 ? 0.0 :
                           premultiplied / output_alpha;
            if (output < 0.0) output = 0.0;
            if (output > 1.0) output = 1.0;
            backdrop[channel] = (uint8_t)lround(output * 255.0);
        }
        backdrop[3] = (uint8_t)lround(output_alpha * 255.0);
    }
}

static bool map_viewport(const svg_shape_document *document,
                         double *scale_x, double *scale_y,
                         double *offset_x, double *offset_y)
{
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
           isfinite(*offset_x) && isfinite(*offset_y) &&
           *scale_x > 0.0 && *scale_y > 0.0;
}

static bool point_in_quad(const geom_point quad[4], double x, double y)
{
    double positive = 0.0, negative = 0.0;
    for (size_t i = 0U; i < 4U; ++i) {
        const geom_point a = quad[i];
        const geom_point b = quad[(i + 1U) % 4U];
        double cross = (b.x - a.x) * (y - a.y) -
                       (b.y - a.y) * (x - a.x);
        if (cross > 1e-9) positive += 1.0;
        else if (cross < -1e-9) negative += 1.0;
    }
    return positive == 0.0 || negative == 0.0;
}

static double linear_luminance(const uint8_t rgba[4])
{
    double channels[3];
    for (size_t i = 0U; i < 3U; ++i) {
        double value = (double)rgba[i] / 255.0;
        channels[i] = value <= 0.04045 ? value / 12.92 :
                      pow((value + 0.055) / 1.055, 2.4);
    }
    return 0.2126 * channels[0] + 0.7152 * channels[1] +
           0.0722 * channels[2];
}

static bool apply_mask(group_render_context *context,
                       const svg_shape_mask *mask, raster_tile *layer)
{
    if (mask == NULL || mask->content == NULL || layer == NULL ||
        layer->rgba == NULL ||
        mask->content->width != context->document->width ||
        mask->content->height != context->document->height ||
        context->mask_depth >= 8U) {
        if (context->diagnostic != NULL)
            *context->diagnostic = (svg_shape_diagnostic){
                context->mask_depth >= 8U ? SVG_SHAPE_ERROR_LIMIT :
                                            SVG_SHAPE_ERROR_INVALID_ARGUMENT,
                mask == NULL ? 0U : mask->source_offset};
        return false;
    }
    if (!reserve_scratch(context)) {
        if (context->diagnostic != NULL)
            *context->diagnostic = (svg_shape_diagnostic){
                SVG_SHAPE_ERROR_LIMIT, mask->source_offset};
        return false;
    }
    raster_tile mask_tile = {0};
    const svg_shape_document *content = mask->content;
    bool rendered = false;
    if (content->opacity_group_count == 0U) {
        rendered = raster_shape_render_flat(content, &mask_tile,
                                             context->diagnostic, NULL, false);
    } else if (content->opacity_groups != NULL &&
               content->opacity_group_count <= SVG_SHAPE_MAX_EDGES &&
               raster_tile_create(content->width, content->height,
                                  &mask_tile)) {
        memset(mask_tile.rgba, 0, mask_tile.byte_count);
        group_render_context nested = {
            .document = content,
            .scratch_bytes = context->scratch_bytes,
            .tile_bytes = context->tile_bytes,
            .work = NULL,
            .scalar_oracle_alpha = false,
            .mask_depth = context->mask_depth + 1U,
            .diagnostic = context->diagnostic};
        rendered = render_group_range(&nested, 0U, content->shape_count,
                                      SVG_SHAPE_NO_GROUP, &mask_tile, 0U);
        context->scratch_bytes = nested.scratch_bytes;
    }
    if (!rendered) {
        if (mask_tile.rgba != NULL) raster_tile_destroy(&mask_tile);
        release_scratch(context);
        if (context->diagnostic != NULL &&
            context->diagnostic->category == SVG_SHAPE_ERROR_NONE)
            *context->diagnostic = (svg_shape_diagnostic){
                SVG_SHAPE_ERROR_LIMIT, mask->source_offset};
        return false;
    }

    double scale_x, scale_y, offset_x, offset_y;
    if (!map_viewport(context->document, &scale_x, &scale_y,
                      &offset_x, &offset_y)) {
        raster_tile_destroy(&mask_tile);
        release_scratch(context);
        if (context->diagnostic != NULL)
            *context->diagnostic = (svg_shape_diagnostic){
                SVG_SHAPE_ERROR_VIEWPORT, mask->source_offset};
        return false;
    }
    double x = mask->region[0], y = mask->region[1];
    double width = mask->region[2], height = mask->region[3];
    geom_point local[4] = {{x, y}, {x + width, y},
                           {x + width, y + height}, {x, y + height}};
    geom_point region[4];
    bool region_valid = true;
    for (size_t i = 0U; i < 4U; ++i) {
        geom_point document_point;
        if (!geom_affine_map_point(mask->region_transform, local[i],
                                   &document_point)) {
            region_valid = false;
            break;
        }
        region[i] = (geom_point){
            offset_x + (document_point.x - context->document->view_x) * scale_x,
            offset_y + (document_point.y - context->document->view_y) * scale_y};
    }
    if (!region_valid) {
        raster_tile_destroy(&mask_tile);
        release_scratch(context);
        if (context->diagnostic != NULL)
            *context->diagnostic = (svg_shape_diagnostic){
                SVG_SHAPE_ERROR_NUMBER, mask->source_offset};
        return false;
    }
    for (uint32_t py = 0U; py < layer->height; ++py) {
        for (uint32_t px = 0U; px < layer->width; ++px) {
            size_t pixel = ((size_t)py * layer->width + px) * 4U;
            uint8_t *destination = layer->rgba + pixel;
            if (!point_in_quad(region, (double)px + 0.5,
                               (double)py + 0.5)) {
                memset(destination, 0, 4U);
                continue;
            }
            const uint8_t *source = mask_tile.rgba + pixel;
            double alpha = (double)source[3] / 255.0;
            if (mask->luminance) alpha *= linear_luminance(source);
            destination[3] = (uint8_t)lround(
                (double)destination[3] * alpha);
            if (destination[3] == 0U)
                memset(destination, 0, 4U);
        }
    }
    raster_tile_destroy(&mask_tile);
    release_scratch(context);
    return true;
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
    view->paints = source->paints;
    view->paint_indices_by_shape = source->paint_indices_by_shape == NULL ?
        NULL : source->paint_indices_by_shape + first_shape;
    view->paint_count = source->paint_count;
    view->paint_capacity = source->paint_capacity;
    view->stroke_paints = source->stroke_paints;
    view->stroke_paint_indices_by_shape =
        source->stroke_paint_indices_by_shape == NULL ? NULL :
        source->stroke_paint_indices_by_shape + first_shape;
    view->stroke_paint_count = source->stroke_paint_count;
    view->stroke_paint_capacity = source->stroke_paint_capacity;
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
        if (ok) composite_tile(destination, &layer, 1.0, 0U);
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
        if (ok && group->filter_index != SVG_SHAPE_NO_FILTER) {
            size_t available = GROUP_SCRATCH_LIMIT - context->scratch_bytes;
            ok = raster_shape_filter_apply(
                doc, group->filter_index, &layer, available,
                context->diagnostic);
        }
        if (ok && group->mask_index != SVG_SHAPE_NO_MASK) {
            if (doc->masks == NULL || group->mask_index >= doc->mask_count) {
                if (context->diagnostic != NULL)
                    *context->diagnostic = (svg_shape_diagnostic){
                        SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U};
                ok = false;
            } else {
                ok = apply_mask(context, &doc->masks[group->mask_index],
                                &layer);
            }
        }
        if (ok) composite_tile(destination, &layer, group->opacity,
                               group->blend_mode);
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
            group->opacity > 1.0 || group->blend_mode > 4U ||
            (group->filter_index != SVG_SHAPE_NO_FILTER &&
             (document->filters == NULL ||
              group->filter_index >= document->filter_count)) ||
            (group->mask_index != SVG_SHAPE_NO_MASK &&
             (document->masks == NULL ||
              group->mask_index >= document->mask_count)) ||
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
