#include "shape.h"
#include "shape_render.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static bool decode(const char *text, svg_shape_document *document)
{
    svg_shape_diagnostic diagnostic = {0};
    base_bytes source = {(const uint8_t *)text, strlen(text)};
    if (svg_shape_decode(source, document, &diagnostic) != SVG_SHAPE_ERROR_NONE) {
        (void)fprintf(stderr, "decode failed: %s at %zu\n",
                      svg_shape_error_name(diagnostic.category), diagnostic.offset);
        return false;
    }
    return true;
}

static bool pixel(const svg_shape_document *document, double time,
                  uint32_t x, uint32_t y, uint8_t rgba[4])
{
    raster_tile tile = {0};
    svg_shape_diagnostic diagnostic = {0};
    if (!raster_shape_render_at(document, time, &tile, &diagnostic)) {
        (void)fprintf(stderr, "render failed at %.3f: %s at %zu\n", time,
                      svg_shape_error_name(diagnostic.category), diagnostic.offset);
        return false;
    }
    bool okay = x < tile.width && y < tile.height;
    if (okay) memcpy(rgba, tile.rgba + ((size_t)y * tile.width + x) * 4U, 4U);
    raster_tile_destroy(&tile);
    return okay;
}

static bool numeric_render_and_immutability(void)
{
    static const char source[] =
        "<svg width='12' height='8'><rect x='1' y='1' width='2' height='4' fill='red'>"
        "<animate attributeName='x' to='5' dur='4s' fill='freeze'/>"
        "</rect></svg>";
    char original[sizeof(source)];
    memcpy(original, source, sizeof(source));
    svg_shape_document document = {0};
    if (!decode(source, &document)) return false;
    if (document.animation.track_count != 1U ||
        document.animation.tracks[0].target_offset == SIZE_MAX) {
        svg_shape_document_destroy(&document);
        return false;
    }
    uint8_t start[4], middle[4], end[4], repeat[4];
    bool okay = pixel(&document, 0.0, 1U, 2U, start) &&
        pixel(&document, 2.0, 3U, 2U, middle) &&
        pixel(&document, 4.0, 5U, 2U, end) &&
        pixel(&document, 2.0, 3U, 2U, repeat) &&
        start[3] == 255U && middle[3] == 255U && end[3] == 255U &&
        memcmp(middle, repeat, sizeof(middle)) == 0 &&
        memcmp(source, original, sizeof(source)) == 0;
    svg_shape_document_destroy(&document);
    return okay;
}

static bool color_opacity_and_discrete(void)
{
    static const char source[] =
        "<svg width='8' height='8'><rect width='8' height='8' fill='red'>"
        "<animate attributeName='fill' values='#ff0000;#0000ff' dur='2s'/>"
        "<animate attributeName='opacity' values='1;0' dur='2s'/>"
        "</rect></svg>";
    svg_shape_document document = {0};
    if (!decode(source, &document)) return false;
    uint8_t rgba[4] = {0};
    bool okay = document.animation.track_count == 2U &&
        pixel(&document, 1.0, 3U, 3U, rgba) && rgba[0] >= 127U &&
        rgba[0] <= 128U && rgba[2] >= 127U && rgba[2] <= 128U &&
        rgba[3] >= 127U && rgba[3] <= 128U;
    svg_shape_document_destroy(&document);
    if (!okay) return false;

    static const char discrete[] =
        "<svg width='8' height='8'><rect width='8' height='8' fill='red'>"
        "<animate attributeName='opacity' values='0.25;1' keyTimes='0;1' "
        "calcMode='discrete' dur='2s'/>"
        "</rect></svg>";
    if (!decode(discrete, &document)) return false;
    okay = pixel(&document, 1.5, 3U, 3U, rgba) && rgba[3] == 64U;
    svg_shape_document_destroy(&document);
    return okay;
}

static bool repeat_sync_and_transform(void)
{
    static const char repeated[] =
        "<svg width='16' height='8'><rect x='1' y='1' width='2' height='4'>"
        "<animate id='move' attributeName='x' values='1;5' dur='2s' "
        "repeatCount='2' fill='freeze'/>"
        "<animate attributeName='y' from='1' to='5' dur='1s' begin='move.end+1s' "
        "fill='freeze'/>"
        "</rect></svg>";
    svg_shape_document document = {0};
    if (!decode(repeated, &document)) return false;
    uint8_t rgba[4] = {0};
    bool okay = document.animation.track_count == 2U &&
        pixel(&document, 3.0, 3U, 2U, rgba) && rgba[3] == 255U &&
        pixel(&document, 5.5, 5U, 3U, rgba) && rgba[3] == 255U;
    svg_shape_document_destroy(&document);
    if (!okay) return false;

    static const char transformed[] =
        "<svg width='12' height='8'><rect x='1' y='1' width='2' height='4'>"
        "<animateTransform attributeName='transform' type='translate' "
        "from='0 0' to='4 0' dur='2s' fill='freeze'/>"
        "</rect></svg>";
    if (!decode(transformed, &document)) return false;
    okay = pixel(&document, 1.0, 3U, 2U, rgba) && rgba[3] == 255U;
    svg_shape_document_destroy(&document);
    return okay;
}

static bool diagnostics_and_limits(void)
{
    static const char unsupported[] =
        "<svg width='4' height='4'><rect width='4' height='4'>"
        "<animate attributeName='opacity' from='0' to='1' dur='1s' begin='click'/>"
        "</rect><style>@keyframes pulse { from { opacity: 0; } }</style></svg>";
    svg_shape_document document = {0};
    if (!decode(unsupported, &document)) return false;
    bool timing = false, css = false;
    for (size_t index = 0U; index < document.animation.diagnostic_count; ++index) {
        timing |= document.animation.diagnostics[index].kind == SVG_ANIM_DIAG_UNSUPPORTED_TIMING;
        css |= document.animation.diagnostics[index].kind == SVG_ANIM_DIAG_CSS_ANIMATION;
    }
    bool okay = document.animation.track_count == 0U && timing && css;
    svg_shape_document_destroy(&document);
    if (!okay) return false;

    static const char too_long[] =
        "<svg width='4' height='4'><rect width='4' height='4'>"
        "<animate attributeName='opacity' from='0' to='1' dur='3601s'/>"
        "</rect></svg>";
    if (!decode(too_long, &document)) return false;
    okay = document.animation.track_count == 0U &&
        document.animation.diagnostic_count > 0U;
    svg_shape_document_destroy(&document);
    return okay;
}

static bool motion_and_mpath(void)
{
    static const char direct[] =
        "<svg width='12' height='8'><rect x='1' y='1' width='2' height='3'>"
        "<animateMotion path='M0 0 L4 0' dur='2s' rotate='auto' fill='freeze'/>"
        "</rect></svg>";
    svg_shape_document document = {0};
    if (!decode(direct, &document)) return false;
    uint8_t rgba[4] = {0};
    bool okay = document.animation.track_count == 1U &&
        pixel(&document, 1.0, 3U, 2U, rgba) && rgba[3] == 255U;
    if (!okay) (void)fprintf(stderr, "direct motion tracks=%zu points=%zu rgba=%u\n",
        document.animation.track_count,
        document.animation.track_count == 0U ? 0U :
            document.animation.tracks[0].motion_point_count,
        document.animation.track_count == 0U ? 0U : rgba[3]);
    svg_shape_document_destroy(&document);
    if (!okay) return false;

    static const char mpath[] =
        "<svg width='12' height='8'><defs><path id='route' d='M0 0 H4'/></defs>"
        "<rect x='1' y='1' width='2' height='3'>"
        "<animateMotion dur='2s' fill='freeze'><mpath href='#route'/></animateMotion>"
        "</rect></svg>";
    if (!decode(mpath, &document)) return false;
    okay = document.animation.track_count == 1U &&
        document.animation.tracks[0].motion_point_count == 2U &&
        pixel(&document, 1.0, 3U, 2U, rgba) && rgba[3] == 255U;
    if (!okay) (void)fprintf(stderr, "mpath tracks=%zu points=%zu rgba=%u\n",
        document.animation.track_count,
        document.animation.track_count == 0U ? 0U :
            document.animation.tracks[0].motion_point_count,
        document.animation.track_count == 0U ? 0U : rgba[3]);
    if (!okay) for (size_t index = 0U; index < document.animation.diagnostic_count; ++index)
        (void)fprintf(stderr, "motion diagnostic: %s at %zu\n",
            svg_anim_diagnostic_name(document.animation.diagnostics[index].kind),
            document.animation.diagnostics[index].offset);
    svg_shape_document_destroy(&document);
    return okay;
}

static bool animated_object_clip_stays_overlay_local(void)
{
    static const char source[] =
        "<svg width='16' height='12'><defs><clipPath id='c' "
        "clipPathUnits='objectBoundingBox'><rect x='0' y='0' width='.5' "
        "height='1'/></clipPath></defs><rect x='1' y='1' width='8' height='8' "
        "fill='red' clip-path='url(#c)'><animate attributeName='width' "
        "from='8' to='4' dur='2s' fill='freeze'/></rect></svg>";
    char source_snapshot[sizeof(source)];
    memcpy(source_snapshot, source, sizeof(source));
    svg_shape_document document = {0};
    if (!decode(source, &document)) return false;
    size_t edge_bytes = document.edge_count * sizeof(*document.edges);
    size_t frame_bytes = document.transform_node_count *
                         sizeof(*document.transform_nodes);
    geom_edge *edges = edge_bytes == 0U ? NULL : malloc(edge_bytes);
    svg_shape_transform_frame *frames = frame_bytes == 0U ? NULL :
        malloc(frame_bytes);
    if ((edge_bytes != 0U && edges == NULL) ||
        (frame_bytes != 0U && frames == NULL)) {
        free(edges);
        free(frames);
        svg_shape_document_destroy(&document);
        return false;
    }
    if (edge_bytes != 0U) memcpy(edges, document.edges, edge_bytes);
    if (frame_bytes != 0U) memcpy(frames, document.transform_nodes, frame_bytes);
    uint8_t animated_before[4], static_middle[4], animated_after[4];
    bool okay = pixel(&document, 1.0, 4U, 3U, animated_before) &&
        pixel(&document, 0.0, 4U, 3U, static_middle) &&
        pixel(&document, 1.0, 4U, 3U, animated_after) &&
        animated_before[3] == 0U && static_middle[3] == 255U &&
        animated_after[3] == 0U &&
        memcmp(animated_before, animated_after, sizeof(animated_before)) == 0 &&
        memcmp(source, source_snapshot, sizeof(source)) == 0 &&
        (edge_bytes == 0U || memcmp(edges, document.edges, edge_bytes) == 0) &&
        (frame_bytes == 0U ||
         memcmp(frames, document.transform_nodes, frame_bytes) == 0);
    if (!okay) (void)fprintf(stderr,
        "clip overlay pixels=%u/%u/%u source=%d edges=%d frames=%d\n",
        animated_before[3], static_middle[3], animated_after[3],
        memcmp(source, source_snapshot, sizeof(source)) == 0,
        edge_bytes == 0U || memcmp(edges, document.edges, edge_bytes) == 0,
        frame_bytes == 0U ||
            memcmp(frames, document.transform_nodes, frame_bytes) == 0);
    free(edges);
    free(frames);
    svg_shape_document_destroy(&document);
    return okay;
}

int main(void)
{
    if (!numeric_render_and_immutability() || !color_opacity_and_discrete() ||
        !repeat_sync_and_transform() || !diagnostics_and_limits() ||
        !motion_and_mpath() || !animated_object_clip_stays_overlay_local()) {
        (void)fputs("animation checks failed\n", stderr);
        return EXIT_FAILURE;
    }
    (void)puts("animation checks passed");
    return EXIT_SUCCESS;
}
