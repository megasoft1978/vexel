#include "shape_render_internal.h"
#include "../svg/clip/clip.h"
#include "../svg/paint/paint.h"

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

void raster_shape_paint_sample(const svg_shape_document *doc, size_t shape,
                               const uint32_t *active_edges, size_t active_count,
                               const geom_edge *local_edges,
                               const bool *dash_seams, double px, double py,
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
    bool allowed = doc->clips == NULL || svg_clip_contains(doc->clips[shape], px, py);
    uint8_t sampled_fill[4];
    if (inside && allowed && fill[3] != 0U) {
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
    if (allowed && raster_shape_stroke_contains(doc, shape, local_edges,
                                               dash_seams, px, py)) {
        if (scalar_oracle_alpha)
            memcpy(sample_rgba, doc->strokes[shape].rgba, 4U);
        else composite_source_over(sample_rgba, doc->strokes[shape].rgba);
    }
}
