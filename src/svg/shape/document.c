#include "shape.h"
#include "../paint/paint.h"
#include "shape_internal.h"

#include "../clip/clip.h"

#include <stdlib.h>

svg_shape_error svg_shape_decode(base_bytes source, svg_shape_document *out,
                                 svg_shape_diagnostic *diagnostic)
{
    if (out == NULL || !base_bytes_valid(source)) {
        if (diagnostic != NULL) *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U};
        return SVG_SHAPE_ERROR_INVALID_ARGUMENT;
    }
    return svg_shape_decode_source(source, out, diagnostic);
}

svg_shape_error svg_shape_decode_with_system_language(
    base_bytes source, base_bytes system_language, svg_shape_document *out,
    svg_shape_diagnostic *diagnostic)
{
    if (out == NULL || !base_bytes_valid(source) ||
        !base_bytes_valid(system_language) || system_language.len == 0U) {
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){
                SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U};
        return SVG_SHAPE_ERROR_INVALID_ARGUMENT;
    }
    return svg_shape_decode_source_with_system_language(
        source, system_language, out, diagnostic);
}

svg_shape_error svg_shape_decode_with_media(
    base_bytes source, const svg_shape_media_providers *providers,
    svg_shape_document *out, svg_shape_diagnostic *diagnostic)
{
    if (out == NULL || !base_bytes_valid(source) || providers == NULL) {
        if (diagnostic != NULL) {
            *diagnostic = (svg_shape_diagnostic){
                SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U};
        }
        return SVG_SHAPE_ERROR_INVALID_ARGUMENT;
    }
    return svg_shape_decode_source_with_media(source, providers, out,
                                               diagnostic);
}

void svg_shape_document_destroy(svg_shape_document *document)
{
    if (document == NULL) return;
    svg_anim_document_destroy(&document->animation);
    if (document->release_image != NULL && document->images != NULL) {
        for (size_t i = 0U; i < document->shape_count; ++i) {
            svg_shape_image *stored = &document->images[i];
            if (stored->rgba == NULL) continue;
            svg_shape_decoded_image image = {
                .rgba = stored->rgba,
                .byte_count = stored->byte_count,
                .row_stride = stored->row_stride,
                .width = stored->pixel_width,
                .height = stored->pixel_height,
                .owner = stored->owner
            };
            document->release_image(document->media_context, &image);
        }
    }
    for (size_t i = 0U; i < document->shape_count; ++i)
        svg_clip_destroy(document->clips == NULL ? NULL : document->clips[i]);
    for (size_t i = 0U; i < document->mask_count; ++i) {
        svg_shape_document *content = document->masks[i].content;
        if (content == NULL) continue;
        svg_shape_document_destroy(content);
        svg_shape_free(content);
    }
    for (size_t i = 0U; i < document->pattern_count; ++i)
        svg_shape_free(document->patterns[i].rgba);
    svg_shape_free(document->edges);
    svg_shape_free(document->colors);
    svg_shape_free(document->shape_ids);
    svg_shape_free(document->shape_evenodd);
    svg_shape_free(document->shape_filled);
    svg_shape_free(document->shape_stroke_closed);
    svg_shape_free(document->shape_edge_start);
    svg_shape_free(document->shape_edge_count);
    svg_shape_free(document->shape_subpath_start);
    svg_shape_free(document->shape_subpath_count);
    svg_shape_free(document->path_subpaths);
    svg_shape_free(document->path_segments);
    svg_shape_free(document->path_numbers);
    svg_shape_free(document->shape_sources);
    svg_shape_free(document->transform_nodes);
    svg_shape_free(document->images);
    svg_shape_free(document->opacity_groups);
    svg_shape_free(document->filters);
    svg_shape_free(document->filter_primitives);
    svg_shape_free(document->masks);
    svg_shape_free(document->patterns);
    svg_shape_free(document->shape_stroke_inverse);
    svg_shape_free(document->strokes);
    svg_shape_free(document->paints);
    svg_shape_free(document->paint_indices_by_shape);
    svg_shape_free(document->stroke_paints);
    svg_shape_free(document->stroke_paint_indices_by_shape);
    svg_shape_free(document->stroke_dash_values);
    svg_shape_free(document->paint_stops);
    svg_shape_free(document->clips);
    *document = (svg_shape_document){0};
}

const svg_shape_paint *svg_shape_paint_for_shape(
    const svg_shape_document *document, size_t shape_index)
{
    if (document == NULL || shape_index >= document->shape_count ||
        document->paint_indices_by_shape == NULL || document->paints == NULL)
        return NULL;
    uint32_t stored = document->paint_indices_by_shape[shape_index];
    if (stored == 0U || (size_t)(stored - 1U) >= document->paint_count)
        return NULL;
    return &document->paints[stored - 1U];
}

svg_shape_paint *svg_shape_paint_for_shape_mutable(
    svg_shape_document *document, size_t shape_index)
{
    return (svg_shape_paint *)svg_shape_paint_for_shape(document, shape_index);
}

const svg_shape_paint *svg_shape_stroke_paint_for_shape(
    const svg_shape_document *document, size_t shape_index)
{
    if (document == NULL || shape_index >= document->shape_count ||
        document->stroke_paint_indices_by_shape == NULL ||
        document->stroke_paints == NULL)
        return NULL;
    uint32_t stored = document->stroke_paint_indices_by_shape[shape_index];
    if (stored == 0U || (size_t)(stored - 1U) >= document->stroke_paint_count)
        return NULL;
    return &document->stroke_paints[stored - 1U];
}

svg_shape_paint *svg_shape_stroke_paint_for_shape_mutable(
    svg_shape_document *document, size_t shape_index)
{
    return (svg_shape_paint *)svg_shape_stroke_paint_for_shape(
        document, shape_index);
}

const double *svg_shape_stroke_dash_values_for_shape(
    const svg_shape_document *document, size_t shape_index)
{
    if (document == NULL || shape_index >= document->shape_count ||
        document->strokes == NULL) return NULL;
    const svg_shape_stroke *stroke = &document->strokes[shape_index];
    if (stroke->dash_count == 0U ||
        stroke->dash_value_start > document->stroke_dash_value_count ||
        stroke->dash_count > document->stroke_dash_value_count -
            stroke->dash_value_start || document->stroke_dash_values == NULL)
        return NULL;
    return &document->stroke_dash_values[stroke->dash_value_start];
}

const char *svg_shape_error_name(svg_shape_error error)
{
    static const char *names[] = {
        "none", "invalid-argument", "xml", "unsupported", "number", "limit",
        "path", "viewport", "oom", "unsupported-feTurbulence",
        "unsupported-feDisplacementMap", "unsupported-feConvolveMatrix",
        "unsupported-feImage", "unsupported-feLighting",
        "unsupported-feMorphology"};
    return (unsigned)error < sizeof(names) / sizeof(names[0]) ? names[error] : "invalid-error";
}
