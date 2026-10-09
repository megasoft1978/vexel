#include "shape.h"
#include "test_reference.h"
#include "shape_render.h"

#include <stdio.h>
#include <math.h>
#include <stdlib.h>
#include <string.h>

bool clip_boundary_accounting_check(void);
bool paint_edge_span_check(void);

static double segment_distance(geom_point p, geom_edge edge)
{
    double dx=edge.b.x-edge.a.x, dy=edge.b.y-edge.a.y;
    double length=dx*dx+dy*dy;
    double t=((p.x-edge.a.x)*dx+(p.y-edge.a.y)*dy)/length;
    if(t<0.0)t=0.0;else if(t>1.0)t=1.0;
    return hypot(p.x-(edge.a.x+t*dx),p.y-(edge.a.y+t*dy));
}

static bool paths_equivalent(const char *left, const char *right)
{
    char left_svg[512], right_svg[512];
    int left_len = snprintf(left_svg, sizeof(left_svg), "<svg width=\"16\" height=\"16\"><path d=\"%s\"/></svg>", left);
    int right_len = snprintf(right_svg, sizeof(right_svg), "<svg width=\"16\" height=\"16\"><path d=\"%s\"/></svg>", right);
    if (left_len < 0 || right_len < 0 || (size_t)left_len >= sizeof(left_svg) || (size_t)right_len >= sizeof(right_svg)) return false;
    svg_shape_document a, b; svg_shape_diagnostic diagnostic;
    bool ok = svg_shape_decode((base_bytes){(const uint8_t *)left_svg, (size_t)left_len}, &a, &diagnostic) == SVG_SHAPE_ERROR_NONE;
    if (!ok) return false;
    ok = svg_shape_decode((base_bytes){(const uint8_t *)right_svg, (size_t)right_len}, &b, &diagnostic) == SVG_SHAPE_ERROR_NONE;
    if (!ok) { svg_shape_document_destroy(&a); return false; }
    ok = a.source.ptr == (const uint8_t *)left_svg && a.source.len == (size_t)left_len &&
         b.source.ptr == (const uint8_t *)right_svg && b.source.len == (size_t)right_len &&
         a.edge_count == b.edge_count;
    if (!ok) fprintf(stderr, "edge count %zu != %zu for %s vs %s\n", a.edge_count, b.edge_count, left, right);
    for (size_t i = 0; ok && i < a.edge_count; i++) {
        ok = fabs(a.edges[i].a.x - b.edges[i].a.x) < 1e-12 &&
             fabs(a.edges[i].a.y - b.edges[i].a.y) < 1e-12 &&
             fabs(a.edges[i].b.x - b.edges[i].b.x) < 1e-12 &&
             fabs(a.edges[i].b.y - b.edges[i].b.y) < 1e-12;
        if (!ok) fprintf(stderr, "edge mismatch %zu: %.15g,%.15g -> %.15g,%.15g / %.15g,%.15g -> %.15g,%.15g\n", i, a.edges[i].a.x, a.edges[i].a.y, a.edges[i].b.x, a.edges[i].b.y, b.edges[i].a.x, b.edges[i].a.y, b.edges[i].b.x, b.edges[i].b.y);
    }
    svg_shape_document_destroy(&a); svg_shape_document_destroy(&b);
    return ok;
}

static bool render_source(const char *source, raster_tile *tile, svg_shape_document *doc,
                         svg_shape_diagnostic *diagnostic)
{
    size_t length = strlen(source);
    if (svg_shape_decode((base_bytes){(const uint8_t *)source, length}, doc, diagnostic) != SVG_SHAPE_ERROR_NONE) return false;
    if (!raster_shape_render(doc, tile, diagnostic)) {
        svg_shape_document_destroy(doc);
        return false;
    }
    return true;
}

static uint8_t pixel_alpha(const raster_tile *tile, uint32_t x, uint32_t y)
{
    return tile->rgba[((size_t)y * tile->width + x) * 4U + 3U];
}

static bool alpha_at(const char *source, uint32_t x, uint32_t y, uint8_t *alpha)
{
    raster_tile tile = {0}; svg_shape_document doc = {0};
    svg_shape_diagnostic diag;
    if (!render_source(source, &tile, &doc, &diag)) return false;
    *alpha = pixel_alpha(&tile, x, y);
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    return true;
}

static bool antialias_and_fill_checks(void)
{
    static const char same_nonzero[] = "<svg width=\"16\" height=\"16\"><path fill=\"red\" d=\"M1 1 H15 V15 H1 Z M5 5 H11 V11 H5 Z\"/></svg>";
    static const char same_evenodd[] = "<svg width=\"16\" height=\"16\"><path fill=\"red\" fill-rule=\"evenodd\" d=\"M1 1 H15 V15 H1 Z M5 5 H11 V11 H5 Z\"/></svg>";
    static const char opposite_nonzero[] = "<svg width=\"16\" height=\"16\"><path fill=\"red\" d=\"M1 1 H15 V15 H1 Z M5 5 V11 H11 V5 Z\"/></svg>";
    static const char fractional[] = "<svg width=\"8\" height=\"8\"><rect x=\"1.25\" y=\"1.25\" width=\"4\" height=\"4\" fill=\"#336699\"/></svg>";
    const char *cases[] = {same_nonzero, same_evenodd, opposite_nonzero, fractional};
    raster_tile tiles[4] = {{0}};
    svg_shape_document docs[4] = {{0}};
    svg_shape_diagnostic diag;
    for (size_t i = 0; i < 4U; i++) {
        if (!render_source(cases[i], &tiles[i], &docs[i], &diag)) return false;
    }
    bool ok = pixel_alpha(&tiles[0], 7U, 7U) == 255U &&
              pixel_alpha(&tiles[1], 7U, 7U) == 0U &&
              pixel_alpha(&tiles[2], 7U, 7U) == 0U &&
              tiles[0].rgba[((size_t)7U * 16U + 7U) * 4U] == 255U &&
              tiles[1].rgba[((size_t)7U * 16U + 7U) * 4U] == 0U &&
              pixel_alpha(&tiles[0], 0U, 0U) == 0U &&
              pixel_alpha(&tiles[3], 1U, 1U) > 0U && pixel_alpha(&tiles[3], 1U, 1U) < 255U &&
              pixel_alpha(&tiles[3], 2U, 2U) == 255U &&
              tiles[3].rgba[((size_t)2U * 8U + 2U) * 4U] == 51U &&
              tiles[3].rgba[((size_t)2U * 8U + 2U) * 4U + 1U] == 102U;
    ok = ok && docs[3].source.ptr == (const uint8_t *)fractional &&
         docs[3].source.len == strlen(fractional) &&
         pixel_alpha(&tiles[3], 0U, 0U) == 0U &&
         tiles[3].rgba[0] == 0U && tiles[3].rgba[1] == 0U && tiles[3].rgba[2] == 0U;
    raster_tile repeat = {0};
    if (ok) ok = raster_shape_render(&docs[3], &repeat, &diag) &&
                 repeat.byte_count == tiles[3].byte_count &&
                 memcmp(repeat.rgba, tiles[3].rgba, repeat.byte_count) == 0;
    raster_tile_destroy(&repeat);
    for (size_t i = 0; i < 4U; i++) {
        raster_tile_destroy(&tiles[i]);
        svg_shape_document_destroy(&docs[i]);
    }
    if (!ok) return false;

    static const char order[] = "<svg width=\"8\" height=\"8\"><rect x=\"1\" y=\"1\" width=\"5\" height=\"5\" fill=\"blue\"/><rect x=\"3.5\" y=\"1\" width=\"3\" height=\"5\" fill=\"red\"/></svg>";
    raster_tile ordered = {0}; svg_shape_document ordered_doc = {0};
    if (!render_source(order, &ordered, &ordered_doc, &diag)) return false;
    ok = ordered.rgba[((size_t)2U * 8U + 4U) * 4U] == 255U &&
         ordered.rgba[((size_t)2U * 8U + 4U) * 4U + 2U] == 0U &&
         pixel_alpha(&ordered, 2U, 2U) == 255U;
    raster_tile_destroy(&ordered); svg_shape_document_destroy(&ordered_doc);
    if (!ok) return false;

    static const char aligned[] = "<svg width=\"16\" height=\"16\"><rect x=\"0\" y=\"0\" width=\"8\" height=\"8\"/><rect x=\"8\" y=\"8\" width=\"8\" height=\"8\"/></svg>";
    raster_tile boundary = {0}; svg_shape_document boundary_doc = {0};
    if (!render_source(aligned, &boundary, &boundary_doc, &diag)) return false;
    ok = pixel_alpha(&boundary, 0U, 0U) == 255U &&
         pixel_alpha(&boundary, 8U, 0U) == 0U &&
         pixel_alpha(&boundary, 0U, 8U) == 0U &&
         pixel_alpha(&boundary, 8U, 8U) == 255U;
    raster_tile_destroy(&boundary); svg_shape_document_destroy(&boundary_doc);
    if (!ok) return false;

    svg_shape_document oversize = {0};
    bool evenodd = false, filled = true;
    oversize.width = 4096U; oversize.height = 4096U;
    oversize.view_width = 4096.0; oversize.view_height = 4096.0;
    oversize.shape_count = 1U;
    oversize.shape_evenodd = &evenodd;
    oversize.shape_filled = &filled;
    raster_tile rejected = {0};
    if (raster_shape_render(&oversize, &rejected, &diag) ||
        diag.category != SVG_SHAPE_ERROR_LIMIT || rejected.rgba != NULL) return false;
    return true;
}

static bool gradient_checks(void)
{
    static const char linear[] = "<svg width=\"16\" height=\"16\"><defs><linearGradient id=\"g\"><stop offset=\"0\" stop-color=\"#ff0000\"/><stop offset=\"1\" stop-color=\"#0000ff\"/></linearGradient></defs><rect width=\"16\" height=\"16\" fill=\"url(#g)\"/></svg>";
    raster_tile tile = {0}; svg_shape_document doc = {0}; svg_shape_diagnostic diag;
    if (!render_source(linear, &tile, &doc, &diag)) return false;
    size_t left = ((size_t)8U * 16U + 1U) * 4U;
    size_t right = ((size_t)8U * 16U + 14U) * 4U;
    bool ok = tile.rgba[left] > tile.rgba[left + 2U] &&
              tile.rgba[right + 2U] > tile.rgba[right] &&
              tile.rgba[left + 3U] == 255U && tile.rgba[right + 3U] == 255U;
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!ok) return false;
    static const char default_stop_color[] = "<svg width=\"8\" height=\"8\"><defs><linearGradient id=\"g\"><stop offset=\"0\"/><stop offset=\"1\" stop-color=\"white\"/></linearGradient></defs><rect width=\"8\" height=\"8\" fill=\"url(#g)\"/></svg>";
    if (!render_source(default_stop_color, &tile, &doc, &diag)) return false;
    ok = tile.rgba[0] < 40U && tile.rgba[1] < 40U && tile.rgba[2] < 40U &&
         tile.rgba[3] == 255U;
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!ok) return false;
    static const char radial[] = "<svg width=\"16\" height=\"16\"><defs><radialGradient id=\"g\"><stop stop-color=\"white\"/><stop offset=\"1\" stop-color=\"black\" stop-opacity=\"0\"/></radialGradient></defs><circle cx=\"8\" cy=\"8\" r=\"7\" fill=\"url(#g)\"/></svg>";
    if (!render_source(radial, &tile, &doc, &diag)) return false;
    size_t center = ((size_t)8U * 16U + 8U) * 4U;
    size_t edge = ((size_t)8U * 16U + 14U) * 4U;
    ok = tile.rgba[center] > tile.rgba[edge] && tile.rgba[center + 3U] > tile.rgba[edge + 3U];
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!ok) return false;
    static const char translucent_gradient_overpaint[] =
        "<svg width=\"8\" height=\"8\"><rect width=\"8\" height=\"8\" fill=\"red\"/>"
        "<defs><linearGradient id=\"g\"><stop stop-color=\"blue\" stop-opacity=\"0.5\"/>"
        "<stop offset=\"1\" stop-color=\"blue\" stop-opacity=\"0.5\"/>"
        "</linearGradient></defs><rect width=\"8\" height=\"8\" fill=\"url(#g)\"/></svg>";
    if (!render_source(translucent_gradient_overpaint, &tile, &doc, &diag)) return false;
    size_t blended = ((size_t)4U * 8U + 4U) * 4U;
    ok = tile.rgba[blended] == 127U && tile.rgba[blended + 1U] == 0U &&
         tile.rgba[blended + 2U] == 128U && tile.rgba[blended + 3U] == 255U;
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!ok) return false;
    static const char translucent_solid_overpaint[] =
        "<svg width=\"8\" height=\"8\"><rect width=\"8\" height=\"8\" fill=\"red\"/>"
        "<rect width=\"8\" height=\"8\" fill=\"blue\"/></svg>";
    if (svg_shape_decode((base_bytes){(const uint8_t *)translucent_solid_overpaint,
                                      strlen(translucent_solid_overpaint)},
                         &doc, &diag) != SVG_SHAPE_ERROR_NONE) return false;
    for (size_t edge_index = 0U; edge_index < doc.edge_count; ++edge_index)
        if (doc.shape_ids[edge_index] == 1U)
            doc.colors[edge_index * 4U + 3U] = 128U;
    bool solid_rendered = raster_shape_render(&doc, &tile, &diag);
    size_t solid_blended = ((size_t)4U * 8U + 4U) * 4U;
    ok = solid_rendered && tile.rgba[solid_blended] == 127U &&
         tile.rgba[solid_blended + 1U] == 0U &&
         tile.rgba[solid_blended + 2U] == 128U &&
         tile.rgba[solid_blended + 3U] == 255U;
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!ok) return false;
    static const char user_space[] = "<svg width=\"16\" height=\"16\"><defs><linearGradient id=\"g\" gradientUnits=\"userSpaceOnUse\" x1=\"0\" y1=\"0\" x2=\"16\" y2=\"0\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient></defs><style>rect{fill:url(#g)}</style><rect width=\"16\" height=\"16\"/></svg>";
    if (!render_source(user_space, &tile, &doc, &diag)) return false;
    left = ((size_t)8U * 16U + 1U) * 4U;
    right = ((size_t)8U * 16U + 14U) * 4U;
    ok = tile.rgba[left] > tile.rgba[left + 2U] && tile.rgba[right + 2U] > tile.rgba[right];
    size_t first = ((size_t)8U * 16U) * 4U;
    size_t last = ((size_t)8U * 16U + 15U) * 4U;
    size_t midpoint = ((size_t)8U * 16U + 8U) * 4U;
    ok = ok && tile.rgba[first] == 247U && tile.rgba[first + 2U] == 8U &&
         tile.rgba[midpoint] == 120U && tile.rgba[midpoint + 2U] == 135U &&
         tile.rgba[last] == 8U && tile.rgba[last + 2U] == 247U;
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!ok) return false;
    static const char user_space_radial_default[] = "<svg width=\"16\" height=\"16\"><defs><radialGradient id=\"g\" gradientUnits=\"userSpaceOnUse\"><stop stop-color=\"white\"/><stop offset=\"1\" stop-color=\"black\"/></radialGradient></defs><rect x=\"6\" y=\"6\" width=\"4\" height=\"4\" fill=\"url(#g)\"/></svg>";
    if (!render_source(user_space_radial_default, &tile, &doc, &diag)) return false;
    size_t radial_edge = ((size_t)6U * 16U + 6U) * 4U;
    size_t radial_center = ((size_t)8U * 16U + 8U) * 4U;
    ok = tile.rgba[radial_edge] > 180U && tile.rgba[radial_edge] < 220U &&
         tile.rgba[radial_center] > 200U;
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!ok) return false;
    static const char transformed_user_gradient[] =
        "<svg width=\"16\" height=\"16\"><defs><linearGradient id=\"g\" gradientUnits=\"userSpaceOnUse\" x1=\"0\" y1=\"0\" x2=\"8\" y2=\"0\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient></defs><g transform=\"translate(4 0)\"><rect width=\"8\" height=\"8\" fill=\"url(#g)\"/></g></svg>";
    if (!render_source(transformed_user_gradient, &tile, &doc, &diag)) return false;
    size_t transformed_left = ((size_t)4U * 16U + 4U) * 4U;
    size_t transformed_right = ((size_t)4U * 16U + 11U) * 4U;
    ok = tile.rgba[transformed_left] > tile.rgba[transformed_left + 2U] &&
         tile.rgba[transformed_right + 2U] > tile.rgba[transformed_right];
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!ok) return false;
    static const char transformed_object_gradient[] =
        "<svg width=\"16\" height=\"16\"><defs><linearGradient id=\"g\" x1=\"0\" y1=\"0\" x2=\"1\" y2=\"0\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient></defs><g transform=\"translate(12 0) rotate(90)\"><rect width=\"4\" height=\"8\" fill=\"url(#g)\"/></g></svg>";
    if (!render_source(transformed_object_gradient, &tile, &doc, &diag)) return false;
    size_t transformed_top = ((size_t)0U * 16U + 8U) * 4U;
    size_t transformed_bottom = ((size_t)3U * 16U + 8U) * 4U;
    ok = tile.rgba[transformed_top] > tile.rgba[transformed_top + 2U] &&
         tile.rgba[transformed_bottom + 2U] > tile.rgba[transformed_bottom];
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!ok) return false;
    static const char transformed_user_radial[] =
        "<svg width=\"16\" height=\"16\"><defs><radialGradient id=\"g\" gradientUnits=\"userSpaceOnUse\" cx=\"2\" cy=\"2\" r=\"4\"><stop stop-color=\"white\"/><stop offset=\"1\" stop-color=\"black\"/></radialGradient></defs><g transform=\"translate(8 8)\"><rect width=\"4\" height=\"4\" fill=\"url(#g)\"/></g></svg>";
    if (!render_source(transformed_user_radial, &tile, &doc, &diag)) return false;
    size_t user_radial_center = ((size_t)10U * 16U + 10U) * 4U;
    size_t user_radial_edge = ((size_t)8U * 16U + 8U) * 4U;
    ok = tile.rgba[user_radial_center] > tile.rgba[user_radial_edge];
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!ok) return false;
    static const char transformed_object_radial[] =
        "<svg width=\"16\" height=\"16\"><defs><radialGradient id=\"g\" cx=\"0.5\" cy=\"0.5\" r=\"1\"><stop stop-color=\"white\"/><stop offset=\"1\" stop-color=\"black\"/></radialGradient></defs><g transform=\"translate(8 8) scale(1.5)\"><rect width=\"4\" height=\"4\" fill=\"url(#g)\"/></g></svg>";
    if (!render_source(transformed_object_radial, &tile, &doc, &diag)) return false;
    size_t object_radial_center = ((size_t)11U * 16U + 11U) * 4U;
    size_t object_radial_edge = ((size_t)8U * 16U + 8U) * 4U;
    ok = tile.rgba[object_radial_center] > tile.rgba[object_radial_edge];
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!ok) return false;
    static const char singular_gradient[] =
        "<svg width=\"8\" height=\"8\"><defs><linearGradient id=\"g\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient></defs><rect width=\"4\" height=\"4\" transform=\"scale(0)\" fill=\"url(#g)\"/></svg>";
    if (svg_shape_decode((base_bytes){(const uint8_t *)singular_gradient,
                                      strlen(singular_gradient)},
                         &doc, &diag) != SVG_SHAPE_ERROR_UNSUPPORTED ||
        diag.category != SVG_SHAPE_ERROR_UNSUPPORTED) return false;
    static const char styled_stops[] =
        "<svg width=\"8\" height=\"8\"><style>.end{stop-color:#00f;stop-opacity:1}</style><defs><linearGradient id=\"g\"><stop style=\"stop-color:#f00;stop-opacity:1\"/><stop class=\"end\" offset=\"1\"/></linearGradient></defs><rect width=\"8\" height=\"8\" fill=\"url(#g)\"/></svg>";
    if (!render_source(styled_stops, &tile, &doc, &diag)) return false;
    ok = tile.rgba[0] > tile.rgba[2] &&
         tile.rgba[((size_t)4U * 8U + 7U) * 4U + 2U] >
             tile.rgba[((size_t)4U * 8U + 7U) * 4U];
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!ok) return false;
    static const char unsupported_stop_style[] =
        "<svg width=\"8\" height=\"8\"><defs><linearGradient id=\"g\"><stop style=\"stop-color:currentColor\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient></defs><rect width=\"8\" height=\"8\" fill=\"url(#g)\"/></svg>";
    if (svg_shape_decode((base_bytes){(const uint8_t *)unsupported_stop_style,
                                      strlen(unsupported_stop_style)},
                         &doc, &diag) != SVG_SHAPE_ERROR_UNSUPPORTED ||
        diag.category != SVG_SHAPE_ERROR_UNSUPPORTED) return false;
    static const char unsupported_stop_stylesheet[] =
        "<svg width=\"8\" height=\"8\"><style>stop{color:red}</style><defs><linearGradient id=\"g\"><stop/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient></defs><rect width=\"8\" height=\"8\" fill=\"url(#g)\"/></svg>";
    if (svg_shape_decode((base_bytes){
            (const uint8_t *)unsupported_stop_stylesheet,
            strlen(unsupported_stop_stylesheet)}, &doc, &diag) !=
            SVG_SHAPE_ERROR_UNSUPPORTED ||
        diag.category != SVG_SHAPE_ERROR_UNSUPPORTED) return false;
    static const char unresolved_stop_class[] =
        "<svg width=\"8\" height=\"8\"><style>.used{stop-color:red}</style><defs><linearGradient id=\"g\"><stop class=\"missing\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient></defs><rect width=\"8\" height=\"8\" fill=\"url(#g)\"/></svg>";
    if (svg_shape_decode((base_bytes){(const uint8_t *)unresolved_stop_class,
                                      strlen(unresolved_stop_class)},
                         &doc, &diag) != SVG_SHAPE_ERROR_UNSUPPORTED ||
        diag.category != SVG_SHAPE_ERROR_UNSUPPORTED) return false;
    static const char *invalid[] = {
        "<svg width=\"8\" height=\"8\"><defs><linearGradient id=\"g\" spreadMethod=\"reflect\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient></defs><rect width=\"8\" height=\"8\" fill=\"url(#g)\"/></svg>",
        "<svg width=\"8\" height=\"8\"><defs><linearGradient id=\"base\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient><linearGradient id=\"g\" href=\"#base\"/></defs><rect width=\"8\" height=\"8\" fill=\"url(#g)\"/></svg>",
        "<svg width=\"8\" height=\"8\"><rect width=\"8\" height=\"8\" fill=\"url(https://example.invalid/g)\"/></svg>",
    };
    for (size_t i = 0U; i < sizeof(invalid) / sizeof(invalid[0]); ++i) {
        if (svg_shape_decode((base_bytes){(const uint8_t *)invalid[i], strlen(invalid[i])},
                             &doc, &diag) != SVG_SHAPE_ERROR_UNSUPPORTED) return false;
    }
    static const char *gradient_href_references[] = {
        "<svg width=\"8\" height=\"8\"><defs><linearGradient id=\"base\" x2=\"8\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient><linearGradient id=\"g\" href=\"#base\"><stop stop-color=\"white\"/><stop offset=\"1\" stop-color=\"black\"/></linearGradient></defs><rect width=\"8\" height=\"8\" fill=\"url(#g)\"/></svg>",
        "<svg xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"8\" height=\"8\"><defs><linearGradient id=\"base\" x2=\"8\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient><linearGradient id=\"g\" xlink:href=\"#base\"><stop stop-color=\"white\"/><stop offset=\"1\" stop-color=\"black\"/></linearGradient></defs><rect width=\"8\" height=\"8\" fill=\"url(#g)\"/></svg>"
    };
    for (size_t i = 0U; i < sizeof(gradient_href_references) /
                            sizeof(gradient_href_references[0]); ++i) {
        const char *source = gradient_href_references[i];
        const char *attribute = strstr(source, i == 0U ? "href=\"" : "xlink:href");
        if (attribute == NULL ||
            svg_shape_decode((base_bytes){(const uint8_t *)source, strlen(source)},
                             &doc, &diag) != SVG_SHAPE_ERROR_UNSUPPORTED ||
            diag.category != SVG_SHAPE_ERROR_UNSUPPORTED ||
            diag.offset != (size_t)(attribute - source)) return false;
    }
    static const char percent_user_space[] = "<svg width=\"8\" height=\"8\"><defs><linearGradient id=\"g\" gradientUnits=\"userSpaceOnUse\" x2=\"50%\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient></defs><rect width=\"8\" height=\"8\" fill=\"url(#g)\"/></svg>";
    if (!render_source(percent_user_space, &tile, &doc, &diag)) return false;
    first = ((size_t)4U * 8U) * 4U;
    last = ((size_t)4U * 8U + 7U) * 4U;
    ok = tile.rgba[first] > tile.rgba[first + 2U] &&
         tile.rgba[last + 2U] > tile.rgba[last];
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!ok) return false;
    char many[32768];
    static const char many_prefix[] = "<svg width=\"8\" height=\"8\"><defs><linearGradient id=\"g\">";
    static const char many_stop[] = "<stop stop-color=\"red\"/>";
    static const char many_suffix[] = "</linearGradient></defs><rect width=\"8\" height=\"8\" fill=\"url(#g)\"/></svg>";
    size_t length = sizeof(many_prefix) - 1U;
    memcpy(many, many_prefix, length);
    for (size_t i = 0U; i < 257U; ++i) {
        memcpy(many + length, many_stop, sizeof(many_stop) - 1U);
        length += sizeof(many_stop) - 1U;
    }
    memcpy(many + length, many_suffix, sizeof(many_suffix) - 1U);
    length += sizeof(many_suffix) - 1U;
    return svg_shape_decode((base_bytes){(const uint8_t *)many, length}, &doc, &diag) ==
           SVG_SHAPE_ERROR_LIMIT && diag.category == SVG_SHAPE_ERROR_LIMIT;
}

static bool clip_checks(void)
{
    static const char object_box[] = "<svg width=\"16\" height=\"16\"><defs><clipPath id=\"c\" clipPathUnits=\"objectBoundingBox\"><rect width=\"0.5\" height=\"1\"/></clipPath></defs><rect x=\"2\" y=\"2\" width=\"8\" height=\"8\" fill=\"red\" clip-path=\"url(#c)\"/></svg>";
    raster_tile tile = {0}; svg_shape_document doc = {0}; svg_shape_diagnostic diag;
    if (!render_source(object_box, &tile, &doc, &diag)) return false;
    bool ok = pixel_alpha(&tile, 3U, 3U) == 255U && pixel_alpha(&tile, 7U, 3U) == 0U;
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!ok) return false;
    static const char user_space[] = "<svg width=\"16\" height=\"16\"><defs><clipPath id=\"c\" clipPathUnits=\"userSpaceOnUse\"><rect x=\"4\" y=\"0\" width=\"4\" height=\"16\"/><rect x=\"10\" y=\"0\" width=\"2\" height=\"16\"/></clipPath></defs><style>.target{clip-path:url(#c)}</style><rect class=\"target\" width=\"16\" height=\"16\" fill=\"blue\"/></svg>";
    if (!render_source(user_space, &tile, &doc, &diag)) return false;
    ok = pixel_alpha(&tile, 5U, 5U) == 255U && pixel_alpha(&tile, 10U, 5U) == 255U &&
         pixel_alpha(&tile, 2U, 5U) == 0U && pixel_alpha(&tile, 8U, 5U) == 0U;
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!ok) return false;
    static const char default_units[] = "<svg width=\"16\" height=\"16\"><defs><clipPath id=\"c\"><rect x=\"4\" y=\"0\" width=\"4\" height=\"16\"/></clipPath></defs><rect width=\"16\" height=\"16\" fill=\"red\" clip-path=\"url(#c)\"/></svg>";
    if (!render_source(default_units, &tile, &doc, &diag)) return false;
    ok = pixel_alpha(&tile, 5U, 5U) == 255U && pixel_alpha(&tile, 2U, 5U) == 0U;
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!ok) return false;
    static const char symbol_clip[] =
        "<svg width=\"8\" height=\"8\"><defs>"
        "<clipPath id=\"c\"><rect width=\"4\" height=\"8\"/></clipPath>"
        "<symbol id=\"s\" viewBox=\"0 0 8 8\" clip-path=\"url(#c)\">"
        "<rect width=\"8\" height=\"8\" fill=\"red\"/>"
        "</symbol></defs><use href=\"#s\" width=\"8\" height=\"8\"/></svg>";
    const char *symbol_clip_attribute = strstr(symbol_clip, "clip-path");
    if (symbol_clip_attribute == NULL ||
        svg_shape_decode((base_bytes){(const uint8_t *)symbol_clip,
                                      strlen(symbol_clip)}, &doc, &diag) !=
            SVG_SHAPE_ERROR_UNSUPPORTED ||
        diag.category != SVG_SHAPE_ERROR_UNSUPPORTED ||
        diag.offset != (size_t)(symbol_clip_attribute - symbol_clip)) return false;
    static const char symbol_css_clip[] =
        "<svg width=\"8\" height=\"8\"><style>symbol{clip-path:url(#c)}</style>"
        "<defs><clipPath id=\"c\"><rect width=\"4\" height=\"8\"/></clipPath>"
        "<symbol id=\"s\" viewBox=\"0 0 8 8\"><rect width=\"8\" height=\"8\"/></symbol>"
        "</defs><use href=\"#s\" width=\"8\" height=\"8\"/></svg>";
    const char *symbol_css_clip_value = strstr(symbol_css_clip, "url(#c)");
    if (symbol_css_clip_value == NULL ||
        svg_shape_decode((base_bytes){(const uint8_t *)symbol_css_clip,
                                      strlen(symbol_css_clip)}, &doc, &diag) !=
            SVG_SHAPE_ERROR_UNSUPPORTED ||
        diag.category != SVG_SHAPE_ERROR_UNSUPPORTED ||
        diag.offset != (size_t)(symbol_css_clip_value - symbol_css_clip)) return false;
    static const char transformed_user_clip[] =
        "<svg width=\"16\" height=\"16\"><defs><clipPath id=\"c\" clipPathUnits=\"userSpaceOnUse\"><rect width=\"4\" height=\"8\"/></clipPath></defs><g transform=\"translate(4 0)\"><rect width=\"8\" height=\"8\" fill=\"red\" clip-path=\"url(#c)\"/></g></svg>";
    if (!render_source(transformed_user_clip, &tile, &doc, &diag)) return false;
    ok = pixel_alpha(&tile, 5U, 5U) == 255U && pixel_alpha(&tile, 9U, 5U) == 0U;
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!ok) return false;
    static const char transformed_object_clip[] =
        "<svg width=\"16\" height=\"16\"><defs><clipPath id=\"c\" clipPathUnits=\"objectBoundingBox\"><rect width=\"0.5\" height=\"1\"/></clipPath></defs><g transform=\"translate(12 0) rotate(90)\"><rect width=\"4\" height=\"4\" fill=\"red\" clip-path=\"url(#c)\"/></g></svg>";
    if (!render_source(transformed_object_clip, &tile, &doc, &diag)) return false;
    ok = pixel_alpha(&tile, 10U, 1U) == 255U && pixel_alpha(&tile, 10U, 3U) == 0U;
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!ok) return false;
    static const char use_space_reference[] =
        "<svg width=\"16\" height=\"16\"><defs><linearGradient id=\"g\" gradientUnits=\"userSpaceOnUse\" x2=\"6\"><stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/></linearGradient><clipPath id=\"c\" clipPathUnits=\"userSpaceOnUse\"><rect width=\"3\" height=\"4\"/></clipPath><g id=\"target\"><rect width=\"6\" height=\"4\" fill=\"url(#g)\" clip-path=\"url(#c)\"/></g></defs><use href=\"#target\" x=\"8\" y=\"8\"/></svg>";
    if (!render_source(use_space_reference, &tile, &doc, &diag)) return false;
    size_t use_left = ((size_t)9U * 16U + 8U) * 4U;
    ok = pixel_alpha(&tile, 9U, 9U) == 255U && pixel_alpha(&tile, 12U, 9U) == 0U &&
         tile.rgba[use_left] > tile.rgba[use_left + 2U];
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!ok) return false;
    static const char group_bbox_clip[] =
        "<svg width=\"20\" height=\"10\"><defs><clipPath id=\"c\" clipPathUnits=\"objectBoundingBox\"><rect width=\"0.5\" height=\"1\"/></clipPath></defs><g clip-path=\"url(#c)\"><rect width=\"10\" height=\"10\" fill=\"red\"/><rect x=\"10\" width=\"10\" height=\"10\" fill=\"blue\"/></g></svg>";
    if (!render_source(group_bbox_clip, &tile, &doc, &diag)) return false;
    ok = pixel_alpha(&tile, 5U, 5U) == 255U &&
         pixel_alpha(&tile, 15U, 5U) == 0U;
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!ok) return false;
    static const char nested_group_clips[] =
        "<svg width=\"20\" height=\"10\"><defs><clipPath id=\"outer\" clipPathUnits=\"userSpaceOnUse\"><rect width=\"10\" height=\"10\"/></clipPath><clipPath id=\"inner\" clipPathUnits=\"userSpaceOnUse\"><rect x=\"5\" width=\"10\" height=\"10\"/></clipPath></defs><g clip-path=\"url(#outer)\"><g clip-path=\"url(#inner)\"><rect width=\"20\" height=\"10\" fill=\"red\"/></g></g></svg>";
    if (!render_source(nested_group_clips, &tile, &doc, &diag)) return false;
    ok = pixel_alpha(&tile, 2U, 5U) == 0U &&
         pixel_alpha(&tile, 7U, 5U) == 255U &&
         pixel_alpha(&tile, 12U, 5U) == 0U;
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!ok) return false;
    static const char singular_clip[] =
        "<svg width=\"8\" height=\"8\"><defs><clipPath id=\"c\"><rect width=\"1\" height=\"1\"/></clipPath></defs><g transform=\"scale(0)\" clip-path=\"url(#c)\"><rect width=\"4\" height=\"4\" fill=\"red\"/></g></svg>";
    if (svg_shape_decode((base_bytes){(const uint8_t *)singular_clip,
                                      strlen(singular_clip)},
                         &doc, &diag) != SVG_SHAPE_ERROR_UNSUPPORTED ||
        diag.category != SVG_SHAPE_ERROR_UNSUPPORTED) return false;
    static const char *invalid[] = {
        "<svg width=\"8\" height=\"8\"><rect width=\"8\" height=\"8\" clip-path=\"url(https://example.invalid/c)\"/></svg>",
        "<svg width=\"8\" height=\"8\"><defs><clipPath id=\"c\" clipPathUnits=\"object-bounding-box\"><rect width=\"1\" height=\"1\"/></clipPath></defs><rect width=\"8\" height=\"8\" clip-path=\"url(#c)\"/></svg>",
        "<svg width=\"8\" height=\"8\"><defs><clipPath id=\"c\"><rect width=\"1\" height=\"1\" clip-path=\"url(#c)\"/></clipPath></defs><rect width=\"8\" height=\"8\" clip-path=\"url(#c)\"/></svg>",
        "<svg width=\"8\" height=\"8\"><defs><clipPath id=\"c\"><rect width=\"1\" height=\"1\" style=\"fill:red\"/></clipPath></defs><rect width=\"8\" height=\"8\" clip-path=\"url(#c)\"/></svg>",
        "<svg width=\"8\" height=\"8\"><style>.k{clip-path:url(#d)}</style><defs><clipPath id=\"d\"><rect width=\"1\" height=\"1\"/></clipPath><clipPath id=\"c\"><rect class=\"k\" width=\"1\" height=\"1\"/></clipPath></defs><rect width=\"8\" height=\"8\" clip-path=\"url(#c)\"/></svg>"
    };
    for (size_t i = 0U; i < sizeof(invalid) / sizeof(invalid[0]); ++i)
        if (svg_shape_decode((base_bytes){(const uint8_t *)invalid[i], strlen(invalid[i])},
                             &doc, &diag) != SVG_SHAPE_ERROR_UNSUPPORTED) return false;
    static const char css_nested_clip[] =
        "<svg width=\"8\" height=\"8\"><style>.k{clip-path:url(#d)}</style>"
        "<defs><clipPath id=\"d\"><rect width=\"1\" height=\"1\"/></clipPath>"
        "<clipPath id=\"c\"><rect class=\"k\" width=\"1\" height=\"1\"/></clipPath>"
        "</defs><rect width=\"8\" height=\"8\" clip-path=\"url(#c)\"/></svg>";
    const char *nested_clip_value = strstr(css_nested_clip, "url(#d)");
    if (nested_clip_value == NULL ||
        svg_shape_decode((base_bytes){(const uint8_t *)css_nested_clip,
                                      strlen(css_nested_clip)}, &doc, &diag) !=
            SVG_SHAPE_ERROR_UNSUPPORTED ||
        diag.category != SVG_SHAPE_ERROR_UNSUPPORTED ||
        diag.offset != (size_t)(nested_clip_value - css_nested_clip)) return false;
    char too_many[32768];
    static const char clip_prefix[] = "<svg width=\"8\" height=\"8\"><defs><clipPath id=\"c\">";
    static const char clip_child[] = "<rect width=\"1\" height=\"1\"/>";
    static const char clip_suffix[] = "</clipPath></defs><rect width=\"8\" height=\"8\" clip-path=\"url(#c)\"/></svg>";
    size_t length = sizeof(clip_prefix) - 1U;
    memcpy(too_many, clip_prefix, length);
    for (size_t i = 0U; i < 65U; ++i) {
        memcpy(too_many + length, clip_child, sizeof(clip_child) - 1U);
        length += sizeof(clip_child) - 1U;
    }
    memcpy(too_many + length, clip_suffix, sizeof(clip_suffix) - 1U);
    length += sizeof(clip_suffix) - 1U;
    if (svg_shape_decode((base_bytes){(const uint8_t *)too_many, length}, &doc, &diag) !=
        SVG_SHAPE_ERROR_LIMIT || diag.category != SVG_SHAPE_ERROR_LIMIT) return false;
    size_t large_capacity = 70U * 1024U;
    char *large = malloc(large_capacity);
    if (large == NULL) return false;
    static const char large_prefix[] = "<svg width=\"8\" height=\"8\"><defs><clipPath id=\"c\"><!--";
    static const char large_suffix[] = "--><rect width=\"8\" height=\"8\"/></clipPath></defs><rect width=\"8\" height=\"8\" clip-path=\"url(#c)\"/></svg>";
    size_t large_len = sizeof(large_prefix) - 1U;
    memcpy(large, large_prefix, large_len);
    memset(large + large_len, 'c', 65U * 1024U);
    large_len += 65U * 1024U;
    memcpy(large + large_len, large_suffix, sizeof(large_suffix) - 1U);
    large_len += sizeof(large_suffix) - 1U;
    bool limited = svg_shape_decode((base_bytes){(const uint8_t *)large, large_len},
                                    &doc, &diag) == SVG_SHAPE_ERROR_LIMIT &&
                   diag.category == SVG_SHAPE_ERROR_LIMIT;
    free(large);
    return limited;
}

static bool clip_work_budget_check(void)
{
    size_t capacity = 256U * 1024U;
    char *source = malloc(capacity);
    if (source == NULL) return false;
    static const char prefix[] = "<svg width=\"1024\" height=\"1024\"><defs><clipPath id=\"c\" clipPathUnits=\"userSpaceOnUse\"><path d=\"M0 0";
    static const char command[] = " L1 0";
    static const char suffix[] = " L1 1 L0 1 Z\"/></clipPath></defs><rect width=\"1024\" height=\"1024\" clip-path=\"url(#c)\"/></svg>";
    size_t length = sizeof(prefix) - 1U;
    memcpy(source, prefix, length);
    for (size_t i = 0U; i < 1000U; ++i) {
        memcpy(source + length, command, sizeof(command) - 1U);
        length += sizeof(command) - 1U;
    }
    memcpy(source + length, suffix, sizeof(suffix) - 1U);
    length += sizeof(suffix) - 1U;
    svg_shape_document doc = {0};
    svg_shape_diagnostic diag = {0};
    bool ok = svg_shape_decode((base_bytes){(const uint8_t *)source, length},
                               &doc, &diag) == SVG_SHAPE_ERROR_NONE;
    raster_tile tile = {0};
    if (ok) ok = !raster_shape_render(&doc, &tile, &diag) &&
                 diag.category == SVG_SHAPE_ERROR_LIMIT && tile.rgba == NULL;
    raster_tile_destroy(&tile);
    svg_shape_document_destroy(&doc);
    free(source);
    return ok;
}

static bool stroke_decode_and_render_checks(void)
{
    static const char defaults[] = "<svg width=\"8\" height=\"8\"><line x1=\"1\" y1=\"1\" x2=\"7\" y2=\"1\"/></svg>";
    svg_shape_document default_doc={0}; svg_shape_diagnostic diag;
    if (svg_shape_decode((base_bytes){(const uint8_t *)defaults,sizeof(defaults)-1U},&default_doc,&diag)!=SVG_SHAPE_ERROR_NONE) return false;
    bool default_ok=default_doc.strokes[0].rgba[3]==0U && default_doc.strokes[0].width==1.0 &&
        default_doc.strokes[0].cap==SVG_SHAPE_CAP_BUTT && default_doc.strokes[0].join==SVG_SHAPE_JOIN_MITER &&
        default_doc.strokes[0].miter_limit==4.0 && default_doc.strokes[0].dash_count==0U;
    svg_shape_document_destroy(&default_doc);
    if (!default_ok) return false;
    static const char source[] = "<svg width=\"16\" height=\"16\"><line x1=\"3\" y1=\"8\" x2=\"13\" y2=\"8\" stroke=\"red\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-dasharray=\"3 2 1\" stroke-dashoffset=\"-1\"/></svg>";
    svg_shape_document doc = {0}; raster_tile tile = {0};
    if (svg_shape_decode((base_bytes){(const uint8_t *)source, sizeof(source)-1U}, &doc, &diag) != SVG_SHAPE_ERROR_NONE) return false;
    bool parsed = doc.source.ptr == (const uint8_t *)source && doc.source.len == sizeof(source)-1U &&
        doc.shape_count == 1U && doc.shape_edge_count[0] == 1U &&
        doc.strokes[0].rgba[0] == 255U && doc.strokes[0].rgba[3] == 255U &&
        doc.strokes[0].width == 2.0 && doc.strokes[0].dash_count == 3U &&
        doc.strokes[0].cap == SVG_SHAPE_CAP_ROUND && doc.strokes[0].dash_offset == -1.0;
    bool rendered = parsed && raster_shape_render(&doc, &tile, &diag);
    bool visible = rendered && pixel_alpha(&tile, 3U, 8U) > 0U && pixel_alpha(&tile, 8U, 8U) > 0U;
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!visible) return false;
    static const char transformed[]="<svg width=\"16\" height=\"16\"><g transform=\"scale(2)\"><line x1=\"1\" y1=\"1\" x2=\"6\" y2=\"1\" stroke=\"blue\" stroke-width=\"1\"/></g></svg>";
    if (svg_shape_decode((base_bytes){(const uint8_t *)transformed,sizeof(transformed)-1U},&doc,&diag)!=SVG_SHAPE_ERROR_NONE ||
        doc.strokes[0].width!=1.0 || doc.shape_stroke_closed[0]) { svg_shape_document_destroy(&doc); return false; }
    svg_shape_document_destroy(&doc);
    static const char anisotropic[]="<svg width=\"16\" height=\"16\"><g transform=\"scale(2 1)\"><line x1=\"2\" y1=\"2\" x2=\"6\" y2=\"2\" stroke=\"blue\" stroke-width=\"2\"/></g></svg>";
    if (!render_source(anisotropic,&tile,&doc,&diag)) return false;
    bool width_follows_normal = pixel_alpha(&tile,8U,2U)>0U && pixel_alpha(&tile,8U,3U)==0U;
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!width_follows_normal) return false;
    static const char joins[]="<svg width=\"16\" height=\"16\"><polyline points=\"2 2 8 8 14 2\" stroke=\"red\" stroke-linejoin=\"round\" stroke-miterlimit=\"1\"/></svg>";
    if (svg_shape_decode((base_bytes){(const uint8_t *)joins,sizeof(joins)-1U},&doc,&diag)!=SVG_SHAPE_ERROR_NONE ||
        doc.strokes[0].join!=SVG_SHAPE_JOIN_ROUND || doc.strokes[0].miter_limit!=1.0 || doc.shape_stroke_closed[0]) {
        svg_shape_document_destroy(&doc); return false;
    }
    svg_shape_document_destroy(&doc);
    static const char painter[]="<svg width=\"8\" height=\"8\"><rect x=\"1\" y=\"1\" width=\"6\" height=\"6\" fill=\"blue\" stroke=\"red\" stroke-width=\"2\"/></svg>";
    if (!render_source(painter,&tile,&doc,&diag)) return false;
    size_t edge=((size_t)1U*8U+3U)*4U, interior=((size_t)3U*8U+3U)*4U;
    bool order_ok=tile.rgba[edge]==255U && tile.rgba[edge+2U]==0U &&
        tile.rgba[interior]==0U && tile.rgba[interior+2U]==255U;
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!order_ok) return false;
    static const char limit_work[]="<svg width=\"4096\" height=\"4096\"><line x1=\"1\" y1=\"1\" x2=\"2\" y2=\"1\" stroke=\"red\" stroke-dasharray=\"0.0000001 0.0000001\"/></svg>";
    if (svg_shape_decode((base_bytes){(const uint8_t *)limit_work,sizeof(limit_work)-1U},&doc,&diag)!=SVG_SHAPE_ERROR_NONE) return false;
    if (raster_shape_render(&doc,&tile,&diag) || diag.category!=SVG_SHAPE_ERROR_LIMIT || tile.rgba!=NULL) {
        svg_shape_document_destroy(&doc); raster_tile_destroy(&tile); return false;
    }
    svg_shape_document_destroy(&doc);
    static const char invalid[] = "<svg width=\"8\" height=\"8\"><line stroke=\"red\" stroke-dasharray=\"1,,2\"/></svg>";
    size_t expected = (size_t)(strstr(invalid, "1,,2") - invalid);
    if (svg_shape_decode((base_bytes){(const uint8_t *)invalid,sizeof(invalid)-1U},&doc,&diag)==SVG_SHAPE_ERROR_NONE ||
        diag.category != SVG_SHAPE_ERROR_NUMBER || diag.offset != expected) return false;
    static const char unsupported[] = "<svg width=\"8\" height=\"8\"><path stroke=\"blue\" d=\"M1 1 L7 7\"/></svg>";
    expected = (size_t)(strstr(unsupported, "blue") - unsupported);
    if (svg_shape_decode((base_bytes){(const uint8_t *)unsupported,sizeof(unsupported)-1U},&doc,&diag)==SVG_SHAPE_ERROR_NONE ||
        diag.category != SVG_SHAPE_ERROR_UNSUPPORTED || diag.offset != expected) return false;
    return true;
}

static bool stroke_cap_join_and_dash_checks(void)
{
    static const char butt[] = "<svg width=\"16\" height=\"16\"><line x1=\"4\" y1=\"8\" x2=\"12\" y2=\"8\" stroke=\"red\" stroke-width=\"2\" stroke-linecap=\"butt\"/></svg>";
    static const char square[] = "<svg width=\"16\" height=\"16\"><line x1=\"4\" y1=\"8\" x2=\"12\" y2=\"8\" stroke=\"red\" stroke-width=\"2\" stroke-linecap=\"square\"/></svg>";
    static const char round[] = "<svg width=\"16\" height=\"16\"><line x1=\"4\" y1=\"8\" x2=\"12\" y2=\"8\" stroke=\"red\" stroke-width=\"2\" stroke-linecap=\"round\"/></svg>";
    uint8_t butt_alpha, square_alpha, round_alpha;
    if (!alpha_at(butt, 3U, 7U, &butt_alpha) || !alpha_at(square, 3U, 7U, &square_alpha) ||
        !alpha_at(round, 3U, 7U, &round_alpha) || butt_alpha != 0U ||
        square_alpha == 0U || round_alpha == 0U) return false;

    static const char miter[] = "<svg width=\"16\" height=\"16\"><polyline points=\"4,10 8,6 12,10\" fill=\"none\" stroke=\"red\" stroke-width=\"4\" stroke-linejoin=\"miter\"/></svg>";
    static const char bevel[] = "<svg width=\"16\" height=\"16\"><polyline points=\"4,10 8,6 12,10\" fill=\"none\" stroke=\"red\" stroke-width=\"4\" stroke-linejoin=\"bevel\"/></svg>";
    static const char fallback[] = "<svg width=\"16\" height=\"16\"><polyline points=\"4,10 8,6 12,10\" fill=\"none\" stroke=\"red\" stroke-width=\"4\" stroke-linejoin=\"miter\" stroke-miterlimit=\"1\"/></svg>";
    uint8_t miter_tip, bevel_tip, fallback_tip, bevel_join;
    if (!alpha_at(miter, 7U, 3U, &miter_tip) || !alpha_at(bevel, 7U, 3U, &bevel_tip) ||
        !alpha_at(fallback, 7U, 3U, &fallback_tip) || !alpha_at(bevel, 8U, 5U, &bevel_join) ||
        miter_tip == 0U || bevel_tip != 0U || fallback_tip != 0U || bevel_join == 0U) return false;

    static const char odd_dash[] = "<svg width=\"16\" height=\"16\"><line x1=\"1\" y1=\"8\" x2=\"15\" y2=\"8\" stroke=\"red\" stroke-width=\"2\" stroke-dasharray=\"3 2 1\"/></svg>";
    static const char phase_dash[] = "<svg width=\"16\" height=\"16\"><line x1=\"1\" y1=\"8\" x2=\"15\" y2=\"8\" stroke=\"red\" stroke-width=\"2\" stroke-dasharray=\"3 2 1\" stroke-dashoffset=\"1\"/></svg>";
    uint8_t odd_on_first, odd_off_first, odd_on_second, odd_off_second;
    uint8_t shifted_first_boundary, shifted_on_second;
    if (!alpha_at(odd_dash, 2U, 8U, &odd_on_first) || !alpha_at(odd_dash, 4U, 8U, &odd_off_first) ||
        !alpha_at(odd_dash, 6U, 8U, &odd_on_second) || !alpha_at(odd_dash, 8U, 8U, &odd_off_second) ||
        !alpha_at(odd_dash, 3U, 8U, &shifted_first_boundary) ||
        !alpha_at(phase_dash, 3U, 8U, &shifted_on_second) || odd_on_first == 0U ||
        odd_off_first != 0U || odd_on_second == 0U || odd_off_second != 0U ||
        shifted_first_boundary == 0U || shifted_on_second != 0U) return false;

    static const char dashed_join[] = "<svg width=\"16\" height=\"16\"><polyline points=\"4,10 8,6 12,10\" fill=\"none\" stroke=\"red\" stroke-width=\"4\" stroke-linejoin=\"miter\" stroke-dasharray=\"9 20\"/></svg>";
    uint8_t dashed_miter_tip;
    if (!alpha_at(dashed_join, 7U, 3U, &dashed_miter_tip) || dashed_miter_tip == 0U)
        return false;
    static const char closed_seam[] = "<svg width=\"16\" height=\"16\"><polygon points=\"8,6 12,10 4,10\" fill=\"none\" stroke=\"black\" stroke-width=\"4\" stroke-linejoin=\"miter\" stroke-dasharray=\"15 2\"/></svg>";
    uint8_t seam_miter_tip;
    return alpha_at(closed_seam, 7U, 3U, &seam_miter_tip) && seam_miter_tip > 0U;
}

int main(void)
{
    if (!svg_shape_reference_tests()) { fprintf(stderr, "reference checks failed\n"); return 49; }
    if (!gradient_checks()) { fprintf(stderr, "gradient rendering failed\n"); return 50; }
    if (!clip_checks()) { fprintf(stderr, "clip rendering failed\n"); return 51; }
    if (!clip_work_budget_check()) { fprintf(stderr, "clip work budget failed\n"); return 52; }
    if (!clip_boundary_accounting_check()) {
        fprintf(stderr, "clip boundary accounting failed\n"); return 54;
    }
    if (!paint_edge_span_check()) {
        fprintf(stderr, "paint edge span safety failed\n"); return 55;
    }
    static const uint8_t zero_area_gradient[] =
        "<svg width=\"8\" height=\"8\"><defs><linearGradient id=\"g\">"
        "<stop stop-color=\"red\"/><stop offset=\"1\" stop-color=\"blue\"/>"
        "</linearGradient></defs><rect width=\"0\" height=\"8\" fill=\"url(#g)\"/>"
        "<rect width=\"4\" height=\"4\" fill=\"red\"/></svg>";
    svg_shape_document zero_doc = {0};
    svg_shape_diagnostic zero_diag = {0};
    const char *zero_fill = strstr((const char *)zero_area_gradient, "url(#g)");
    if (zero_fill == NULL ||
        svg_shape_decode((base_bytes){zero_area_gradient,
            sizeof(zero_area_gradient) - 1U}, &zero_doc, &zero_diag) !=
                SVG_SHAPE_ERROR_UNSUPPORTED ||
        zero_diag.offset != (size_t)(zero_fill - (const char *)zero_area_gradient) ||
        zero_doc.edge_count != 0U || zero_doc.edges != NULL) {
        svg_shape_document_destroy(&zero_doc);
        return 53;
    }
    svg_shape_document_destroy(&zero_doc);
    static const uint8_t definitions[] = "<svg width=\"8\" height=\"8\"><defs><rect width=\"2\" height=\"2\"/><symbol id=\"unused\"><circle r=\"1\"/></symbol></defs><rect x=\"3\" width=\"2\" height=\"2\"/></svg>";
    svg_shape_document definition_doc = {0};
    svg_shape_diagnostic definition_diag;
    if (svg_shape_decode((base_bytes){definitions, sizeof(definitions)-1U},
                         &definition_doc, &definition_diag) != SVG_SHAPE_ERROR_NONE ||
        definition_doc.shape_count != 1U || definition_doc.edge_count != 4U ||
        definition_doc.source.ptr != definitions ||
        definition_doc.source.len != sizeof(definitions)-1U) return 48;
    svg_shape_document_destroy(&definition_doc);
    if (!antialias_and_fill_checks()) { fprintf(stderr, "antialias/fill checks failed\n"); return 45; }
    if (!stroke_decode_and_render_checks()) { fprintf(stderr, "stroke checks failed\n"); return 46; }
    if (!stroke_cap_join_and_dash_checks()) { fprintf(stderr, "stroke cap/join/dash checks failed\n"); return 47; }
    const bool smooth_checks[] = {
        paths_equivalent("M1 1 C2 1 3 2 4 2 S6 3 7 4", "M1 1 C2 1 3 2 4 2 C5 2 6 3 7 4"),
        paths_equivalent("M1 1 c1 0 2 1 3 1 s2 1 3 2 2 1 3 2", "M1 1 c1 0 2 1 3 1 c1 0 2 1 3 2 c1 1 2 1 3 2"),
        paths_equivalent("M1 1 Q2 3 4 1 T7 1", "M1 1 Q2 3 4 1 Q6 -1 7 1"),
        paths_equivalent("M1 1 q1 2 3 0 t3 0 3 0", "M1 1 q1 2 3 0 q2 -2 3 0 q1 2 3 0"),
        paths_equivalent("M1 1 C2 2 3 3 4 4 L7 7 S8 8 9 9", "M1 1 C2 2 3 3 4 4 L7 7 C7 7 8 8 9 9"),
        paths_equivalent("M1 1 Q2 3 4 1 L7 1 T9 1", "M1 1 Q2 3 4 1 L7 1 Q7 1 9 1"),
        paths_equivalent("M1 1 C2 2 3 3 4 4 Z S6 6 7 7", "M1 1 C2 2 3 3 4 4 Z C1 1 6 6 7 7")
    };
    for (size_t i = 0; i < sizeof(smooth_checks) / sizeof(smooth_checks[0]); i++) if (!smooth_checks[i]) { fprintf(stderr, "smooth equivalence %zu failed\n", i); return 40; }

    geom_affine translate={1,0,0,1,10,20}, scale={2,0,0,3,0,0}, composed;
    geom_point mapped;
    double bounds[4];
    if (!geom_affine_compose(translate,scale,&composed) ||
        !geom_affine_map_point(composed,(geom_point){1,2},&mapped) ||
        fabs(mapped.x-12.0)>1e-12 || fabs(mapped.y-26.0)>1e-12 ||
        !geom_affine_map_bounds(composed,0,0,2,1,bounds) ||
        fabs(bounds[0]-10.0)>1e-12 || fabs(bounds[1]-20.0)>1e-12 ||
        fabs(bounds[2]-4.0)>1e-12 || fabs(bounds[3]-3.0)>1e-12) return 20;
    geom_affine singular={0,0,0,0,1,1};
    if (!geom_affine_map_point(singular,(geom_point){3,4},&mapped) ||
        mapped.x!=1.0 || mapped.y!=1.0) return 21;
    geom_affine invalid={NAN,0,0,1,0,0};
    if (geom_affine_compose(invalid,translate,&composed)) return 22;
    for (unsigned i=1U;i<=64U;i++) {
        double k=(double)i;
        geom_affine left={1.0+k/100.0,0.25,-0.125,0.75+k/200.0,k,-k/2.0};
        geom_affine right={0.5+k/300.0,-0.2,0.4,1.0+k/400.0,-k/3.0,k/4.0};
        geom_point p={k/5.0,-k/7.0}, inner, expected, actual;
        if (!geom_affine_map_point(right,p,&inner)) return 28;
        expected=(geom_point){left.a*inner.x+left.c*inner.y+left.e,
                              left.b*inner.x+left.d*inner.y+left.f};
        if (!geom_affine_compose(left,right,&composed) ||
            !geom_affine_map_point(composed,p,&actual) ||
            fabs(actual.x-expected.x)>1e-10 || fabs(actual.y-expected.y)>1e-10) return 29;
    }
    geom_affine quarter_turn={0,1,-1,0,0,0};
    if (!geom_affine_map_bounds(quarter_turn,0,0,2,1,bounds) ||
        fabs(bounds[0]+1.0)>1e-12 || fabs(bounds[1])>1e-12 ||
        fabs(bounds[2]-1.0)>1e-12 || fabs(bounds[3]-2.0)>1e-12) return 30;

    static const uint8_t source[] = "<svg width=\"8\" height=\"8\"><rect x=\"1\" y=\"1\" width=\"6\" height=\"6\" fill=\"#f00\"/></svg>";
    svg_shape_document doc; svg_shape_diagnostic diagnostic;
    if (svg_shape_decode((base_bytes){source, sizeof(source)-1U}, &doc, &diagnostic) != SVG_SHAPE_ERROR_NONE) return 1;
    raster_tile tile;
    if (!raster_shape_render(&doc, &tile, &diagnostic)) return 2;
    size_t inside = ((size_t)3U * 8U + 3U) * 4U;
    size_t outside = ((size_t)0U * 8U + 0U) * 4U;
    bool valid = tile.rgba[inside] == 255U && tile.rgba[inside+1U] == 0U && tile.rgba[inside+3U] == 255U && tile.rgba[outside+3U] == 0U && doc.source.ptr == source && doc.source.len == sizeof(source)-1U;
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    static const uint8_t bad[] = "<svg width=\"8\" height=\"8\"><path d=\"M0 0 A3 3 0 2 0 4 4\"/></svg>";
    if (svg_shape_decode((base_bytes){bad, sizeof(bad)-1U}, &doc, &diagnostic) == SVG_SHAPE_ERROR_NONE) { svg_shape_document_destroy(&doc); return 3; }
    if (diagnostic.category != SVG_SHAPE_ERROR_PATH || diagnostic.offset == 0U || !valid) return 4;
    static const uint8_t malformed_smooth[] = "<svg width=\"8\" height=\"8\"><path d=\"M0 0 S1 2 3\"/></svg>";
    size_t malformed_smooth_group = (size_t)(strstr((const char *)malformed_smooth, "S1") - (const char *)malformed_smooth) + strlen("S1 2 3");
    if (svg_shape_decode((base_bytes){malformed_smooth, sizeof(malformed_smooth)-1U}, &doc, &diagnostic) == SVG_SHAPE_ERROR_NONE ||
        diagnostic.category != SVG_SHAPE_ERROR_PATH || diagnostic.offset != malformed_smooth_group) { fprintf(stderr, "malformed smooth got %d@%zu expected path@%zu\n", diagnostic.category, diagnostic.offset, malformed_smooth_group); return 41; }
    static const uint8_t malformed_quad_smooth[] = "<svg width=\"8\" height=\"8\"><path d=\"M0 0 T1\"/></svg>";
    size_t malformed_quad_group = (size_t)(strstr((const char *)malformed_quad_smooth, "T1") - (const char *)malformed_quad_smooth) + strlen("T1");
    if (svg_shape_decode((base_bytes){malformed_quad_smooth, sizeof(malformed_quad_smooth)-1U}, &doc, &diagnostic) == SVG_SHAPE_ERROR_NONE ||
        diagnostic.category != SVG_SHAPE_ERROR_PATH || diagnostic.offset != malformed_quad_group) return 44;
    static const uint8_t smooth_overflow[] = "<svg width=\"8\" height=\"8\"><path d=\"M999999 0 s2 2 2 2\"/></svg>";
    size_t smooth_overflow_group = (size_t)(strstr((const char *)smooth_overflow, "s2") - (const char *)smooth_overflow) + 1U;
    if (svg_shape_decode((base_bytes){smooth_overflow, sizeof(smooth_overflow)-1U}, &doc, &diagnostic) == SVG_SHAPE_ERROR_NONE ||
        diagnostic.category != SVG_SHAPE_ERROR_NUMBER || diagnostic.offset != smooth_overflow_group) { fprintf(stderr, "smooth overflow got %d@%zu expected number@%zu\n", diagnostic.category, diagnostic.offset, smooth_overflow_group); return 42; }
    static const uint8_t quad_overflow[] = "<svg width=\"8\" height=\"8\"><path d=\"M999999 0 t2 2\"/></svg>";
    size_t quad_overflow_group = (size_t)(strstr((const char *)quad_overflow, "t2") - (const char *)quad_overflow) + 1U;
    if (svg_shape_decode((base_bytes){quad_overflow, sizeof(quad_overflow)-1U}, &doc, &diagnostic) == SVG_SHAPE_ERROR_NONE ||
        diagnostic.category != SVG_SHAPE_ERROR_NUMBER || diagnostic.offset != quad_overflow_group) return 43;
    static const uint8_t overshoot[] = "<svg width=\"8\" height=\"8\"><path d=\"M0 0 C100 0 100 0 1 0\"/></svg>";
    if (svg_shape_decode((base_bytes){overshoot, sizeof(overshoot)-1U}, &doc, &diagnostic) != SVG_SHAPE_ERROR_NONE) return 8;
    bool retained_overshoot = false;
    for (size_t i = 0; i < doc.edge_count; i++) {
        if (doc.edges[i].a.x > 1.0 || doc.edges[i].b.x > 1.0) retained_overshoot = true;
    }
    svg_shape_document_destroy(&doc);
    if (!retained_overshoot) return 9;
    size_t source_bytes = 330000U + 256U;
    uint8_t *many_paths = malloc(source_bytes);
    if (many_paths == NULL) return 10;
    static const char path_start[] = "<svg width=\"8\" height=\"8\"><path d=\"M0 0";
    static const char path_middle[] = "\"/><path d=\"M0 0";
    static const char path_end[] = "\"/></svg>";
    size_t offset = sizeof(path_start) - 1U;
    memcpy(many_paths, path_start, offset);
    for (size_t path = 0; path < 2U; path++) {
        for (size_t i = 0; i < 33000U; i++) {
            memcpy(many_paths + offset, " L1 1", 5U);
            offset += 5U;
        }
        if (path == 0U) {
            memcpy(many_paths + offset, path_middle, sizeof(path_middle) - 1U);
            offset += sizeof(path_middle) - 1U;
        }
    }
    memcpy(many_paths + offset, path_end, sizeof(path_end) - 1U);
    offset += sizeof(path_end) - 1U;
    if (svg_shape_decode((base_bytes){many_paths, offset}, &doc, &diagnostic) == SVG_SHAPE_ERROR_NONE) {
        svg_shape_document_destroy(&doc); free(many_paths); return 11;
    }
    free(many_paths);
    if (diagnostic.category != SVG_SHAPE_ERROR_LIMIT) return 12;
    static const uint8_t blank[] = "<svg width=\"8\" height=\"8\"/>";
    if (svg_shape_decode((base_bytes){blank, sizeof(blank)-1U}, &doc, &diagnostic) != SVG_SHAPE_ERROR_NONE) return 5;
    if (!raster_shape_render(&doc, &tile, &diagnostic)) { svg_shape_document_destroy(&doc); return 6; }
    bool transparent = true;
    for (size_t i = 3U; i < tile.byte_count; i += 4U) transparent = transparent && tile.rgba[i] == 0U;
    raster_tile_destroy(&tile); svg_shape_document_destroy(&doc);
    if (!transparent) return 7;
    static const uint8_t transformed[] = "<svg width=\"16\" height=\"16\"><g transform=\"translate(4 5 )\"><g transform=\"scale(2)\"><rect width=\"1\" height=\"1\"/></g></g><circle cx=\"10\" cy=\"10\" r=\"2\"/><ellipse cx=\"10\" cy=\"10\" rx=\"2\" ry=\"1\"/><polygon points=\"1,1 3,1 3,3\"/><polyline points=\"5 5 7 5 7 7\"/><line x1=\"0\" y1=\"0\" x2=\"15\" y2=\"15\" fill=\"red\"/></svg>";
    size_t transformed_len=sizeof(transformed)-1U;
    if (svg_shape_decode((base_bytes){transformed,transformed_len},&doc,&diagnostic)!=SVG_SHAPE_ERROR_NONE) return 23;
    bool composed_edge=false;
    for(size_t i=0U;i<doc.edge_count;i++)if(fabs(doc.edges[i].a.x-4.0)<1e-12&&fabs(doc.edges[i].a.y-5.0)<1e-12&&fabs(doc.edges[i].b.x-6.0)<1e-12)composed_edge=true;
    if(!composed_edge||doc.source.ptr!=transformed||doc.source.len!=transformed_len||doc.shape_count!=6U||doc.shape_filled[5])return 24;
    if(!raster_shape_render(&doc,&tile,&diagnostic))return 25;
    raster_tile_destroy(&tile);svg_shape_document_destroy(&doc);
    static const uint8_t diagonal_line[]="<svg width=\"16\" height=\"16\"><line x1=\"0\" y1=\"0\" x2=\"15\" y2=\"15\" fill=\"red\"/></svg>";
    if(svg_shape_decode((base_bytes){diagonal_line,sizeof(diagonal_line)-1U},&doc,&diagnostic)!=SVG_SHAPE_ERROR_NONE||doc.edge_count!=1U||doc.shape_filled[0])return 26;
    if(!raster_shape_render(&doc,&tile,&diagnostic))return 32;
    for(size_t pixel=0U;pixel<(size_t)16U*16U;pixel++)if(tile.rgba[pixel*4U+3U]!=0U)return 33;
    raster_tile_destroy(&tile);svg_shape_document_destroy(&doc);
    static const uint8_t extreme_ellipse[]="<svg width=\"8\" height=\"8\"><circle cx=\"1000000\" cy=\"0\" r=\"1\"/></svg>";
    if(svg_shape_decode((base_bytes){extreme_ellipse,sizeof(extreme_ellipse)-1U},&doc,&diagnostic)==SVG_SHAPE_ERROR_NONE||diagnostic.category!=SVG_SHAPE_ERROR_NUMBER||diagnostic.offset==0U)return 31;
    static const uint8_t eccentric_ellipse[]="<svg width=\"8\" height=\"8\"><ellipse cx=\"100000\" cy=\"100000\" rx=\"100000\" ry=\"1000\"/></svg>";
    if(svg_shape_decode((base_bytes){eccentric_ellipse,sizeof(eccentric_ellipse)-1U},&doc,&diagnostic)!=SVG_SHAPE_ERROR_NONE||doc.edge_count>4096U)return 38;
    double max_ellipse_error=0.0;
    for(size_t edge=0U;edge<doc.edge_count;edge++){
        for(size_t sample=1U;sample<8U;sample++){
            double angle=6.283185307179586*((double)edge+(double)sample/8.0)/(double)doc.edge_count;
            geom_point point={100000.0+100000.0*cos(angle),100000.0+1000.0*sin(angle)};
            double error=segment_distance(point,doc.edges[edge]);
            if(error>max_ellipse_error)max_ellipse_error=error;
        }
    }
    if(max_ellipse_error>0.250001)return 39;
    svg_shape_document_destroy(&doc);
    static const uint8_t leading_transform_comma[]="<svg width=\"8\" height=\"8\"><rect width=\"1\" height=\"1\" transform=\",translate(1)\"/></svg>";
    if(svg_shape_decode((base_bytes){leading_transform_comma,sizeof(leading_transform_comma)-1U},&doc,&diagnostic)==SVG_SHAPE_ERROR_NONE||diagnostic.category!=SVG_SHAPE_ERROR_UNSUPPORTED||diagnostic.offset==0U)return 34;
    static const uint8_t repeated_transform_comma[]="<svg width=\"8\" height=\"8\"><rect width=\"1\" height=\"1\" transform=\"translate(1),,scale(2)\"/></svg>";
    if(svg_shape_decode((base_bytes){repeated_transform_comma,sizeof(repeated_transform_comma)-1U},&doc,&diagnostic)==SVG_SHAPE_ERROR_NONE||diagnostic.category!=SVG_SHAPE_ERROR_UNSUPPORTED||diagnostic.offset==0U)return 35;
    static const uint8_t adjacent_transform_numbers[]="<svg width=\"8\" height=\"8\"><rect width=\"1\" height=\"1\" transform=\"translate(1-2)\"/></svg>";
    if(svg_shape_decode((base_bytes){adjacent_transform_numbers,sizeof(adjacent_transform_numbers)-1U},&doc,&diagnostic)==SVG_SHAPE_ERROR_NONE||diagnostic.category!=SVG_SHAPE_ERROR_UNSUPPORTED||diagnostic.offset==0U)return 36;
    static const uint8_t adjacent_point_numbers[]="<svg width=\"8\" height=\"8\"><polygon points=\"1,1-2,2\"/></svg>";
    if(svg_shape_decode((base_bytes){adjacent_point_numbers,sizeof(adjacent_point_numbers)-1U},&doc,&diagnostic)==SVG_SHAPE_ERROR_NONE||diagnostic.category!=SVG_SHAPE_ERROR_PATH||diagnostic.offset==0U)return 37;
    static const uint8_t zero_shape[]="<svg width=\"8\" height=\"8\"><circle r=\"0\"/><ellipse rx=\"0\" ry=\"2\"/></svg>";
    if(svg_shape_decode((base_bytes){zero_shape,sizeof(zero_shape)-1U},&doc,&diagnostic)!=SVG_SHAPE_ERROR_NONE||doc.edge_count!=0U)return 27;
    svg_shape_document_destroy(&doc);
    puts("shape decoder and 4x4 antialiased raster checks passed");
    return 0;
}
