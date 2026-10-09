#include "shape.h"
#include "shape_render.h"

#include <stdio.h>
#include <math.h>
#include <stdlib.h>
#include <string.h>

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

int main(void)
{
    if (!antialias_and_fill_checks()) { fprintf(stderr, "antialias/fill checks failed\n"); return 45; }
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
