#include "traverse_internal.h"

#include <math.h>
#include <string.h>

#define SHAPE_MARKER_MAX_INSTANCES 1024U
#define SHAPE_MARKER_PI 3.14159265358979323846

typedef enum marker_position_kind {
    MARKER_POSITION_START,
    MARKER_POSITION_MID,
    MARKER_POSITION_END
} marker_position_kind;

typedef struct marker_subpath {
    /* Edge values, not pointers: marker emission appends to the document and
     * may reallocate its edge array. */
    geom_edge first;
    geom_edge last;
    bool has_edges;
} marker_subpath;

static void fail(svg_shape_diagnostic *diagnostic, svg_shape_error error,
                 size_t offset)
{
    if (diagnostic != NULL)
        *diagnostic = (svg_shape_diagnostic){error, offset};
}

static bool attr(base_bytes source, const svg_xml_token *tokens,
                 size_t token_count, size_t element, const char *name,
                 svg_xml_span *value)
{
    return svg_shape_traverse_attr(source, (svg_xml_token *)tokens,
                                   token_count, element, name, NULL, value);
}

static bool parse_url_id(base_bytes source, svg_xml_span value,
                         svg_xml_span *id)
{
    if (value.offset > source.len || value.length > source.len - value.offset)
        return false;
    size_t first = value.offset, end = first + value.length;
    while (first < end && (source.ptr[first] == ' ' ||
                           source.ptr[first] == '\t' ||
                           source.ptr[first] == '\r' ||
                           source.ptr[first] == '\n')) first++;
    while (end > first && (source.ptr[end - 1U] == ' ' ||
                           source.ptr[end - 1U] == '\t' ||
                           source.ptr[end - 1U] == '\r' ||
                           source.ptr[end - 1U] == '\n')) end--;
    if (end - first < 7U || memcmp(source.ptr + first, "url(", 4U) != 0 ||
        source.ptr[end - 1U] != ')') return false;
    first += 4U;
    end--;
    while (first < end && (source.ptr[first] == ' ' ||
                           source.ptr[first] == '\t' ||
                           source.ptr[first] == '\r' ||
                           source.ptr[first] == '\n')) first++;
    while (end > first && (source.ptr[end - 1U] == ' ' ||
                           source.ptr[end - 1U] == '\t' ||
                           source.ptr[end - 1U] == '\r' ||
                           source.ptr[end - 1U] == '\n')) end--;
    if (end - first < 2U || source.ptr[first] != '#') return false;
    *id = (svg_xml_span){first + 1U, end - first - 1U};
    return true;
}

static bool parse_marker_attributes(base_bytes source,
                                    const svg_xml_token *tokens,
                                    size_t token_count, size_t marker,
                                    svg_shape_diagnostic *diagnostic)
{
    static const char *const supported[] = {
        "id", "class", "style", "markerWidth", "markerHeight", "refX",
        "refY", "markerUnits", "orient", "viewBox",
        "preserveAspectRatio", "overflow"};
    for (size_t i = marker + 1U;
         i < token_count && tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE; ++i) {
        if (tokens[i].prefix.length != 0U) {
            fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, tokens[i].name.offset);
            return false;
        }
        bool found = false;
        for (size_t j = 0U; j < sizeof(supported) / sizeof(supported[0]); ++j)
            if (svg_shape_span_eq(source, tokens[i].local_name, supported[j])) {
                found = true;
                break;
            }
        if (!found) {
            fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, tokens[i].name.offset);
            return false;
        }
        if (svg_shape_span_eq(source, tokens[i].local_name, "overflow") &&
            !svg_css_keyword_equals(source, tokens[i].value, "hidden") &&
            !svg_css_keyword_equals(source, tokens[i].value, "visible")) {
            fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, tokens[i].value.offset);
            return false;
        }
    }
    return true;
}

static double direction_angle(geom_edge edge)
{
    return atan2(edge.b.y - edge.a.y, edge.b.x - edge.a.x);
}

static double middle_angle(geom_edge incoming, geom_edge outgoing)
{
    double first = direction_angle(incoming);
    double difference = remainder(direction_angle(outgoing) - first,
                                  2.0 * SHAPE_MARKER_PI);
    return first + difference * 0.5;
}

static double element_scale(geom_affine transform)
{
    double xx = transform.a * transform.a + transform.b * transform.b;
    double yy = transform.c * transform.c + transform.d * transform.d;
    double xy = transform.a * transform.c + transform.b * transform.d;
    double maximum = (xx + yy + hypot(xx - yy, 2.0 * xy)) * 0.5;
    return sqrt(fmax(0.0, maximum));
}

static bool parse_orient(base_bytes source, svg_xml_span span, bool *automatic,
                         bool *reverse, double *degrees)
{
    *automatic = false;
    *reverse = false;
    if (svg_css_keyword_equals(source, span, "auto")) {
        *automatic = true;
        return true;
    }
    if (svg_css_keyword_equals(source, span, "auto-start-reverse")) {
        *automatic = true;
        *reverse = true;
        return true;
    }
    if (span.offset > source.len || span.length > source.len - span.offset)
        return false;
    svg_shape_number_reader reader = {
        source.ptr + span.offset, span.length, 0U, span.offset};
    if (!svg_shape_read_number_value(&reader, degrees)) return false;
    svg_shape_skip_number_space(&reader);
    if (reader.i < reader.n && reader.n - reader.i == 3U &&
        memcmp(reader.p + reader.i, "deg", 3U) == 0) reader.i += 3U;
    svg_shape_skip_number_space(&reader);
    return reader.i == reader.n && isfinite(*degrees);
}

static bool compose_marker_transform(geom_point position, double angle,
                                     double scale, geom_affine content,
                                     geom_point reference,
                                     geom_affine *out)
{
    geom_affine translation = {
        1.0, 0.0, 0.0, 1.0, position.x, position.y};
    geom_affine rotation = {
        cos(angle), sin(angle), -sin(angle), cos(angle), 0.0, 0.0};
    geom_affine marker_scale = {scale, 0.0, 0.0, scale, 0.0, 0.0};
    geom_affine reference_shift = {
        1.0, 0.0, 0.0, 1.0, -reference.x, -reference.y};
    geom_affine result;
    if (!geom_affine_compose(translation, rotation, &result) ||
        !geom_affine_compose(result, marker_scale, &result) ||
        !geom_affine_compose(result, reference_shift, &result) ||
        !geom_affine_compose(result, content, out)) return false;
    return true;
}

static bool make_marker(shape_traversal *traversal, size_t marker,
                        const svg_css_paint *parent_style,
                        size_t shape, marker_position_kind kind,
                        geom_point position, double tangent,
                        geom_affine source_transform,
                        size_t render_depth)
{
    if (traversal->active_marker_depth >=
            sizeof(traversal->active_markers) /
                sizeof(traversal->active_markers[0]) ||
        traversal->marker_instances >= SHAPE_MARKER_MAX_INSTANCES) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
             traversal->tokens[marker].name.offset);
        return false;
    }
    for (size_t i = 0U; i < traversal->active_marker_depth; ++i) {
        if (traversal->active_markers[i] == marker) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                 traversal->tokens[marker].name.offset);
            return false;
        }
    }
    if (!parse_marker_attributes(traversal->source, traversal->tokens,
            traversal->token_count, marker, traversal->diagnostic)) return false;
    size_t marker_end = svg_shape_traverse_matching_end(traversal, marker);
    if (marker_end == SIZE_MAX) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_XML,
             traversal->tokens[marker].name.offset);
        return false;
    }
    svg_css_paint marker_style;
    if (!svg_shape_traverse_compute_style(traversal, marker, parent_style,
                                          &marker_style)) return false;
    marker_style.values[SVG_CSS_MARKER] = (svg_css_value){0};
    marker_style.values[SVG_CSS_MARKER_START] = (svg_css_value){0};
    marker_style.values[SVG_CSS_MARKER_MID] = (svg_css_value){0};
    marker_style.values[SVG_CSS_MARKER_END] = (svg_css_value){0};
    svg_xml_span value = {0};
    double width = 3.0, height = 3.0;
    if (attr(traversal->source, traversal->tokens, traversal->token_count,
             marker, "markerWidth", &value) &&
        !svg_shape_read_length(traversal->source, value,
                               traversal->viewport_width, &width)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_NUMBER, value.offset);
        return false;
    }
    if (attr(traversal->source, traversal->tokens, traversal->token_count,
             marker, "markerHeight", &value) &&
        !svg_shape_read_length(traversal->source, value,
                               traversal->viewport_height, &height)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_NUMBER, value.offset);
        return false;
    }
    if (!isfinite(width) || !isfinite(height) || width <= 0.0 || height <= 0.0 ||
        width > 4096.0 || height > 4096.0) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
             traversal->tokens[marker].name.offset);
        return false;
    }
    double ref_x = 0.0, ref_y = 0.0;
    if (attr(traversal->source, traversal->tokens, traversal->token_count,
             marker, "refX", &value) &&
        !svg_shape_read_length(traversal->source, value, width, &ref_x)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_NUMBER, value.offset);
        return false;
    }
    if (attr(traversal->source, traversal->tokens, traversal->token_count,
             marker, "refY", &value) &&
        !svg_shape_read_length(traversal->source, value, height, &ref_y)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_NUMBER, value.offset);
        return false;
    }
    bool automatic = false, reverse = false;
    double degrees = 0.0;
    if (attr(traversal->source, traversal->tokens, traversal->token_count,
             marker, "orient", &value) &&
        !parse_orient(traversal->source, value, &automatic, &reverse,
                      &degrees)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, value.offset);
        return false;
    }
    bool stroke_units = true;
    if (attr(traversal->source, traversal->tokens, traversal->token_count,
             marker, "markerUnits", &value)) {
        if (svg_shape_span_eq(traversal->source, value, "userSpaceOnUse"))
            stroke_units = false;
        else if (!svg_shape_span_eq(traversal->source, value, "strokeWidth")) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                 value.offset);
            return false;
        }
    }
    double units = 1.0;
    if (stroke_units) {
        if (shape >= traversal->builder.doc->shape_count ||
            traversal->builder.doc->strokes == NULL) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_INVALID_ARGUMENT,
                 traversal->tokens[marker].name.offset);
            return false;
        }
        units = traversal->builder.doc->strokes[shape].width *
                element_scale(source_transform);
        if (!isfinite(units) || units <= 0.0) units = 1.0;
    }
    geom_affine content = geom_affine_identity();
    geom_point reference = {ref_x, ref_y};
    svg_xml_span view_box_span = {0};
    double child_width = width, child_height = height;
    if (attr(traversal->source, traversal->tokens, traversal->token_count,
             marker, "viewBox", &view_box_span)) {
        double box[4];
        svg_shape_aspect aspect = {0.5, 0.5, false, false, true};
        if (!svg_shape_read_number_list(traversal->source, view_box_span,
                                        box, 4U) ||
            box[2] <= 0.0 || box[3] <= 0.0) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_VIEWPORT,
                 view_box_span.offset);
            return false;
        }
        if (attr(traversal->source, traversal->tokens,
                 traversal->token_count, marker, "preserveAspectRatio", &value) &&
            !svg_shape_parse_preserve_aspect(traversal->source, value, &aspect)) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                 value.offset);
            return false;
        }
        if (!svg_shape_viewbox_transform(box[0], box[1], box[2], box[3],
                0.0, 0.0, width, height, aspect, &content) ||
            !geom_affine_map_point(content, reference, &reference)) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_VIEWPORT,
                 view_box_span.offset);
            return false;
        }
        child_width = box[2];
        child_height = box[3];
    }
    double angle = automatic ? tangent : degrees * SHAPE_MARKER_PI / 180.0;
    if (kind == MARKER_POSITION_START && reverse) angle += SHAPE_MARKER_PI;
    if (!svg_shape_traverse_account_transform(traversal, 1U,
            traversal->tokens[marker].name.offset)) return false;
    geom_affine marker_transform;
    if (!compose_marker_transform(position, angle, units, content, reference,
                                  &marker_transform)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_NUMBER,
             traversal->tokens[marker].name.offset);
        return false;
    }
    if (marker_end == marker) return true;

    traversal->active_markers[traversal->active_marker_depth++] = marker;
    traversal->marker_instances++;
    size_t group = SVG_SHAPE_NO_GROUP;
    bool ok = svg_shape_traverse_begin_opacity_group(
        traversal, &marker_style, &group);
    double previous_width = traversal->viewport_width;
    double previous_height = traversal->viewport_height;
    traversal->viewport_width = child_width;
    traversal->viewport_height = child_height;
    traversal->builder.viewport_width = child_width;
    traversal->builder.viewport_height = child_height;
    if (ok) {
        svg_xml_span clip = {0};
        if (svg_css_paint_value(&marker_style, SVG_CSS_CLIP_PATH, &clip) &&
            !svg_css_keyword_equals(traversal->source, clip, "none")) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, clip.offset);
            ok = false;
        } else {
            ok = svg_shape_traverse_walk_range(
                traversal, marker + 1U, marker_end, marker_transform,
                &marker_style, render_depth + 1U);
        }
    }
    if (ok)
        ok = svg_shape_traverse_attach_mask(traversal, marker, group,
                                            &marker_style, marker_transform);
    svg_shape_traverse_finish_opacity_group(traversal, group);
    traversal->viewport_width = previous_width;
    traversal->viewport_height = previous_height;
    traversal->builder.viewport_width = previous_width;
    traversal->builder.viewport_height = previous_height;
    traversal->active_marker_depth--;
    return ok;
}

static bool emit_marker(shape_traversal *traversal, size_t shape,
                        const svg_css_paint *style, svg_css_property property,
                        marker_position_kind kind, geom_point point,
                        double tangent, geom_affine source_transform,
                        size_t render_depth)
{
    svg_xml_span value = {0}, id;
    if (!svg_css_paint_value(style, property, &value) ||
        svg_css_keyword_equals(traversal->source, value, "none")) return true;
    size_t marker = 0U;
    if (!parse_url_id(traversal->source, value, &id) ||
        !svg_reference_index_find(traversal->source, traversal->tokens,
            traversal->token_count, &traversal->references, id, &marker)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, value.offset);
        return false;
    }
    if (!svg_shape_traverse_element_name(traversal->source,
            &traversal->tokens[marker], "marker")) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, value.offset);
        return false;
    }
    return make_marker(traversal, marker, style, shape, kind, point,
                       tangent, source_transform, render_depth);
}

static bool emit_mid_marker(shape_traversal *traversal, size_t shape,
                            const svg_css_paint *style, geom_point point,
                            geom_edge incoming, geom_edge outgoing,
                            geom_affine transform, size_t render_depth)
{
    return emit_marker(traversal, shape, style, SVG_CSS_MARKER_MID,
                       MARKER_POSITION_MID, point,
                       middle_angle(incoming, outgoing), transform,
                       render_depth);
}

static bool flush_path_subpath(shape_traversal *traversal, size_t shape,
                               const svg_css_paint *style,
                               marker_subpath *subpath,
                               geom_affine transform, size_t render_depth)
{
    if (!subpath->has_edges) return true;
    bool ok = emit_marker(traversal, shape, style, SVG_CSS_MARKER_START,
        MARKER_POSITION_START, subpath->first.a,
        direction_angle(subpath->first), transform, render_depth) &&
        emit_marker(traversal, shape, style, SVG_CSS_MARKER_END,
        MARKER_POSITION_END, subpath->last.b,
        direction_angle(subpath->last), transform, render_depth);
    *subpath = (marker_subpath){0};
    return ok;
}

static bool path_markers(shape_traversal *traversal, size_t shape,
                         const svg_css_paint *style, geom_affine transform,
                         size_t render_depth)
{
    const svg_shape_document *document = traversal->builder.doc;
    size_t lower = 0U, upper = document->path_segment_count;
    while (lower < upper) {
        size_t middle = lower + (upper - lower) / 2U;
        if (document->path_segments[middle].shape_index < shape)
            lower = middle + 1U;
        else upper = middle;
    }
    marker_subpath current = {0};
    for (size_t i = lower; i < document->path_segment_count &&
         document->path_segments[i].shape_index == shape; ++i) {
        /* Copy by value: flushing emits markers whose nested paths append to
         * and may reallocate path_segments. */
        const svg_shape_path_segment segment_copy = document->path_segments[i];
        const svg_shape_path_segment *segment = &segment_copy;
        char command = segment->command;
        if (command >= 'a' && command <= 'z') command = (char)(command - 'a' + 'A');
        if (command == 'M' && !flush_path_subpath(
                traversal, shape, style, &current, transform, render_depth))
            return false;
        if (segment->edge_count == 0U) continue;
        if (segment->first_edge > document->edge_count ||
            segment->edge_count > document->edge_count - segment->first_edge) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_INVALID_ARGUMENT,
                 segment->source_offset);
            return false;
        }
        const geom_edge first = document->edges[segment->first_edge];
        const geom_edge last = document->edges[
            segment->first_edge + segment->edge_count - 1U];
        if (!current.has_edges) {
            current.first = first;
            current.last = last;
            current.has_edges = true;
            if (!emit_marker(traversal, shape, style, SVG_CSS_MARKER_START,
                    MARKER_POSITION_START, first.a, direction_angle(first),
                    transform, render_depth)) return false;
        } else {
            if (!emit_mid_marker(traversal, shape, style, current.last.b,
                    current.last, first, transform, render_depth)) return false;
            current.last = last;
        }
        if (command == 'Z' && !flush_path_subpath(
                traversal, shape, style, &current, transform, render_depth))
            return false;
    }
    return flush_path_subpath(traversal, shape, style, &current,
                              transform, render_depth);
}

static bool primitive_markers(shape_traversal *traversal, size_t element,
                              size_t shape, const svg_css_paint *style,
                              geom_affine transform, size_t render_depth)
{
    const svg_shape_document *document = traversal->builder.doc;
    if (shape >= document->shape_count || document->shape_edge_start == NULL ||
        document->shape_edge_count == NULL) return false;
    size_t first = document->shape_edge_start[shape];
    size_t count = document->shape_edge_count[shape];
    bool line = svg_shape_traverse_element_name(traversal->source,
        &traversal->tokens[element], "line");
    bool polyline = svg_shape_traverse_element_name(traversal->source,
        &traversal->tokens[element], "polyline");
    bool polygon = svg_shape_traverse_element_name(traversal->source,
        &traversal->tokens[element], "polygon");
    if (!line && !polyline && !polygon) return true;
    size_t path_edges = count;
    if (polyline && path_edges != 0U) path_edges--;
    if (first > document->edge_count ||
        path_edges > document->edge_count - first) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_INVALID_ARGUMENT,
             traversal->tokens[element].name.offset);
        return false;
    }
    if (path_edges == 0U) return true;
    /* Re-read edges by index after every emission: it may reallocate the array. */
    const geom_edge start = document->edges[first];
    if (!emit_marker(traversal, shape, style, SVG_CSS_MARKER_START,
            MARKER_POSITION_START, start.a, direction_angle(start),
            transform, render_depth)) return false;
    for (size_t i = 0U; i + 1U < path_edges; ++i) {
        const geom_edge incoming = document->edges[first + i];
        const geom_edge outgoing = document->edges[first + i + 1U];
        if (!emit_mid_marker(traversal, shape, style, incoming.b,
                incoming, outgoing, transform, render_depth)) return false;
    }
    const geom_edge end = document->edges[first + path_edges - 1U];
    return emit_marker(traversal, shape, style, SVG_CSS_MARKER_END,
        MARKER_POSITION_END, end.b, direction_angle(end), transform, render_depth);
}

bool svg_shape_traverse_attach_markers(shape_traversal *traversal,
                                       size_t element, size_t first_shape,
                                       size_t end_shape,
                                       const svg_css_paint *style,
                                       size_t render_depth)
{
    if (traversal == NULL || style == NULL || first_shape > end_shape ||
        end_shape > traversal->builder.doc->shape_count) {
        fail(traversal == NULL ? NULL : traversal->diagnostic,
             SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U);
        return false;
    }
    bool marker_present = style->values[SVG_CSS_MARKER_START].present ||
        style->values[SVG_CSS_MARKER_MID].present ||
        style->values[SVG_CSS_MARKER_END].present;
    if (!marker_present || first_shape == end_shape) return true;
    if (end_shape - first_shape > 1024U) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
             traversal->tokens[element].name.offset);
        return false;
    }
    geom_affine transform = traversal->builder.doc->shape_sources[first_shape].transform;
    for (size_t shape = first_shape; shape < end_shape; ++shape) {
        if (svg_shape_traverse_element_name(traversal->source,
                &traversal->tokens[element], "path")) {
            if (!path_markers(traversal, shape, style, transform, render_depth))
                return false;
        } else if (!primitive_markers(traversal, element, shape, style,
                                      transform, render_depth)) return false;
    }
    return true;
}
