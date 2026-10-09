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

int main(void)
{
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
    puts("shape decoder and scalar raster checks passed");
    return 0;
}
