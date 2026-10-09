#include "shape.h"
#include "css.h"
#include "../paint/paint.h"
#include "../clip/clip.h"
#include "shape_internal.h"
#include <math.h>
#include <stdlib.h>
#include <string.h>
#define SHAPE_MAX_COMMANDS 65536U
#define SHAPE_MAX_COORD 1000000.0
typedef svg_shape_number_reader number_reader;
typedef svg_shape_builder builder;
static bool span_eq(base_bytes s, svg_xml_span span, const char *literal)
{
    size_t n = strlen(literal);
    return span.length == n && span.offset <= s.len && n <= s.len - span.offset &&
        memcmp(s.ptr + span.offset, literal, n) == 0;
}
static bool point_bounded(geom_point point)
{
    return geom_finite_point(point) && fabs(point.x) <= SHAPE_MAX_COORD &&
           fabs(point.y) <= SHAPE_MAX_COORD;
}
static bool affine_inverse(geom_affine matrix, geom_affine *inverse)
{
    double determinant = matrix.a * matrix.d - matrix.b * matrix.c;
    if (inverse == NULL || !geom_affine_valid(matrix) || !isfinite(determinant) || fabs(determinant) < 1e-15) return false;
    double reciprocal = 1.0 / determinant;
    geom_affine result = {
        matrix.d * reciprocal, -matrix.b * reciprocal,
        -matrix.c * reciprocal, matrix.a * reciprocal, 0.0, 0.0
    };
    result.e = -(result.a * matrix.e + result.c * matrix.f);
    result.f = -(result.b * matrix.e + result.d * matrix.f);
    if (!geom_affine_valid(result)) return false;
    *inverse = result;
    return true;
}
static double transformed_flatness_tolerance(const builder *b)
{
    double row_norm=fmax(fabs(b->transform.a)+fabs(b->transform.c),
                         fabs(b->transform.b)+fabs(b->transform.d));
    double column_norm=fmax(fabs(b->transform.a)+fabs(b->transform.b),
                            fabs(b->transform.c)+fabs(b->transform.d));
    double scale=sqrt(row_norm*column_norm);
    if (!isfinite(scale) || scale < 1.0) scale=1.0;
    return 0.25/scale;
}
static void fail(svg_shape_diagnostic *d, svg_shape_error e, size_t offset)
{
    if (d != NULL) *d = (svg_shape_diagnostic){e, offset};
}
static void skip_space(number_reader *r)
{
    while (r->i < r->n && (r->p[r->i] == ' ' || r->p[r->i] == '\t' || r->p[r->i] == '\r' || r->p[r->i] == '\n')) r->i++;
}
static void skip_path_separators(number_reader *r)
{
    while (r->i < r->n && (r->p[r->i] == ' ' || r->p[r->i] == '\t' || r->p[r->i] == '\r' || r->p[r->i] == '\n' || r->p[r->i] == ',')) r->i++;
}
static bool path_operand_boundary(number_reader *r, bool previous_number,
                                 bool previous_arc_flag, char command,
                                 unsigned argument)
{
    size_t before = r->i;
    skip_path_separators(r);
    if (!previous_number || r->i != before) return true;
    if (r->i < r->n && (r->p[r->i] == '+' || r->p[r->i] == '-')) return true;
    return command == 'A' &&
           ((argument == 4U) || (argument == 5U && previous_arc_flag));
}
static bool next_number_separator(number_reader *r)
{
    size_t before = r->i;
    skip_space(r);
    bool had_space = r->i != before;
    if (r->i < r->n && r->p[r->i] == ',') {
        r->i++;
        skip_space(r);
        return r->i < r->n && r->p[r->i] != ',';
    }
    if (had_space) return r->i < r->n;
    return false;
}
/* Decimal conversion is byte-based and independent of the process locale. */
static bool read_number(number_reader *r, double *out)
{
    skip_space(r);
    size_t start = r->i;
    bool negative = false;
    if (r->i < r->n && (r->p[r->i] == '+' || r->p[r->i] == '-')) { negative = r->p[r->i] == '-'; r->i++; }
    double value = 0.0;
    size_t digits = 0;
    while (r->i < r->n && r->p[r->i] >= '0' && r->p[r->i] <= '9') {
        value = value * 10.0 + (double)(r->p[r->i++] - '0'); digits++;
        if (!isfinite(value) || value > 1e300) return false;
    }
    if (r->i < r->n && r->p[r->i] == '.') {
        r->i++; double scale = 0.1;
        while (r->i < r->n && r->p[r->i] >= '0' && r->p[r->i] <= '9') {
            value += (double)(r->p[r->i++] - '0') * scale; scale *= 0.1; digits++;
        }
    }
    if (digits == 0) { r->i = start; return false; }
    int exponent = 0; bool exp_negative = false;
    if (r->i < r->n && (r->p[r->i] == 'e' || r->p[r->i] == 'E')) {
        r->i++;
        if (r->i < r->n && (r->p[r->i] == '+' || r->p[r->i] == '-')) { exp_negative = r->p[r->i] == '-'; r->i++; }
        size_t exp_digits = 0;
        while (r->i < r->n && r->p[r->i] >= '0' && r->p[r->i] <= '9') {
            if (exponent < 10000) exponent = exponent * 10 + (int)(r->p[r->i] - '0');
            r->i++; exp_digits++;
        }
        if (exp_digits == 0) return false;
    }
    if (exp_negative) exponent = -exponent;
    value *= pow(10.0, (double)exponent);
    if (negative) value = -value;
    if (!isfinite(value) || fabs(value) > SHAPE_MAX_COORD) return false;
    *out = value;
    return true;
}
void svg_shape_skip_number_space(svg_shape_number_reader *reader)
{
    skip_space(reader);
}
bool svg_shape_next_number_separator(svg_shape_number_reader *reader)
{
    return next_number_separator(reader);
}
bool svg_shape_read_number_value(svg_shape_number_reader *reader, double *out)
{
    return read_number(reader, out);
}
static bool read_arc_flag(number_reader *r, double *out)
{
    skip_space(r);
    if (r->i >= r->n || (r->p[r->i] != '0' && r->p[r->i] != '1')) return false;
    *out = (double)(r->p[r->i++] - '0');
    return true;
}
bool svg_shape_read_number(base_bytes source, svg_xml_span span, double *out)
{
    if (out == NULL || span.offset > source.len ||
        span.length > source.len - span.offset) return false;
    number_reader reader = {source.ptr + span.offset, span.length, 0U,
                            span.offset};
    if (!read_number(&reader, out)) return false;
    skip_space(&reader);
    return reader.i == reader.n;
}

bool svg_shape_read_number_list(base_bytes source, svg_xml_span span,
                                double *values, size_t count)
{
    if (values == NULL || count == 0U || span.offset > source.len ||
        span.length > source.len - span.offset) return false;
    number_reader reader = {source.ptr + span.offset, span.length, 0U,
                            span.offset};
    for (size_t i = 0U; i < count; ++i) {
        if (i != 0U && !next_number_separator(&reader)) return false;
        if (!read_number(&reader, &values[i])) return false;
    }
    skip_space(&reader);
    return reader.i == reader.n;
}

bool svg_shape_read_dimension(base_bytes source, svg_xml_span span, double *out)
{
    if (out == NULL || span.offset > source.len ||
        span.length > source.len - span.offset) return false;
    number_reader r={source.ptr+span.offset,span.length,0,span.offset};
    if(!read_number(&r,out))return false;
    skip_space(&r);
    if(r.i==r.n)return true;
    if(r.n-r.i==2U&&r.p[r.i]=='p'&&r.p[r.i+1U]=='x')return true;
    return false;
}
static bool add_edge(builder *b, geom_point a, geom_point z)
{
    geom_point mapped_a, mapped_z;
    if (!geom_affine_map_point(b->transform, a, &mapped_a) ||
        !geom_affine_map_point(b->transform, z, &mapped_z)) {
        fail(b->diag, SVG_SHAPE_ERROR_NUMBER, b->offset);
        return false;
    }
    a = mapped_a;
    z = mapped_z;
    if (b->doc->edge_count >= SVG_SHAPE_MAX_EDGES) { fail(b->diag, SVG_SHAPE_ERROR_LIMIT, b->offset); return false; }
    if (b->doc->edge_count == b->edge_capacity) {
        size_t next = b->edge_capacity == 0 ? 64U : b->edge_capacity * 2U;
        if (next > SVG_SHAPE_MAX_EDGES) next = SVG_SHAPE_MAX_EDGES;
        if (next > SIZE_MAX / sizeof(*b->doc->edges)) { fail(b->diag, SVG_SHAPE_ERROR_LIMIT, b->offset); return false; }
        geom_edge *edges = realloc(b->doc->edges, next * sizeof(*edges));
        if (edges == NULL) { fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset); return false; }
        b->doc->edges = edges;
        uint8_t *colors = realloc(b->doc->colors, next * 4U);
        if (colors == NULL) { fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset); return false; }
        b->doc->colors = colors;
        if (next > SIZE_MAX / sizeof(*b->doc->shape_ids)) { fail(b->diag, SVG_SHAPE_ERROR_LIMIT, b->offset); return false; }
        uint32_t *shape_ids = realloc(b->doc->shape_ids, next * sizeof(*shape_ids));
        if (shape_ids == NULL) { fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset); return false; }
        b->doc->shape_ids = shape_ids;
        b->edge_capacity = next;
    }
    b->doc->edges[b->doc->edge_count] = (geom_edge){a,z};
    b->doc->colors[b->doc->edge_count * 4U] = 0U;
    b->doc->colors[b->doc->edge_count * 4U + 1U] = 0U;
    b->doc->colors[b->doc->edge_count * 4U + 2U] = 0U;
    b->doc->colors[b->doc->edge_count * 4U + 3U] = 255U;
    b->doc->shape_ids[b->doc->edge_count] = (uint32_t)b->shape_index;
    b->doc->edge_count++;
    return true;
}
static bool begin_shape(builder *b, bool evenodd)
{
    if (b->doc->shape_count >= SVG_SHAPE_MAX_EDGES ||
        b->doc->shape_count > UINT32_MAX) {
        fail(b->diag, SVG_SHAPE_ERROR_LIMIT, b->offset);
        return false;
    }
    if (b->doc->shape_count == b->shape_capacity) {
        size_t next = b->shape_capacity == 0U ? 32U : b->shape_capacity * 2U;
        if (next > SVG_SHAPE_MAX_EDGES) next = SVG_SHAPE_MAX_EDGES;
        if (next > SIZE_MAX / sizeof(*b->doc->shape_evenodd)) {
            fail(b->diag, SVG_SHAPE_ERROR_LIMIT, b->offset);
            return false;
        }
        bool *rules = realloc(b->doc->shape_evenodd, next * sizeof(*rules));
        if (rules == NULL) {
            fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset);
            return false;
        }
        b->doc->shape_evenodd = rules;
        bool *filled = realloc(b->doc->shape_filled, next * sizeof(*filled));
        if (filled == NULL) {
            fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset);
            return false;
        }
        b->doc->shape_filled = filled;
        bool *closed = realloc(b->doc->shape_stroke_closed, next * sizeof(*closed));
        if (closed == NULL) { fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset); return false; }
        b->doc->shape_stroke_closed = closed;
        size_t *starts = realloc(b->doc->shape_edge_start, next * sizeof(*starts));
        if (starts == NULL) { fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset); return false; }
        b->doc->shape_edge_start = starts;
        size_t *counts = realloc(b->doc->shape_edge_count, next * sizeof(*counts));
        if (counts == NULL) { fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset); return false; }
        b->doc->shape_edge_count = counts;
        svg_shape_stroke *strokes = realloc(b->doc->strokes, next * sizeof(*strokes));
        if (strokes == NULL) { fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset); return false; }
        b->doc->strokes = strokes;
        svg_shape_paint *paints = realloc(b->doc->paints, next * sizeof(*paints));
        if (paints == NULL) { fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset); return false; }
        b->doc->paints = paints;
        svg_shape_clip **clips = realloc(b->doc->clips, next * sizeof(*clips));
        if (clips == NULL) { fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset); return false; }
        b->doc->clips = clips;
        geom_affine *inverses = realloc(b->doc->shape_stroke_inverse, next * sizeof(*inverses));
        if (inverses == NULL) { fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset); return false; }
        b->doc->shape_stroke_inverse = inverses;
        b->shape_capacity = next;
    }
    b->shape_index = b->doc->shape_count++;
    b->doc->shape_evenodd[b->shape_index] = evenodd;
    b->doc->shape_filled[b->shape_index] = true;
    b->doc->shape_stroke_closed[b->shape_index] = true;
    b->doc->shape_edge_start[b->shape_index] = b->doc->edge_count;
    b->doc->shape_edge_count[b->shape_index] = 0U;
    b->doc->shape_stroke_inverse[b->shape_index] = geom_affine_identity();
    (void)affine_inverse(b->transform, &b->doc->shape_stroke_inverse[b->shape_index]);
    b->doc->strokes[b->shape_index] = (svg_shape_stroke){
        .width = 1.0, .miter_limit = 4.0, .cap = SVG_SHAPE_CAP_BUTT,
        .join = SVG_SHAPE_JOIN_MITER
    };
    b->doc->paints[b->shape_index] = (svg_shape_paint){.kind = SVG_SHAPE_PAINT_SOLID};
    b->doc->clips[b->shape_index] = NULL;
    return true;
}
static geom_point midpoint(geom_point a, geom_point z)
{
    return (geom_point){(a.x+z.x)*0.5,(a.y+z.y)*0.5};
}
static double point_segment_distance(geom_point p,geom_point a,geom_point z)
{
    double dx=z.x-a.x,dy=z.y-a.y,length_squared=dx*dx+dy*dy;
    if(length_squared==0.0)return hypot(p.x-a.x,p.y-a.y);
    double projection=((p.x-a.x)*dx+(p.y-a.y)*dy)/length_squared;
    if(projection<0.0)projection=0.0;
    if(projection>1.0)projection=1.0;
    geom_point nearest={a.x+projection*dx,a.y+projection*dy};
    return hypot(p.x-nearest.x,p.y-nearest.y);
}
static bool add_curve_recursive(builder *b,geom_point p0,geom_point p1,geom_point p2,geom_point p3,unsigned depth)
{
    if(fmax(point_segment_distance(p1,p0,p3),point_segment_distance(p2,p0,p3))<=transformed_flatness_tolerance(b))return add_edge(b,p0,p3);
    if(depth>=16U){fail(b->diag,SVG_SHAPE_ERROR_LIMIT,b->offset);return false;}
    geom_point a=midpoint(p0,p1),c=midpoint(p1,p2),d=midpoint(p2,p3);
    geom_point e=midpoint(a,c),f=midpoint(c,d),m=midpoint(e,f);
    return add_curve_recursive(b,p0,a,e,m,depth+1U)&&add_curve_recursive(b,m,f,d,p3,depth+1U);
}
static bool add_curve(builder *b,geom_point p0,geom_point p1,geom_point p2,geom_point p3)
{
    return add_curve_recursive(b,p0,p1,p2,p3,0U);
}
static bool add_quad_recursive(builder *b,geom_point p0,geom_point p1,geom_point p2,unsigned depth)
{
    if(point_segment_distance(p1,p0,p2)<=transformed_flatness_tolerance(b))return add_edge(b,p0,p2);
    if(depth>=16U){fail(b->diag,SVG_SHAPE_ERROR_LIMIT,b->offset);return false;}
    geom_point a=midpoint(p0,p1),c=midpoint(p1,p2),m=midpoint(a,c);
    return add_quad_recursive(b,p0,a,m,depth+1U)&&add_quad_recursive(b,m,c,p2,depth+1U);
}
static bool add_quad(builder *b,geom_point p0,geom_point p1,geom_point p2)
{
    return add_quad_recursive(b,p0,p1,p2,0U);
}
static bool add_arc(builder *b, geom_point from, double rx, double ry, double rotation,
                    bool large, bool sweep, geom_point to, size_t offset)
{
    rx = fabs(rx); ry = fabs(ry);
    if (rx == 0.0 || ry == 0.0) return add_edge(b, from, to);
    if (from.x == to.x && from.y == to.y) return true;
    double phi = rotation * 0.017453292519943295;
    double cp = cos(phi), sp = sin(phi);
    double dx = (from.x - to.x) * 0.5, dy = (from.y - to.y) * 0.5;
    double xp = cp * dx + sp * dy, yp = -sp * dx + cp * dy;
    double scale = xp*xp/(rx*rx) + yp*yp/(ry*ry);
    if (!isfinite(scale)) { fail(b->diag, SVG_SHAPE_ERROR_NUMBER, offset); return false; }
    if (scale > 1.0) { double factor=sqrt(scale); rx*=factor; ry*=factor; }
    double numerator=rx*rx*ry*ry-rx*rx*yp*yp-ry*ry*xp*xp;
    double denominator=rx*rx*yp*yp+ry*ry*xp*xp;
    double ratio=denominator==0.0?0.0:fmax(0.0,numerator/denominator);
    double sign=large==sweep?-1.0:1.0;
    double coef=sign*sqrt(ratio);
    double cxp=coef*(rx*yp/ry), cyp=coef*(-ry*xp/rx);
    double cx=cp*cxp-sp*cyp+(from.x+to.x)*0.5;
    double cy=sp*cxp+cp*cyp+(from.y+to.y)*0.5;
    double ux=(xp-cxp)/rx, uy=(yp-cyp)/ry;
    double vx=(-xp-cxp)/rx, vy=(-yp-cyp)/ry;
    double start=atan2(uy,ux), delta=atan2(ux*vy-uy*vx,ux*vx+uy*vy);
    if (!sweep && delta>0.0) delta-=6.283185307179586;
    if (sweep && delta<0.0) delta+=6.283185307179586;
    double radius=fmax(rx,ry);
    double tolerance=transformed_flatness_tolerance(b);
    double max_angle=radius<=tolerance?6.283185307179586:2.0*acos(fmax(-1.0,1.0-tolerance/radius));
    double raw=ceil(fabs(delta)/max_angle);
    if (!isfinite(raw) || raw>(double)SVG_SHAPE_MAX_EDGES) { fail(b->diag,SVG_SHAPE_ERROR_LIMIT,offset); return false; }
    unsigned steps=(unsigned)raw; if(steps==0U)steps=1U;
    geom_point previous=from;
    for(unsigned i=1U;i<=steps;i++){
        double angle=start+delta*(double)i/(double)steps;
        geom_point point={cx+cp*rx*cos(angle)-sp*ry*sin(angle),cy+sp*rx*cos(angle)+cp*ry*sin(angle)};
        if(i==steps)point=to;
        if(!geom_finite_point(point)||fabs(point.x)>SHAPE_MAX_COORD||fabs(point.y)>SHAPE_MAX_COORD){fail(b->diag,SVG_SHAPE_ERROR_NUMBER,offset);return false;}
        if(!add_edge(b,previous,point))return false; previous=point;
    }
    return true;
}
static bool path_decode(builder *b, base_bytes src, svg_xml_span span)
{
    number_reader r={src.ptr+span.offset,span.length,0,span.offset};
    for(size_t i=0;i<span.length;i++)if(r.p[i]==','){
        size_t left=i,right=i+1U;
        while(left>0U&&(r.p[left-1U]==' '||r.p[left-1U]=='\t'||r.p[left-1U]=='\r'||r.p[left-1U]=='\n'))left--;
        while(right<span.length&&(r.p[right]==' '||r.p[right]=='\t'||r.p[right]=='\r'||r.p[right]=='\n'))right++;
        if(left==0U||right==span.length||r.p[left-1U]==','||r.p[right]==','||
           !((r.p[left-1U]>='0'&&r.p[left-1U]<='9')||r.p[left-1U]=='.')||
           !((r.p[right]>='0'&&r.p[right]<='9')||r.p[right]=='.'||r.p[right]=='+'||r.p[right]=='-')){
            fail(b->diag,SVG_SHAPE_ERROR_PATH,span.offset+i);return false;
        }
    }
    geom_point current={0,0}, first={0,0}, cubic_control={0,0}, quad_control={0,0};
    char cmd=0; bool open=false, path_first=true;
    bool cubic_reflect=false, quad_reflect=false;
    bool previous_number=false, previous_arc_flag=false;
    while (r.i < r.n) {
        size_t gap_start=r.i;skip_path_separators(&r);bool had_separator=r.i!=gap_start;if(r.i>=r.n)break;
        if ((r.p[r.i] >= 'A'&&r.p[r.i]<='Z')||(r.p[r.i]>='a'&&r.p[r.i]<='z')) {cmd=(char)r.p[r.i++];previous_number=false;previous_arc_flag=false;}
        else if(cmd==0 || cmd=='Z' || cmd=='z') { fail(b->diag,SVG_SHAPE_ERROR_PATH,span.offset+r.i); return false; }
        else {
            if(previous_number&&!had_separator&&!(r.p[r.i]=='+'||r.p[r.i]=='-')) {fail(b->diag,SVG_SHAPE_ERROR_PATH,span.offset+r.i);return false;}
            previous_number=false;previous_arc_flag=false;
        }
        if (++b->command_count > SHAPE_MAX_COMMANDS) { fail(b->diag,SVG_SHAPE_ERROR_LIMIT,span.offset+r.i); return false; }
        bool rel=cmd>='a'&&cmd<='z'; char upper=rel?(char)(cmd-'a'+'A'):cmd;
        if(path_first&&upper!='M'){fail(b->diag,SVG_SHAPE_ERROR_PATH,span.offset+r.i-1U);return false;}
        path_first=false;
        if(upper=='Z') { if(open&&!add_edge(b,current,first))return false; current=first;open=false;cmd=0;cubic_reflect=false;quad_reflect=false;continue; }
        if(upper!='M'&&upper!='L'&&upper!='H'&&upper!='V'&&upper!='C'&&upper!='S'&&upper!='Q'&&upper!='T'&&upper!='A') { fail(b->diag,SVG_SHAPE_ERROR_UNSUPPORTED,span.offset+r.i-1U);return false; }
        double v[7]={0}; unsigned count=upper=='H'||upper=='V'?1U:upper=='C'?6U:(upper=='S'||upper=='Q')?4U:upper=='T'?2U:upper=='A'?7U:2U;
        size_t group_start=r.i; b->offset=span.offset+group_start;
        for(unsigned i=0;i<count;i++){
            if(!path_operand_boundary(&r,previous_number,previous_arc_flag,upper,i)){fail(b->diag,SVG_SHAPE_ERROR_PATH,span.offset+r.i);return false;}
            bool ok=upper=='A'&&(i==3U||i==4U)?read_arc_flag(&r,&v[i]):read_number(&r,&v[i]);
            if(!ok){fail(b->diag,SVG_SHAPE_ERROR_PATH,span.offset+group_start);return false;}
            previous_number=true;previous_arc_flag=upper=='A'&&(i==3U||i==4U);
        }
        geom_point p=current; bool drawn=false;
        if(upper=='H')p.x=rel?current.x+v[0]:v[0];
        else if(upper=='V')p.y=rel?current.y+v[0]:v[0];
        else if(upper=='C'||upper=='S'||upper=='Q'||upper=='T'||upper=='A') {
            if(upper=='C') { geom_point c1={v[0],v[1]},c2={v[2],v[3]};p=(geom_point){v[4],v[5]}; if(rel){c1.x+=current.x;c1.y+=current.y;c2.x+=current.x;c2.y+=current.y;p.x+=current.x;p.y+=current.y;} if(!point_bounded(c1)||!point_bounded(c2)||!point_bounded(p)){fail(b->diag,SVG_SHAPE_ERROR_NUMBER,span.offset+group_start);return false;} if(!add_curve(b,current,c1,c2,p))return false;drawn=true; }
            else if(upper=='S') { geom_point c1=cubic_reflect?(geom_point){2.0*current.x-cubic_control.x,2.0*current.y-cubic_control.y}:current; geom_point c2={v[0],v[1]};p=(geom_point){v[2],v[3]};if(rel){c2.x+=current.x;c2.y+=current.y;p.x+=current.x;p.y+=current.y;}if(!point_bounded(c1)||!point_bounded(c2)||!point_bounded(p)){fail(b->diag,SVG_SHAPE_ERROR_NUMBER,span.offset+group_start);return false;}if(!add_curve(b,current,c1,c2,p))return false;cubic_control=c2;cubic_reflect=true;quad_reflect=false;drawn=true; }
            else if(upper=='Q') {geom_point c={v[0],v[1]};p=(geom_point){v[2],v[3]};if(rel){c.x+=current.x;c.y+=current.y;p.x+=current.x;p.y+=current.y;}if(!point_bounded(c)||!point_bounded(p)){fail(b->diag,SVG_SHAPE_ERROR_NUMBER,span.offset+group_start);return false;}if(!add_quad(b,current,c,p))return false;quad_control=c;quad_reflect=true;cubic_reflect=false;drawn=true;}
            else if(upper=='T') {geom_point c=quad_reflect?(geom_point){2.0*current.x-quad_control.x,2.0*current.y-quad_control.y}:current;p=(geom_point){v[0],v[1]};if(rel){p.x+=current.x;p.y+=current.y;}if(!point_bounded(c)||!point_bounded(p)){fail(b->diag,SVG_SHAPE_ERROR_NUMBER,span.offset+group_start);return false;}if(!add_quad(b,current,c,p))return false;quad_control=c;quad_reflect=true;cubic_reflect=false;drawn=true;}
            else { if((v[3]!=0.0&&v[3]!=1.0)||(v[4]!=0.0&&v[4]!=1.0)||v[0]<0.0||v[1]<0.0){fail(b->diag,SVG_SHAPE_ERROR_PATH,span.offset+group_start);return false;} p=(geom_point){v[5],v[6]};if(rel){p.x+=current.x;p.y+=current.y;}if(!point_bounded(p)){fail(b->diag,SVG_SHAPE_ERROR_NUMBER,span.offset+group_start);return false;} if(!add_arc(b,current,v[0],v[1],v[2],v[3]==1.0,v[4]==1.0,p,span.offset+group_start))return false;drawn=true; }
            if(upper=='C'){cubic_control=(geom_point){v[2],v[3]};if(rel){cubic_control.x+=current.x;cubic_control.y+=current.y;}cubic_reflect=true;quad_reflect=false;}
            if(upper=='A'){cubic_reflect=false;quad_reflect=false;}
        } else {p=(geom_point){v[0],v[1]};if(rel){p.x+=current.x;p.y+=current.y;} if(upper=='M'){if(!point_bounded(p)){fail(b->diag,SVG_SHAPE_ERROR_NUMBER,span.offset+group_start);return false;}if(open&&!add_edge(b,current,first))return false;current=p;first=p;open=true;cmd=rel?'l':'L';cubic_reflect=false;quad_reflect=false;continue;}}
        if(upper=='M'||upper=='L'||upper=='H'||upper=='V'){cubic_reflect=false;quad_reflect=false;}
        if(!point_bounded(p)){fail(b->diag,SVG_SHAPE_ERROR_NUMBER,span.offset+group_start);return false;}
        if(!drawn&&!add_edge(b,current,p))return false; current=p; open=true;
    }
    if(open && !add_edge(b,current,first))return false;
    return true;
}
static bool parse_color(base_bytes src, svg_xml_span s, uint8_t out[4])
{
    out[3]=255U;
    if(svg_css_keyword_equals(src,s,"none")){out[3]=0U;return true;}
    if(svg_css_keyword_equals(src,s,"black")){out[0]=out[1]=out[2]=0U;return true;}
    if(svg_css_keyword_equals(src,s,"white")){out[0]=out[1]=out[2]=255U;return true;}
    if(svg_css_keyword_equals(src,s,"red")){out[0]=255U;out[1]=out[2]=0U;return true;}
    if(svg_css_keyword_equals(src,s,"green")){out[0]=0U;out[1]=128U;out[2]=0U;return true;}
    if(svg_css_keyword_equals(src,s,"blue")){out[0]=out[1]=0U;out[2]=255U;return true;}
    if(s.length==4U&&src.ptr[s.offset]=='#'){for(size_t i=0;i<3U;i++){uint8_t c=src.ptr[s.offset+1U+i];int v=c>='0'&&c<='9'?c-'0':c>='a'&&c<='f'?c-'a'+10:c>='A'&&c<='F'?c-'A'+10:-1;if(v<0)return false;out[i]=(uint8_t)(v*17); }return true;}
    if(s.length==7U&&src.ptr[s.offset]=='#'){for(size_t i=0;i<3U;i++){uint8_t a=src.ptr[s.offset+1U+i*2U],c=src.ptr[s.offset+2U+i*2U];int h=a>='0'&&a<='9'?a-'0':a>='a'&&a<='f'?a-'a'+10:a>='A'&&a<='F'?a-'A'+10:-1;int l=c>='0'&&c<='9'?c-'0':c>='a'&&c<='f'?c-'a'+10:c>='A'&&c<='F'?c-'A'+10:-1;if(h<0||l<0)return false;out[i]=(uint8_t)(h*16+l);}return true;}
    return false;
}
static bool attr(base_bytes src, svg_xml_token *tokens, size_t count,
                 size_t element, const char *name, svg_xml_span *value);
static bool parse_stroke_number(base_bytes src, svg_xml_span span, double *out)
{
    number_reader r = {src.ptr + span.offset, span.length, 0U, span.offset};
    if (!read_number(&r, out)) return false;
    skip_space(&r);
    return r.i == r.n && isfinite(*out);
}
static bool parse_dasharray(base_bytes src, svg_xml_span span, svg_shape_stroke *stroke)
{
    if (svg_css_keyword_equals(src, span, "none")) return true;
    number_reader r = {src.ptr + span.offset, span.length, 0U, span.offset};
    while (true) {
        skip_space(&r);
        if (r.i == r.n) break;
        if (stroke->dash_count == SVG_SHAPE_MAX_DASH_VALUES) return false;
        double value;
        if (!read_number(&r, &value) || value <= 0.0 || value > 1000000.0) return false;
        stroke->dash[stroke->dash_count++] = value;
        size_t before = r.i;
        skip_space(&r);
        bool spaced = r.i != before;
        if (r.i < r.n && r.p[r.i] == ',') {
            r.i++; skip_space(&r);
            if (r.i == r.n || r.p[r.i] == ',') return false;
        } else if (r.i < r.n && !spaced) return false;
    }
    if (stroke->dash_count == 0U) return false;
    return true;
}
static bool parse_stroke(base_bytes src, svg_shape_stroke *stroke,
                         const svg_css_paint *paint, svg_shape_diagnostic *diag)
{
    svg_xml_span span;
    if (svg_css_paint_value(paint, SVG_CSS_STROKE, &span) &&
        !parse_color(src, span, stroke->rgba)) {
        fail(diag, SVG_SHAPE_ERROR_UNSUPPORTED, span.offset); return false;
    }
    if (!svg_css_paint_value(paint, SVG_CSS_STROKE, &span)) stroke->rgba[3] = 0U;
    if (svg_css_paint_value(paint, SVG_CSS_STROKE_WIDTH, &span)) {
        if (!parse_stroke_number(src, span, &stroke->width) || stroke->width < 0.0 || stroke->width > 1024.0) {
            fail(diag, SVG_SHAPE_ERROR_NUMBER, span.offset); return false;
        }
    }
    if (svg_css_paint_value(paint, SVG_CSS_STROKE_MITERLIMIT, &span)) {
        if (!parse_stroke_number(src, span, &stroke->miter_limit) || stroke->miter_limit < 1.0 || stroke->miter_limit > 64.0) {
            fail(diag, SVG_SHAPE_ERROR_NUMBER, span.offset); return false;
        }
    }
    if (svg_css_paint_value(paint, SVG_CSS_STROKE_DASHOFFSET, &span)) {
        if (!parse_stroke_number(src, span, &stroke->dash_offset) || fabs(stroke->dash_offset) > 1000000.0) {
            fail(diag, SVG_SHAPE_ERROR_NUMBER, span.offset); return false;
        }
    }
    if (svg_css_paint_value(paint, SVG_CSS_STROKE_LINECAP, &span)) {
        if (svg_css_keyword_equals(src, span, "butt")) stroke->cap = SVG_SHAPE_CAP_BUTT;
        else if (svg_css_keyword_equals(src, span, "square")) stroke->cap = SVG_SHAPE_CAP_SQUARE;
        else if (svg_css_keyword_equals(src, span, "round")) stroke->cap = SVG_SHAPE_CAP_ROUND;
        else { fail(diag, SVG_SHAPE_ERROR_UNSUPPORTED, span.offset); return false; }
    }
    if (svg_css_paint_value(paint, SVG_CSS_STROKE_LINEJOIN, &span)) {
        if (svg_css_keyword_equals(src, span, "miter")) stroke->join = SVG_SHAPE_JOIN_MITER;
        else if (svg_css_keyword_equals(src, span, "bevel")) stroke->join = SVG_SHAPE_JOIN_BEVEL;
        else if (svg_css_keyword_equals(src, span, "round")) stroke->join = SVG_SHAPE_JOIN_ROUND;
        else { fail(diag, SVG_SHAPE_ERROR_UNSUPPORTED, span.offset); return false; }
    }
    if (svg_css_paint_value(paint, SVG_CSS_STROKE_DASHARRAY, &span) &&
        !parse_dasharray(src, span, stroke)) {
        fail(diag, SVG_SHAPE_ERROR_NUMBER, span.offset); return false;
    }
    return true;
}
static bool attr(base_bytes src, svg_xml_token *tokens,size_t count,size_t element,const char *name,svg_xml_span *value)
{
    for(size_t i=element+1U;i<count&&tokens[i].kind==SVG_XML_TOKEN_ATTRIBUTE;i++)if(tokens[i].prefix.length==0U&&span_eq(src,tokens[i].local_name,name)){*value=tokens[i].value;return true;}
    return false;
}
static bool number_attr(base_bytes src, svg_xml_token *tokens, size_t count,
                        size_t element, const char *name, double *value)
{
    svg_xml_span span;
    if (!attr(src, tokens, count, element, name, &span)) return true;
    number_reader reader = {src.ptr + span.offset, span.length, 0U, span.offset};
    if (!read_number(&reader, value)) return false;
    skip_space(&reader);
    return reader.i == reader.n;
}
static bool ellipse_edges(builder *b, double cx, double cy, double rx, double ry)
{
    if (rx == 0.0 || ry == 0.0) return true;
    /* The ellipse is a diagonal affine image of the unit circle. Its maximum
       axis scales the circle's chord sagitta conservatively; the active matrix
       is already bounded by transformed_flatness_tolerance. */
    double radius=fmax(rx,ry);
    double tolerance=transformed_flatness_tolerance(b);
    double angle=radius<=tolerance?6.283185307179586:
        2.0*acos(fmax(-1.0,1.0-tolerance/radius));
    double raw=ceil(6.283185307179586/angle);
    if (!isfinite(raw) || raw > 4096.0) {
        fail(b->diag,SVG_SHAPE_ERROR_LIMIT,b->offset);
        return false;
    }
    unsigned steps=(unsigned)raw;
    if (steps<12U) steps=12U;
    geom_point previous={cx+rx,cy};
    for (unsigned i=1U;i<=steps;i++) {
        double theta=6.283185307179586*(double)i/(double)steps;
        geom_point next=i==steps?(geom_point){cx+rx,cy}:
            (geom_point){cx+rx*cos(theta),cy+ry*sin(theta)};
        if (!point_bounded(next)) {
            fail(b->diag, SVG_SHAPE_ERROR_NUMBER, b->offset);
            return false;
        }
        if (!add_edge(b,previous,next)) return false;
        previous=next;
    }
    return true;
}
bool svg_shape_builder_add_element(svg_shape_builder *builder,
                                   base_bytes source,
                                   svg_xml_token *tokens, size_t token_count,
                                   size_t element,
                                   const svg_css_paint *style,
                                   geom_affine transform)
{
    if (builder == NULL || builder->doc == NULL || tokens == NULL ||
        style == NULL || element >= token_count) {
        fail(builder == NULL ? NULL : builder->diag,
             SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U);
        return false;
    }
    builder->offset = tokens[element].name.offset;
    builder->transform = transform;
    svg_shape_document *out = builder->doc;
    uint8_t color[4] = {0U, 0U, 0U, 255U};
    svg_xml_span fill = {0};
    bool has_fill = svg_css_paint_value(style, SVG_CSS_FILL, &fill);
    bool gradient_fill = has_fill && fill.length >= 5U &&
        fill.offset <= source.len && fill.length <= source.len - fill.offset &&
        memcmp(source.ptr + fill.offset, "url(#", 5U) == 0;
    if (has_fill && !gradient_fill && !parse_color(source, fill, color)) {
        fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED, fill.offset);
        return false;
    }
    svg_xml_span rule = {0};
    if (svg_css_paint_value(style, SVG_CSS_FILL_RULE, &rule) &&
        !svg_css_keyword_equals(source, rule, "evenodd") &&
        !svg_css_keyword_equals(source, rule, "nonzero")) {
        fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED, rule.offset);
        return false;
    }
    bool evenodd = svg_css_paint_value(style, SVG_CSS_FILL_RULE, &rule) &&
                   svg_css_keyword_equals(source, rule, "evenodd");
    if (!begin_shape(builder, evenodd)) return false;
    size_t before = out->edge_count;
    if (!parse_stroke(source, &out->strokes[builder->shape_index], style,
                      builder->diag)) return false;
    if (out->strokes[builder->shape_index].rgba[3] != 0U &&
        !affine_inverse(transform,
                        &out->shape_stroke_inverse[builder->shape_index])) {
        svg_xml_span stroke_span = {0};
        (void)svg_css_paint_value(style, SVG_CSS_STROKE, &stroke_span);
        fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED, stroke_span.offset);
        return false;
    }
    if (span_eq(source, tokens[element].local_name, "path") &&
        out->strokes[builder->shape_index].rgba[3] != 0U) {
        svg_xml_span stroke_span = {0};
        (void)svg_css_paint_value(style, SVG_CSS_STROKE, &stroke_span);
        fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED, stroke_span.offset);
        return false;
    }

    if (span_eq(source, tokens[element].local_name, "rect")) {
        double q[4] = {0.0, 0.0, 0.0, 0.0};
        const char *names[4] = {"x", "y", "width", "height"};
        bool has_width = false, has_height = false;
        for (size_t k = 0U; k < 4U; ++k) {
            svg_xml_span value;
            if (!attr(source, tokens, token_count, element, names[k], &value))
                continue;
            if (k == 2U) has_width = true;
            if (k == 3U) has_height = true;
            number_reader reader = {source.ptr + value.offset, value.length,
                                    0U, value.offset};
            if (!read_number(&reader, &q[k])) {
                fail(builder->diag, SVG_SHAPE_ERROR_NUMBER, value.offset);
                return false;
            }
            skip_space(&reader);
            if (reader.i != reader.n) {
                fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED, value.offset);
                return false;
            }
        }
        if (!has_width || !has_height) {
            fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED,
                 tokens[element].name.offset);
            return false;
        }
        if (q[2] < 0.0 || q[3] < 0.0) {
            fail(builder->diag, SVG_SHAPE_ERROR_NUMBER,
                 tokens[element].name.offset);
            return false;
        }
        if (q[2] > 0.0 && q[3] > 0.0) {
            geom_point points[4] = {{q[0], q[1]}, {q[0] + q[2], q[1]},
                                    {q[0] + q[2], q[1] + q[3]},
                                    {q[0], q[1] + q[3]}};
            for (size_t k = 0U; k < 4U; ++k) {
                if (!geom_finite_point(points[k]) ||
                    fabs(points[k].x) > SHAPE_MAX_COORD ||
                    fabs(points[k].y) > SHAPE_MAX_COORD) {
                    fail(builder->diag, SVG_SHAPE_ERROR_NUMBER,
                         tokens[element].name.offset);
                    return false;
                }
            }
            for (size_t k = 0U; k < 4U; ++k)
                if (!add_edge(builder, points[k], points[(k + 1U) % 4U]))
                    return false;
        }
    } else if (span_eq(source, tokens[element].local_name, "path")) {
        svg_xml_span data;
        if (!attr(source, tokens, token_count, element, "d", &data)) {
            fail(builder->diag, SVG_SHAPE_ERROR_PATH,
                 tokens[element].name.offset);
            return false;
        }
        if (!path_decode(builder, source, data)) return false;
    } else if (span_eq(source, tokens[element].local_name, "circle") ||
               span_eq(source, tokens[element].local_name, "ellipse")) {
        double cx = 0.0, cy = 0.0, rx = 0.0, ry = 0.0;
        if (!number_attr(source, tokens, token_count, element, "cx", &cx) ||
            !number_attr(source, tokens, token_count, element, "cy", &cy)) {
            fail(builder->diag, SVG_SHAPE_ERROR_NUMBER,
                 tokens[element].name.offset);
            return false;
        }
        if (span_eq(source, tokens[element].local_name, "circle")) {
            if (!number_attr(source, tokens, token_count, element, "r", &rx)) {
                fail(builder->diag, SVG_SHAPE_ERROR_NUMBER,
                     tokens[element].name.offset);
                return false;
            }
            ry = rx;
        } else if (!number_attr(source, tokens, token_count, element, "rx", &rx) ||
                   !number_attr(source, tokens, token_count, element, "ry", &ry)) {
            fail(builder->diag, SVG_SHAPE_ERROR_NUMBER,
                 tokens[element].name.offset);
            return false;
        }
        if (rx < 0.0 || ry < 0.0 || !point_bounded((geom_point){cx, cy}) ||
            rx > SHAPE_MAX_COORD || ry > SHAPE_MAX_COORD) {
            fail(builder->diag, SVG_SHAPE_ERROR_NUMBER,
                 tokens[element].name.offset);
            return false;
        }
        if (!ellipse_edges(builder, cx, cy, rx, ry)) return false;
    } else if (span_eq(source, tokens[element].local_name, "line")) {
        double xy[4] = {0.0, 0.0, 0.0, 0.0};
        const char *names[4] = {"x1", "y1", "x2", "y2"};
        for (size_t k = 0U; k < 4U; ++k) {
            if (!number_attr(source, tokens, token_count, element, names[k],
                             &xy[k])) {
                fail(builder->diag, SVG_SHAPE_ERROR_NUMBER,
                     tokens[element].name.offset);
                return false;
            }
        }
        if (!point_bounded((geom_point){xy[0], xy[1]}) ||
            !point_bounded((geom_point){xy[2], xy[3]})) {
            fail(builder->diag, SVG_SHAPE_ERROR_NUMBER,
                 tokens[element].name.offset);
            return false;
        }
        out->shape_filled[builder->shape_index] = false;
        out->shape_stroke_closed[builder->shape_index] = false;
        if (!add_edge(builder, (geom_point){xy[0], xy[1]},
                      (geom_point){xy[2], xy[3]})) return false;
    } else {
        svg_xml_span points;
        if (!attr(source, tokens, token_count, element, "points", &points)) {
            fail(builder->diag, SVG_SHAPE_ERROR_PATH,
                 tokens[element].name.offset);
            return false;
        }
        number_reader reader = {source.ptr + points.offset, points.length, 0U,
                                points.offset};
        geom_point first = {0.0, 0.0}, previous = {0.0, 0.0};
        size_t point_count = 0U;
        while (true) {
            size_t before_separator = reader.i;
            skip_space(&reader);
            bool had_space = reader.i != before_separator;
            if (point_count != 0U) {
                if (reader.i < reader.n && reader.p[reader.i] == ',') {
                    reader.i++;
                    skip_space(&reader);
                    if (reader.i == reader.n || reader.p[reader.i] == ',') {
                        fail(builder->diag, SVG_SHAPE_ERROR_PATH,
                             points.offset + reader.i);
                        return false;
                    }
                } else if (reader.i < reader.n && !had_space) {
                    fail(builder->diag, SVG_SHAPE_ERROR_PATH,
                         points.offset + reader.i);
                    return false;
                }
            }
            if (reader.i == reader.n) break;
            double x, y;
            if (!read_number(&reader, &x) || !next_number_separator(&reader) ||
                !read_number(&reader, &y)) {
                fail(builder->diag, SVG_SHAPE_ERROR_PATH,
                     points.offset + reader.i);
                return false;
            }
            geom_point point = {x, y};
            if (!point_bounded(point)) {
                fail(builder->diag, SVG_SHAPE_ERROR_NUMBER,
                     points.offset + reader.i);
                return false;
            }
            if (builder->point_count >= SVG_SHAPE_MAX_COMMANDS) {
                fail(builder->diag, SVG_SHAPE_ERROR_LIMIT,
                     points.offset + reader.i);
                return false;
            }
            builder->point_count++;
            if (point_count == 0U) first = point;
            else if (!add_edge(builder, previous, point)) return false;
            previous = point;
            point_count++;
        }
        if (point_count < 2U) {
            fail(builder->diag, SVG_SHAPE_ERROR_PATH, points.offset);
            return false;
        }
        if (!add_edge(builder, previous, first)) return false;
        if (span_eq(source, tokens[element].local_name, "polyline"))
            out->shape_stroke_closed[builder->shape_index] = false;
    }

    for (size_t k = before; k < out->edge_count; ++k)
        memcpy(out->colors + k * 4U, color, 4U);
    out->shape_edge_start[builder->shape_index] = before;
    out->shape_edge_count[builder->shape_index] = out->edge_count - before;
    if (gradient_fill) {
        svg_shape_error result = svg_paint_resolve_fill(
            source, tokens, token_count, fill, out->edges + before,
            out->edge_count - before, out->view_width, out->view_height,
            &out->paints[builder->shape_index], &out->paint_stops,
            &out->paint_stop_count, builder->diag);
        if (result != SVG_SHAPE_ERROR_NONE) return false;
    }
    svg_xml_span clip_value;
    if (svg_css_paint_value(style, SVG_CSS_CLIP_PATH, &clip_value) &&
        !svg_css_keyword_equals(source, clip_value, "none")) {
        svg_shape_error result = svg_clip_resolve(
            source, tokens, token_count, clip_value, out, &out->clip_bytes,
            builder->shape_index, &out->clips[builder->shape_index],
            builder->diag);
        if (result != SVG_SHAPE_ERROR_NONE) return false;
    }
    return true;
}
