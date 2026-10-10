#include "shape_render_internal.h"
#include "../svg/clip/clip.h"
#include "../svg/paint/paint.h"

#include <math.h>
#include <stdint.h>
#include <string.h>

static bool edge_crosses_right(geom_edge edge, double px, double py)
{
    double xcross = edge.a.x + (py - edge.a.y) * (edge.b.x - edge.a.x) /
                                      (edge.b.y - edge.a.y);
    return xcross > px;
}

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

static bool image_sample(const svg_shape_document *document, size_t shape,
                         double x, double y, uint8_t rgba[4])
{
    if (document->images == NULL || document->shape_stroke_inverse == NULL)
        return false;
    const svg_shape_image *image = &document->images[shape];
    if (image->rgba == NULL || image->pixel_width == 0U ||
        image->pixel_height == 0U || image->draw_width <= 0.0 ||
        image->draw_height <= 0.0) return false;
    geom_point local;
    if (!geom_affine_map_point(document->shape_stroke_inverse[shape],
                               (geom_point){x, y}, &local)) return false;
    if (local.x < image->x || local.y < image->y ||
        local.x >= image->x + image->width ||
        local.y >= image->y + image->height ||
        local.x < image->draw_x || local.y < image->draw_y ||
        local.x >= image->draw_x + image->draw_width ||
        local.y >= image->draw_y + image->draw_height) return false;
    double u = (local.x - image->draw_x) / image->draw_width;
    double v = (local.y - image->draw_y) / image->draw_height;
    double source_x = u * (double)image->pixel_width - 0.5;
    double source_y = v * (double)image->pixel_height - 0.5;
    if (source_x < 0.0) source_x = 0.0;
    if (source_y < 0.0) source_y = 0.0;
    double max_x = (double)(image->pixel_width - 1U);
    double max_y = (double)(image->pixel_height - 1U);
    if (source_x > max_x) source_x = max_x;
    if (source_y > max_y) source_y = max_y;
    uint32_t x0 = (uint32_t)floor(source_x);
    uint32_t y0 = (uint32_t)floor(source_y);
    uint32_t x1 = x0 + 1U < image->pixel_width ? x0 + 1U : x0;
    uint32_t y1 = y0 + 1U < image->pixel_height ? y0 + 1U : y0;
    double fx = source_x - (double)x0;
    double fy = source_y - (double)y0;
    const uint8_t *p00 = image->rgba + (size_t)y0 * image->row_stride +
                         (size_t)x0 * 4U;
    const uint8_t *p10 = image->rgba + (size_t)y0 * image->row_stride +
                         (size_t)x1 * 4U;
    const uint8_t *p01 = image->rgba + (size_t)y1 * image->row_stride +
                         (size_t)x0 * 4U;
    const uint8_t *p11 = image->rgba + (size_t)y1 * image->row_stride +
                         (size_t)x1 * 4U;
    double channels[4];
    for (size_t channel = 0U; channel < 4U; ++channel) {
        double top = (double)p00[channel] * (1.0 - fx) +
                     (double)p10[channel] * fx;
        double bottom = (double)p01[channel] * (1.0 - fx) +
                        (double)p11[channel] * fx;
        channels[channel] = top * (1.0 - fy) + bottom * fy;
    }
    uint8_t alpha = (uint8_t)floor(channels[3] + 0.5);
    if (alpha == 0U) {
        memset(rgba, 0, 4U);
        return true;
    }
    for (size_t channel = 0U; channel < 3U; ++channel) {
        double straight = channels[channel] * 255.0 / channels[3];
        if (straight > 255.0) straight = 255.0;
        rgba[channel] = (uint8_t)floor(straight + 0.5);
    }
    rgba[3] = alpha;
    return true;
}

static uint32_t wrap_index(int64_t index, uint32_t dimension)
{
    int64_t wrapped = index % (int64_t)dimension;
    if (wrapped < 0) wrapped += (int64_t)dimension;
    return (uint32_t)wrapped;
}

static bool pattern_sample(const svg_shape_document *document, size_t shape,
                           size_t pattern_index, geom_affine pattern_inverse,
                           double pattern_opacity, double x, double y,
                           uint8_t rgba[4])
{
    if (document->patterns == NULL ||
        document->shape_stroke_inverse == NULL) return false;
    if (pattern_index >= document->pattern_count) return false;
    const svg_shape_pattern *pattern = &document->patterns[pattern_index];
    if (pattern->rgba == NULL || pattern->width == 0U ||
        pattern->height == 0U) return false;
    size_t pattern_pixels = 0U;
    if (pattern->width > SIZE_MAX / pattern->height) return false;
    pattern_pixels = (size_t)pattern->width * pattern->height;
    if (pattern_pixels > SIZE_MAX / 4U ||
        pattern->byte_count != pattern_pixels * 4U) return false;
    geom_point local, tile;
    if (!geom_affine_map_point(document->shape_stroke_inverse[shape],
                               (geom_point){x, y}, &local) ||
        !geom_affine_map_point(pattern_inverse, local, &tile) ||
        !isfinite(tile.x) || !isfinite(tile.y) ||
        fabs(tile.x) > 1e12 || fabs(tile.y) > 1e12) return false;
    double sample_x = fmod(tile.x - 0.5, (double)pattern->width);
    double sample_y = fmod(tile.y - 0.5, (double)pattern->height);
    if (sample_x < 0.0) sample_x += (double)pattern->width;
    if (sample_y < 0.0) sample_y += (double)pattern->height;
    int64_t x0 = (int64_t)floor(sample_x);
    int64_t y0 = (int64_t)floor(sample_y);
    uint32_t ix0 = wrap_index(x0, pattern->width);
    uint32_t iy0 = wrap_index(y0, pattern->height);
    uint32_t ix1 = wrap_index(x0 + 1, pattern->width);
    uint32_t iy1 = wrap_index(y0 + 1, pattern->height);
    double fx = sample_x - floor(sample_x);
    double fy = sample_y - floor(sample_y);
    const uint8_t *samples[4] = {
        pattern->rgba + ((size_t)iy0 * pattern->width + ix0) * 4U,
        pattern->rgba + ((size_t)iy0 * pattern->width + ix1) * 4U,
        pattern->rgba + ((size_t)iy1 * pattern->width + ix0) * 4U,
        pattern->rgba + ((size_t)iy1 * pattern->width + ix1) * 4U};
    double weights[4] = {(1.0 - fx) * (1.0 - fy), fx * (1.0 - fy),
                         (1.0 - fx) * fy, fx * fy};
    double alpha = 0.0, premultiplied[3] = {0.0, 0.0, 0.0};
    for (size_t i = 0U; i < 4U; ++i) {
        double a = (double)samples[i][3] / 255.0;
        alpha += a * weights[i];
        for (size_t c = 0U; c < 3U; ++c)
            premultiplied[c] += (double)samples[i][c] / 255.0 * a * weights[i];
    }
    double opacity = pattern_opacity;
    if (!isfinite(opacity) || opacity < 0.0 || opacity > 1.0) return false;
    alpha *= opacity;
    if (alpha <= 0.0) {
        memset(rgba, 0, 4U);
        return true;
    }
    for (size_t c = 0U; c < 3U; ++c) {
        double color = premultiplied[c] /
            (opacity == 0.0 ? 1.0 : (alpha / opacity));
        rgba[c] = (uint8_t)floor(fmin(255.0, fmax(0.0, color * 255.0)) + 0.5);
    }
    rgba[3] = (uint8_t)floor(fmin(1.0, alpha) * 255.0 + 0.5);
    return true;
}

static void paint_stroke_sample(const svg_shape_document *document,
                                size_t shape, const geom_edge *local_edges,
                                const bool *dash_seams,
                                const raster_shape_dash_cache *dash_cache,
                                bool allowed, double px, double py,
                                bool scalar_oracle_alpha,
                                uint8_t sample_rgba[4])
{
    if (!allowed || !raster_shape_stroke_contains(
            document, shape, local_edges, dash_seams, dash_cache, px, py))
        return;
    if (document->stroke_paints != NULL &&
        document->stroke_paints[shape].kind == SVG_SHAPE_PAINT_PATTERN) {
        uint8_t sampled_stroke[4];
        if (pattern_sample(document, shape,
                document->stroke_paints[shape].pattern_index,
                document->stroke_paints[shape].pattern_inverse,
                document->stroke_paints[shape].pattern_opacity,
                px, py, sampled_stroke) && sampled_stroke[3] != 0U)
            composite_source_over(sample_rgba, sampled_stroke);
    } else if (scalar_oracle_alpha) {
        memcpy(sample_rgba, document->strokes[shape].rgba, 4U);
    } else {
        composite_source_over(sample_rgba, document->strokes[shape].rgba);
    }
}

static void paint_sample_result(const svg_shape_document *doc, size_t shape,
                                const geom_edge *local_edges,
                                const bool *dash_seams,
                                const raster_shape_dash_cache *dash_cache,
                                double px, double py,
                                double paint_x, double paint_y,
                                bool scalar_oracle_alpha, bool inside,
                                const uint8_t fill[4], uint8_t sample_rgba[4])
{
    bool allowed = doc->clips == NULL || svg_clip_contains(doc->clips[shape], px, py);
    bool stroke_first = doc->strokes != NULL &&
                        doc->strokes[shape].paint_order == 1U;
    if (stroke_first)
        paint_stroke_sample(doc, shape, local_edges, dash_seams, dash_cache,
                            allowed, px, py, scalar_oracle_alpha, sample_rgba);
    uint8_t sampled_fill[4];
    bool image = doc->paints != NULL &&
                 doc->paints[shape].kind == SVG_SHAPE_PAINT_IMAGE;
    bool pattern = doc->paints != NULL &&
                   doc->paints[shape].kind == SVG_SHAPE_PAINT_PATTERN;
    if (inside && allowed && image) {
        uint8_t sampled_image[4];
        if (image_sample(doc, shape, px, py, sampled_image) &&
            sampled_image[3] != 0U)
            composite_source_over(sample_rgba, sampled_image);
    } else if (inside && allowed && pattern) {
        uint8_t sampled_pattern[4];
        if (pattern_sample(doc, shape, doc->paints[shape].pattern_index,
                           doc->paints[shape].pattern_inverse,
                           doc->paints[shape].pattern_opacity,
                           px, py, sampled_pattern) &&
            sampled_pattern[3] != 0U)
            composite_source_over(sample_rgba, sampled_pattern);
    } else if (inside && allowed && fill[3] != 0U) {
        memcpy(sampled_fill, fill, sizeof(sampled_fill));
        bool gradient = doc->paints != NULL &&
            doc->paints[shape].kind != SVG_SHAPE_PAINT_SOLID;
        if (gradient) {
            svg_paint_sample(&doc->paints[shape], doc->paint_stops,
                             paint_x, paint_y, sampled_fill);
        }
        if (scalar_oracle_alpha && !gradient) {
            /* The test-only alpha mode preserves the frozen scalar oracle. */
            memcpy(sample_rgba, sampled_fill, sizeof(sampled_fill));
        } else composite_source_over(sample_rgba, sampled_fill);
    }
    if (!stroke_first)
        paint_stroke_sample(doc, shape, local_edges, dash_seams, dash_cache,
                            allowed, px, py, scalar_oracle_alpha, sample_rgba);
}

void raster_shape_paint_sample(const svg_shape_document *doc, size_t shape,
                               const uint32_t *active_edges, size_t active_count,
                               const geom_edge *local_edges,
                               const bool *dash_seams,
                               const raster_shape_dash_cache *dash_cache,
                               double px, double py,
                               double paint_x, double paint_y,
                               bool scalar_oracle_alpha,
                               uint8_t sample_rgba[4])
{
    int32_t winding = 0;
    uint8_t parity = 0U;
    uint8_t fill[4] = {0U, 0U, 0U, 0U};
    for (size_t i = 0U; i < active_count; i++) {
        size_t edge_index = active_edges != NULL ? active_edges[i] :
                            doc->shape_edge_start[shape] + i;
        geom_edge edge = doc->edges[edge_index];
        bool upward = edge.a.y <= py && edge.b.y > py;
        bool downward = edge.b.y <= py && edge.a.y > py;
        if ((!upward && !downward) || !edge_crosses_right(edge, px, py)) continue;
        winding += upward ? 1 : -1;
        parity ^= 1U;
        memcpy(fill, doc->colors + edge_index * 4U, sizeof(fill));
    }
    bool inside = doc->shape_evenodd[shape] ? parity != 0U : winding != 0;
    paint_sample_result(doc, shape, local_edges, dash_seams, dash_cache,
                        px, py, paint_x, paint_y, scalar_oracle_alpha,
                        inside, fill, sample_rgba);
}

void raster_shape_paint_sample_inside(
    const svg_shape_document *doc, size_t shape, const uint8_t fill_rgba[4],
    const geom_edge *local_edges, const bool *dash_seams,
    const raster_shape_dash_cache *dash_cache, double px, double py,
    double paint_x, double paint_y, bool scalar_oracle_alpha,
    uint8_t sample_rgba[4])
{
    if (fill_rgba == NULL) return;
    paint_sample_result(doc, shape, local_edges, dash_seams, dash_cache,
                        px, py, paint_x, paint_y, scalar_oracle_alpha,
                        true, fill_rgba, sample_rgba);
}
