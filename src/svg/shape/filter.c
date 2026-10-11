#include "shape_internal.h"

#include "../css/color.h"

#include <math.h>
#include <string.h>

#define SHAPE_FILTER_MAX_COUNT 4096U
#define SHAPE_FILTER_MAX_TOTAL_PRIMITIVES 65536U

typedef struct filter_result_name {
    svg_xml_span span;
    bool present;
} filter_result_name;

static bool span_equal(base_bytes source, svg_xml_span span,
                       const char *text)
{
    return svg_shape_span_eq(source, span, text);
}

static bool attribute(base_bytes source, const svg_xml_token *tokens,
                      size_t token_count, size_t element, const char *name,
                      svg_xml_span *value)
{
    for (size_t i = element + 1U;
         i < token_count && tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE; ++i) {
        if (tokens[i].prefix.length == 0U &&
            span_equal(source, tokens[i].local_name, name)) {
            if (value != NULL) *value = tokens[i].value;
            return true;
        }
    }
    return false;
}

static void fail(svg_shape_diagnostic *diagnostic, svg_shape_error error,
                 size_t offset)
{
    if (diagnostic != NULL)
        *diagnostic = (svg_shape_diagnostic){error, offset};
}

static bool parse_url_id(base_bytes source, svg_xml_span value,
                         svg_xml_span *id)
{
    if (value.offset > source.len || value.length > source.len - value.offset)
        return false;
    size_t first = value.offset, end = first + value.length;
    while (first < end && (source.ptr[first] == ' ' || source.ptr[first] == '\t' ||
                            source.ptr[first] == '\r' || source.ptr[first] == '\n'))
        first++;
    while (end > first && (source.ptr[end - 1U] == ' ' ||
                           source.ptr[end - 1U] == '\t' ||
                           source.ptr[end - 1U] == '\r' ||
                           source.ptr[end - 1U] == '\n')) end--;
    if (end - first < 7U || memcmp(source.ptr + first, "url(", 4U) != 0 ||
        source.ptr[end - 1U] != ')') return false;
    first += 4U;
    end--;
    while (first < end && (source.ptr[first] == ' ' || source.ptr[first] == '\t' ||
                            source.ptr[first] == '\r' || source.ptr[first] == '\n'))
        first++;
    while (end > first && (source.ptr[end - 1U] == ' ' ||
                           source.ptr[end - 1U] == '\t' ||
                           source.ptr[end - 1U] == '\r' ||
                           source.ptr[end - 1U] == '\n')) end--;
    if (end - first < 2U || source.ptr[first] != '#') return false;
    *id = (svg_xml_span){first + 1U, end - first - 1U};
    return true;
}

static bool parse_number(base_bytes source, svg_xml_span span, double fallback,
                         double *out)
{
    if (span.length == 0U) {
        *out = fallback;
        return true;
    }
    if (span.offset > source.len || span.length > source.len - span.offset)
        return false;
    svg_shape_number_reader reader = {
        source.ptr + span.offset, span.length, 0U, span.offset};
    if (!svg_shape_read_number_value(&reader, out)) return false;
    svg_shape_skip_number_space(&reader);
    return reader.i == reader.n && isfinite(*out);
}

static bool parse_region_number(base_bytes source, svg_xml_span span,
                                double fallback, double *out)
{
    if (span.length == 0U) {
        *out = fallback;
        return true;
    }
    if (span.offset > source.len || span.length > source.len - span.offset)
        return false;
    svg_xml_span number = span;
    bool percent = source.ptr[span.offset + span.length - 1U] == '%';
    if (percent) number.length--;
    if (!parse_number(source, number, fallback, out)) return false;
    if (percent) *out /= 100.0;
    return isfinite(*out);
}

static bool parse_number_list(base_bytes source, svg_xml_span span,
                              double *values, size_t maximum,
                              size_t *count)
{
    if (span.offset > source.len || span.length > source.len - span.offset)
        return false;
    svg_shape_number_reader reader = {
        source.ptr + span.offset, span.length, 0U, span.offset};
    size_t used = 0U;
    while (true) {
        svg_shape_skip_number_space(&reader);
        if (reader.i == reader.n) break;
        if (used == maximum || !svg_shape_read_number_value(&reader,
                                                             &values[used]) ||
            !isfinite(values[used])) return false;
        used++;
        size_t before = reader.i;
        svg_shape_skip_number_space(&reader);
        bool separated = reader.i != before;
        if (reader.i < reader.n && reader.p[reader.i] == ',') {
            reader.i++;
            svg_shape_skip_number_space(&reader);
            if (reader.i == reader.n || reader.p[reader.i] == ',') return false;
        } else if (reader.i < reader.n && !separated) {
            return false;
        }
    }
    *count = used;
    return true;
}

static bool resolve_input(base_bytes source, svg_xml_span value,
                          bool present, int32_t fallback,
                          const filter_result_name *results,
                          size_t result_count, int32_t *out)
{
    if (!present || value.length == 0U) {
        *out = fallback;
        return true;
    }
    if (span_equal(source, value, "SourceGraphic")) {
        *out = SVG_SHAPE_FILTER_INPUT_GRAPHIC;
        return true;
    }
    if (span_equal(source, value, "SourceAlpha")) {
        *out = SVG_SHAPE_FILTER_INPUT_ALPHA;
        return true;
    }
    for (size_t i = 0U; i < result_count; ++i) {
        if (results[i].present && results[i].span.length == value.length &&
            results[i].span.offset <= source.len && value.offset <= source.len &&
            results[i].span.length <= source.len - results[i].span.offset &&
            value.length <= source.len - value.offset &&
            memcmp(source.ptr + results[i].span.offset,
                   source.ptr + value.offset, value.length) == 0) {
            *out = (int32_t)i;
            return true;
        }
    }
    return false;
}

static svg_shape_error unsupported_primitive(base_bytes source,
                                              const svg_xml_token *token)
{
    if (span_equal(source, token->local_name, "feTurbulence"))
        return SVG_SHAPE_ERROR_UNSUPPORTED_FE_TURBULENCE;
    if (span_equal(source, token->local_name, "feDisplacementMap"))
        return SVG_SHAPE_ERROR_UNSUPPORTED_FE_DISPLACEMENT_MAP;
    if (span_equal(source, token->local_name, "feConvolveMatrix"))
        return SVG_SHAPE_ERROR_UNSUPPORTED_FE_CONVOLVE_MATRIX;
    if (span_equal(source, token->local_name, "feImage"))
        return SVG_SHAPE_ERROR_UNSUPPORTED_FE_IMAGE;
    if (span_equal(source, token->local_name, "feDiffuseLighting") ||
        span_equal(source, token->local_name, "feSpecularLighting"))
        return SVG_SHAPE_ERROR_UNSUPPORTED_FE_LIGHTING;
    if (span_equal(source, token->local_name, "feMorphology"))
        return SVG_SHAPE_ERROR_UNSUPPORTED_FE_MORPHOLOGY;
    return SVG_SHAPE_ERROR_UNSUPPORTED;
}

static bool parse_blend_mode(base_bytes source, svg_xml_span value,
                             uint8_t *mode)
{
    static const char *const names[] = {
        "normal", "multiply", "screen", "darken", "lighten"};
    for (size_t i = 0U; i < sizeof(names) / sizeof(names[0]); ++i) {
        if (span_equal(source, value, names[i])) {
            *mode = (uint8_t)i;
            return true;
        }
    }
    return false;
}

static bool parse_color(base_bytes source, const svg_xml_token *tokens,
                        size_t token_count, size_t element,
                        const char *name, uint8_t rgba[4],
                        double *opacity)
{
    svg_xml_span color = {0}, alpha = {0};
    if (attribute(source, tokens, token_count, element, name, &color) &&
        !svg_css_parse_color(source, color, rgba)) return false;
    bool has_opacity = attribute(source, tokens, token_count, element,
                                 "flood-opacity", &alpha);
    if (has_opacity && !parse_number(source, alpha, 1.0, opacity)) return false;
    if (*opacity < 0.0 || *opacity > 1.0) return false;
    rgba[3] = (uint8_t)floor((double)rgba[3] * *opacity + 0.5);
    return true;
}

static bool parse_merge_inputs(base_bytes source,
                               const svg_xml_token *tokens,
                               size_t token_count, size_t element, size_t end,
                               const filter_result_name *results,
                               size_t result_count,
                               svg_shape_filter_primitive *primitive,
                               svg_shape_diagnostic *diagnostic)
{
    size_t used = 0U;
    for (size_t i = element + 1U; i < end; ++i) {
        if ((tokens[i].kind != SVG_XML_TOKEN_START_ELEMENT &&
             tokens[i].kind != SVG_XML_TOKEN_EMPTY_ELEMENT) ||
            tokens[i].depth != tokens[element].depth + 1U) continue;
        if (!span_equal(source, tokens[i].local_name, "feMergeNode")) {
            fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, tokens[i].name.offset);
            return false;
        }
        if (used == SVG_SHAPE_MAX_FILTER_MERGE_INPUTS) {
            fail(diagnostic, SVG_SHAPE_ERROR_LIMIT, tokens[i].name.offset);
            return false;
        }
        svg_xml_span in = {0};
        bool present = attribute(source, tokens, token_count, i, "in", &in);
        int32_t resolved = SVG_SHAPE_FILTER_INPUT_GRAPHIC;
        if (!resolve_input(source, in, present,
                          SVG_SHAPE_FILTER_INPUT_GRAPHIC,
                          results, result_count, &resolved)) {
            fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, in.offset);
            return false;
        }
        primitive->merge_inputs[used++] = resolved;
    }
    if (used == 0U) {
        fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
             tokens[element].name.offset);
        return false;
    }
    primitive->merge_count = (uint8_t)used;
    return true;
}

static bool parse_primitive(base_bytes source, const svg_xml_token *tokens,
                            size_t token_count, size_t element, size_t end,
                            const filter_result_name *results,
                            size_t result_count,
                            svg_shape_filter_primitive *primitive,
                            svg_shape_error *failure,
                            svg_shape_diagnostic *diagnostic)
{
    svg_xml_span in = {0}, in2 = {0}, result = {0};
    bool has_in = attribute(source, tokens, token_count, element, "in", &in);
    bool has_in2 = attribute(source, tokens, token_count, element, "in2", &in2);
    svg_shape_filter_kind kind;
    if (span_equal(source, tokens[element].local_name, "feGaussianBlur"))
        kind = SVG_SHAPE_FILTER_GAUSSIAN_BLUR;
    else if (span_equal(source, tokens[element].local_name, "feOffset"))
        kind = SVG_SHAPE_FILTER_OFFSET;
    else if (span_equal(source, tokens[element].local_name, "feFlood"))
        kind = SVG_SHAPE_FILTER_FLOOD;
    else if (span_equal(source, tokens[element].local_name, "feBlend"))
        kind = SVG_SHAPE_FILTER_BLEND;
    else if (span_equal(source, tokens[element].local_name, "feComposite"))
        kind = SVG_SHAPE_FILTER_COMPOSITE;
    else if (span_equal(source, tokens[element].local_name, "feMerge"))
        kind = SVG_SHAPE_FILTER_MERGE;
    else if (span_equal(source, tokens[element].local_name, "feColorMatrix"))
        kind = SVG_SHAPE_FILTER_COLOR_MATRIX;
    else if (span_equal(source, tokens[element].local_name, "feDropShadow"))
        kind = SVG_SHAPE_FILTER_DROP_SHADOW;
    else {
        *failure = unsupported_primitive(source, &tokens[element]);
        fail(diagnostic, *failure, tokens[element].name.offset);
        return false;
    }
    *primitive = (svg_shape_filter_primitive){
        .kind = kind,
        .input = SVG_SHAPE_FILTER_INPUT_PREVIOUS,
        .input2 = SVG_SHAPE_FILTER_INPUT_GRAPHIC,
        .source_offset = tokens[element].name.offset};
    if (!resolve_input(source, in, has_in,
                       result_count == 0U ? SVG_SHAPE_FILTER_INPUT_GRAPHIC :
                                            SVG_SHAPE_FILTER_INPUT_PREVIOUS,
                       results, result_count, &primitive->input)) {
        *failure = SVG_SHAPE_ERROR_UNSUPPORTED;
        fail(diagnostic, *failure, in.offset);
        return false;
    }
    if (kind == SVG_SHAPE_FILTER_BLEND || kind == SVG_SHAPE_FILTER_COMPOSITE) {
        if (!resolve_input(source, in2, has_in2,
                           SVG_SHAPE_FILTER_INPUT_GRAPHIC,
                           results, result_count, &primitive->input2)) {
            *failure = SVG_SHAPE_ERROR_UNSUPPORTED;
            fail(diagnostic, *failure, in2.offset);
            return false;
        }
    }
    if (attribute(source, tokens, token_count, element, "result", &result)) {
        if (result.length == 0U || result_count >= SVG_SHAPE_MAX_FILTER_PRIMITIVES) {
            *failure = SVG_SHAPE_ERROR_LIMIT;
            fail(diagnostic, *failure, result.offset);
            return false;
        }
    }

    if (kind == SVG_SHAPE_FILTER_GAUSSIAN_BLUR ||
        kind == SVG_SHAPE_FILTER_DROP_SHADOW) {
        svg_xml_span deviation = {0};
        double values[2] = {0.0, 0.0};
        size_t count = 0U;
        if (attribute(source, tokens, token_count, element,
                      "stdDeviation", &deviation)) {
            if (!parse_number_list(source, deviation, values, 2U, &count) ||
                count == 0U || (count != 1U && count != 2U)) {
                *failure = SVG_SHAPE_ERROR_NUMBER;
                fail(diagnostic, *failure, deviation.offset);
                return false;
            }
        }
        primitive->values[0] = values[0];
        primitive->values[1] = count == 1U ? values[0] : values[1];
        if (primitive->values[0] < 0.0 || primitive->values[1] < 0.0 ||
            primitive->values[0] > 32.0 || primitive->values[1] > 32.0) {
            *failure = SVG_SHAPE_ERROR_LIMIT;
            fail(diagnostic, *failure, deviation.offset);
            return false;
        }
    }
    if (kind == SVG_SHAPE_FILTER_OFFSET ||
        kind == SVG_SHAPE_FILTER_DROP_SHADOW) {
        svg_xml_span dx = {0}, dy = {0};
        if (!parse_number(source,
                attribute(source, tokens, token_count, element, "dx", &dx) ? dx :
                (svg_xml_span){0}, 0.0, &primitive->values[2]) ||
            !parse_number(source,
                attribute(source, tokens, token_count, element, "dy", &dy) ? dy :
                (svg_xml_span){0}, 0.0, &primitive->values[3]) ||
            fabs(primitive->values[2]) > 4096.0 ||
            fabs(primitive->values[3]) > 4096.0) {
            *failure = SVG_SHAPE_ERROR_NUMBER;
            fail(diagnostic, *failure, dx.offset != 0U ? dx.offset : dy.offset);
            return false;
        }
    }
    if (kind == SVG_SHAPE_FILTER_FLOOD ||
        kind == SVG_SHAPE_FILTER_DROP_SHADOW) {
        double opacity = 1.0;
        primitive->rgba[0] = 0U;
        primitive->rgba[1] = 0U;
        primitive->rgba[2] = 0U;
        primitive->rgba[3] = 255U;
        if (!parse_color(source, tokens, token_count, element, "flood-color",
                         primitive->rgba, &opacity)) {
            *failure = SVG_SHAPE_ERROR_UNSUPPORTED;
            fail(diagnostic, *failure, tokens[element].name.offset);
            return false;
        }
    }
    if (kind == SVG_SHAPE_FILTER_BLEND) {
        svg_xml_span mode = {0};
        if (!attribute(source, tokens, token_count, element, "mode", &mode))
            mode = (svg_xml_span){0};
        if (mode.length == 0U) {
            primitive->mode = 0U;
        } else if (!parse_blend_mode(source, mode, &primitive->mode)) {
            *failure = SVG_SHAPE_ERROR_UNSUPPORTED;
            fail(diagnostic, *failure, mode.offset);
            return false;
        }
    }
    if (kind == SVG_SHAPE_FILTER_COMPOSITE) {
        static const char *const operators[] = {
            "over", "in", "out", "atop", "xor", "lighter", "arithmetic"};
        svg_xml_span op = {0};
        bool found = attribute(source, tokens, token_count, element,
                               "operator", &op);
        primitive->mode = 0U;
        if (found) {
            bool matched = false;
            for (size_t i = 0U; i < sizeof(operators) / sizeof(operators[0]); ++i) {
                if (span_equal(source, op, operators[i])) {
                    primitive->mode = (uint8_t)i;
                    matched = true;
                    break;
                }
            }
            if (!matched) {
                *failure = SVG_SHAPE_ERROR_UNSUPPORTED;
                fail(diagnostic, *failure, op.offset);
                return false;
            }
        }
        if (primitive->mode == 6U) {
            static const char *const names[] = {"k1", "k2", "k3", "k4"};
            for (size_t i = 0U; i < 4U; ++i) {
                svg_xml_span coefficient = {0};
                double fallback = 0.0;
                if (attribute(source, tokens, token_count, element,
                              names[i], &coefficient) &&
                    !parse_number(source, coefficient, fallback,
                                  &primitive->values[i])) {
                    *failure = SVG_SHAPE_ERROR_NUMBER;
                    fail(diagnostic, *failure, coefficient.offset);
                    return false;
                }
            }
        }
    }
    if (kind == SVG_SHAPE_FILTER_COLOR_MATRIX) {
        svg_xml_span type = {0}, value = {0};
        attribute(source, tokens, token_count, element, "type", &type);
        if (span_equal(source, type, "saturate")) primitive->mode = 1U;
        else if (span_equal(source, type, "hueRotate")) primitive->mode = 2U;
        else if (span_equal(source, type, "luminanceToAlpha")) primitive->mode = 3U;
        else if (type.length == 0U || span_equal(source, type, "matrix"))
            primitive->mode = 0U;
        else {
            *failure = SVG_SHAPE_ERROR_UNSUPPORTED;
            fail(diagnostic, *failure, type.offset);
            return false;
        }
        if (attribute(source, tokens, token_count, element, "values", &value)) {
            size_t count = 0U;
            if (!parse_number_list(source, value, primitive->values, 20U,
                                   &count)) {
                *failure = SVG_SHAPE_ERROR_NUMBER;
                fail(diagnostic, *failure, value.offset);
                return false;
            }
            if ((primitive->mode == 0U && count != 20U) ||
                (primitive->mode == 1U && count != 1U) ||
                (primitive->mode == 2U && count != 1U) ||
                (primitive->mode == 3U && count != 0U)) {
                *failure = SVG_SHAPE_ERROR_NUMBER;
                fail(diagnostic, *failure, value.offset);
                return false;
            }
            if (primitive->mode == 1U &&
                (primitive->values[0] < 0.0 || primitive->values[0] > 1.0)) {
                *failure = SVG_SHAPE_ERROR_NUMBER;
                fail(diagnostic, *failure, value.offset);
                return false;
            }
        } else if (primitive->mode == 0U) {
            static const double identity[20] = {
                1, 0, 0, 0, 0, 0, 1, 0, 0, 0,
                0, 0, 1, 0, 0, 0, 0, 0, 1, 0};
            memcpy(primitive->values, identity, sizeof(identity));
        } else if (primitive->mode == 1U) {
            primitive->values[0] = 1.0;
        }
    }
    if (kind == SVG_SHAPE_FILTER_MERGE &&
        !parse_merge_inputs(source, tokens, token_count, element, end,
                            results, result_count, primitive, diagnostic)) {
        *failure = diagnostic == NULL ? SVG_SHAPE_ERROR_UNSUPPORTED :
                   diagnostic->category;
        return false;
    }
    return true;
}

static size_t matching_end(const svg_xml_token *tokens, size_t count,
                           size_t element)
{
    if (tokens[element].kind == SVG_XML_TOKEN_EMPTY_ELEMENT) return element;
    for (size_t i = element + 1U; i < count; ++i)
        if (tokens[i].kind == SVG_XML_TOKEN_END_ELEMENT &&
            tokens[i].depth == tokens[element].depth) return i;
    return SIZE_MAX;
}

bool svg_shape_filter_parse(base_bytes source,
                            const svg_xml_token *tokens, size_t token_count,
                            svg_reference_index *references,
                            svg_shape_document *document, svg_xml_span value,
                            size_t *filter_index,
                            svg_shape_diagnostic *diagnostic)
{
    if (!base_bytes_valid(source) || tokens == NULL || references == NULL ||
        document == NULL || filter_index == NULL) {
        fail(diagnostic, SVG_SHAPE_ERROR_INVALID_ARGUMENT, value.offset);
        return false;
    }
    svg_xml_span id;
    size_t element = 0U;
    if (!parse_url_id(source, value, &id) ||
        !svg_reference_index_find(source, tokens, token_count, references,
                                  id, &element) ||
        !span_equal(source, tokens[element].local_name, "filter")) {
        fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, value.offset);
        return false;
    }
    if (document->filter_count >= SHAPE_FILTER_MAX_COUNT ||
        document->filter_primitive_count >
            SHAPE_FILTER_MAX_TOTAL_PRIMITIVES - SVG_SHAPE_MAX_FILTER_PRIMITIVES) {
        fail(diagnostic, SVG_SHAPE_ERROR_LIMIT, tokens[element].name.offset);
        return false;
    }
    size_t end = matching_end(tokens, token_count, element);
    if (end == SIZE_MAX) {
        fail(diagnostic, SVG_SHAPE_ERROR_XML, tokens[element].name.offset);
        return false;
    }
    size_t old_primitives = document->filter_primitive_count;
    bool object_bbox = true;
    bool primitive_bbox = false;
    svg_xml_span unit_value = {0};
    if (attribute(source, tokens, token_count, element, "filterUnits",
                  &unit_value)) {
        if (span_equal(source, unit_value, "userSpaceOnUse")) object_bbox = false;
        else if (!span_equal(source, unit_value, "objectBoundingBox")) {
            fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, unit_value.offset);
            return false;
        }
    }
    if (attribute(source, tokens, token_count, element, "primitiveUnits",
                  &unit_value)) {
        if (span_equal(source, unit_value, "objectBoundingBox"))
            primitive_bbox = true;
        else if (!span_equal(source, unit_value, "userSpaceOnUse")) {
            fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, unit_value.offset);
            return false;
        }
    }
    double region[4] = {-0.1, -0.1, 1.2, 1.2};
    bool custom_region = false;
    static const char *const region_names[] = {"x", "y", "width", "height"};
    for (size_t r = 0U; r < 4U; ++r) {
        svg_xml_span region_value = {0};
        if (!attribute(source, tokens, token_count, element,
                       region_names[r], &region_value)) continue;
        custom_region = true;
        double fallback = region[r];
        if (!parse_region_number(source, region_value, fallback,
                                 &region[r]) ||
            (r >= 2U && region[r] <= 0.0)) {
            fail(diagnostic, SVG_SHAPE_ERROR_NUMBER, region_value.offset);
            return false;
        }
    }
    svg_shape_filter_primitive pending[SVG_SHAPE_MAX_FILTER_PRIMITIVES] = {{0}};
    filter_result_name results[SVG_SHAPE_MAX_FILTER_PRIMITIVES] = {{0}};
    size_t count = 0U;
    svg_shape_error failure = SVG_SHAPE_ERROR_UNSUPPORTED;
    for (size_t i = element + 1U; i < end; ++i) {
        if ((tokens[i].kind != SVG_XML_TOKEN_START_ELEMENT &&
             tokens[i].kind != SVG_XML_TOKEN_EMPTY_ELEMENT) ||
            tokens[i].depth != tokens[element].depth + 1U) continue;
        if (count == SVG_SHAPE_MAX_FILTER_PRIMITIVES) {
            fail(diagnostic, SVG_SHAPE_ERROR_LIMIT, tokens[i].name.offset);
            return false;
        }
        size_t child_end = matching_end(tokens, token_count, i);
        if (child_end == SIZE_MAX || child_end > end) {
            fail(diagnostic, SVG_SHAPE_ERROR_XML, tokens[i].name.offset);
            return false;
        }
        if (!parse_primitive(source, tokens, token_count, i, child_end,
                             results, count, &pending[count], &failure,
                             diagnostic)) return false;
        svg_xml_span result = {0};
        if (attribute(source, tokens, token_count, i, "result", &result))
            results[count] = (filter_result_name){result, true};
        count++;
        i = child_end;
    }
    if (count == 0U) {
        fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, tokens[element].name.offset);
        return false;
    }
    size_t needed = old_primitives + count;
    if (needed < old_primitives ||
        needed > SHAPE_FILTER_MAX_TOTAL_PRIMITIVES ||
        needed > SIZE_MAX / sizeof(*document->filter_primitives)) {
        fail(diagnostic, SVG_SHAPE_ERROR_LIMIT, tokens[element].name.offset);
        return false;
    }
    svg_shape_filter_primitive *grown = svg_shape_realloc(
        document->filter_primitives, needed * sizeof(*grown));
    if (grown == NULL) {
        fail(diagnostic, SVG_SHAPE_ERROR_OOM, tokens[element].name.offset);
        return false;
    }
    document->filter_primitives = grown;
    memcpy(grown + old_primitives, pending, count * sizeof(*pending));

    size_t new_count = document->filter_count + 1U;
    if (new_count > SIZE_MAX / sizeof(*document->filters)) {
        document->filter_primitive_count = old_primitives;
        fail(diagnostic, SVG_SHAPE_ERROR_LIMIT, tokens[element].name.offset);
        return false;
    }
    svg_shape_filter *filters = svg_shape_realloc(
        document->filters, new_count * sizeof(*filters));
    if (filters == NULL) {
        document->filter_primitive_count = old_primitives;
        fail(diagnostic, SVG_SHAPE_ERROR_OOM, tokens[element].name.offset);
        return false;
    }
    document->filters = filters;
    document->filters[document->filter_count] = (svg_shape_filter){
        .first_primitive = old_primitives,
        .primitive_count = count,
        .linear_rgb = true,
        .filter_units_object_bounding_box = object_bbox,
        .primitive_units_object_bounding_box = primitive_bbox,
        .custom_region = custom_region,
        .region = {region[0], region[1], region[2], region[3]}};
    *filter_index = document->filter_count++;
    document->filter_primitive_count = needed;
    return true;
}
