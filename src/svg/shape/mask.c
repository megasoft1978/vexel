#include "traverse_internal.h"

#include <math.h>
#include <string.h>

#define SHAPE_MASK_MAX_INSTANCES 1024U

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
    for (size_t i = element + 1U;
         i < token_count && tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE; ++i) {
        if (tokens[i].prefix.length == 0U &&
            svg_shape_span_eq(source, tokens[i].local_name, name)) {
            if (value != NULL) *value = tokens[i].value;
            return true;
        }
    }
    return false;
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

static size_t matching_end(const shape_traversal *traversal, size_t element)
{
    if (traversal->tokens[element].kind == SVG_XML_TOKEN_EMPTY_ELEMENT)
        return element;
    for (size_t i = element + 1U; i < traversal->token_count; ++i)
        if (traversal->tokens[i].kind == SVG_XML_TOKEN_END_ELEMENT &&
            traversal->tokens[i].depth == traversal->tokens[element].depth)
            return i;
    return SIZE_MAX;
}

static bool read_mask_region(base_bytes source, svg_xml_span value,
                             bool is_present, bool object_bbox,
                             double percentage_reference, double fallback,
                             double *out)
{
    if (!is_present) {
        *out = fallback;
        return true;
    }
    if (object_bbox) {
        if (value.offset > source.len || value.length > source.len - value.offset)
            return false;
        svg_shape_number_reader reader = {
            source.ptr + value.offset, value.length, 0U, value.offset};
        if (!svg_shape_read_number_value(&reader, out)) return false;
        svg_shape_skip_number_space(&reader);
        bool percent = reader.i < reader.n && reader.p[reader.i] == '%';
        if (percent) {
            reader.i++;
            *out /= 100.0;
        }
        svg_shape_skip_number_space(&reader);
        return reader.i == reader.n && isfinite(*out);
    }
    return svg_shape_read_length(source, value, percentage_reference, out) &&
           isfinite(*out);
}

static bool target_bounds(const svg_shape_document *document,
                          const svg_shape_opacity_group *group,
                          geom_affine transform, double bounds[4])
{
    if (group->first_shape >= group->end_shape ||
        group->end_shape > document->shape_count ||
        document->shape_edge_start == NULL || document->shape_edge_count == NULL)
        return false;
    geom_affine inverse;
    if (!svg_shape_affine_inverse(transform, &inverse)) return false;
    double min_x = INFINITY, min_y = INFINITY;
    double max_x = -INFINITY, max_y = -INFINITY;
    for (size_t shape = group->first_shape; shape < group->end_shape; ++shape) {
        size_t first = document->shape_edge_start[shape];
        size_t count = document->shape_edge_count[shape];
        if (first > document->edge_count || count > document->edge_count - first)
            return false;
        for (size_t edge = 0U; edge < count; ++edge) {
            geom_point points[2] = {document->edges[first + edge].a,
                                    document->edges[first + edge].b};
            for (size_t i = 0U; i < 2U; ++i) {
                geom_point local;
                if (!geom_affine_map_point(inverse, points[i], &local))
                    return false;
                if (local.x < min_x) min_x = local.x;
                if (local.x > max_x) max_x = local.x;
                if (local.y < min_y) min_y = local.y;
                if (local.y > max_y) max_y = local.y;
            }
        }
    }
    if (!isfinite(min_x) || !isfinite(min_y) || !isfinite(max_x) ||
        !isfinite(max_y) || max_x <= min_x || max_y <= min_y) return false;
    bounds[0] = min_x;
    bounds[1] = min_y;
    bounds[2] = max_x - min_x;
    bounds[3] = max_y - min_y;
    return true;
}

static void sync_mask_work(shape_traversal *destination,
                           const shape_traversal *source)
{
    destination->references.lookup_probes = source->references.lookup_probes;
    destination->use_instances = source->use_instances;
    destination->reference_bytes = source->reference_bytes;
    destination->element_visits = source->element_visits;
    destination->transform_operations = source->transform_operations;
    destination->paint_token_visits = source->paint_token_visits;
    destination->paint_cache = source->paint_cache;
    destination->builder.command_count = source->builder.command_count;
    destination->builder.point_count = source->builder.point_count;
    destination->builder.decoded_image_pixels =
        source->builder.decoded_image_pixels;
    destination->builder.text_bytes = source->builder.text_bytes;
    destination->builder.text_glyphs = source->builder.text_glyphs;
    destination->builder.text_commands = source->builder.text_commands;
    destination->mask_instances = source->mask_instances;
    destination->pattern_instances = source->pattern_instances;
    destination->pattern_bytes = source->pattern_bytes;
    destination->marker_instances = source->marker_instances;
}

static bool append_mask(svg_shape_document *document,
                        const svg_shape_mask *mask, size_t *index,
                        svg_shape_diagnostic *diagnostic)
{
    if (document->mask_count >= SVG_SHAPE_MAX_MASKS ||
        document->mask_count == SIZE_MAX / sizeof(*document->masks)) {
        fail(diagnostic, SVG_SHAPE_ERROR_LIMIT, mask->source_offset);
        return false;
    }
    size_t count = document->mask_count + 1U;
    if (count > SIZE_MAX / sizeof(*document->masks)) {
        fail(diagnostic, SVG_SHAPE_ERROR_LIMIT, mask->source_offset);
        return false;
    }
    svg_shape_mask *grown = svg_shape_realloc(
        document->masks, count * sizeof(*grown));
    if (grown == NULL) {
        fail(diagnostic, SVG_SHAPE_ERROR_OOM, mask->source_offset);
        return false;
    }
    document->masks = grown;
    document->masks[document->mask_count] = *mask;
    *index = document->mask_count++;
    return true;
}

bool svg_shape_traverse_attach_mask(shape_traversal *traversal,
                                    size_t element, size_t group_index,
                                    const svg_css_paint *style,
                                    geom_affine transform)
{
    svg_xml_span value = {0};
    if (!svg_css_paint_value(style, SVG_CSS_MASK, &value) ||
        svg_css_keyword_equals(traversal->source, value, "none")) return true;
    svg_shape_document *document = traversal->builder.doc;
    if (group_index == SVG_SHAPE_NO_GROUP ||
        group_index >= document->opacity_group_count) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_INVALID_ARGUMENT,
             value.offset);
        return false;
    }
    svg_xml_span id;
    size_t mask_element = 0U;
    if (!parse_url_id(traversal->source, value, &id) ||
        !svg_reference_index_find(traversal->source, traversal->tokens,
                                  traversal->token_count,
                                  &traversal->references, id, &mask_element) ||
        !svg_shape_span_eq(traversal->source,
                           traversal->tokens[mask_element].local_name, "mask")) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, value.offset);
        return false;
    }
    for (size_t i = 0U; i < traversal->active_mask_depth; ++i) {
        if (traversal->active_masks[i] == mask_element) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                 traversal->tokens[mask_element].name.offset);
            return false;
        }
    }
    if (traversal->active_mask_depth >=
            sizeof(traversal->active_masks) / sizeof(traversal->active_masks[0]) ||
        traversal->mask_instances >= SHAPE_MASK_MAX_INSTANCES) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
             traversal->tokens[mask_element].name.offset);
        return false;
    }
    svg_shape_opacity_group *group = &document->opacity_groups[group_index];
    group->end_shape = document->shape_count;
    if (group->first_shape == group->end_shape) return true;
    double target_box[4];
    if (!target_bounds(document, group, transform, target_box)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, value.offset);
        return false;
    }
    size_t end = matching_end(traversal, mask_element);
    if (end == SIZE_MAX) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_XML,
             traversal->tokens[mask_element].name.offset);
        return false;
    }

    bool region_bbox = true;
    bool content_bbox = false;
    svg_xml_span units = {0};
    if (attr(traversal->source, traversal->tokens, traversal->token_count,
             mask_element, "maskUnits", &units)) {
        if (svg_shape_span_eq(traversal->source, units, "userSpaceOnUse"))
            region_bbox = false;
        else if (!svg_shape_span_eq(traversal->source, units,
                                    "objectBoundingBox")) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                 units.offset);
            return false;
        }
    }
    if (attr(traversal->source, traversal->tokens, traversal->token_count,
             mask_element, "maskContentUnits", &units)) {
        if (svg_shape_span_eq(traversal->source, units, "objectBoundingBox"))
            content_bbox = true;
        else if (!svg_shape_span_eq(traversal->source, units,
                                    "userSpaceOnUse")) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                 units.offset);
            return false;
        }
    }
    double region[4] = {-0.1, -0.1, 1.2, 1.2};
    static const char *const region_names[] = {"x", "y", "width", "height"};
    bool custom_region = false;
    for (size_t i = 0U; i < 4U; ++i) {
        svg_xml_span region_value = {0};
        bool present = attr(traversal->source, traversal->tokens,
                            traversal->token_count, mask_element,
                            region_names[i], &region_value);
        custom_region |= present;
        double reference = i == 0U || i == 2U ?
                           traversal->viewport_width :
                           traversal->viewport_height;
        if (!read_mask_region(traversal->source, region_value, present,
                              region_bbox, reference, region[i],
                              &region[i]) ||
            (i >= 2U && region[i] <= 0.0)) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_NUMBER,
                 present ? region_value.offset :
                           traversal->tokens[mask_element].name.offset);
            return false;
        }
    }
    geom_affine box_map = {target_box[2], 0.0, 0.0, target_box[3],
                           target_box[0], target_box[1]};
    geom_affine content_transform = transform;
    geom_affine region_transform = transform;
    if (content_bbox &&
        !geom_affine_compose(transform, box_map, &content_transform)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_NUMBER, value.offset);
        return false;
    }
    if (region_bbox &&
        !geom_affine_compose(transform, box_map, &region_transform)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_NUMBER, value.offset);
        return false;
    }
    svg_css_paint mask_style;
    svg_css_error css_result = svg_css_compute(
        traversal->css, traversal->source, traversal->tokens,
        traversal->token_count, mask_element, NULL, &mask_style);
    if (css_result != SVG_CSS_OK) {
        fail(traversal->diagnostic,
             css_result == SVG_CSS_LIMIT ? SVG_SHAPE_ERROR_LIMIT :
             css_result == SVG_CSS_OOM ? SVG_SHAPE_ERROR_OOM :
             SVG_SHAPE_ERROR_UNSUPPORTED,
             traversal->tokens[mask_element].name.offset);
        return false;
    }
    bool luminance = true;
    svg_xml_span mask_type = {0};
    if (svg_css_paint_value(&mask_style, SVG_CSS_MASK_TYPE, &mask_type)) {
        if (svg_css_keyword_equals(traversal->source, mask_type, "alpha"))
            luminance = false;
        else if (!svg_css_keyword_equals(traversal->source, mask_type,
                                         "luminance")) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                 mask_type.offset);
            return false;
        }
    }
    svg_shape_document *content = svg_shape_calloc(1U, sizeof(*content));
    if (content == NULL) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_OOM,
             traversal->tokens[mask_element].name.offset);
        return false;
    }
    content->width = document->width;
    content->height = document->height;
    content->view_x = document->view_x;
    content->view_y = document->view_y;
    content->view_width = document->view_width;
    content->view_height = document->view_height;
    content->preserve_aspect = document->preserve_aspect;
    content->source = traversal->source;
    content->media_context = document->media_context;
    content->release_image = document->release_image;

    traversal->active_masks[traversal->active_mask_depth++] = mask_element;
    traversal->mask_instances++;
    shape_traversal nested = *traversal;
    nested.root_seen = true;
    nested.selection_element = SIZE_MAX;
    nested.opacity_group_depth = 0U;
    nested.opacity_group_capacity = 0U;
    nested.text_state = NULL;
    nested.text_depth = 0U;
    nested.builder.doc = content;
    nested.builder.traversal = &nested;
    nested.builder.edge_capacity = 0U;
    nested.builder.shape_capacity = 0U;
    nested.builder.subpath_capacity = 0U;
    nested.builder.path_segment_capacity = 0U;
    nested.builder.path_number_capacity = 0U;
    nested.builder.shape_index = 0U;
    nested.builder.transform = content_transform;
    nested.builder.viewport_width = traversal->viewport_width;
    nested.builder.viewport_height = traversal->viewport_height;
    nested.viewport_width = traversal->viewport_width;
    nested.viewport_height = traversal->viewport_height;
    if (!svg_shape_traverse_walk_range(&nested, mask_element + 1U, end,
                                       content_transform, &mask_style, 0U)) {
        sync_mask_work(traversal, &nested);
        traversal->active_mask_depth--;
        svg_shape_document_destroy(content);
        svg_shape_free(content);
        return false;
    }
    sync_mask_work(traversal, &nested);
    traversal->active_mask_depth--;
    svg_shape_mask mask = {
        .content = content,
        .luminance = luminance,
        .region_object_bounding_box = region_bbox,
        .custom_region = custom_region,
        .region = {region[0], region[1], region[2], region[3]},
        .region_transform = region_transform,
        .source_offset = traversal->tokens[mask_element].name.offset};
    size_t mask_index = SVG_SHAPE_NO_MASK;
    if (!append_mask(document, &mask, &mask_index, traversal->diagnostic)) {
        svg_shape_document_destroy(content);
        svg_shape_free(content);
        return false;
    }
    group = &document->opacity_groups[group_index];
    group->mask_index = mask_index;
    (void)element;
    return true;
}
