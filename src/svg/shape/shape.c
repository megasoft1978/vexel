#include "shape.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

#define SHAPE_MAX_BYTES (16U * 1024U * 1024U)
#define SHAPE_MAX_COMMANDS 65536U
#define SHAPE_MAX_COORD 1000000.0

typedef struct number_reader { const uint8_t *p; size_t n, i, base; } number_reader;
typedef struct builder {
    svg_shape_document *doc;
    size_t edge_capacity;
    size_t shape_capacity;
    size_t shape_index;
    size_t command_count;
    size_t point_count;
    size_t offset;
    svg_shape_diagnostic *diag;
    geom_affine transform;
} builder;

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

static bool read_arc_flag(number_reader *r, double *out)
{
    skip_space(r);
    if (r->i >= r->n || (r->p[r->i] != '0' && r->p[r->i] != '1')) return false;
    *out = (double)(r->p[r->i++] - '0');
    return true;
}

static bool read_dimension(base_bytes source, svg_xml_span span, double *out)
{
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
        b->shape_capacity = next;
    }
    b->shape_index = b->doc->shape_count++;
    b->doc->shape_evenodd[b->shape_index] = evenodd;
    b->doc->shape_filled[b->shape_index] = true;
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
    if(span_eq(src,s,"none")){out[3]=0U;return true;}
    if(span_eq(src,s,"black")){out[0]=out[1]=out[2]=0U;return true;}
    if(span_eq(src,s,"white")){out[0]=out[1]=out[2]=255U;return true;}
    if(span_eq(src,s,"red")){out[0]=255U;out[1]=out[2]=0U;return true;}
    if(span_eq(src,s,"green")){out[0]=0U;out[1]=128U;out[2]=0U;return true;}
    if(span_eq(src,s,"blue")){out[0]=out[1]=0U;out[2]=255U;return true;}
    if(s.length==4U&&src.ptr[s.offset]=='#'){for(size_t i=0;i<3U;i++){uint8_t c=src.ptr[s.offset+1U+i];int v=c>='0'&&c<='9'?c-'0':c>='a'&&c<='f'?c-'a'+10:c>='A'&&c<='F'?c-'A'+10:-1;if(v<0)return false;out[i]=(uint8_t)(v*17); }return true;}
    if(s.length==7U&&src.ptr[s.offset]=='#'){for(size_t i=0;i<3U;i++){uint8_t a=src.ptr[s.offset+1U+i*2U],c=src.ptr[s.offset+2U+i*2U];int h=a>='0'&&a<='9'?a-'0':a>='a'&&a<='f'?a-'a'+10:a>='A'&&a<='F'?a-'A'+10:-1;int l=c>='0'&&c<='9'?c-'0':c>='a'&&c<='f'?c-'a'+10:c>='A'&&c<='F'?c-'A'+10:-1;if(h<0||l<0)return false;out[i]=(uint8_t)(h*16+l);}return true;}
    return false;
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

static bool read_transform_args(number_reader *r, double *values, size_t count)
{
    for (size_t i=0U;i<count;i++) {
        if (i != 0U && !next_number_separator(r)) return false;
        if (!read_number(r, &values[i])) return false;
    }
    return true;
}

static bool transform_parse(base_bytes src, svg_xml_span span, geom_affine *out,
                            size_t *operation_count)
{
    number_reader r = {src.ptr + span.offset, span.length, 0U, span.offset};
    geom_affine result = geom_affine_identity();
    size_t operations = 0U;
    while (true) {
        skip_space(&r);
        if (r.i == r.n) break;
        size_t name = r.i;
        while (r.i < r.n && src.ptr[span.offset+r.i] >= 'a' &&
               src.ptr[span.offset+r.i] <= 'z') r.i++;
        if (r.i == name || r.i >= r.n || src.ptr[span.offset+r.i++] != '(') return false;
        size_t name_len = r.i - name - 1U;
        double v[6] = {0};
        size_t n = 0U;
        geom_affine op = geom_affine_identity();
        if (name_len == 6U && memcmp(src.ptr+span.offset+name, "matrix", 6U) == 0) {
            n=6U;
        } else if (name_len == 9U && memcmp(src.ptr+span.offset+name, "translate", 9U) == 0) {
            n=2U;
        } else if (name_len == 5U && memcmp(src.ptr+span.offset+name, "scale", 5U) == 0) {
            n=2U;
        } else if (name_len == 6U && memcmp(src.ptr+span.offset+name, "rotate", 6U) == 0) {
            n=3U;
        } else return false;
        skip_space(&r);
        if (n == 2U && (name_len == 9U || name_len == 5U)) {
            if (!read_number(&r, &v[0])) return false;
            size_t before = r.i;
            skip_space(&r);
            if (r.i < r.n && src.ptr[span.offset+r.i] != ')') {
                r.i = before;
                if (!next_number_separator(&r)) return false;
                if (!read_number(&r, &v[1])) return false;
            } else {
                v[1] = name_len == 5U ? v[0] : 0.0;
            }
        } else if (n == 3U) {
            if (!read_number(&r, &v[0])) return false;
            size_t before = r.i;
            skip_space(&r);
            if (r.i < r.n && src.ptr[span.offset+r.i] != ')') {
                r.i = before;
                if (!next_number_separator(&r)) return false;
                if (!read_number(&r, &v[1]) || !next_number_separator(&r) ||
                    !read_number(&r, &v[2])) return false;
            } else {
                v[1] = 0.0;
                v[2] = 0.0;
            }
        } else if (!read_transform_args(&r, v, n)) return false;
        skip_space(&r);
        if (r.i >= r.n || src.ptr[span.offset+r.i++] != ')') return false;
        if (name_len == 6U && n == 6U) op=(geom_affine){v[0],v[1],v[2],v[3],v[4],v[5]};
        else if (name_len == 9U) op.e=v[0],op.f=v[1];
        else if (name_len == 5U) op.a=v[0],op.d=v[1];
        else {
            double angle=v[0]*0.017453292519943295, cs=cos(angle), sn=sin(angle);
            op=(geom_affine){cs,sn,-sn,cs,0.0,0.0};
            if (n == 3U) { op.e=v[1]-cs*v[1]+sn*v[2]; op.f=v[2]-sn*v[1]-cs*v[2]; }
        }
        if (++operations > 64U || !geom_affine_valid(op) ||
            !geom_affine_compose(result, op, &result)) return false;
        size_t after_close=r.i;
        skip_space(&r);
        bool had_space=r.i!=after_close;
        if (r.i < r.n && src.ptr[span.offset+r.i] == ',') {
            r.i++;
            skip_space(&r);
            if (r.i==r.n) return false;
        } else if (r.i < r.n && !had_space) return false;
    }
    *out=result;
    *operation_count=operations;
    return true;
}

static bool transformed_node(base_bytes src, svg_xml_token *tokens, size_t count,
                             size_t element, geom_affine parent, geom_affine *out,
                             size_t *operations, svg_shape_diagnostic *diag)
{
    svg_xml_span span;
    geom_affine local=geom_affine_identity();
    *operations=0U;
    if (attr(src,tokens,count,element,"transform",&span) &&
        !transform_parse(src,span,&local,operations)) {
        fail(diag,SVG_SHAPE_ERROR_UNSUPPORTED,span.offset);
        return false;
    }
    if (!geom_affine_compose(parent,local,out)) {
        fail(diag,SVG_SHAPE_ERROR_NUMBER,tokens[element].name.offset);
        return false;
    }
    return true;
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

static bool attributes_supported(base_bytes source,svg_xml_token *tokens,size_t count,size_t element,bool root,svg_xml_span kind)
{
    for(size_t i=element+1U;i<count&&tokens[i].kind==SVG_XML_TOKEN_ATTRIBUTE;i++){
        if(tokens[i].prefix.length!=0U)return false;
        svg_xml_span n=tokens[i].local_name;
        bool common=span_eq(source,n,"fill")||span_eq(source,n,"fill-rule")||span_eq(source,n,"transform");
        bool shape=span_eq(source,kind,"rect") ? (span_eq(source,n,"x")||span_eq(source,n,"y")||span_eq(source,n,"width")||span_eq(source,n,"height")) :
            span_eq(source,kind,"path") ? span_eq(source,n,"d") :
            span_eq(source,kind,"circle") ? (span_eq(source,n,"cx")||span_eq(source,n,"cy")||span_eq(source,n,"r")) :
            span_eq(source,kind,"ellipse") ? (span_eq(source,n,"cx")||span_eq(source,n,"cy")||span_eq(source,n,"rx")||span_eq(source,n,"ry")) :
            span_eq(source,kind,"line") ? (span_eq(source,n,"x1")||span_eq(source,n,"y1")||span_eq(source,n,"x2")||span_eq(source,n,"y2")) :
            (span_eq(source,n,"points"));
        bool ok=root?(span_eq(source,n,"xmlns")||span_eq(source,n,"version")||span_eq(source,n,"width")||span_eq(source,n,"height")||span_eq(source,n,"viewBox")||span_eq(source,n,"transform")):
            span_eq(source,kind,"g") ? span_eq(source,n,"transform") : (common||shape);
        if(!ok)return false;
    }
    return true;
}

svg_shape_error svg_shape_decode(base_bytes source, svg_shape_document *out, svg_shape_diagnostic *diagnostic)
{
    svg_shape_diagnostic local_diagnostic;
    if(diagnostic==NULL)diagnostic=&local_diagnostic;
    if(out==NULL||!base_bytes_valid(source)){fail(diagnostic,SVG_SHAPE_ERROR_INVALID_ARGUMENT,0);return SVG_SHAPE_ERROR_INVALID_ARGUMENT;}
    *out=(svg_shape_document){0}; if(diagnostic!=NULL)*diagnostic=(svg_shape_diagnostic){SVG_SHAPE_ERROR_NONE,0};
    if(source.len>SHAPE_MAX_BYTES){fail(diagnostic,SVG_SHAPE_ERROR_LIMIT,SHAPE_MAX_BYTES);return SVG_SHAPE_ERROR_LIMIT;}
    size_t cap=source.len/2U+2U; if(cap>SIZE_MAX/sizeof(svg_xml_token)){fail(diagnostic,SVG_SHAPE_ERROR_LIMIT,0);return SVG_SHAPE_ERROR_LIMIT;}
    svg_xml_token *tokens=calloc(cap,sizeof(*tokens));if(tokens==NULL){fail(diagnostic,SVG_SHAPE_ERROR_OOM,0);return SVG_SHAPE_ERROR_OOM;}
    svg_xml_result xr=svg_xml_tokenize(source,svg_xml_default_options(),tokens,cap);
    if(xr.diagnostic.category!=SVG_XML_ERROR_NONE){free(tokens);fail(diagnostic,SVG_SHAPE_ERROR_XML,xr.diagnostic.offset);return SVG_SHAPE_ERROR_XML;}
    builder b={.doc=out,.diag=diagnostic,.transform=geom_affine_identity()}; size_t root=SIZE_MAX; svg_xml_span v={0};
    geom_affine transform_stack[65];
    size_t transform_ops=0U;
    out->source=source;
    for(size_t i=0;i<xr.token_count;i++){
        svg_xml_token *t=&tokens[i];
        if(t->kind==SVG_XML_TOKEN_PROCESSING_INSTRUCTION){fail(diagnostic,SVG_SHAPE_ERROR_UNSUPPORTED,t->name.offset);goto bad;}
        if(t->kind==SVG_XML_TOKEN_CDATA){fail(diagnostic,SVG_SHAPE_ERROR_UNSUPPORTED,t->value.offset);goto bad;}
        if(t->kind==SVG_XML_TOKEN_TEXT){
            for(size_t j=0;j<t->value.length;j++){
                uint8_t c=source.ptr[t->value.offset+j];
                if(c!=' '&&c!='\t'&&c!='\r'&&c!='\n'){fail(diagnostic,SVG_SHAPE_ERROR_UNSUPPORTED,t->value.offset+j);goto bad;}
            }
        }
        if(t->kind!=SVG_XML_TOKEN_START_ELEMENT&&t->kind!=SVG_XML_TOKEN_EMPTY_ELEMENT)continue;
        bool is_root=t->depth==0;
        bool is_group=span_eq(source,t->local_name,"g");
        if(t->depth>64U){fail(diagnostic,SVG_SHAPE_ERROR_LIMIT,t->name.offset);goto bad;}
        if(is_root){if(!span_eq(source,t->local_name,"svg")||t->prefix.length!=0U){fail(diagnostic,SVG_SHAPE_ERROR_UNSUPPORTED,t->name.offset);goto bad;}root=i;}
        else if(t->prefix.length!=0U||(!is_group&&!span_eq(source,t->local_name,"rect")&&!span_eq(source,t->local_name,"path")&&!span_eq(source,t->local_name,"circle")&&!span_eq(source,t->local_name,"ellipse")&&!span_eq(source,t->local_name,"line")&&!span_eq(source,t->local_name,"polygon")&&!span_eq(source,t->local_name,"polyline"))){fail(diagnostic,SVG_SHAPE_ERROR_UNSUPPORTED,t->name.offset);goto bad;}
        if(!attributes_supported(source,tokens,xr.token_count,i,is_root,t->local_name)){fail(diagnostic,SVG_SHAPE_ERROR_UNSUPPORTED,t->name.offset);goto bad;}
        geom_affine parent=t->depth==0U?geom_affine_identity():transform_stack[t->depth-1U];
        geom_affine current; size_t local_ops=0U;
        if(!transformed_node(source,tokens,xr.token_count,i,parent,&current,&local_ops,diagnostic))goto bad;
        if(local_ops>4096U-transform_ops){fail(diagnostic,SVG_SHAPE_ERROR_LIMIT,t->name.offset);goto bad;}
        transform_ops+=local_ops;
        transform_stack[t->depth]=current;
        if(is_root){
            svg_xml_span ns;if(attr(source,tokens,xr.token_count,i,"xmlns",&ns)&&!span_eq(source,ns,"http://www.w3.org/2000/svg")){fail(diagnostic,SVG_SHAPE_ERROR_UNSUPPORTED,ns.offset);goto bad;}
            svg_xml_span ws,hs,vs; if(!attr(source,tokens,xr.token_count,i,"width",&ws)||!attr(source,tokens,xr.token_count,i,"height",&hs)){fail(diagnostic,SVG_SHAPE_ERROR_VIEWPORT,t->name.offset);goto bad;}
            double w,h;if(!read_dimension(source,ws,&w)){fail(diagnostic,SVG_SHAPE_ERROR_NUMBER,ws.offset);goto bad;}if(!read_dimension(source,hs,&h)){fail(diagnostic,SVG_SHAPE_ERROR_NUMBER,hs.offset);goto bad;}
            if(w<1||w>4096){fail(diagnostic,SVG_SHAPE_ERROR_VIEWPORT,ws.offset);goto bad;}if(h<1||h>4096){fail(diagnostic,SVG_SHAPE_ERROR_VIEWPORT,hs.offset);goto bad;}out->width=(uint32_t)ceil(w);out->height=(uint32_t)ceil(h);out->view_x=0;out->view_y=0;out->view_width=w;out->view_height=h;
            if(attr(source,tokens,xr.token_count,i,"viewBox",&vs)){number_reader vr={source.ptr+vs.offset,vs.length,0,vs.offset};if(!read_number(&vr,&out->view_x)||!next_number_separator(&vr)||!read_number(&vr,&out->view_y)||!next_number_separator(&vr)||!read_number(&vr,&out->view_width)||!next_number_separator(&vr)||!read_number(&vr,&out->view_height)||out->view_width<=0||out->view_height<=0){fail(diagnostic,SVG_SHAPE_ERROR_VIEWPORT,vs.offset);goto bad;}skip_space(&vr);if(vr.i!=vr.n||fabs(out->view_x+out->view_width)>SHAPE_MAX_COORD||fabs(out->view_y+out->view_height)>SHAPE_MAX_COORD){fail(diagnostic,SVG_SHAPE_ERROR_VIEWPORT,vs.offset);goto bad;}}
        } else if (!is_group) {
            uint8_t color[4]={0,0,0,255};svg_xml_span fs;if(attr(source,tokens,xr.token_count,i,"fill",&fs)&&!parse_color(source,fs,color)){fail(diagnostic,SVG_SHAPE_ERROR_UNSUPPORTED,fs.offset);goto bad;}
            svg_xml_span rule;if(attr(source,tokens,xr.token_count,i,"fill-rule",&rule)&&!span_eq(source,rule,"evenodd")&&!span_eq(source,rule,"nonzero")){fail(diagnostic,SVG_SHAPE_ERROR_UNSUPPORTED,rule.offset);goto bad;}
            bool evenodd=false;
            if(attr(source,tokens,xr.token_count,i,"fill-rule",&rule))evenodd=span_eq(source,rule,"evenodd");
            b.offset=t->name.offset;
            if(!begin_shape(&b,evenodd))goto bad;
            b.transform=current;
            size_t before=out->edge_count;
            if(span_eq(source,t->local_name,"rect")){
                double q[4]={0,0,0,0};const char *names[4]={"x","y","width","height"};bool has_width=false,has_height=false;for(size_t k=0;k<4U;k++)if(attr(source,tokens,xr.token_count,i,names[k],&v)){if(k==2U)has_width=true;if(k==3U)has_height=true;number_reader nr={source.ptr+v.offset,v.length,0,v.offset};if(!read_number(&nr,&q[k])){fail(diagnostic,SVG_SHAPE_ERROR_NUMBER,v.offset);goto bad;}skip_space(&nr);if(nr.i!=nr.n){fail(diagnostic,SVG_SHAPE_ERROR_UNSUPPORTED,v.offset);goto bad;}}
                if(!has_width||!has_height){fail(diagnostic,SVG_SHAPE_ERROR_UNSUPPORTED,t->name.offset);goto bad;}
                if(q[2]<0||q[3]<0){fail(diagnostic,SVG_SHAPE_ERROR_NUMBER,t->name.offset);goto bad;}if(q[2]>0&&q[3]>0){geom_point p[4]={{q[0],q[1]},{q[0]+q[2],q[1]},{q[0]+q[2],q[1]+q[3]},{q[0],q[1]+q[3]}};b.offset=t->name.offset;for(size_t k=0;k<4U;k++)if(!geom_finite_point(p[k])||fabs(p[k].x)>SHAPE_MAX_COORD||fabs(p[k].y)>SHAPE_MAX_COORD){fail(diagnostic,SVG_SHAPE_ERROR_NUMBER,t->name.offset);goto bad;}for(size_t k=0;k<4U;k++)if(!add_edge(&b,p[k],p[(k+1U)%4U]))goto bad;}
            }else if(span_eq(source,t->local_name,"path")){svg_xml_span d;if(!attr(source,tokens,xr.token_count,i,"d",&d)){fail(diagnostic,SVG_SHAPE_ERROR_PATH,t->name.offset);goto bad;}if(!path_decode(&b,source,d))goto bad;}
            else if(span_eq(source,t->local_name,"circle")||span_eq(source,t->local_name,"ellipse")){
                double cx=0.0,cy=0.0,rx=0.0,ry=0.0;
                if(!number_attr(source,tokens,xr.token_count,i,"cx",&cx)||!number_attr(source,tokens,xr.token_count,i,"cy",&cy)){fail(diagnostic,SVG_SHAPE_ERROR_NUMBER,t->name.offset);goto bad;}
                if(span_eq(source,t->local_name,"circle")){if(!number_attr(source,tokens,xr.token_count,i,"r",&rx)){fail(diagnostic,SVG_SHAPE_ERROR_NUMBER,t->name.offset);goto bad;}ry=rx;}
                else if(!number_attr(source,tokens,xr.token_count,i,"rx",&rx)||!number_attr(source,tokens,xr.token_count,i,"ry",&ry)){fail(diagnostic,SVG_SHAPE_ERROR_NUMBER,t->name.offset);goto bad;}
                if(rx<0.0||ry<0.0||!point_bounded((geom_point){cx,cy})||rx>SHAPE_MAX_COORD||ry>SHAPE_MAX_COORD){fail(diagnostic,SVG_SHAPE_ERROR_NUMBER,t->name.offset);goto bad;}
                if(!ellipse_edges(&b,cx,cy,rx,ry))goto bad;
            } else if(span_eq(source,t->local_name,"line")){
                double xy[4]={0};const char *names[4]={"x1","y1","x2","y2"};
                for(size_t k=0U;k<4U;k++)if(!number_attr(source,tokens,xr.token_count,i,names[k],&xy[k])){fail(diagnostic,SVG_SHAPE_ERROR_NUMBER,t->name.offset);goto bad;}
                if(!point_bounded((geom_point){xy[0],xy[1]})||!point_bounded((geom_point){xy[2],xy[3]})){fail(diagnostic,SVG_SHAPE_ERROR_NUMBER,t->name.offset);goto bad;}
                out->shape_filled[b.shape_index]=false;
                if(!add_edge(&b,(geom_point){xy[0],xy[1]},(geom_point){xy[2],xy[3]}))goto bad;
            } else {
                svg_xml_span points;
                if(!attr(source,tokens,xr.token_count,i,"points",&points)){fail(diagnostic,SVG_SHAPE_ERROR_PATH,t->name.offset);goto bad;}
                number_reader nr={source.ptr+points.offset,points.length,0U,points.offset};
                geom_point first={0},previous={0};size_t point_count=0U;
                while(true){
                    size_t before_separator=nr.i;
                    skip_space(&nr);
                    bool had_space=nr.i!=before_separator;
                    if(point_count!=0U){
                        if(nr.i<nr.n&&nr.p[nr.i]==','){
                            nr.i++;skip_space(&nr);
                            if(nr.i==nr.n||nr.p[nr.i]==','){fail(diagnostic,SVG_SHAPE_ERROR_PATH,points.offset+nr.i);goto bad;}
                        }else if(nr.i<nr.n&&!had_space){
                            fail(diagnostic,SVG_SHAPE_ERROR_PATH,points.offset+nr.i);goto bad;
                        }
                    }
                    if(nr.i==nr.n)break;
                    double x,y;
                    if(!read_number(&nr,&x)||!next_number_separator(&nr)||!read_number(&nr,&y)){
                        fail(diagnostic,SVG_SHAPE_ERROR_PATH,points.offset+nr.i);goto bad;
                    }
                    geom_point p={x,y};
                    if(!point_bounded(p)){fail(diagnostic,SVG_SHAPE_ERROR_NUMBER,points.offset+nr.i);goto bad;}
                    if(b.point_count>=SVG_SHAPE_MAX_COMMANDS){fail(diagnostic,SVG_SHAPE_ERROR_LIMIT,points.offset+nr.i);goto bad;}
                    b.point_count++;
                    if(point_count==0U)first=p;else if(!add_edge(&b,previous,p))goto bad;
                    previous=p;point_count++;
                }
                if(point_count<2U){fail(diagnostic,SVG_SHAPE_ERROR_PATH,points.offset);goto bad;}
                if(!add_edge(&b,previous,first))goto bad;
            }
            for(size_t k=before;k<out->edge_count;k++)memcpy(out->colors+k*4U,color,4U);
        }
    }
    if(root==SIZE_MAX){fail(diagnostic,SVG_SHAPE_ERROR_VIEWPORT,0);goto bad;}
    free(tokens);return SVG_SHAPE_ERROR_NONE;
bad:
    free(tokens);svg_shape_document_destroy(out);return diagnostic->category;
}

void svg_shape_document_destroy(svg_shape_document *document)
{
    if(document!=NULL){free(document->edges);free(document->colors);free(document->shape_ids);free(document->shape_evenodd);free(document->shape_filled);*document=(svg_shape_document){0};}
}

const char *svg_shape_error_name(svg_shape_error e)
{
    static const char *names[]={"none","invalid-argument","xml","unsupported","number","limit","path","viewport","oom"};
    return (unsigned)e<sizeof(names)/sizeof(names[0])?names[e]:"invalid-error";
}
