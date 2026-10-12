#include "clip.h"
#include "../shape/shape_alloc.h"
#include "../shape/shape_internal.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CLIP_MAX_CHILDREN 64U
#define CLIP_MAX_REFERENCE_BYTES (64U * 1024U)
#define CLIP_MAX_JOB_REFERENCE_BYTES (512U * 1024U)
#define CLIP_WRAPPER_BUFFER_BYTES 192U
#define CLIP_WRAPPER_LIMIT_BYTES 4096U
/* Worst-case userSpaceOnUse wrapper: fixed text, two 10-digit %u values, four
 * "-d.<16 digits>e-308" (24 byte) %.17g values, and the terminating NUL. */
#define CLIP_WRAPPER_WORST_BYTES (12U + 10U + 11U + 3U + 2U + 2U * 10U + 4U * 24U + 1U)
_Static_assert(CLIP_WRAPPER_BUFFER_BYTES <= CLIP_WRAPPER_LIMIT_BYTES,
               "clip wrapper buffer exceeds the 4 KiB generated-syntax bound");
_Static_assert(CLIP_WRAPPER_WORST_BYTES <= CLIP_WRAPPER_BUFFER_BYTES,
               "worst-case clip wrapper must fit its buffer");
#define CLIP_MAX_EDGES 65536U
#define CLIP_CLONE_MAX_DEPTH 64U
#define CLIP_CLONE_MAX_BYTES (64U * 1024U * 1024U)

struct svg_shape_clip {
    svg_shape_document geometry;
    uint8_t *source;
    size_t source_length;
    size_t owner_node;
    bool owner_before_suffix;
    size_t references;
    struct svg_shape_clip *parent;
    bool object_bounding_box;
    double bounds[4];
    geom_affine user_inverse;
};

static bool eq(base_bytes source, svg_xml_span span, const char *text)
{
    size_t length = strlen(text);
    return span.offset <= source.len && span.length == length &&
           length <= source.len - span.offset &&
           memcmp(source.ptr + span.offset, text, length) == 0;
}

static bool attr(base_bytes source, const svg_xml_token *tokens, size_t count,
                 size_t element, const char *name, svg_xml_span *value)
{
    for (size_t i = element + 1U; i < count && tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE; ++i) {
        if (tokens[i].prefix.length == 0U && eq(source, tokens[i].local_name, name)) {
            *value = tokens[i].value;
            return true;
        }
    }
    return false;
}

static size_t element_end(const svg_xml_token *tokens, size_t count, size_t start)
{
    if (tokens[start].kind == SVG_XML_TOKEN_EMPTY_ELEMENT) return start;
    for (size_t i = start + 1U; i < count; ++i) {
        if (tokens[i].kind == SVG_XML_TOKEN_END_ELEMENT &&
            tokens[i].depth == tokens[start].depth) return i;
    }
    return start;
}

static svg_shape_error check_clip_child_style(
    svg_css_sheet *css, base_bytes source, const svg_xml_token *tokens,
    size_t token_count, size_t element, svg_shape_diagnostic *diagnostic)
{
    svg_css_paint style;
    svg_css_error css_error = svg_css_compute(css, source, tokens, token_count,
                                              element, NULL, &style);
    if (css_error != SVG_CSS_OK) {
        svg_shape_error error = css_error == SVG_CSS_LIMIT ?
            SVG_SHAPE_ERROR_LIMIT : css_error == SVG_CSS_OOM ?
            SVG_SHAPE_ERROR_OOM : SVG_SHAPE_ERROR_UNSUPPORTED;
        if (diagnostic != NULL)
            *diagnostic = (svg_shape_diagnostic){error, tokens[element].name.offset};
        return error;
    }
    for (size_t property = 0U; property < SVG_CSS_PROPERTY_COUNT; ++property) {
        if (style.values[property].present) {
            if (diagnostic != NULL)
                *diagnostic = (svg_shape_diagnostic){
                    SVG_SHAPE_ERROR_UNSUPPORTED,
                    style.values[property].span.offset};
            return SVG_SHAPE_ERROR_UNSUPPORTED;
        }
    }
    return SVG_SHAPE_ERROR_NONE;
}

static bool parse_reference(base_bytes source, svg_xml_span value,
                            svg_xml_span *id)
{
    if (value.offset > source.len || value.length < 7U ||
        value.length > source.len - value.offset ||
        memcmp(source.ptr + value.offset, "url(#", 5U) != 0 ||
        source.ptr[value.offset + value.length - 1U] != ')') return false;
    *id = (svg_xml_span){value.offset + 5U, value.length - 6U};
    return id->length != 0U;
}

static bool point_inside(const svg_shape_document *doc, size_t shape,
                         double x, double y)
{
    int winding = 0;
    uint8_t parity = 0U;
    size_t first = doc->shape_edge_start[shape];
    size_t end = first + doc->shape_edge_count[shape];
    for (size_t i = first; i < end; ++i) {
        geom_edge edge = doc->edges[i];
        bool up = edge.a.y <= y && edge.b.y > y;
        bool down = edge.b.y <= y && edge.a.y > y;
        if (!up && !down) continue;
        double cross_x = edge.a.x + (y - edge.a.y) *
                         (edge.b.x - edge.a.x) / (edge.b.y - edge.a.y);
        if (cross_x > x) { winding += up ? 1 : -1; parity ^= 1U; }
    }
    return doc->shape_evenodd[shape] ? parity != 0U : winding != 0;
}

svg_shape_error svg_clip_resolve(base_bytes source, const svg_xml_token *tokens,
                                 size_t token_count,
                                 svg_reference_index *references,
                                 svg_css_sheet *css,
                                 svg_xml_span value,
                                 svg_shape_document *document,
                                 size_t *document_clip_bytes,
                                 geom_affine user_inverse,
                                 const double local_bounds[4],
                                 size_t owner_node,
                                 svg_shape_clip **out,
                                 svg_shape_diagnostic *diagnostic)
{
    if (out == NULL || document == NULL || document_clip_bytes == NULL ||
        css == NULL ||
        local_bounds == NULL) {
        if (diagnostic != NULL) *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_INVALID_ARGUMENT, value.offset};
        return SVG_SHAPE_ERROR_INVALID_ARGUMENT;
    }
    *out = NULL;
    svg_xml_span id;
    if (!parse_reference(source, value, &id)) goto unsupported;
    size_t target;
    if (!svg_reference_index_find(source, tokens, token_count, references,
                                  id, &target) ||
        !eq(source, tokens[target].local_name, "clipPath") ||
        tokens[target].prefix.length != 0U) goto unsupported;
    if (attr(source, tokens, token_count, target, "clip-path", &id) ||
        attr(source, tokens, token_count, target, "transform", &id) ||
        attr(source, tokens, token_count, target, "style", &id)) goto unsupported;
    bool object_box = false;
    svg_xml_span units;
    bool has_units = attr(source, tokens, token_count, target, "clipPathUnits", &units);
    if (has_units) {
        if (eq(source, units, "userSpaceOnUse")) object_box = false;
        else if (eq(source, units, "objectBoundingBox")) object_box = true;
        else goto unsupported;
    }
    size_t end_token = element_end(tokens, token_count, target);
    for (size_t i = target + 1U; i < end_token; ++i) {
        if (tokens[i].kind != SVG_XML_TOKEN_START_ELEMENT &&
            tokens[i].kind != SVG_XML_TOKEN_EMPTY_ELEMENT) continue;
        svg_shape_error style_error = check_clip_child_style(
            css, source, tokens, token_count, i, diagnostic);
        if (style_error != SVG_SHAPE_ERROR_NONE) return style_error;
        if (attr(source, tokens, token_count, i, "style", &id) ||
            (eq(source, tokens[i].local_name, "style") && tokens[i].source.length != 0U))
            goto unsupported;
    }
    size_t child_start = tokens[target].source.offset + tokens[target].source.length;
    size_t child_end = end_token == target ? child_start : tokens[end_token].source.offset;
    size_t child_count = 0U;
    for (size_t i = target + 1U; i < end_token; ++i) {
        if ((tokens[i].kind == SVG_XML_TOKEN_START_ELEMENT ||
             tokens[i].kind == SVG_XML_TOKEN_EMPTY_ELEMENT) &&
            tokens[i].depth == tokens[target].depth + 1U) {
            if (++child_count > CLIP_MAX_CHILDREN ||
                attr(source, tokens, token_count, i, "clip-path", &units)) goto limit;
        }
    }
    if (child_count == 0U || child_start > child_end || child_end > source.len) goto unsupported;
    /* The prefix permits 191 payload bytes; suffix and NUL count in scratch. */
    char prefix_pattern[CLIP_WRAPPER_BUFFER_BYTES];
    int prefix_result = object_box
        ? snprintf(prefix_pattern, sizeof(prefix_pattern),
                   "<svg width=\"1\" height=\"1\" viewBox=\"0 0 1 1\">")
        : snprintf(prefix_pattern, sizeof(prefix_pattern),
                   "<svg width=\"%u\" height=\"%u\" viewBox=\"%.17g %.17g %.17g %.17g\">",
                   document->width, document->height, document->view_x,
                   document->view_y, document->view_width, document->view_height);
    if (prefix_result < 0 || (size_t)prefix_result >= sizeof(prefix_pattern)) goto limit;
    size_t prefix_length = (size_t)prefix_result;
    static const char suffix[] = "</svg>";
    size_t child_length = child_end - child_start;
    if (child_length > CLIP_MAX_REFERENCE_BYTES ||
        prefix_length > SIZE_MAX - sizeof(suffix) - child_length) goto limit;
    size_t total = prefix_length + child_length + sizeof(suffix) - 1U;
    if (total >= CLIP_MAX_JOB_REFERENCE_BYTES) goto limit;
    size_t allocation_bytes = total + 1U;
    if (*document_clip_bytes > CLIP_MAX_JOB_REFERENCE_BYTES - allocation_bytes)
        goto limit;
    uint8_t *scratch = svg_shape_malloc(allocation_bytes);
    svg_shape_clip *clip = svg_shape_calloc(1U, sizeof(*clip));
    if (scratch == NULL || clip == NULL) {
        svg_shape_free(scratch); svg_shape_free(clip);
        if (diagnostic != NULL) *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_OOM, value.offset};
        return SVG_SHAPE_ERROR_OOM;
    }
    (void)svg_shape_alloc_set_tag(scratch, SVG_SHAPE_ALLOC_TAG_CLIP_COPY);
    memcpy(scratch, prefix_pattern, prefix_length);
    memcpy(scratch + prefix_length, source.ptr + child_start, child_length);
    memcpy(scratch + prefix_length + child_length, suffix, sizeof(suffix) - 1U);
    scratch[total] = 0U;
    clip->source = scratch;
    clip->source_length = total;
    clip->owner_node = owner_node;
    clip->references = 1U;
    clip->object_bounding_box = object_box;
    clip->user_inverse = user_inverse;
    memcpy(clip->bounds, local_bounds, sizeof(clip->bounds));
    if (object_box && (clip->bounds[2] <= clip->bounds[0] ||
                       clip->bounds[3] <= clip->bounds[1])) {
        svg_clip_destroy(clip); goto unsupported;
    }
    svg_shape_error error = svg_shape_decode((base_bytes){scratch, total}, &clip->geometry, diagnostic);
    if (error != SVG_SHAPE_ERROR_NONE) {
        if (diagnostic != NULL) {
            size_t generated = diagnostic->offset;
            diagnostic->offset = generated >= prefix_length &&
                generated - prefix_length < child_length
                ? child_start + generated - prefix_length : value.offset;
        }
        svg_clip_destroy(clip);
        return error;
    }
    if (clip->geometry.edge_count > CLIP_MAX_EDGES) { svg_clip_destroy(clip); goto limit; }
    *document_clip_bytes += allocation_bytes;
    document->has_clips = true;
    *out = clip;
    return SVG_SHAPE_ERROR_NONE;
limit:
    if (diagnostic != NULL) *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_LIMIT, value.offset};
    return SVG_SHAPE_ERROR_LIMIT;
unsupported:
    if (diagnostic != NULL) *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_UNSUPPORTED, value.offset};
    return SVG_SHAPE_ERROR_UNSUPPORTED;
}

bool svg_clip_contains(const svg_shape_clip *clip, double x, double y)
{
    if (clip == NULL) return true;
    double root_x = x, root_y = y;
    geom_point local;
    if (!geom_affine_map_point(clip->user_inverse, (geom_point){x, y}, &local))
        return false;
    x = local.x;
    y = local.y;
    if (clip->object_bounding_box) {
        if (clip->bounds[2] <= clip->bounds[0] ||
            clip->bounds[3] <= clip->bounds[1]) return false;
        x = (x - clip->bounds[0]) / (clip->bounds[2] - clip->bounds[0]);
        y = (y - clip->bounds[1]) / (clip->bounds[3] - clip->bounds[1]);
    }
    bool inside = false;
    for (size_t i = 0U; i < clip->geometry.shape_count; ++i) {
        if (clip->geometry.shape_filled[i] && point_inside(&clip->geometry, i, x, y)) {
            inside = true;
            break;
        }
    }
    return inside && svg_clip_contains(clip->parent, root_x, root_y);
}

static bool clip_local_to_document(const svg_shape_clip *clip,
                                   geom_affine local_to_document,
                                   geom_point point, geom_point *out)
{
    if (clip->object_bounding_box) {
        point.x = clip->bounds[0] + point.x *
                  (clip->bounds[2] - clip->bounds[0]);
        point.y = clip->bounds[1] + point.y *
                  (clip->bounds[3] - clip->bounds[1]);
    }
    return geom_affine_map_point(local_to_document, point, out);
}

bool svg_clip_visit_boundary_edges(const svg_shape_clip *clip,
                                   svg_clip_boundary_edge_visitor visitor,
                                   void *context, bool *mapping_valid)
{
    if (mapping_valid != NULL) *mapping_valid = true;
    if (visitor == NULL) return false;
    for (const svg_shape_clip *current = clip; current != NULL;
         current = current->parent) {
        geom_affine local_to_document;
        if (!svg_shape_affine_inverse(current->user_inverse,
                                      &local_to_document)) {
            if (mapping_valid != NULL) *mapping_valid = false;
            return true;
        }
        const svg_shape_document *geometry = &current->geometry;
        for (size_t shape = 0U; shape < geometry->shape_count; shape++) {
            if (!geometry->shape_filled[shape]) continue;
            size_t first = geometry->shape_edge_start[shape];
            size_t end = first + geometry->shape_edge_count[shape];
            for (size_t edge_index = first; edge_index < end; edge_index++) {
                geom_edge edge = geometry->edges[edge_index];
                if (!clip_local_to_document(current, local_to_document,
                                            edge.a, &edge.a) ||
                    !clip_local_to_document(current, local_to_document,
                                            edge.b, &edge.b)) {
                    if (mapping_valid != NULL) *mapping_valid = false;
                    return true;
                }
                if (!visitor(context, edge)) return false;
            }
        }
    }
    return true;
}

bool svg_clip_boundary_work_counts(const svg_shape_clip *clip,
                                   svg_clip_work_counts *out)
{
    if (out == NULL) return false;
    *out = (svg_clip_work_counts){0};
    for (const svg_shape_clip *current = clip; current != NULL;
         current = current->parent) {
        if (out->chain_elements == SIZE_MAX ||
            current->geometry.shape_count > SIZE_MAX - out->child_shapes ||
            current->geometry.edge_count > SIZE_MAX - out->edges) return false;
        out->chain_elements++;
        out->child_shapes += current->geometry.shape_count;
        out->edges += current->geometry.edge_count;
    }
    return true;
}

size_t svg_clip_work_units(const svg_shape_clip *clip)
{
    svg_clip_work_counts counts;
    if (!svg_clip_boundary_work_counts(clip, &counts) ||
        counts.child_shapes > SIZE_MAX - counts.chain_elements) return SIZE_MAX;
    size_t units = counts.chain_elements + counts.child_shapes;
    if (counts.edges > SIZE_MAX - units) return SIZE_MAX;
    return units + counts.edges;
}

void svg_clip_destroy(svg_shape_clip *clip)
{
    if (clip == NULL) return;
    if (clip->references > 1U) { clip->references--; return; }
    svg_shape_clip *parent = clip->parent;
    svg_shape_document_destroy(&clip->geometry);
    svg_shape_free(clip->source);
    svg_shape_free(clip);
    svg_clip_destroy(parent);
}

bool svg_clip_retain(svg_shape_clip *clip)
{
    if (clip == NULL || clip->references == SIZE_MAX) return false;
    clip->references++;
    return true;
}

static bool clone_bytes(size_t count, size_t stride, size_t *bytes)
{
    if (count != 0U && stride > SIZE_MAX / count) return false;
    *bytes = count * stride;
    return true;
}

static bool clone_array(const void *source, size_t count, size_t stride,
                        size_t *total, bool *limit, void **out)
{
    size_t bytes = 0U;
    if (!clone_bytes(count, stride, &bytes) ||
        bytes > CLIP_CLONE_MAX_BYTES - *total) {
        *limit = true;
        return false;
    }
    if (bytes == 0U) {
        *out = NULL;
        return true;
    }
    void *copy = svg_shape_malloc(bytes);
    if (copy == NULL) return false;
    memcpy(copy, source, bytes);
    *total += bytes;
    *out = copy;
    return true;
}

static svg_shape_clip *clone_clip_chain(const svg_shape_clip *source,
                                        size_t depth, size_t *total,
                                        bool *limit)
{
    if (source == NULL || total == NULL || limit == NULL ||
        depth >= CLIP_CLONE_MAX_DEPTH) {
        if (limit != NULL) *limit = true;
        return NULL;
    }
    if (sizeof(svg_shape_clip) > CLIP_CLONE_MAX_BYTES - *total) {
        *limit = true;
        return NULL;
    }
    svg_shape_clip *copy = svg_shape_calloc(1U, sizeof(*copy));
    if (copy == NULL) return NULL;
    *total += sizeof(*copy);
    copy->references = 1U;
    copy->owner_node = source->owner_node;
    copy->owner_before_suffix = source->owner_before_suffix;
    copy->object_bounding_box = source->object_bounding_box;
    copy->user_inverse = source->user_inverse;
    memcpy(copy->bounds, source->bounds, sizeof(copy->bounds));
    if (!clone_array(source->source, source->source_length, 1U, total, limit,
                     (void **)&copy->source)) goto fail;
    copy->source_length = source->source_length;
    svg_shape_document *dst = &copy->geometry;
    const svg_shape_document *src = &source->geometry;
    dst->shape_count = src->shape_count;
    dst->edge_count = src->edge_count;
    if (!clone_array(src->edges, src->edge_count, sizeof(*src->edges), total, limit,
                     (void **)&dst->edges) ||
        !clone_array(src->shape_evenodd, src->shape_count,
                     sizeof(*src->shape_evenodd), total, limit,
                     (void **)&dst->shape_evenodd) ||
        !clone_array(src->shape_filled, src->shape_count,
                     sizeof(*src->shape_filled), total, limit,
                     (void **)&dst->shape_filled) ||
        !clone_array(src->shape_edge_start, src->shape_count,
                     sizeof(*src->shape_edge_start), total, limit,
                     (void **)&dst->shape_edge_start) ||
        !clone_array(src->shape_edge_count, src->shape_count,
                     sizeof(*src->shape_edge_count), total, limit,
                     (void **)&dst->shape_edge_count)) goto fail;
    if (source->parent != NULL) {
        copy->parent = clone_clip_chain(source->parent, depth + 1U, total,
                                        limit);
        if (copy->parent == NULL) goto fail;
    }
    return copy;
fail:
    svg_clip_destroy(copy);
    return NULL;
}

svg_shape_clip *svg_clip_clone(const svg_shape_clip *clip,
                               size_t *total_bytes, bool *limit)
{
    if (total_bytes == NULL || limit == NULL) return NULL;
    *limit = false;
    return clone_clip_chain(clip, 0U, total_bytes, limit);
}

void svg_clip_update_owner(svg_shape_clip *clip, size_t owner_node,
                           geom_affine user_inverse,
                           geom_affine attribute_inverse,
                           const double local_bounds[4])
{
    size_t depth = 0U;
    for (svg_shape_clip *current = clip; current != NULL &&
         depth++ < CLIP_CLONE_MAX_DEPTH; current = current->parent) {
        if (current->owner_node != owner_node) continue;
        geom_affine inverse = current->owner_before_suffix ?
                              attribute_inverse : user_inverse;
        if (!geom_affine_valid(inverse)) continue;
        current->user_inverse = inverse;
        if (local_bounds != NULL && current->object_bounding_box &&
            isfinite(local_bounds[0]) && isfinite(local_bounds[1]) &&
            isfinite(local_bounds[2]) && isfinite(local_bounds[3]) &&
            local_bounds[2] >= local_bounds[0] &&
            local_bounds[3] >= local_bounds[1])
            memcpy(current->bounds, local_bounds, sizeof(current->bounds));
    }
}

bool svg_clip_local_bounds(const svg_shape_document *document,
                           size_t first_edge, size_t edge_count,
                           geom_affine user_inverse, double bounds[4])
{
    if (document == NULL || bounds == NULL || edge_count == 0U ||
        first_edge > document->edge_count ||
        edge_count > document->edge_count - first_edge) return false;
    geom_point first;
    if (!geom_affine_map_point(user_inverse, document->edges[first_edge].a,
                               &first)) return false;
    bounds[0] = bounds[2] = first.x;
    bounds[1] = bounds[3] = first.y;
    for (size_t i = first_edge; i < first_edge + edge_count; ++i) {
        geom_point points[2] = {document->edges[i].a, document->edges[i].b};
        for (size_t j = 0U; j < 2U; ++j) {
            geom_point local;
            if (!geom_affine_map_point(user_inverse, points[j], &local))
                return false;
            if (local.x < bounds[0]) bounds[0] = local.x;
            if (local.y < bounds[1]) bounds[1] = local.y;
            if (local.x > bounds[2]) bounds[2] = local.x;
            if (local.y > bounds[3]) bounds[3] = local.y;
        }
    }
    return true;
}

bool svg_clip_apply_group(svg_shape_document *document,
                          size_t first_shape, size_t end_shape,
                          svg_shape_clip *clip)
{
    if (document == NULL || clip == NULL || first_shape > end_shape ||
        end_shape > document->shape_count) return false;
    for (size_t i = first_shape; i < end_shape; ++i) {
        svg_shape_clip *root = document->clips[i];
        if (root == NULL) {
            if (!svg_clip_retain(clip)) return false;
            document->clips[i] = clip;
            continue;
        }
        svg_shape_clip *tail = root;
        while (true) {
            if (tail == clip) break;
            if (tail->parent == NULL) {
                if (!svg_clip_retain(clip)) return false;
                tail->parent = clip;
                break;
            }
            tail = tail->parent;
        }
    }
    return true;
}

svg_shape_clip *svg_clip_create_rect(geom_point p0, geom_point p1,
                                    geom_point p2, geom_point p3,
                                    geom_affine user_inverse,
                                    size_t owner_node)
{
    const geom_point points[4] = {p0, p1, p2, p3};
    for (size_t i = 0U; i < 4U; ++i)
        if (!geom_finite_point(points[i])) return NULL;
    svg_shape_clip *clip = svg_shape_calloc(1U, sizeof(*clip));
    if (clip == NULL) return NULL;
    svg_shape_document *geometry = &clip->geometry;
    geometry->edges = svg_shape_calloc(4U, sizeof(*geometry->edges));
    geometry->shape_evenodd = svg_shape_calloc(1U, sizeof(*geometry->shape_evenodd));
    geometry->shape_filled = svg_shape_calloc(1U, sizeof(*geometry->shape_filled));
    geometry->shape_edge_start = svg_shape_calloc(1U, sizeof(*geometry->shape_edge_start));
    geometry->shape_edge_count = svg_shape_calloc(1U, sizeof(*geometry->shape_edge_count));
    if (geometry->edges == NULL || geometry->shape_evenodd == NULL ||
        geometry->shape_filled == NULL || geometry->shape_edge_start == NULL ||
        geometry->shape_edge_count == NULL) {
        svg_clip_destroy(clip);
        return NULL;
    }
    for (size_t i = 0U; i < 4U; ++i)
        geometry->edges[i] = (geom_edge){points[i], points[(i + 1U) % 4U]};
    geometry->shape_count = 1U;
    geometry->edge_count = 4U;
    geometry->shape_edge_count[0] = 4U;
    geometry->shape_filled[0] = true;
    clip->references = 1U;
    clip->user_inverse = user_inverse;
    clip->owner_node = owner_node;
    clip->owner_before_suffix = true;
    return clip;
}
