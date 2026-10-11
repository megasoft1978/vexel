#include "shape_internal.h"

#include <math.h>
#include <string.h>

#define SHAPE_MAX_COMMANDS 65536U
#define SHAPE_MAX_COORD 1000000.0
typedef svg_shape_builder builder;
typedef svg_shape_number_reader number_reader;

static bool record_path_subpath(builder *b, size_t first_edge, bool closed)
{
    size_t count = b->doc->edge_count - first_edge;
    if (count == 0U) return true;
    if (b->doc->path_subpath_count >= SVG_SHAPE_MAX_EDGES ||
        b->doc->path_subpath_count >= SIZE_MAX / sizeof(*b->doc->path_subpaths)) {
        svg_shape_fail(b->diag, SVG_SHAPE_ERROR_LIMIT, b->offset);
        return false;
    }
    if (b->doc->path_subpath_count == b->subpath_capacity) {
        size_t next = b->subpath_capacity == 0U ? 16U : b->subpath_capacity * 2U;
        if (next < b->subpath_capacity || next > SVG_SHAPE_MAX_EDGES)
            next = SVG_SHAPE_MAX_EDGES;
        if (next > SIZE_MAX / sizeof(*b->doc->path_subpaths)) {
            svg_shape_fail(b->diag, SVG_SHAPE_ERROR_LIMIT, b->offset);
            return false;
        }
        svg_shape_path_subpath *grown = svg_shape_realloc(
            b->doc->path_subpaths, next * sizeof(*grown));
        if (grown == NULL) {
            svg_shape_fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset);
            return false;
        }
        b->doc->path_subpaths = grown;
        b->subpath_capacity = next;
    }
    b->doc->path_subpaths[b->doc->path_subpath_count++] =
        (svg_shape_path_subpath){first_edge, count, closed};
    return true;
}
static bool add_arc(builder *b, geom_point from, double rx, double ry, double rotation,
                    bool large, bool sweep, geom_point to, size_t offset)
{
    rx = fabs(rx); ry = fabs(ry);
    if (rx == 0.0 || ry == 0.0) return svg_shape_builder_add_edge(b, from, to);
    if (from.x == to.x && from.y == to.y) return true;
    double phi = rotation * 0.017453292519943295;
    double cp = cos(phi), sp = sin(phi);
    double dx = (from.x - to.x) * 0.5, dy = (from.y - to.y) * 0.5;
    double xp = cp * dx + sp * dy, yp = -sp * dx + cp * dy;
    double scale = xp*xp/(rx*rx) + yp*yp/(ry*ry);
    if (!isfinite(scale)) { svg_shape_fail(b->diag, SVG_SHAPE_ERROR_NUMBER, offset); return false; }
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
    double tolerance=svg_shape_transformed_flatness_tolerance(b);
    double max_angle=radius<=tolerance?6.283185307179586:2.0*acos(fmax(-1.0,1.0-tolerance/radius));
    double raw=ceil(fabs(delta)/max_angle);
    if (!isfinite(raw) || raw>(double)SVG_SHAPE_MAX_EDGES) { svg_shape_fail(b->diag,SVG_SHAPE_ERROR_LIMIT,offset); return false; }
    unsigned steps=(unsigned)raw; if(steps==0U)steps=1U;
    geom_point previous=from;
    for(unsigned i=1U;i<=steps;i++){
        double angle=start+delta*(double)i/(double)steps;
        geom_point point={cx+cp*rx*cos(angle)-sp*ry*sin(angle),cy+sp*rx*cos(angle)+cp*ry*sin(angle)};
        if(i==steps)point=to;
        if(!geom_finite_point(point)||fabs(point.x)>SHAPE_MAX_COORD||fabs(point.y)>SHAPE_MAX_COORD){svg_shape_fail(b->diag,SVG_SHAPE_ERROR_NUMBER,offset);return false;}
        if(!svg_shape_builder_add_edge(b,previous,point))return false; previous=point;
    }
    return true;
}

static bool record_path_segment(builder *builder, char command,
                                size_t source_offset, size_t source_length,
                                size_t command_offset,
                                const svg_shape_path_number *numbers,
                                size_t number_count)
{
    svg_shape_document *document = builder->doc;
    if (document->path_segment_count >= SVG_SHAPE_MAX_COMMANDS ||
        number_count > SVG_SHAPE_MAX_PATH_NUMBERS -
                       document->path_number_count) {
        svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_LIMIT, source_offset);
        return false;
    }
    if (document->path_segment_count == builder->path_segment_capacity) {
        size_t next = builder->path_segment_capacity == 0U ? 32U :
                      builder->path_segment_capacity * 2U;
        if (next < builder->path_segment_capacity ||
            next > SVG_SHAPE_MAX_COMMANDS) next = SVG_SHAPE_MAX_COMMANDS;
        if (next > SIZE_MAX / sizeof(*document->path_segments)) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_LIMIT, source_offset);
            return false;
        }
        svg_shape_path_segment *grown = svg_shape_realloc(
            document->path_segments, next * sizeof(*grown));
        if (grown == NULL) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_OOM, source_offset);
            return false;
        }
        document->path_segments = grown;
        builder->path_segment_capacity = next;
    }
    size_t required_numbers = document->path_number_count + number_count;
    if (required_numbers > builder->path_number_capacity) {
        size_t next = builder->path_number_capacity == 0U ? 64U :
                      builder->path_number_capacity;
        while (next < required_numbers) {
            if (next > SVG_SHAPE_MAX_PATH_NUMBERS / 2U) {
                next = SVG_SHAPE_MAX_PATH_NUMBERS;
                break;
            }
            next *= 2U;
        }
        if (next > SIZE_MAX / sizeof(*document->path_numbers)) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_LIMIT, source_offset);
            return false;
        }
        svg_shape_path_number *grown = svg_shape_realloc(
            document->path_numbers, next * sizeof(*grown));
        if (grown == NULL) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_OOM, source_offset);
            return false;
        }
        document->path_numbers = grown;
        builder->path_number_capacity = next;
    }
    size_t first_number = document->path_number_count;
    if (number_count != 0U) {
        memcpy(document->path_numbers + first_number, numbers,
               number_count * sizeof(*numbers));
        document->path_number_count += number_count;
    }
    document->path_segments[document->path_segment_count++] =
        (svg_shape_path_segment){
            .command = command,
            .source_offset = source_offset,
            .source_length = source_length,
            .command_offset = command_offset,
            .first_number = first_number,
            .number_count = number_count,
            .first_edge = document->edge_count,
            .edge_count = 0U,
            .shape_index = builder->shape_index};
    return true;
}

static bool add_path_quad(builder *b, geom_point p0, geom_point control,
                          geom_point p2)
{
    geom_point c1 = {p0.x + (control.x - p0.x) * (2.0 / 3.0),
                     p0.y + (control.y - p0.y) * (2.0 / 3.0)};
    geom_point c2 = {p2.x + (control.x - p2.x) * (2.0 / 3.0),
                     p2.y + (control.y - p2.y) * (2.0 / 3.0)};
    return svg_shape_builder_add_curve(b, p0, c1, c2, p2);
}

bool svg_shape_path_decode(builder *b, base_bytes src, svg_xml_span span)
{
    size_t shape_subpath_start = b->doc->path_subpath_count;
    size_t current_subpath_edge = b->doc->edge_count;
    number_reader r={src.ptr+span.offset,span.length,0,span.offset};
    for(size_t i=0;i<span.length;i++)if(r.p[i]==','){
        size_t left=i,right=i+1U;
        while(left>0U&&(r.p[left-1U]==' '||r.p[left-1U]=='\t'||r.p[left-1U]=='\r'||r.p[left-1U]=='\n'))left--;
        while(right<span.length&&(r.p[right]==' '||r.p[right]=='\t'||r.p[right]=='\r'||r.p[right]=='\n'))right++;
        if(left==0U||right==span.length||r.p[left-1U]==','||r.p[right]==','||
           !((r.p[left-1U]>='0'&&r.p[left-1U]<='9')||r.p[left-1U]=='.')||
           !((r.p[right]>='0'&&r.p[right]<='9')||r.p[right]=='.'||r.p[right]=='+'||r.p[right]=='-')){
            svg_shape_fail(b->diag,SVG_SHAPE_ERROR_PATH,span.offset+i);return false;
        }
    }
    geom_point current={0,0}, first={0,0}, cubic_control={0,0}, quad_control={0,0};
    char cmd=0; bool open=false, path_first=true;
    bool cubic_reflect=false, quad_reflect=false;
    bool previous_number=false, previous_arc_flag=false;
    while (r.i < r.n) {
        size_t gap_start=r.i;svg_shape_skip_path_separators(&r);bool had_separator=r.i!=gap_start;if(r.i>=r.n)break;
        size_t segment_start = r.i;
        size_t command_offset = SIZE_MAX;
        if ((r.p[r.i] >= 'A'&&r.p[r.i]<='Z')||(r.p[r.i]>='a'&&r.p[r.i]<='z')) {command_offset=span.offset+r.i;cmd=(char)r.p[r.i++];previous_number=false;previous_arc_flag=false;}
        else if(cmd==0 || cmd=='Z' || cmd=='z') { svg_shape_fail(b->diag,SVG_SHAPE_ERROR_PATH,span.offset+r.i); return false; }
        else {
            if(previous_number&&!had_separator&&!(r.p[r.i]=='+'||r.p[r.i]=='-')) {svg_shape_fail(b->diag,SVG_SHAPE_ERROR_PATH,span.offset+r.i);return false;}
            previous_number=false;previous_arc_flag=false;
        }
        if (++b->command_count > SHAPE_MAX_COMMANDS) { svg_shape_fail(b->diag,SVG_SHAPE_ERROR_LIMIT,span.offset+r.i); return false; }
        char segment_command = cmd;
        bool rel=cmd>='a'&&cmd<='z'; char upper=rel?(char)(cmd-'a'+'A'):cmd;
        if(path_first&&upper!='M'){svg_shape_fail(b->diag,SVG_SHAPE_ERROR_PATH,span.offset+r.i-1U);return false;}
        path_first=false;
        if(upper=='Z') {
            size_t segment_index = b->doc->path_segment_count;
            size_t closing_edge = b->doc->edge_count;
            if (open && (!svg_shape_builder_add_edge(b,current,first) ||
                         !record_path_subpath(b,current_subpath_edge,true)))
                return false;
            if (!record_path_segment(b, segment_command,
                    span.offset + segment_start, r.i - segment_start,
                    command_offset, NULL, 0U)) return false;
            if (segment_index < b->doc->path_segment_count) {
                b->doc->path_segments[segment_index].first_edge = closing_edge;
                b->doc->path_segments[segment_index].edge_count =
                    b->doc->edge_count - closing_edge;
            }
            current=first;open=false;cmd=0;cubic_reflect=false;quad_reflect=false;continue;
        }
        if(upper!='M'&&upper!='L'&&upper!='H'&&upper!='V'&&upper!='C'&&upper!='S'&&upper!='Q'&&upper!='T'&&upper!='A') { svg_shape_fail(b->diag,SVG_SHAPE_ERROR_UNSUPPORTED,span.offset+r.i-1U);return false; }
        double v[7]={0}; unsigned count=upper=='H'||upper=='V'?1U:upper=='C'?6U:(upper=='S'||upper=='Q')?4U:upper=='T'?2U:upper=='A'?7U:2U;
        size_t group_start=r.i; b->offset=span.offset+group_start;
        svg_shape_path_number source_numbers[7] = {{0}};
        for(unsigned i=0;i<count;i++){
            if(!svg_shape_path_operand_boundary(&r,previous_number,previous_arc_flag,upper,i)){svg_shape_fail(b->diag,SVG_SHAPE_ERROR_PATH,span.offset+r.i);return false;}
            size_t number_start = r.i;
            bool ok=upper=='A'&&(i==3U||i==4U)?svg_shape_read_arc_flag(&r,&v[i]):svg_shape_read_number_value(&r,&v[i]);
            if(!ok){svg_shape_fail(b->diag,SVG_SHAPE_ERROR_PATH,span.offset+group_start);return false;}
            source_numbers[i] = (svg_shape_path_number){
                span.offset + number_start, r.i - number_start, v[i]};
            previous_number=true;previous_arc_flag=upper=='A'&&(i==3U||i==4U);
        }
        size_t segment_index = b->doc->path_segment_count;
        size_t segment_first_edge = b->doc->edge_count;
        if (!record_path_segment(b, segment_command,
                span.offset + segment_start, r.i - segment_start,
                command_offset, source_numbers, count)) return false;
        geom_point p=current; bool drawn=false;
        if(upper=='H')p.x=rel?current.x+v[0]:v[0];
        else if(upper=='V')p.y=rel?current.y+v[0]:v[0];
        else if(upper=='C'||upper=='S'||upper=='Q'||upper=='T'||upper=='A') {
            if(upper=='C') { geom_point c1={v[0],v[1]},c2={v[2],v[3]};p=(geom_point){v[4],v[5]}; if(rel){c1.x+=current.x;c1.y+=current.y;c2.x+=current.x;c2.y+=current.y;p.x+=current.x;p.y+=current.y;} if(!svg_shape_point_bounded(c1)||!svg_shape_point_bounded(c2)||!svg_shape_point_bounded(p)){svg_shape_fail(b->diag,SVG_SHAPE_ERROR_NUMBER,span.offset+group_start);return false;} if(!svg_shape_builder_add_curve(b,current,c1,c2,p))return false;drawn=true; }
            else if(upper=='S') { geom_point c1=cubic_reflect?(geom_point){2.0*current.x-cubic_control.x,2.0*current.y-cubic_control.y}:current; geom_point c2={v[0],v[1]};p=(geom_point){v[2],v[3]};if(rel){c2.x+=current.x;c2.y+=current.y;p.x+=current.x;p.y+=current.y;}if(!svg_shape_point_bounded(c1)||!svg_shape_point_bounded(c2)||!svg_shape_point_bounded(p)){svg_shape_fail(b->diag,SVG_SHAPE_ERROR_NUMBER,span.offset+group_start);return false;}if(!svg_shape_builder_add_curve(b,current,c1,c2,p))return false;cubic_control=c2;cubic_reflect=true;quad_reflect=false;drawn=true; }
            else if(upper=='Q') {geom_point c={v[0],v[1]};p=(geom_point){v[2],v[3]};if(rel){c.x+=current.x;c.y+=current.y;p.x+=current.x;p.y+=current.y;}if(!svg_shape_point_bounded(c)||!svg_shape_point_bounded(p)){svg_shape_fail(b->diag,SVG_SHAPE_ERROR_NUMBER,span.offset+group_start);return false;}if(!add_path_quad(b,current,c,p))return false;quad_control=c;quad_reflect=true;cubic_reflect=false;drawn=true;}
            else if(upper=='T') {geom_point c=quad_reflect?(geom_point){2.0*current.x-quad_control.x,2.0*current.y-quad_control.y}:current;p=(geom_point){v[0],v[1]};if(rel){p.x+=current.x;p.y+=current.y;}if(!svg_shape_point_bounded(c)||!svg_shape_point_bounded(p)){svg_shape_fail(b->diag,SVG_SHAPE_ERROR_NUMBER,span.offset+group_start);return false;}if(!add_path_quad(b,current,c,p))return false;quad_control=c;quad_reflect=true;cubic_reflect=false;drawn=true;}
            else { if((v[3]!=0.0&&v[3]!=1.0)||(v[4]!=0.0&&v[4]!=1.0)||v[0]<0.0||v[1]<0.0){svg_shape_fail(b->diag,SVG_SHAPE_ERROR_PATH,span.offset+group_start);return false;} p=(geom_point){v[5],v[6]};if(rel){p.x+=current.x;p.y+=current.y;}if(!svg_shape_point_bounded(p)){svg_shape_fail(b->diag,SVG_SHAPE_ERROR_NUMBER,span.offset+group_start);return false;} if(!add_arc(b,current,v[0],v[1],v[2],v[3]==1.0,v[4]==1.0,p,span.offset+group_start))return false;drawn=true; }
            if(upper=='C'){cubic_control=(geom_point){v[2],v[3]};if(rel){cubic_control.x+=current.x;cubic_control.y+=current.y;}cubic_reflect=true;quad_reflect=false;}
            if(upper=='A'){cubic_reflect=false;quad_reflect=false;}
        } else {p=(geom_point){v[0],v[1]};if(rel){p.x+=current.x;p.y+=current.y;} if(upper=='M'){if(!svg_shape_point_bounded(p)){svg_shape_fail(b->diag,SVG_SHAPE_ERROR_NUMBER,span.offset+group_start);return false;}if(open&&(!svg_shape_builder_add_edge(b,current,first)||!record_path_subpath(b,current_subpath_edge,false)))return false;current=p;first=p;current_subpath_edge=b->doc->edge_count;open=true;cmd=rel?'l':'L';cubic_reflect=false;quad_reflect=false;continue;}}
        if(upper=='M'||upper=='L'||upper=='H'||upper=='V'){cubic_reflect=false;quad_reflect=false;}
        if(!svg_shape_point_bounded(p)){svg_shape_fail(b->diag,SVG_SHAPE_ERROR_NUMBER,span.offset+group_start);return false;}
        if(!drawn&&!svg_shape_builder_add_edge(b,current,p))return false;
        if (segment_index < b->doc->path_segment_count) {
            b->doc->path_segments[segment_index].first_edge = segment_first_edge;
            b->doc->path_segments[segment_index].edge_count =
                b->doc->edge_count - segment_first_edge;
        }
        current=p; open=true;
    }
    if(open && (!svg_shape_builder_add_edge(b,current,first) ||
                !record_path_subpath(b,current_subpath_edge,false))) return false;
    b->doc->shape_subpath_start[b->shape_index] = shape_subpath_start;
    b->doc->shape_subpath_count[b->shape_index] =
        b->doc->path_subpath_count - shape_subpath_start;
    return true;
}
