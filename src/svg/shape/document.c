#include "shape.h"
#include "shape_internal.h"

#include "../clip/clip.h"
#include "../reference/reference.h"

#include <stdlib.h>

svg_shape_error svg_shape_decode(base_bytes source, svg_shape_document *out,
                                 svg_shape_diagnostic *diagnostic)
{
    if (out == NULL || !base_bytes_valid(source)) {
        if (diagnostic != NULL) *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U};
        return SVG_SHAPE_ERROR_INVALID_ARGUMENT;
    }
    base_bytes expanded = {0};
    svg_reference_source_map map = {0};
    svg_shape_error error = svg_reference_expand_mapped(source, &expanded, &map, diagnostic);
    if (error != SVG_SHAPE_ERROR_NONE) { *out = (svg_shape_document){0}; return error; }
    error = svg_shape_decode_source(expanded, out, diagnostic);
    if (error != SVG_SHAPE_ERROR_NONE && expanded.ptr != source.ptr && diagnostic != NULL)
        diagnostic->offset = svg_reference_map_offset(&map, diagnostic->offset);
    if (error == SVG_SHAPE_ERROR_NONE) out->source = source;
    if (expanded.ptr != source.ptr) free((void *)expanded.ptr);
    svg_reference_source_map_destroy(&map);
    return error;
}

void svg_shape_document_destroy(svg_shape_document *document)
{
    if (document == NULL) return;
    for (size_t i = 0U; i < document->shape_count; ++i)
        svg_clip_destroy(document->clips == NULL ? NULL : document->clips[i]);
    free(document->edges);
    free(document->colors);
    free(document->shape_ids);
    free(document->shape_evenodd);
    free(document->shape_filled);
    free(document->shape_stroke_closed);
    free(document->shape_edge_start);
    free(document->shape_edge_count);
    free(document->shape_stroke_inverse);
    free(document->strokes);
    free(document->paints);
    free(document->paint_stops);
    free(document->clips);
    *document = (svg_shape_document){0};
}

const char *svg_shape_error_name(svg_shape_error error)
{
    static const char *names[] = {"none", "invalid-argument", "xml", "unsupported",
                                  "number", "limit", "path", "viewport", "oom"};
    return (unsigned)error < sizeof(names) / sizeof(names[0]) ? names[error] : "invalid-error";
}
