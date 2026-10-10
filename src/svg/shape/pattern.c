#include "traverse_internal.h"

#include "../../raster/shape_render.h"

#include <math.h>
#include <string.h>

#define SHAPE_PATTERN_MAX_INSTANCES 1024U
#define SHAPE_PATTERN_MAX_PIXELS 1048576U
#define SHAPE_PATTERN_MAX_AXIS 4096U

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

static bool parse_bbox_number(base_bytes source, svg_xml_span span,
                              double *out)
{
    if (span.offset > source.len || span.length > source.len - span.offset)
        return false;
    svg_shape_number_reader reader = {
        source.ptr + span.offset, span.length, 0U, span.offset};
    if (!svg_shape_read_number_value(&reader, out)) return false;
    svg_shape_skip_number_space(&reader);
    if (reader.i < reader.n && reader.p[reader.i] == '%') {
        reader.i++;
        *out /= 100.0;
    }
    svg_shape_skip_number_space(&reader);
    return reader.i == reader.n && isfinite(*out);
}

static bool read_unit_value(base_bytes source, svg_xml_span value,
                            bool present, bool object_bbox,
                            double reference, double fallback, double *out)
{
    if (!present) {
        *out = fallback;
        return true;
    }
    if (object_bbox) return parse_bbox_number(source, value, out);
    return svg_shape_read_length(source, value, reference, out) &&
           isfinite(*out);
}

static bool local_bounds(const svg_shape_document *document, size_t shape,
                         geom_affine transform, double bounds[4])
{
    if (shape >= document->shape_count || document->shape_edge_start == NULL ||
        document->shape_edge_count == NULL || document->edges == NULL)
        return false;
    geom_affine inverse;
    if (!svg_shape_affine_inverse(transform, &inverse)) return false;
    size_t first = document->shape_edge_start[shape];
    size_t count = document->shape_edge_count[shape];
    if (first > document->edge_count || count > document->edge_count - first ||
        count == 0U) return false;
    double min_x = INFINITY, min_y = INFINITY;
    double max_x = -INFINITY, max_y = -INFINITY;
    for (size_t i = 0U; i < count; ++i) {
        geom_point points[2] = {document->edges[first + i].a,
                                document->edges[first + i].b};
        for (size_t p = 0U; p < 2U; ++p) {
            geom_point local;
            if (!geom_affine_map_point(inverse, points[p], &local)) return false;
            if (local.x < min_x) min_x = local.x;
            if (local.x > max_x) max_x = local.x;
            if (local.y < min_y) min_y = local.y;
            if (local.y > max_y) max_y = local.y;
        }
    }
    if (!isfinite(min_x) || !isfinite(min_y) || !isfinite(max_x) ||
        !isfinite(max_y) || max_x < min_x || max_y < min_y) return false;
    bounds[0] = min_x;
    bounds[1] = min_y;
    bounds[2] = max_x - min_x;
    bounds[3] = max_y - min_y;
    return true;
}

static bool matrix_equal(geom_affine a, geom_affine b)
{
    return a.a == b.a && a.b == b.b && a.c == b.c && a.d == b.d &&
           a.e == b.e && a.f == b.f;
}

static void sync_work(shape_traversal *destination,
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

static bool parse_supported_attributes(base_bytes source,
                                       const svg_xml_token *tokens,
                                       size_t token_count, size_t element,
                                       svg_shape_diagnostic *diagnostic)
{
    static const char *const allowed[] = {
        "id", "class", "style", "x", "y", "width", "height",
        "patternUnits", "patternContentUnits", "patternTransform",
        "viewBox", "preserveAspectRatio"};
    for (size_t i = element + 1U;
         i < token_count && tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE; ++i) {
        bool found = false;
        if (tokens[i].prefix.length != 0U) {
            fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                 tokens[i].name.offset);
            return false;
        }
        for (size_t j = 0U; j < sizeof(allowed) / sizeof(allowed[0]); ++j)
            if (svg_shape_span_eq(source, tokens[i].local_name, allowed[j])) {
                found = true;
                break;
            }
        if (!found) {
            fail(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                 tokens[i].name.offset);
            return false;
        }
    }
    return true;
}

static bool append_pattern(svg_shape_document *document,
                           svg_shape_pattern pattern, size_t *index,
                           svg_shape_diagnostic *diagnostic)
{
    if (document->pattern_count >= SVG_SHAPE_MAX_PATTERNS ||
        document->pattern_count == SIZE_MAX / sizeof(*document->patterns)) {
        fail(diagnostic, SVG_SHAPE_ERROR_LIMIT,
             pattern.definition_element);
        return false;
    }
    size_t count = document->pattern_count + 1U;
    if (count > SIZE_MAX / sizeof(*document->patterns)) {
        fail(diagnostic, SVG_SHAPE_ERROR_LIMIT,
             pattern.definition_element);
        return false;
    }
    svg_shape_pattern *grown = svg_shape_realloc(
        document->patterns, count * sizeof(*grown));
    if (grown == NULL) {
        fail(diagnostic, SVG_SHAPE_ERROR_OOM,
             pattern.definition_element);
        return false;
    }
    document->patterns = grown;
    document->patterns[document->pattern_count] = pattern;
    document->pattern_bytes += pattern.byte_count;
    *index = document->pattern_count++;
    return true;
}

static bool cached_pattern(const svg_shape_document *document,
                           size_t definition, uint32_t width, uint32_t height,
                           double tile_width, double tile_height,
                           geom_affine content_transform, size_t *index)
{
    for (size_t i = 0U; i < document->pattern_count; ++i) {
        const svg_shape_pattern *candidate = &document->patterns[i];
        if (candidate->definition_element == definition &&
            candidate->width == width && candidate->height == height &&
            candidate->tile_width == tile_width &&
            candidate->tile_height == tile_height &&
            matrix_equal(candidate->content_transform, content_transform)) {
            *index = i;
            return true;
        }
    }
    return false;
}

static bool decode_pattern_content(shape_traversal *traversal,
                                   size_t definition, size_t end,
                                   geom_affine content_transform,
                                   double tile_width, double tile_height,
                                   uint32_t width, uint32_t height,
                                   size_t *pattern_index)
{
    svg_shape_document *owner = traversal->builder.doc;
    if (traversal->active_pattern_depth >=
            sizeof(traversal->active_patterns) /
                sizeof(traversal->active_patterns[0]) ||
        traversal->pattern_instances >= SHAPE_PATTERN_MAX_INSTANCES) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
             traversal->tokens[definition].name.offset);
        return false;
    }
    for (size_t i = 0U; i < traversal->active_pattern_depth; ++i) {
        if (traversal->active_patterns[i] == definition) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                 traversal->tokens[definition].name.offset);
            return false;
        }
    }
    traversal->active_patterns[traversal->active_pattern_depth++] = definition;
    traversal->pattern_instances++;
    svg_shape_document *content = svg_shape_calloc(1U, sizeof(*content));
    if (content == NULL) {
        traversal->active_pattern_depth--;
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_OOM,
             traversal->tokens[definition].name.offset);
        return false;
    }
    content->width = width;
    content->height = height;
    content->view_x = 0.0;
    content->view_y = 0.0;
    content->view_width = tile_width;
    content->view_height = tile_height;
    content->preserve_aspect = (svg_shape_aspect){
        0.0, 0.0, true, false, true};
    content->source = traversal->source;
    content->media_context = owner->media_context;
    content->release_image = owner->release_image;
    svg_css_paint pattern_style;
    svg_css_error css_result = svg_css_compute(
        traversal->css, traversal->source, traversal->tokens,
        traversal->token_count, definition, NULL, &pattern_style);
    if (css_result != SVG_CSS_OK) {
        traversal->active_pattern_depth--;
        svg_shape_document_destroy(content);
        svg_shape_free(content);
        fail(traversal->diagnostic,
             css_result == SVG_CSS_LIMIT ? SVG_SHAPE_ERROR_LIMIT :
             css_result == SVG_CSS_OOM ? SVG_SHAPE_ERROR_OOM :
             SVG_SHAPE_ERROR_UNSUPPORTED,
             traversal->tokens[definition].name.offset);
        return false;
    }
    shape_traversal nested = *traversal;
    nested.root_seen = true;
    nested.selection_element = SIZE_MAX;
    nested.opacity_group_depth = 0U;
    nested.opacity_group_capacity = 0U;
    nested.text_state = NULL;
    nested.text_depth = 0U;
    nested.viewport_width = tile_width;
    nested.viewport_height = tile_height;
    nested.builder.doc = content;
    nested.builder.traversal = &nested;
    nested.builder.edge_capacity = 0U;
    nested.builder.shape_capacity = 0U;
    nested.builder.subpath_capacity = 0U;
    nested.builder.path_segment_capacity = 0U;
    nested.builder.path_number_capacity = 0U;
    nested.builder.shape_index = 0U;
    nested.builder.transform = content_transform;
    nested.builder.viewport_width = tile_width;
    nested.builder.viewport_height = tile_height;
    if (!svg_shape_traverse_walk_range(&nested, definition + 1U, end,
                                       content_transform, &pattern_style, 0U)) {
        sync_work(traversal, &nested);
        traversal->active_pattern_depth--;
        svg_shape_document_destroy(content);
        svg_shape_free(content);
        return false;
    }
    sync_work(traversal, &nested);
    traversal->active_pattern_depth--;
    raster_tile tile = {0};
    if (!raster_shape_render(content, &tile, traversal->diagnostic)) {
        svg_shape_document_destroy(content);
        svg_shape_free(content);
        return false;
    }
    size_t bytes = tile.byte_count;
    if (bytes == 0U || bytes > SVG_SHAPE_MAX_PATTERN_BYTES ||
        traversal->pattern_bytes >
            SVG_SHAPE_MAX_PATTERN_BYTES - bytes ||
        owner->pattern_bytes > SVG_SHAPE_MAX_PATTERN_BYTES - bytes) {
        raster_tile_destroy(&tile);
        svg_shape_document_destroy(content);
        svg_shape_free(content);
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
             traversal->tokens[definition].name.offset);
        return false;
    }
    uint8_t *pixels = svg_shape_malloc(bytes);
    if (pixels == NULL) {
        raster_tile_destroy(&tile);
        svg_shape_document_destroy(content);
        svg_shape_free(content);
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_OOM,
             traversal->tokens[definition].name.offset);
        return false;
    }
    memcpy(pixels, tile.rgba, bytes);
    raster_tile_destroy(&tile);
    svg_shape_document_destroy(content);
    svg_shape_free(content);
    svg_shape_pattern pattern = {
        .rgba = pixels,
        .byte_count = bytes,
        .width = width,
        .height = height,
        .definition_element = definition,
        .tile_width = tile_width,
        .tile_height = tile_height,
        .content_transform = content_transform};
    if (!append_pattern(owner, pattern, pattern_index,
                        traversal->diagnostic)) {
        svg_shape_free(pixels);
        return false;
    }
    traversal->pattern_bytes += bytes;
    return true;
}

bool svg_shape_pattern_resolve(svg_shape_builder *builder, base_bytes source,
                               const svg_xml_token *tokens,
                               size_t token_count, svg_xml_span fill,
                               svg_css_paint style, size_t shape_index,
                               geom_affine transform, bool stroke_paint,
                               bool *resolved)
{
    if (resolved != NULL) *resolved = false;
    if (builder == NULL || builder->doc == NULL || builder->traversal == NULL ||
        tokens == NULL || resolved == NULL || shape_index >= builder->doc->shape_count)
        return false;
    shape_traversal *traversal = builder->traversal;
    svg_xml_span id;
    size_t definition = 0U;
    if (!parse_url_id(source, fill, &id) ||
        !svg_reference_index_find(source, tokens, token_count,
                                  &traversal->references, id, &definition))
        return true;
    if (!svg_shape_span_eq(source, tokens[definition].local_name, "pattern"))
        return true;
    *resolved = true;
    if (!parse_supported_attributes(source, tokens, token_count, definition,
                                    traversal->diagnostic)) return false;
    svg_xml_span href = {0};
    if (attr(source, tokens, token_count, definition, "href", &href)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, href.offset);
        return false;
    }
    size_t end = svg_shape_traverse_matching_end(traversal, definition);
    if (end == SIZE_MAX) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_XML,
             tokens[definition].name.offset);
        return false;
    }
    double bounds[4];
    if (!local_bounds(builder->doc, shape_index, transform, bounds)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, fill.offset);
        return false;
    }
    bool object_bbox = true;
    bool content_bbox = false;
    svg_xml_span units = {0};
    if (attr(source, tokens, token_count, definition, "patternUnits", &units)) {
        if (svg_shape_span_eq(source, units, "userSpaceOnUse")) object_bbox = false;
        else if (!svg_shape_span_eq(source, units, "objectBoundingBox")) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, units.offset);
            return false;
        }
    }
    if (attr(source, tokens, token_count, definition,
             "patternContentUnits", &units)) {
        if (svg_shape_span_eq(source, units, "objectBoundingBox"))
            content_bbox = true;
        else if (!svg_shape_span_eq(source, units, "userSpaceOnUse")) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, units.offset);
            return false;
        }
    }
    if ((object_bbox || content_bbox) &&
        (bounds[2] <= 0.0 || bounds[3] <= 0.0)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, fill.offset);
        return false;
    }
    double values[4] = {0.0, 0.0, 0.0, 0.0};
    static const char *const names[] = {"x", "y", "width", "height"};
    for (size_t i = 0U; i < 4U; ++i) {
        svg_xml_span value = {0};
        bool present = attr(source, tokens, token_count, definition,
                            names[i], &value);
        double reference = i == 0U || i == 2U ?
                           traversal->viewport_width :
                           traversal->viewport_height;
        if (!read_unit_value(source, value, present, object_bbox, reference,
                             0.0, &values[i])) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_NUMBER,
                 present ? value.offset : tokens[definition].name.offset);
            return false;
        }
    }
    if (values[2] <= 0.0 || values[3] <= 0.0) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_NUMBER,
             tokens[definition].name.offset);
        return false;
    }
    double tile_x = values[0], tile_y = values[1];
    double tile_width = values[2], tile_height = values[3];
    if (object_bbox) {
        tile_x = bounds[0] + values[0] * bounds[2];
        tile_y = bounds[1] + values[1] * bounds[3];
        tile_width = values[2] * bounds[2];
        tile_height = values[3] * bounds[3];
    }
    if (!isfinite(tile_x) || !isfinite(tile_y) ||
        !isfinite(tile_width) || !isfinite(tile_height) ||
        tile_width <= 0.0 || tile_height <= 0.0 ||
        tile_width > 65536.0 || tile_height > 65536.0) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
             tokens[definition].name.offset);
        return false;
    }
    geom_affine pattern_transform = geom_affine_identity();
    svg_xml_span transform_span = {0};
    if (attr(source, tokens, token_count, definition,
             "patternTransform", &transform_span)) {
        size_t operations = 0U;
        if (!svg_shape_transform_parse(source, transform_span,
                                       &pattern_transform, &operations) ||
            !svg_shape_traverse_account_transform(
                traversal, operations, transform_span.offset)) {
            if (traversal->diagnostic->category == SVG_SHAPE_ERROR_NONE)
                fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                     transform_span.offset);
            return false;
        }
    }
    geom_affine pattern_inverse;
    if (!svg_shape_affine_inverse(pattern_transform, &pattern_inverse)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
             transform_span.offset);
        return false;
    }
    geom_affine translate = {1.0, 0.0, 0.0, 1.0, -tile_x, -tile_y};
    geom_affine user_to_base;
    if (!geom_affine_compose(translate, pattern_inverse, &user_to_base)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_NUMBER,
             tokens[definition].name.offset);
        return false;
    }
    svg_shape_document *document = builder->doc;
    double device_scale_x = (double)document->width / document->view_width;
    double device_scale_y = (double)document->height / document->view_height;
    svg_shape_aspect aspect = document->preserve_aspect;
    if (!aspect.set) aspect = (svg_shape_aspect){0.5, 0.5, false, false, true};
    if (!aspect.none) {
        double scale = aspect.slice ? fmax(device_scale_x, device_scale_y) :
                                      fmin(device_scale_x, device_scale_y);
        device_scale_x = scale;
        device_scale_y = scale;
    }
    double current_scale = fmax(hypot(transform.a, transform.b),
                                hypot(transform.c, transform.d));
    double pattern_scale = fmax(hypot(pattern_transform.a, pattern_transform.b),
                                hypot(pattern_transform.c, pattern_transform.d));
    double density = fmax(device_scale_x, device_scale_y) *
                     current_scale * pattern_scale * 4.0;
    if (!isfinite(density) || density <= 0.0) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT, fill.offset);
        return false;
    }
    double pixel_width = ceil(tile_width * density);
    double pixel_height = ceil(tile_height * density);
    if (pixel_width < 1.0) pixel_width = 1.0;
    if (pixel_height < 1.0) pixel_height = 1.0;
    if (!isfinite(pixel_width) || !isfinite(pixel_height) ||
        pixel_width > SHAPE_PATTERN_MAX_AXIS ||
        pixel_height > SHAPE_PATTERN_MAX_AXIS ||
        pixel_width * pixel_height > SHAPE_PATTERN_MAX_PIXELS) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT,
             tokens[definition].name.offset);
        return false;
    }
    uint32_t texture_width = (uint32_t)pixel_width;
    uint32_t texture_height = (uint32_t)pixel_height;

    svg_xml_span viewbox = {0}, aspect_span = {0};
    geom_affine content_transform;
    if (attr(source, tokens, token_count, definition, "viewBox", &viewbox)) {
        double box[4];
        svg_shape_aspect preserve = {0.5, 0.5, false, false, true};
        if (!svg_shape_read_number_list(source, viewbox, box, 4U) ||
            box[2] <= 0.0 || box[3] <= 0.0) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_VIEWPORT,
                 viewbox.offset);
            return false;
        }
        if (attr(source, tokens, token_count, definition,
                 "preserveAspectRatio", &aspect_span) &&
            !svg_shape_parse_preserve_aspect(source, aspect_span, &preserve)) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                 aspect_span.offset);
            return false;
        }
        if (!svg_shape_viewbox_transform(box[0], box[1], box[2], box[3],
                                         0.0, 0.0, tile_width, tile_height,
                                         preserve, &content_transform)) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_VIEWPORT,
                 viewbox.offset);
            return false;
        }
    } else {
        geom_affine object_mapping = {1.0, 0.0, 0.0, 1.0, 0.0, 0.0};
        if (content_bbox) {
            object_mapping = (geom_affine){bounds[2], 0.0, 0.0,
                bounds[3], bounds[0], bounds[1]};
        }
        if (!geom_affine_compose(translate, object_mapping,
                                 &content_transform)) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_NUMBER,
                 tokens[definition].name.offset);
            return false;
        }
    }
    size_t pattern_index = SVG_SHAPE_NO_PATTERN;
    if (!cached_pattern(document, definition, texture_width, texture_height,
                        tile_width, tile_height, content_transform,
                        &pattern_index)) {
        if (!decode_pattern_content(traversal, definition, end,
                                    content_transform, tile_width, tile_height,
                                    texture_width, texture_height,
                                    &pattern_index)) return false;
    }
    geom_affine pixel_scale = {
        (double)texture_width / tile_width, 0.0, 0.0,
        (double)texture_height / tile_height, 0.0, 0.0};
    geom_affine pattern_inverse_pixels;
    if (!geom_affine_compose(pixel_scale, user_to_base,
                             &pattern_inverse_pixels)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_NUMBER, fill.offset);
        return false;
    }
    double opacity = 1.0;
    svg_xml_span opacity_span = {0};
    svg_css_property opacity_property = stroke_paint ?
        SVG_CSS_STROKE_OPACITY : SVG_CSS_FILL_OPACITY;
    if (svg_css_paint_value(&style, opacity_property, &opacity_span) &&
        !svg_shape_parse_opacity(source, opacity_span, &opacity)) {
        fail(traversal->diagnostic, SVG_SHAPE_ERROR_NUMBER, opacity_span.offset);
        return false;
    }
    if (stroke_paint) {
        if (builder->doc->stroke_paints == NULL) {
            size_t capacity = builder->shape_capacity;
            if (capacity == 0U || capacity > SIZE_MAX /
                    sizeof(*builder->doc->stroke_paints)) {
                fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT, fill.offset);
                return false;
            }
            builder->doc->stroke_paints = svg_shape_calloc(
                capacity, sizeof(*builder->doc->stroke_paints));
            if (builder->doc->stroke_paints == NULL) {
                fail(traversal->diagnostic, SVG_SHAPE_ERROR_OOM, fill.offset);
                return false;
            }
            builder->doc->stroke_paint_capacity = capacity;
        }
        if (shape_index >= builder->doc->stroke_paint_capacity) {
            fail(traversal->diagnostic, SVG_SHAPE_ERROR_LIMIT, fill.offset);
            return false;
        }
        builder->doc->stroke_paints[shape_index] = (svg_shape_paint){
            .kind = SVG_SHAPE_PAINT_PATTERN,
            .pattern_index = pattern_index,
            .pattern_inverse = pattern_inverse_pixels,
            .pattern_opacity = opacity};
    } else {
        builder->doc->paints[shape_index] = (svg_shape_paint){
            .kind = SVG_SHAPE_PAINT_PATTERN,
            .pattern_index = pattern_index,
            .pattern_inverse = pattern_inverse_pixels,
            .pattern_opacity = opacity};
    }
    return true;
}
