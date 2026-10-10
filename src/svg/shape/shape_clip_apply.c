#include "shape_internal.h"

#include "../clip/clip.h"

static void fail(svg_shape_diagnostic *diagnostic, svg_shape_error error,
                 size_t offset)
{
    if (diagnostic != NULL)
        *diagnostic = (svg_shape_diagnostic){error, offset};
}

bool svg_shape_apply_declared_clip(base_bytes source,
                                   const svg_xml_token *tokens,
                                   size_t token_count,
                                   svg_reference_index *references,
                                   svg_css_sheet *css,
                                   svg_shape_document *document,
                                   svg_xml_span value,
                                   size_t first_shape, size_t first_edge,
                                   size_t owner_node,
                                   geom_affine transform,
                                   svg_shape_diagnostic *diagnostic)
{
    if (document == NULL || tokens == NULL || references == NULL || css == NULL ||
        first_shape > document->shape_count || first_edge > document->edge_count) {
        fail(diagnostic, SVG_SHAPE_ERROR_INVALID_ARGUMENT, value.offset);
        return false;
    }
    size_t end_shape = document->shape_count;
    if (first_shape == end_shape) return true;
    geom_affine inverse;
    double bounds[4];
    if (!svg_shape_affine_inverse(transform, &inverse) ||
        !svg_clip_local_bounds(document, first_edge,
                               document->edge_count - first_edge,
                               inverse, bounds)) {
        fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, value.offset);
        return false;
    }
    svg_shape_clip *clip = NULL;
    svg_shape_error result = svg_clip_resolve(
        source, tokens, token_count, references, css, value, document,
        &document->clip_bytes, inverse, bounds, owner_node, &clip,
        diagnostic);
    if (result != SVG_SHAPE_ERROR_NONE) return false;
    bool applied = svg_clip_apply_group(document, first_shape, end_shape, clip);
    svg_clip_destroy(clip);
    if (!applied) {
        fail(diagnostic, SVG_SHAPE_ERROR_LIMIT, value.offset);
        return false;
    }
    document->has_clips = true;
    return true;
}
