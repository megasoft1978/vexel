/* Regression: marker emission appends shapes/edges to the document and reallocates
 * its edge array; the marker code must not hold pointers into it (heap-use-after-free
 * under ASan before the fix). */
#include "shape.h"

#include <stdio.h>
#include <string.h>

static int render_case(const char *label, const char *source)
{
    svg_shape_document document;
    svg_shape_diagnostic diagnostic = {SVG_SHAPE_ERROR_NONE, 0U};
    svg_shape_error error = svg_shape_decode(
        (base_bytes){(const uint8_t *)source, strlen(source)}, &document, &diagnostic);
    if (error != SVG_SHAPE_ERROR_NONE) {
        (void)fprintf(stderr, "%s: decode failed (%d at %zu)\n", label, (int)error,
                      diagnostic.offset);
        return 1;
    }
    svg_shape_document_destroy(&document);
    return 0;
}

int main(void)
{
    static char polyline[16384];
    static char path[16384];
    size_t used = 0U, path_used = 0U;
    used += (size_t)snprintf(polyline + used, sizeof polyline - used,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"500\" height=\"60\"><defs>"
        "<marker id=\"m\" markerWidth=\"8\" markerHeight=\"8\" refX=\"4\" refY=\"4\">");
    for (int i = 0; i < 40; ++i)
        used += (size_t)snprintf(polyline + used, sizeof polyline - used,
            "<circle cx=\"%d\" cy=\"3\" r=\"2\"/>", i);
    used += (size_t)snprintf(polyline + used, sizeof polyline - used,
        "</marker></defs><polyline points=\"");
    for (int i = 0; i < 40; ++i)
        used += (size_t)snprintf(polyline + used, sizeof polyline - used, "%d,%d ",
                                 10 * i + 5, (i % 2) * 20 + 10);
    used += (size_t)snprintf(polyline + used, sizeof polyline - used,
        "\" fill=\"none\" stroke=\"black\" marker-start=\"url(#m)\" "
        "marker-mid=\"url(#m)\" marker-end=\"url(#m)\"/></svg>");
    path_used += (size_t)snprintf(path + path_used, sizeof path - path_used,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"300\" height=\"60\"><defs>"
        "<marker id=\"m\" markerWidth=\"8\" markerHeight=\"8\" refX=\"4\" refY=\"4\">");
    for (int i = 0; i < 40; ++i)
        path_used += (size_t)snprintf(path + path_used, sizeof path - path_used,
            "<circle cx=\"%d\" cy=\"3\" r=\"2\"/>", i);
    path_used += (size_t)snprintf(path + path_used, sizeof path - path_used,
        "</marker></defs><path d=\"M5 10 L50 40 L90 10 L130 40 L170 10 L210 40 Z\" "
        "fill=\"none\" stroke=\"red\" marker-start=\"url(#m)\" marker-mid=\"url(#m)\" "
        "marker-end=\"url(#m)\"/></svg>");
    static char segments[16384];
    size_t seg_used = (size_t)snprintf(segments, sizeof segments,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"64\" height=\"64\" viewBox=\"0 0 64 64\">"
        "<defs><marker id=\"m\" markerWidth=\"1\" markerHeight=\"1\"><path d=\"M0 0 L1 1\"/></marker></defs>"
        "<path style=\"marker-start:none;marker-mid:none;marker-end:url(#m)\" d=\"M0 0");
    for (int i = 1; i <= 30; ++i) /* + M0 and the final M = exactly 32 segments */
        seg_used += (size_t)snprintf(segments + seg_used, sizeof segments - seg_used, " L%d 0", i);
    seg_used += (size_t)snprintf(segments + seg_used, sizeof segments - seg_used, " M31 0\"/></svg>");
    if (used >= sizeof polyline || path_used >= sizeof path || seg_used >= sizeof segments) return 2;
    return render_case("polyline", polyline) | render_case("path", path) |
           render_case("path-segments", segments);
}
