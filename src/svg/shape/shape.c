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
bool svg_shape_span_eq(base_bytes s, svg_xml_span span, const char *literal)
{
    size_t n = strlen(literal);
    return span.length == n && span.offset <= s.len && n <= s.len - span.offset &&
        memcmp(s.ptr + span.offset, literal, n) == 0;
}
bool svg_shape_point_bounded(geom_point point)
{
    return geom_finite_point(point) && fabs(point.x) <= SHAPE_MAX_COORD &&
           fabs(point.y) <= SHAPE_MAX_COORD;
}
double svg_shape_transformed_flatness_tolerance(const svg_shape_builder *b)
{
    double row_norm=fmax(fabs(b->transform.a)+fabs(b->transform.c),
                         fabs(b->transform.b)+fabs(b->transform.d));
    double column_norm=fmax(fabs(b->transform.a)+fabs(b->transform.b),
                            fabs(b->transform.c)+fabs(b->transform.d));
    double scale=sqrt(row_norm*column_norm);
    if (!isfinite(scale) || scale < 1.0) scale=1.0;
    return 0.25/scale;
}
void svg_shape_fail(svg_shape_diagnostic *d, svg_shape_error e, size_t offset)
{
    if (d != NULL) *d = (svg_shape_diagnostic){e, offset};
}
static void skip_space(number_reader *r)
{
    while (r->i < r->n && (r->p[r->i] == ' ' || r->p[r->i] == '\t' || r->p[r->i] == '\r' || r->p[r->i] == '\n')) r->i++;
}
void svg_shape_skip_path_separators(number_reader *r)
{
    while (r->i < r->n && (r->p[r->i] == ' ' || r->p[r->i] == '\t' || r->p[r->i] == '\r' || r->p[r->i] == '\n' || r->p[r->i] == ',')) r->i++;
}
bool svg_shape_path_operand_boundary(number_reader *r, bool previous_number,
                                 bool previous_arc_flag, char command,
                                 unsigned argument)
{
    size_t before = r->i;
    svg_shape_skip_path_separators(r);
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
        size_t lookahead = r->i + 1U;
        if (lookahead < r->n && (r->p[lookahead] == '+' || r->p[lookahead] == '-'))
            lookahead++;
        if (lookahead < r->n && r->p[lookahead] >= '0' && r->p[lookahead] <= '9') {
            r->i++;
            if (r->i < r->n && (r->p[r->i] == '+' || r->p[r->i] == '-')) { exp_negative = r->p[r->i] == '-'; r->i++; }
            size_t exp_digits = 0;
            while (r->i < r->n && r->p[r->i] >= '0' && r->p[r->i] <= '9') {
                if (exponent < 10000) exponent = exponent * 10 + (int)(r->p[r->i] - '0');
                r->i++; exp_digits++;
            }
            if (exp_digits == 0) return false;
        }
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
bool svg_shape_read_arc_flag(number_reader *r, double *out)
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
    return svg_shape_read_length(source, span, 0.0, out);
}

bool svg_shape_read_length_value(svg_shape_number_reader *reader,
                                 double percentage_reference, double *out)
{
    if (reader == NULL || out == NULL || !read_number(reader, out)) return false;
    size_t suffix = reader->i;
    while (reader->i < reader->n &&
           ((reader->p[reader->i] >= 'a' && reader->p[reader->i] <= 'z') ||
            (reader->p[reader->i] >= 'A' && reader->p[reader->i] <= 'Z') ||
            reader->p[reader->i] == '%')) reader->i++;
    size_t length = reader->i - suffix;
    uint8_t unit[2] = {0U, 0U};
    if (length <= sizeof(unit)) {
        for (size_t i = 0U; i < length; ++i) {
            unit[i] = reader->p[suffix + i];
            if (unit[i] >= 'A' && unit[i] <= 'Z')
                unit[i] = (uint8_t)(unit[i] + ('a' - 'A'));
        }
    }
    if (length == 0U) return true;
    if (length == 1U && reader->p[suffix] == '%') {
        if (!isfinite(percentage_reference) || percentage_reference <= 0.0)
            return false;
        *out *= percentage_reference / 100.0;
    } else if (length == 2U && unit[0] == 'p' && unit[1] == 'x') {
        /* CSS pixels are the SVG user unit. */
    } else if (length == 2U && unit[0] == 'p' && unit[1] == 't') {
        *out *= 96.0 / 72.0;
    } else if (length == 2U && unit[0] == 'm' && unit[1] == 'm') {
        *out *= 96.0 / 25.4;
    } else if (length == 2U && unit[0] == 'c' && unit[1] == 'm') {
        *out *= 96.0 / 2.54;
    } else if (length == 2U && unit[0] == 'i' && unit[1] == 'n') {
        *out *= 96.0;
    } else if (length == 2U && unit[0] == 'e' && unit[1] == 'm') {
        *out *= 16.0;
    } else return false;
    return isfinite(*out);
}

bool svg_shape_read_length(base_bytes source, svg_xml_span span,
                           double percentage_reference, double *out)
{
    if (out == NULL || span.offset > source.len ||
        span.length > source.len - span.offset) return false;
    number_reader r={source.ptr+span.offset,span.length,0,span.offset};
    if(!svg_shape_read_length_value(&r,percentage_reference,out))return false;
    skip_space(&r);
    return r.i == r.n && isfinite(*out);
}
