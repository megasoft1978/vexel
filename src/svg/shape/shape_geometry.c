#include "shape_internal.h"
#include "../clip/clip.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

#define SHAPE_MAX_COORD 1000000.0
typedef svg_shape_builder builder;
typedef svg_shape_number_reader number_reader;

static bool add_edge(builder *b, geom_point a, geom_point z)
{
    geom_point mapped_a, mapped_z;
    if (!geom_affine_map_point(b->transform, a, &mapped_a) ||
        !geom_affine_map_point(b->transform, z, &mapped_z)) {
        svg_shape_fail(b->diag, SVG_SHAPE_ERROR_NUMBER, b->offset);
        return false;
    }
    a = mapped_a;
    z = mapped_z;
    if (b->doc->edge_count >= SVG_SHAPE_MAX_EDGES) { svg_shape_fail(b->diag, SVG_SHAPE_ERROR_LIMIT, b->offset); return false; }
    if (b->doc->edge_count == b->edge_capacity) {
        size_t next = b->edge_capacity == 0 ? 64U : b->edge_capacity * 2U;
        if (next > SVG_SHAPE_MAX_EDGES) next = SVG_SHAPE_MAX_EDGES;
        if (next > SIZE_MAX / sizeof(*b->doc->edges)) { svg_shape_fail(b->diag, SVG_SHAPE_ERROR_LIMIT, b->offset); return false; }
        geom_edge *edges = svg_shape_realloc(b->doc->edges, next * sizeof(*edges));
        if (edges == NULL) { svg_shape_fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset); return false; }
        b->doc->edges = edges;
        uint8_t *colors = svg_shape_realloc(b->doc->colors, next * 4U);
        if (colors == NULL) { svg_shape_fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset); return false; }
        b->doc->colors = colors;
        if (next > SIZE_MAX / sizeof(*b->doc->shape_ids)) { svg_shape_fail(b->diag, SVG_SHAPE_ERROR_LIMIT, b->offset); return false; }
        uint32_t *shape_ids = svg_shape_realloc(b->doc->shape_ids, next * sizeof(*shape_ids));
        if (shape_ids == NULL) { svg_shape_fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset); return false; }
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

bool svg_shape_builder_add_edge(svg_shape_builder *builder, geom_point from,
                                geom_point to)
{
    return add_edge(builder, from, to);
}

bool svg_shape_builder_begin_shape(builder *b, bool evenodd)
{
    if (b->doc->shape_count >= SVG_SHAPE_MAX_SHAPES ||
        b->doc->shape_count > UINT32_MAX) {
        svg_shape_fail(b->diag, SVG_SHAPE_ERROR_LIMIT, b->offset);
        return false;
    }
    if (b->doc->shape_count == b->shape_capacity) {
        size_t next = b->shape_capacity == 0U ? 32U : b->shape_capacity * 2U;
        if (next > SVG_SHAPE_MAX_SHAPES) next = SVG_SHAPE_MAX_SHAPES;
        if (next > SIZE_MAX / sizeof(*b->doc->shape_evenodd)) {
            svg_shape_fail(b->diag, SVG_SHAPE_ERROR_LIMIT, b->offset);
            return false;
        }
        bool *rules = svg_shape_realloc(b->doc->shape_evenodd, next * sizeof(*rules));
        if (rules == NULL) {
            svg_shape_fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset);
            return false;
        }
        b->doc->shape_evenodd = rules;
        bool *filled = svg_shape_realloc(b->doc->shape_filled, next * sizeof(*filled));
        if (filled == NULL) {
            svg_shape_fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset);
            return false;
        }
        b->doc->shape_filled = filled;
        bool *closed = svg_shape_realloc(b->doc->shape_stroke_closed, next * sizeof(*closed));
        if (closed == NULL) { svg_shape_fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset); return false; }
        b->doc->shape_stroke_closed = closed;
        size_t *starts = svg_shape_realloc(b->doc->shape_edge_start, next * sizeof(*starts));
        if (starts == NULL) { svg_shape_fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset); return false; }
        b->doc->shape_edge_start = starts;
        size_t *counts = svg_shape_realloc(b->doc->shape_edge_count, next * sizeof(*counts));
        if (counts == NULL) { svg_shape_fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset); return false; }
        b->doc->shape_edge_count = counts;
        size_t *subpath_starts = svg_shape_realloc(
            b->doc->shape_subpath_start, next * sizeof(*subpath_starts));
        if (subpath_starts == NULL) { svg_shape_fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset); return false; }
        b->doc->shape_subpath_start = subpath_starts;
        size_t *subpath_counts = svg_shape_realloc(
            b->doc->shape_subpath_count, next * sizeof(*subpath_counts));
        if (subpath_counts == NULL) { svg_shape_fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset); return false; }
        b->doc->shape_subpath_count = subpath_counts;
        svg_shape_stroke *strokes = svg_shape_realloc(b->doc->strokes, next * sizeof(*strokes));
        if (strokes == NULL) { svg_shape_fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset); return false; }
        b->doc->strokes = strokes;
        svg_shape_paint *paints = svg_shape_realloc(b->doc->paints, next * sizeof(*paints));
        if (paints == NULL) { svg_shape_fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset); return false; }
        b->doc->paints = paints;
        if (b->doc->stroke_paints != NULL) {
            size_t old_capacity = b->doc->stroke_paint_capacity;
            svg_shape_paint *stroke_paints = svg_shape_realloc(
                b->doc->stroke_paints, next * sizeof(*stroke_paints));
            if (stroke_paints == NULL) {
                svg_shape_fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset);
                return false;
            }
            b->doc->stroke_paints = stroke_paints;
            memset(stroke_paints + old_capacity, 0,
                   (next - old_capacity) * sizeof(*stroke_paints));
            b->doc->stroke_paint_capacity = next;
        }
        svg_shape_clip **clips = svg_shape_realloc(b->doc->clips, next * sizeof(*clips));
        if (clips == NULL) { svg_shape_fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset); return false; }
        b->doc->clips = clips;
        geom_affine *inverses = svg_shape_realloc(b->doc->shape_stroke_inverse, next * sizeof(*inverses));
        if (inverses == NULL) { svg_shape_fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset); return false; }
        b->doc->shape_stroke_inverse = inverses;
        if (next > SIZE_MAX / sizeof(*b->doc->shape_sources) ||
            next > SIZE_MAX / sizeof(*b->doc->images)) {
            svg_shape_fail(b->diag, SVG_SHAPE_ERROR_LIMIT, b->offset);
            return false;
        }
        svg_shape_source_map *sources = svg_shape_realloc(
            b->doc->shape_sources, next * sizeof(*sources));
        if (sources == NULL) {
            svg_shape_fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset);
            return false;
        }
        b->doc->shape_sources = sources;
        svg_shape_image *images = svg_shape_realloc(
            b->doc->images, next * sizeof(*images));
        if (images == NULL) {
            svg_shape_fail(b->diag, SVG_SHAPE_ERROR_OOM, b->offset);
            return false;
        }
        b->doc->images = images;
        b->shape_capacity = next;
    }
    b->shape_index = b->doc->shape_count++;
    b->doc->shape_evenodd[b->shape_index] = evenodd;
    b->doc->shape_filled[b->shape_index] = true;
    b->doc->shape_stroke_closed[b->shape_index] = true;
    b->doc->shape_edge_start[b->shape_index] = b->doc->edge_count;
    b->doc->shape_edge_count[b->shape_index] = 0U;
    b->doc->shape_subpath_start[b->shape_index] = b->doc->path_subpath_count;
    b->doc->shape_subpath_count[b->shape_index] = 0U;
    b->doc->shape_stroke_inverse[b->shape_index] = geom_affine_identity();
    (void)svg_shape_affine_inverse(b->transform, &b->doc->shape_stroke_inverse[b->shape_index]);
    b->doc->shape_sources[b->shape_index] = (svg_shape_source_map){
        .transform = b->transform,
        .transform_node = SVG_SHAPE_NO_TRANSFORM_NODE
    };
    b->doc->images[b->shape_index] = (svg_shape_image){0};
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
    if(fmax(point_segment_distance(p1,p0,p3),point_segment_distance(p2,p0,p3))<=svg_shape_transformed_flatness_tolerance(b))return add_edge(b,p0,p3);
    if(depth>=16U){svg_shape_fail(b->diag,SVG_SHAPE_ERROR_LIMIT,b->offset);return false;}
    geom_point a=midpoint(p0,p1),c=midpoint(p1,p2),d=midpoint(p2,p3);
    geom_point e=midpoint(a,c),f=midpoint(c,d),m=midpoint(e,f);
    return add_curve_recursive(b,p0,a,e,m,depth+1U)&&add_curve_recursive(b,m,f,d,p3,depth+1U);
}
static bool add_curve(builder *b,geom_point p0,geom_point p1,geom_point p2,geom_point p3)
{
    return add_curve_recursive(b,p0,p1,p2,p3,0U);
}

bool svg_shape_builder_add_curve(svg_shape_builder *builder, geom_point from,
                                 geom_point control1, geom_point control2,
                                 geom_point to)
{
    return add_curve(builder, from, control1, control2, to);
}
static bool attr(base_bytes src, svg_xml_token *tokens, size_t count,
                 size_t element, const char *name, svg_xml_span *value);
static bool attr(base_bytes src, svg_xml_token *tokens,size_t count,size_t element,const char *name,svg_xml_span *value)
{
    for(size_t i=element+1U;i<count&&tokens[i].kind==SVG_XML_TOKEN_ATTRIBUTE;i++)if(tokens[i].prefix.length==0U&&svg_shape_span_eq(src,tokens[i].local_name,name)){*value=tokens[i].value;return true;}
    return false;
}
static bool number_attr(base_bytes src, svg_xml_token *tokens, size_t count,
                        size_t element, const char *name,
                        double percentage_reference, double *value)
{
    svg_xml_span span;
    if (!attr(src, tokens, count, element, name, &span)) return true;
    return svg_shape_read_length(src, span, percentage_reference, value);
}
static bool ellipse_edges(builder *b, double cx, double cy, double rx, double ry)
{
    if (rx == 0.0 || ry == 0.0) return true;
    /* The ellipse is a diagonal affine image of the unit circle. Its maximum
       axis scales the circle's chord sagitta conservatively; the active matrix
       is already bounded by transformed_flatness_tolerance. */
    double radius=fmax(rx,ry);
    double tolerance=svg_shape_transformed_flatness_tolerance(b);
    double angle=radius<=tolerance?6.283185307179586:
        2.0*acos(fmax(-1.0,1.0-tolerance/radius));
    double raw=ceil(6.283185307179586/angle);
    if (!isfinite(raw) || raw > 4096.0) {
        svg_shape_fail(b->diag,SVG_SHAPE_ERROR_LIMIT,b->offset);
        return false;
    }
    unsigned steps=(unsigned)raw;
    if (steps<12U) steps=12U;
    geom_point previous={cx+rx,cy};
    for (unsigned i=1U;i<=steps;i++) {
        double theta=6.283185307179586*(double)i/(double)steps;
        geom_point next=i==steps?(geom_point){cx+rx,cy}:
            (geom_point){cx+rx*cos(theta),cy+ry*sin(theta)};
        if (!svg_shape_point_bounded(next)) {
            svg_shape_fail(b->diag, SVG_SHAPE_ERROR_NUMBER, b->offset);
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
        svg_shape_fail(builder == NULL ? NULL : builder->diag,
             SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U);
        return false;
    }
    svg_xml_span visibility;
    if (svg_css_paint_value(style, SVG_CSS_VISIBILITY, &visibility)) {
        if (svg_css_keyword_equals(source, visibility, "hidden") ||
            svg_css_keyword_equals(source, visibility, "collapse")) return true;
        if (!svg_css_keyword_equals(source, visibility, "visible")) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED,
                           visibility.offset);
            return false;
        }
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
    if (has_fill && !gradient_fill &&
        !svg_shape_parse_style_color(source, fill, style, color)) {
        svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED, fill.offset);
        return false;
    }
    svg_xml_span rule = {0};
    if (svg_css_paint_value(style, SVG_CSS_FILL_RULE, &rule) &&
        !svg_css_keyword_equals(source, rule, "evenodd") &&
        !svg_css_keyword_equals(source, rule, "nonzero")) {
        svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED, rule.offset);
        return false;
    }
    bool evenodd = svg_css_paint_value(style, SVG_CSS_FILL_RULE, &rule) &&
                   svg_css_keyword_equals(source, rule, "evenodd");
    if (!svg_shape_builder_begin_shape(builder, evenodd)) return false;
    size_t before = out->edge_count;
    svg_xml_span fill_opacity_span;
    if (svg_css_paint_value(style, SVG_CSS_FILL_OPACITY, &fill_opacity_span)) {
        double opacity;
        if (!svg_shape_parse_opacity(source, fill_opacity_span, &opacity)) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_NUMBER, fill_opacity_span.offset);
            return false;
        }
        svg_shape_multiply_alpha(color, opacity);
    }
    geom_affine user_inverse = geom_affine_identity();
    bool invertible = svg_shape_affine_inverse(transform, &user_inverse);
    if (invertible) out->shape_stroke_inverse[builder->shape_index] = user_inverse;
    double normalized_diagonal = hypot(builder->viewport_width,
                                       builder->viewport_height) / sqrt(2.0);
    if (!svg_shape_parse_stroke(source,
            &out->strokes[builder->shape_index], style,
            normalized_diagonal, builder->diag)) return false;
    if (!svg_shape_apply_vector_effect(
            source, style, transform, out,
            &out->strokes[builder->shape_index], builder->diag,
            tokens[element].name.offset)) return false;
    if (out->strokes[builder->shape_index].rgba[3] != 0U && !invertible) {
        svg_xml_span stroke_span = {0};
        (void)svg_css_paint_value(style, SVG_CSS_STROKE, &stroke_span);
        svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED, stroke_span.offset);
        return false;
    }
    if (svg_shape_span_eq(source, tokens[element].local_name, "rect")) {
        double q[4] = {0.0, 0.0, 0.0, 0.0};
        const char *names[4] = {"x", "y", "width", "height"};
        bool has_width = false, has_height = false;
        for (size_t k = 0U; k < 4U; ++k) {
            svg_xml_span value;
            if (!attr(source, tokens, token_count, element, names[k], &value))
                continue;
            if (k == 2U) has_width = true;
            if (k == 3U) has_height = true;
            double reference = (k == 0U || k == 2U) ?
                builder->viewport_width : builder->viewport_height;
            if (!svg_shape_read_length(source, value, reference, &q[k])) {
                svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_NUMBER, value.offset);
                return false;
            }
        }
        if (!has_width || !has_height) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED,
                 tokens[element].name.offset);
            return false;
        }
        if (q[2] < 0.0 || q[3] < 0.0) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_NUMBER,
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
                    svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_NUMBER,
                         tokens[element].name.offset);
                    return false;
                }
            }
            double rx = 0.0, ry = 0.0;
            svg_xml_span rx_span = {0}, ry_span = {0};
            bool has_rx = attr(source, tokens, token_count, element, "rx", &rx_span);
            bool has_ry = attr(source, tokens, token_count, element, "ry", &ry_span);
            if ((has_rx && !svg_shape_read_length(source, rx_span,
                    builder->viewport_width, &rx)) ||
                (has_ry && !svg_shape_read_length(source, ry_span,
                    builder->viewport_height, &ry)) ||
                rx < 0.0 || ry < 0.0) {
                svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_NUMBER,
                     has_rx ? rx_span.offset : has_ry ? ry_span.offset :
                     tokens[element].name.offset);
                return false;
            }
            if (has_rx && !has_ry) ry = rx;
            else if (!has_rx && has_ry) rx = ry;
            if (rx > q[2] * 0.5) rx = q[2] * 0.5;
            if (ry > q[3] * 0.5) ry = q[3] * 0.5;
            if (rx == 0.0 || ry == 0.0) {
                for (size_t k = 0U; k < 4U; ++k)
                    if (!add_edge(builder, points[k], points[(k + 1U) % 4U]))
                        return false;
            } else {
                const double kappa = 0.5522847498307936;
                geom_point start = {q[0] + rx, q[1]};
                geom_point top_right = {q[0] + q[2] - rx, q[1]};
                geom_point right_top = {q[0] + q[2], q[1] + ry};
                geom_point right_bottom = {q[0] + q[2], q[1] + q[3] - ry};
                geom_point bottom_right = {q[0] + q[2] - rx, q[1] + q[3]};
                geom_point bottom_left = {q[0] + rx, q[1] + q[3]};
                geom_point left_bottom = {q[0], q[1] + q[3] - ry};
                geom_point left_top = {q[0], q[1] + ry};
                if (!add_edge(builder, start, top_right) ||
                    !add_curve(builder, top_right,
                        (geom_point){top_right.x + kappa * rx, top_right.y},
                        (geom_point){right_top.x, right_top.y - kappa * ry}, right_top) ||
                    !add_edge(builder, right_top, right_bottom) ||
                    !add_curve(builder, right_bottom,
                        (geom_point){right_bottom.x, right_bottom.y + kappa * ry},
                        (geom_point){bottom_right.x + kappa * rx, bottom_right.y}, bottom_right) ||
                    !add_edge(builder, bottom_right, bottom_left) ||
                    !add_curve(builder, bottom_left,
                        (geom_point){bottom_left.x - kappa * rx, bottom_left.y},
                        (geom_point){left_bottom.x, left_bottom.y + kappa * ry}, left_bottom) ||
                    !add_edge(builder, left_bottom, left_top) ||
                    !add_curve(builder, left_top,
                        (geom_point){left_top.x, left_top.y - kappa * ry},
                        (geom_point){start.x - kappa * rx, start.y}, start))
                    return false;
            }
        }
    } else if (svg_shape_span_eq(source, tokens[element].local_name, "path")) {
        svg_xml_span data;
        if (!attr(source, tokens, token_count, element, "d", &data)) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_PATH,
                 tokens[element].name.offset);
            return false;
        }
        if (!svg_shape_path_decode(builder, source, data)) return false;
    } else if (svg_shape_span_eq(source, tokens[element].local_name, "circle") ||
               svg_shape_span_eq(source, tokens[element].local_name, "ellipse")) {
        double cx = 0.0, cy = 0.0, rx = 0.0, ry = 0.0;
        if (!number_attr(source, tokens, token_count, element, "cx",
                         builder->viewport_width, &cx) ||
            !number_attr(source, tokens, token_count, element, "cy",
                         builder->viewport_height, &cy)) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_NUMBER,
                 tokens[element].name.offset);
            return false;
        }
        if (svg_shape_span_eq(source, tokens[element].local_name, "circle")) {
            if (!number_attr(source, tokens, token_count, element, "r",
                    normalized_diagonal, &rx)) {
                svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_NUMBER,
                     tokens[element].name.offset);
                return false;
            }
            ry = rx;
        } else if (!number_attr(source, tokens, token_count, element, "rx",
                           builder->viewport_width, &rx) ||
                   !number_attr(source, tokens, token_count, element, "ry",
                           builder->viewport_height, &ry)) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_NUMBER,
                 tokens[element].name.offset);
            return false;
        }
        if (rx < 0.0 || ry < 0.0 || !svg_shape_point_bounded((geom_point){cx, cy}) ||
            rx > SHAPE_MAX_COORD || ry > SHAPE_MAX_COORD) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_NUMBER,
                 tokens[element].name.offset);
            return false;
        }
        if (!ellipse_edges(builder, cx, cy, rx, ry)) return false;
    } else if (svg_shape_span_eq(source, tokens[element].local_name, "line")) {
        double xy[4] = {0.0, 0.0, 0.0, 0.0};
        const char *names[4] = {"x1", "y1", "x2", "y2"};
        for (size_t k = 0U; k < 4U; ++k) {
            double reference = (k == 0U || k == 2U) ?
                builder->viewport_width : builder->viewport_height;
            if (!number_attr(source, tokens, token_count, element, names[k],
                             reference, &xy[k])) {
                svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_NUMBER,
                     tokens[element].name.offset);
                return false;
            }
        }
        if (!svg_shape_point_bounded((geom_point){xy[0], xy[1]}) ||
            !svg_shape_point_bounded((geom_point){xy[2], xy[3]})) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_NUMBER,
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
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_PATH,
                 tokens[element].name.offset);
            return false;
        }
        number_reader reader = {source.ptr + points.offset, points.length, 0U,
                                points.offset};
        geom_point first = {0.0, 0.0}, previous = {0.0, 0.0};
        size_t point_count = 0U;
        while (true) {
            size_t before_separator = reader.i;
            svg_shape_skip_number_space(&reader);
            bool had_space = reader.i != before_separator;
            if (point_count != 0U) {
                if (reader.i < reader.n && reader.p[reader.i] == ',') {
                    reader.i++;
                    svg_shape_skip_number_space(&reader);
                    if (reader.i == reader.n || reader.p[reader.i] == ',') {
                        svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_PATH,
                             points.offset + reader.i);
                        return false;
                    }
                } else if (reader.i < reader.n && !had_space) {
                    svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_PATH,
                         points.offset + reader.i);
                    return false;
                }
            }
            if (reader.i == reader.n) break;
            double x, y;
            if (!svg_shape_read_length_value(&reader, builder->viewport_width, &x) ||
                !svg_shape_next_number_separator(&reader) ||
                !svg_shape_read_length_value(&reader, builder->viewport_height, &y)) {
                svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_PATH,
                     points.offset + reader.i);
                return false;
            }
            geom_point point = {x, y};
            if (!svg_shape_point_bounded(point)) {
                svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_NUMBER,
                     points.offset + reader.i);
                return false;
            }
            if (builder->point_count >= SVG_SHAPE_MAX_COMMANDS) {
                svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_LIMIT,
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
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_PATH, points.offset);
            return false;
        }
        if (!add_edge(builder, previous, first)) return false;
        if (svg_shape_span_eq(source, tokens[element].local_name, "polyline"))
            out->shape_stroke_closed[builder->shape_index] = false;
    }
    for (size_t k = before; k < out->edge_count; ++k)
        memcpy(out->colors + k * 4U, color, 4U);
    out->shape_edge_start[builder->shape_index] = before;
    out->shape_edge_count[builder->shape_index] = out->edge_count - before;
    if (gradient_fill) {
        if (!invertible) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED, fill.offset);
            return false;
        }
        bool pattern_resolved = false;
        if (!svg_shape_pattern_resolve(
                builder, source, tokens, token_count, fill, *style,
                builder->shape_index, transform, false,
                &pattern_resolved)) return false;
        if (!pattern_resolved) {
            svg_shape_error result = svg_paint_resolve_fill(
                source, tokens, token_count, builder->references, builder->css,
                builder->paint_cache, builder->paint_token_visits, fill,
                svg_paint_edge_span(out, before, out->edge_count - before, NULL),
                out->edge_count - before, user_inverse,
                builder->viewport_width, builder->viewport_height,
                &out->paints[builder->shape_index], &out->paint_stops,
                &out->paint_stop_count, builder->diag);
            if (result != SVG_SHAPE_ERROR_NONE) return false;
        }
    }
    svg_shape_stroke *stroke = &out->strokes[builder->shape_index];
    if (stroke->paint_server) {
        svg_xml_span stroke_value = {0};
        bool stroke_resolved = false;
        (void)svg_css_paint_value(style, SVG_CSS_STROKE, &stroke_value);
        if (!svg_shape_pattern_resolve(
                builder, source, tokens, token_count, stroke_value, *style,
                builder->shape_index, transform, true, &stroke_resolved))
            return false;
        if (!stroke_resolved) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED,
                           stroke_value.offset);
            return false;
        }
    }
    svg_xml_span clip_value;
    if (svg_css_paint_value(style, SVG_CSS_CLIP_PATH, &clip_value) &&
        !svg_css_keyword_equals(source, clip_value, "none")) {
        if (!invertible) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED, clip_value.offset);
            return false;
        }
        double local_bounds[4];
        if (!svg_clip_local_bounds(out, before, out->edge_count - before,
                                   user_inverse, local_bounds)) {
            svg_shape_fail(builder->diag, SVG_SHAPE_ERROR_UNSUPPORTED, clip_value.offset);
            return false;
        }
        svg_shape_error result = svg_clip_resolve(source, tokens, token_count,
            builder->references, builder->css,
            clip_value,
            out, &out->clip_bytes, user_inverse, local_bounds,
            builder->transform_node,
            &out->clips[builder->shape_index],
            builder->diag);
        if (result != SVG_SHAPE_ERROR_NONE) return false;
    }
    return true;
}
