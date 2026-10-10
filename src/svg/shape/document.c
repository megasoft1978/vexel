#include "shape.h"
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
    svg_shape_free(document->images);
    svg_shape_free(document->opacity_groups);
    svg_shape_free(document->shape_stroke_inverse);
    svg_shape_free(document->strokes);
    svg_shape_free(document->paints);
    svg_shape_free(document->paint_stops);
    svg_shape_free(document->clips);
    *document = (svg_shape_document){0};
}

const char *svg_shape_error_name(svg_shape_error error)
{
    static const char *names[] = {"none", "invalid-argument", "xml", "unsupported",
                                  "number", "limit", "path", "viewport", "oom"};
    return (unsigned)error < sizeof(names) / sizeof(names[0]) ? names[error] : "invalid-error";
}
