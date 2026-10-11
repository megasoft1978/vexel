#include "shape_internal.h"

#include <math.h>
#include <string.h>

static bool space(uint8_t byte)
{
    return byte == ' ' || byte == '\t' || byte == '\r' || byte == '\n' ||
           byte == '\f';
}

static bool token_eq(base_bytes source, size_t first, size_t end,
                     const char *literal)
{
    size_t length = 0U;
    while (literal[length] != '\0') length++;
    return end - first == length && first <= source.len &&
           length <= source.len - first &&
           memcmp(source.ptr + first, literal, length) == 0;
}

bool svg_shape_parse_preserve_aspect(base_bytes source, svg_xml_span value,
                                     svg_shape_aspect *out)
{
    if (out == NULL || value.offset > source.len ||
        value.length > source.len - value.offset) return false;
    svg_shape_aspect parsed = {.align_x = 0.5, .align_y = 0.5, .set = true};
    size_t first = value.offset, end = value.offset + value.length;
    while (first < end && space(source.ptr[first])) first++;
    while (end > first && space(source.ptr[end - 1U])) end--;
    if (first == end) return false;
    size_t split = first;
    while (split < end && !space(source.ptr[split])) split++;
    if (token_eq(source, first, split, "defer")) {
        first = split;
        while (first < end && space(source.ptr[first])) first++;
        split = first;
        while (split < end && !space(source.ptr[split])) split++;
    }
    if (token_eq(source, first, split, "none")) {
        while (split < end && space(source.ptr[split])) split++;
        if (split != end) return false;
        parsed.none = true;
        *out = parsed;
        return true;
    }
    static const char *const alignments[] = {
        "xMinYMin", "xMidYMin", "xMaxYMin", "xMinYMid", "xMidYMid",
        "xMaxYMid", "xMinYMax", "xMidYMax", "xMaxYMax"
    };
    size_t alignment = 9U;
    for (size_t i = 0U; i < 9U; ++i)
        if (token_eq(source, first, split, alignments[i])) alignment = i;
    if (alignment == 9U) return false;
    parsed.align_x = (double)(alignment % 3U) * 0.5;
    parsed.align_y = (double)(alignment / 3U) * 0.5;
    first = split;
    while (first < end && space(source.ptr[first])) first++;
    if (first < end) {
        split = first;
        while (split < end && !space(source.ptr[split])) split++;
        parsed.slice = token_eq(source, first, split, "slice");
        if (!parsed.slice && !token_eq(source, first, split, "meet"))
            return false;
        first = split;
        while (first < end && space(source.ptr[first])) first++;
        if (first != end) return false;
    }
    *out = parsed;
    return true;
}

bool svg_shape_viewbox_transform(double view_x, double view_y,
                                 double view_width, double view_height,
                                 double x, double y, double width,
                                 double height, svg_shape_aspect aspect,
                                 geom_affine *out)
{
    if (out == NULL || !isfinite(view_x) || !isfinite(view_y) ||
        !isfinite(view_width) || !isfinite(view_height) ||
        !isfinite(x) || !isfinite(y) || !isfinite(width) ||
        !isfinite(height) || view_width <= 0.0 || view_height <= 0.0 ||
        width <= 0.0 || height <= 0.0) return false;
    if (!aspect.set) {
        aspect.align_x = 0.5;
        aspect.align_y = 0.5;
    }
    double scale_x = width / view_width;
    double scale_y = height / view_height;
    double free_x = 0.0, free_y = 0.0;
    if (!aspect.none) {
        double scale = aspect.slice ? fmax(scale_x, scale_y) :
                                      fmin(scale_x, scale_y);
        scale_x = scale;
        scale_y = scale;
        free_x = width - view_width * scale;
        free_y = height - view_height * scale;
    }
    geom_affine result = {scale_x, 0.0, 0.0, scale_y,
        x + aspect.align_x * free_x - view_x * scale_x,
        y + aspect.align_y * free_y - view_y * scale_y};
    if (!geom_affine_valid(result)) return false;
    *out = result;
    return true;
}

static bool attr(base_bytes source, const svg_xml_token *tokens, size_t count,
                 size_t element, const char *name, svg_xml_span *value)
{
    size_t length = 0U;
    while (name[length] != '\0') length++;
    for (size_t i = element + 1U;
         i < count && tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE; ++i) {
        svg_xml_span candidate = tokens[i].local_name;
        if (tokens[i].prefix.length == 0U && candidate.length == length &&
            candidate.offset <= source.len &&
            length <= source.len - candidate.offset &&
            memcmp(source.ptr + candidate.offset, name, length) == 0) {
            *value = tokens[i].value;
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

bool svg_shape_symbol_attributes_supported(base_bytes source,
                                          const svg_xml_token *tokens,
                                          size_t token_count, size_t element,
                                          size_t *unsupported_offset)
{
    for (size_t i = element + 1U;
         i < token_count && tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE; ++i) {
        svg_xml_span name = tokens[i].local_name;
        if (tokens[i].prefix.length != 0U) goto unsupported;
        bool allowed = svg_shape_span_eq(source, name, "id") ||
            svg_shape_span_eq(source, name, "class") ||
            svg_shape_span_eq(source, name, "style") ||
            svg_shape_span_eq(source, name, "viewBox") ||
            svg_shape_span_eq(source, name, "preserveAspectRatio") ||
            svg_shape_span_eq(source, name, "width") ||
            svg_shape_span_eq(source, name, "height") ||
            svg_shape_span_eq(source, name, "fill") ||
            svg_shape_span_eq(source, name, "fill-rule") ||
            svg_shape_span_eq(source, name, "fill-opacity") ||
            svg_shape_span_eq(source, name, "stroke") ||
            svg_shape_span_eq(source, name, "stroke-width") ||
            svg_shape_span_eq(source, name, "stroke-linecap") ||
            svg_shape_span_eq(source, name, "stroke-linejoin") ||
            svg_shape_span_eq(source, name, "stroke-miterlimit") ||
            svg_shape_span_eq(source, name, "stroke-dasharray") ||
            svg_shape_span_eq(source, name, "stroke-dashoffset") ||
            svg_shape_span_eq(source, name, "stroke-opacity") ||
            svg_shape_span_eq(source, name, "opacity") ||
            svg_shape_span_eq(source, name, "visibility") ||
            svg_shape_span_eq(source, name, "display") ||
            svg_shape_span_eq(source, name, "color");
        if (!allowed) goto unsupported;
        continue;
unsupported:
        if (unsupported_offset != NULL)
            *unsupported_offset = tokens[i].name.offset;
        return false;
    }
    return true;
}

bool svg_shape_symbol_mapping(base_bytes source,
                              const svg_xml_token *tokens,
                              size_t token_count, size_t target, size_t use,
                              double viewport_width, double viewport_height,
                              geom_affine *mapping)
{
    svg_xml_span view_span, width_span, height_span, value;
    if (!attr(source, tokens, token_count, target, "viewBox", &view_span) ||
        !attr(source, tokens, token_count, use, "width", &width_span) ||
        !attr(source, tokens, token_count, use, "height", &height_span))
        return false;
    double view_box[4], width, height, x = 0.0, y = 0.0;
    if (!svg_shape_read_number_list(source, view_span, view_box, 4U) ||
        !svg_shape_read_length(source, width_span, viewport_width, &width) ||
        !svg_shape_read_length(source, height_span, viewport_height, &height) ||
        view_box[2] <= 0.0 || view_box[3] <= 0.0 ||
        width <= 0.0 || height <= 0.0) return false;
    if (attr(source, tokens, token_count, use, "x", &value) &&
        !svg_shape_read_length(source, value, viewport_width, &x)) return false;
    if (attr(source, tokens, token_count, use, "y", &value) &&
        !svg_shape_read_length(source, value, viewport_height, &y)) return false;
    svg_shape_aspect aspect = {.align_x = 0.5, .align_y = 0.5};
    if (!attr(source, tokens, token_count, use, "preserveAspectRatio", &value) &&
        !attr(source, tokens, token_count, target,
              "preserveAspectRatio", &value)) {
        *mapping = (geom_affine){fmin(width / view_box[2], height / view_box[3]),
            0.0, 0.0, fmin(width / view_box[2], height / view_box[3]),
            0.0, 0.0};
        double scale = mapping->a;
        mapping->e = x + (width - view_box[2] * scale) * 0.5 - view_box[0] * scale;
        mapping->f = y + (height - view_box[3] * scale) * 0.5 - view_box[1] * scale;
        return geom_affine_valid(*mapping);
    }
    if (value.length != 0U &&
        !svg_shape_parse_preserve_aspect(source, value, &aspect)) return false;
    return svg_shape_viewbox_transform(view_box[0], view_box[1],
        view_box[2], view_box[3], x, y, width, height, aspect, mapping);
}

bool svg_shape_nested_viewport(base_bytes source,
                               const svg_xml_token *tokens,
                               size_t token_count, size_t element,
                               double parent_width, double parent_height,
                               geom_affine element_transform,
                               geom_affine *child_transform,
                               geom_affine *viewport_transform,
                               geom_point clip_points[4],
                               double *child_width, double *child_height,
                               bool *empty,
                               svg_shape_diagnostic *diagnostic)
{
    if (tokens == NULL || child_transform == NULL ||
        viewport_transform == NULL || clip_points == NULL ||
        child_width == NULL || child_height == NULL || empty == NULL ||
        element >= token_count) {
        fail(diagnostic, SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U);
        return false;
    }
    *empty = false;
    double x = 0.0, y = 0.0;
    double width = parent_width, height = parent_height;
    svg_xml_span value;
    if (attr(source, tokens, token_count, element, "x", &value) &&
        !svg_shape_read_length(source, value, parent_width, &x)) {
        fail(diagnostic, SVG_SHAPE_ERROR_NUMBER, value.offset);
        return false;
    }
    if (attr(source, tokens, token_count, element, "y", &value) &&
        !svg_shape_read_length(source, value, parent_height, &y)) {
        fail(diagnostic, SVG_SHAPE_ERROR_NUMBER, value.offset);
        return false;
    }
    if (attr(source, tokens, token_count, element, "width", &value) &&
        !svg_shape_read_length(source, value, parent_width, &width)) {
        fail(diagnostic, SVG_SHAPE_ERROR_NUMBER, value.offset);
        return false;
    }
    if (attr(source, tokens, token_count, element, "height", &value) &&
        !svg_shape_read_length(source, value, parent_height, &height)) {
        fail(diagnostic, SVG_SHAPE_ERROR_NUMBER, value.offset);
        return false;
    }
    if (width < 0.0 || height < 0.0 || fabs(x) > 1000000.0 ||
        fabs(y) > 1000000.0 || width > 1000000.0 || height > 1000000.0) {
        fail(diagnostic, SVG_SHAPE_ERROR_NUMBER, tokens[element].name.offset);
        return false;
    }
    if (width == 0.0 || height == 0.0) {
        *empty = true;
        return true;
    }
    geom_point local_clip[4] = {{x, y}, {x + width, y},
                                {x + width, y + height}, {x, y + height}};
    for (size_t i = 0U; i < 4U; ++i)
        if (!geom_affine_map_point(element_transform, local_clip[i],
                                   &clip_points[i])) {
            fail(diagnostic, SVG_SHAPE_ERROR_NUMBER, tokens[element].name.offset);
            return false;
        }

    geom_affine viewport = {1.0, 0.0, 0.0, 1.0, x, y};
    svg_xml_span view_span;
    if (attr(source, tokens, token_count, element, "viewBox", &view_span)) {
        double view_box[4];
        if (!svg_shape_read_number_list(source, view_span, view_box, 4U) ||
            view_box[2] <= 0.0 || view_box[3] <= 0.0 ||
            fabs(view_box[0] + view_box[2]) > 1000000.0 ||
            fabs(view_box[1] + view_box[3]) > 1000000.0) {
            fail(diagnostic, SVG_SHAPE_ERROR_VIEWPORT, view_span.offset);
            return false;
        }
        svg_shape_aspect aspect = {.align_x = 0.5, .align_y = 0.5};
        if (attr(source, tokens, token_count, element,
                 "preserveAspectRatio", &value) &&
            !svg_shape_parse_preserve_aspect(source, value, &aspect)) {
            fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, value.offset);
            return false;
        }
        if (!svg_shape_viewbox_transform(view_box[0], view_box[1],
                view_box[2], view_box[3], x, y, width, height,
                aspect, &viewport)) {
            fail(diagnostic, SVG_SHAPE_ERROR_NUMBER, view_span.offset);
            return false;
        }
        *child_width = view_box[2];
        *child_height = view_box[3];
    } else {
        *child_width = width;
        *child_height = height;
    }
    if (!geom_affine_compose(element_transform, viewport, child_transform)) {
        fail(diagnostic, SVG_SHAPE_ERROR_NUMBER, tokens[element].name.offset);
        return false;
    }
    *viewport_transform = viewport;
    return true;
}

bool svg_shape_root_viewport(base_bytes source, svg_xml_token *tokens,
                             size_t token_count, size_t element,
                             svg_shape_document *document,
                             double *viewport_width,
                             double *viewport_height,
                             svg_shape_diagnostic *diagnostic)
{
    if (tokens == NULL || document == NULL || viewport_width == NULL ||
        viewport_height == NULL || element >= token_count) {
        fail(diagnostic, SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U);
        return false;
    }
    svg_xml_span value = {0}, width_span = {0}, height_span = {0};
    if (attr(source, tokens, token_count, element, "xmlns", &value) &&
        !token_eq(source, value.offset, value.offset + value.length,
                  "http://www.w3.org/2000/svg")) {
        fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, value.offset);
        return false;
    }
    if (!attr(source, tokens, token_count, element, "width", &width_span) ||
        !attr(source, tokens, token_count, element, "height", &height_span)) {
        fail(diagnostic, SVG_SHAPE_ERROR_VIEWPORT, tokens[element].name.offset);
        return false;
    }
    double width, height;
    if (!svg_shape_read_dimension(source, width_span, &width)) {
        fail(diagnostic, SVG_SHAPE_ERROR_NUMBER, width_span.offset);
        return false;
    }
    if (!svg_shape_read_dimension(source, height_span, &height)) {
        fail(diagnostic, SVG_SHAPE_ERROR_NUMBER, height_span.offset);
        return false;
    }
    if (width < 1.0 || width > 4096.0 || height < 1.0 || height > 4096.0) {
        fail(diagnostic, SVG_SHAPE_ERROR_VIEWPORT,
             width < 1.0 || width > 4096.0 ? width_span.offset : height_span.offset);
        return false;
    }
    document->width = (uint32_t)ceil(width);
    document->height = (uint32_t)ceil(height);
    document->view_x = 0.0;
    document->view_y = 0.0;
    document->view_width = width;
    document->view_height = height;
    document->preserve_aspect = (svg_shape_aspect){0.5, 0.5, false, false, true};
    if (attr(source, tokens, token_count, element, "viewBox", &value)) {
        double view_box[4];
        if (!svg_shape_read_number_list(source, value, view_box, 4U) ||
            view_box[2] <= 0.0 || view_box[3] <= 0.0 ||
            fabs(view_box[0] + view_box[2]) > 1000000.0 ||
            fabs(view_box[1] + view_box[3]) > 1000000.0) {
            fail(diagnostic, SVG_SHAPE_ERROR_VIEWPORT, value.offset);
            return false;
        }
        document->view_x = view_box[0];
        document->view_y = view_box[1];
        document->view_width = view_box[2];
        document->view_height = view_box[3];
    }
    if (attr(source, tokens, token_count, element,
             "preserveAspectRatio", &value) &&
        !svg_shape_parse_preserve_aspect(source, value,
                                         &document->preserve_aspect)) {
        fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, value.offset);
        return false;
    }
    *viewport_width = document->view_width;
    *viewport_height = document->view_height;
    return true;
}
