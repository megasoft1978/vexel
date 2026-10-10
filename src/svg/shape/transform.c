#include "shape_internal.h"

#include <math.h>
#include <string.h>

bool svg_shape_affine_inverse(geom_affine matrix, geom_affine *inverse)
{
    double determinant = matrix.a * matrix.d - matrix.b * matrix.c;
    if (inverse == NULL || !geom_affine_valid(matrix) || !isfinite(determinant) ||
        fabs(determinant) < 1e-15) return false;
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

typedef svg_shape_number_reader number_reader;

static void fail(svg_shape_diagnostic *diagnostic, svg_shape_error category,
                 size_t offset)
{
    if (diagnostic != NULL)
        *diagnostic = (svg_shape_diagnostic){category, offset};
}

static bool span_eq(base_bytes source, svg_xml_span span, const char *text)
{
    size_t length = strlen(text);
    return span.offset <= source.len && span.length == length &&
           length <= source.len - span.offset &&
           memcmp(source.ptr + span.offset, text, length) == 0;
}

static bool attr(base_bytes source, svg_xml_token *tokens, size_t count,
                 size_t element, const char *name, svg_xml_span *value)
{
    for (size_t i = element + 1U;
         i < count && tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE; ++i) {
        if (tokens[i].prefix.length == 0U &&
            span_eq(source, tokens[i].local_name, name)) {
            *value = tokens[i].value;
            return true;
        }
    }
    return false;
}

static bool transform_args(number_reader *reader, double *values, size_t count)
{
    for (size_t i = 0U; i < count; ++i) {
        if (i != 0U && !svg_shape_next_number_separator(reader)) return false;
        if (!svg_shape_read_number_value(reader, &values[i])) return false;
    }
    return true;
}

static bool transform_parse(base_bytes source, svg_xml_span span,
                            geom_affine *out, size_t *operation_count)
{
    if (span.offset > source.len || span.length > source.len - span.offset)
        return false;
    number_reader reader = {source.ptr + span.offset, span.length, 0U,
                            span.offset};
    geom_affine result = geom_affine_identity();
    size_t operations = 0U;
    while (true) {
        svg_shape_skip_number_space(&reader);
        if (reader.i == reader.n) break;
        size_t name = reader.i;
        while (reader.i < reader.n && source.ptr[span.offset + reader.i] >= 'a' &&
               source.ptr[span.offset + reader.i] <= 'z') reader.i++;
        if (reader.i == name || reader.i >= reader.n ||
            source.ptr[span.offset + reader.i++] != '(') return false;
        size_t name_length = reader.i - name - 1U;
        double values[6] = {0.0};
        size_t value_count = 0U;
        geom_affine operation = geom_affine_identity();
        if (name_length == 6U &&
            memcmp(source.ptr + span.offset + name, "matrix", 6U) == 0)
            value_count = 6U;
        else if (name_length == 9U &&
                 memcmp(source.ptr + span.offset + name, "translate", 9U) == 0)
            value_count = 2U;
        else if (name_length == 5U &&
                 memcmp(source.ptr + span.offset + name, "scale", 5U) == 0)
            value_count = 2U;
        else if (name_length == 6U &&
                 memcmp(source.ptr + span.offset + name, "rotate", 6U) == 0)
            value_count = 3U;
        else return false;

        svg_shape_skip_number_space(&reader);
        if (value_count == 2U && (name_length == 9U || name_length == 5U)) {
            if (!svg_shape_read_number_value(&reader, &values[0])) return false;
            size_t before = reader.i;
            svg_shape_skip_number_space(&reader);
            if (reader.i < reader.n &&
                source.ptr[span.offset + reader.i] != ')') {
                reader.i = before;
                if (!svg_shape_next_number_separator(&reader) ||
                    !svg_shape_read_number_value(&reader, &values[1])) return false;
            } else {
                values[1] = name_length == 5U ? values[0] : 0.0;
            }
        } else if (value_count == 3U) {
            if (!svg_shape_read_number_value(&reader, &values[0])) return false;
            size_t before = reader.i;
            svg_shape_skip_number_space(&reader);
            if (reader.i < reader.n &&
                source.ptr[span.offset + reader.i] != ')') {
                reader.i = before;
                if (!svg_shape_next_number_separator(&reader) ||
                    !svg_shape_read_number_value(&reader, &values[1]) ||
                    !svg_shape_next_number_separator(&reader) ||
                    !svg_shape_read_number_value(&reader, &values[2])) return false;
            } else {
                values[1] = 0.0;
                values[2] = 0.0;
            }
        } else if (!transform_args(&reader, values, value_count)) {
            return false;
        }
        svg_shape_skip_number_space(&reader);
        if (reader.i >= reader.n ||
            source.ptr[span.offset + reader.i++] != ')') return false;

        if (name_length == 6U && value_count == 6U) {
            operation = (geom_affine){values[0], values[1], values[2],
                                      values[3], values[4], values[5]};
        } else if (name_length == 9U) {
            operation.e = values[0];
            operation.f = values[1];
        } else if (name_length == 5U) {
            operation.a = values[0];
            operation.d = values[1];
        } else {
            double angle = values[0] * 0.017453292519943295;
            double cosine = cos(angle), sine = sin(angle);
            operation = (geom_affine){cosine, sine, -sine, cosine, 0.0, 0.0};
            if (value_count == 3U) {
                operation.e = values[1] - cosine * values[1] +
                              sine * values[2];
                operation.f = values[2] - sine * values[1] -
                              cosine * values[2];
            }
        }
        if (++operations > 64U || !geom_affine_valid(operation) ||
            !geom_affine_compose(result, operation, &result)) return false;
        size_t after_close = reader.i;
        svg_shape_skip_number_space(&reader);
        bool had_space = reader.i != after_close;
        if (reader.i < reader.n && source.ptr[span.offset + reader.i] == ',') {
            reader.i++;
            svg_shape_skip_number_space(&reader);
            if (reader.i == reader.n) return false;
        } else if (reader.i < reader.n && !had_space) {
            return false;
        }
    }
    *out = result;
    *operation_count = operations;
    return true;
}

bool svg_shape_transform_node(base_bytes source, svg_xml_token *tokens,
                              size_t count, size_t element,
                              geom_affine parent, geom_affine *out,
                              size_t *operations,
                              svg_shape_diagnostic *diagnostic)
{
    if (tokens == NULL || out == NULL || operations == NULL || element >= count) {
        fail(diagnostic, SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U);
        return false;
    }
    svg_xml_span span;
    geom_affine local = geom_affine_identity();
    *operations = 0U;
    if (attr(source, tokens, count, element, "transform", &span) &&
        !transform_parse(source, span, &local, operations)) {
        fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, span.offset);
        return false;
    }
    if (!geom_affine_compose(parent, local, out)) {
        fail(diagnostic, SVG_SHAPE_ERROR_NUMBER, tokens[element].name.offset);
        return false;
    }
    return true;
}
