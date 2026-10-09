#include "paint.h"

bool paint_edge_span_check(void);

bool paint_edge_span_check(void)
{
    svg_shape_document empty_document = {0};
    const geom_edge *edges = NULL;
    bool formed = true;
    edges = svg_paint_edge_span(&empty_document, 0U, 0U, &formed);
    if (edges != NULL || formed) return false;

    geom_edge edge = {0};
    svg_shape_document one_edge_document = {.edges = &edge, .edge_count = 1U};
    formed = false;
    return svg_paint_edge_span(&one_edge_document, 0U, 1U, &formed) == &edge &&
           formed;
}
