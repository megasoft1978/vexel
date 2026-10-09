#include "clip.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define CLIP_MAX_CHILDREN 64U
#define CLIP_MAX_REFERENCE_BYTES (64U * 1024U)
#define CLIP_MAX_JOB_REFERENCE_BYTES (512U * 1024U)
#define CLIP_MAX_EDGES 65536U

struct svg_shape_clip {
    svg_shape_document geometry;
    uint8_t *source;
    bool object_bounding_box;
    double bounds[4];
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
                                 size_t token_count, svg_xml_span value,
                                 const svg_shape_document *document,
                                 size_t *document_clip_bytes,
                                 size_t target_shape, svg_shape_clip **out,
                                 svg_shape_diagnostic *diagnostic)
{
    if (out == NULL || document == NULL || document_clip_bytes == NULL ||
        target_shape >= document->shape_count) {
        if (diagnostic != NULL) *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_INVALID_ARGUMENT, value.offset};
        return SVG_SHAPE_ERROR_INVALID_ARGUMENT;
    }
    *out = NULL;
    svg_xml_span id;
    if (!parse_reference(source, value, &id)) goto unsupported;
    size_t target = SIZE_MAX;
    for (size_t i = 0U; i < token_count; ++i) {
        if (tokens[i].kind != SVG_XML_TOKEN_START_ELEMENT &&
            tokens[i].kind != SVG_XML_TOKEN_EMPTY_ELEMENT) continue;
        svg_xml_span candidate;
        if (attr(source, tokens, token_count, i, "id", &candidate) &&
            candidate.length == id.length &&
            memcmp(source.ptr + candidate.offset, source.ptr + id.offset, id.length) == 0) {
            target = i;
            break;
        }
    }
    if (target == SIZE_MAX || !eq(source, tokens[target].local_name, "clipPath") ||
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
        if (attr(source, tokens, token_count, i, "clip-path", &id) ||
            attr(source, tokens, token_count, i, "style", &id) ||
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
    char prefix_pattern[192];
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
    if (total > CLIP_MAX_JOB_REFERENCE_BYTES ||
        *document_clip_bytes > CLIP_MAX_JOB_REFERENCE_BYTES - total) goto limit;
    uint8_t *scratch = malloc(total + 1U);
    svg_shape_clip *clip = calloc(1U, sizeof(*clip));
    if (scratch == NULL || clip == NULL) {
        free(scratch); free(clip);
        if (diagnostic != NULL) *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_OOM, value.offset};
        return SVG_SHAPE_ERROR_OOM;
    }
    memcpy(scratch, prefix_pattern, prefix_length);
    memcpy(scratch + prefix_length, source.ptr + child_start, child_length);
    memcpy(scratch + prefix_length + child_length, suffix, sizeof(suffix) - 1U);
    scratch[total] = 0U;
    clip->source = scratch;
    clip->object_bounding_box = object_box;
    size_t first = document->shape_edge_start[target_shape];
    size_t count = document->shape_edge_count[target_shape];
    if (count == 0U) { svg_clip_destroy(clip); goto unsupported; }
    clip->bounds[0] = clip->bounds[2] = document->edges[first].a.x;
    clip->bounds[1] = clip->bounds[3] = document->edges[first].a.y;
    for (size_t i = first; i < first + count; ++i) {
        geom_point points[2] = {document->edges[i].a, document->edges[i].b};
        for (size_t j = 0U; j < 2U; ++j) {
            if (points[j].x < clip->bounds[0]) clip->bounds[0] = points[j].x;
            if (points[j].y < clip->bounds[1]) clip->bounds[1] = points[j].y;
            if (points[j].x > clip->bounds[2]) clip->bounds[2] = points[j].x;
            if (points[j].y > clip->bounds[3]) clip->bounds[3] = points[j].y;
        }
    }
    if (object_box && (clip->bounds[2] <= clip->bounds[0] || clip->bounds[3] <= clip->bounds[1])) {
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
    *document_clip_bytes += total;
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
    if (clip->object_bounding_box) {
        x = (x - clip->bounds[0]) / (clip->bounds[2] - clip->bounds[0]);
        y = (y - clip->bounds[1]) / (clip->bounds[3] - clip->bounds[1]);
    }
    for (size_t i = 0U; i < clip->geometry.shape_count; ++i) {
        if (clip->geometry.shape_filled[i] && point_inside(&clip->geometry, i, x, y)) return true;
    }
    return false;
}

size_t svg_clip_work_units(const svg_shape_clip *clip)
{
    return clip == NULL ? 0U : clip->geometry.edge_count + clip->geometry.shape_count;
}

void svg_clip_destroy(svg_shape_clip *clip)
{
    if (clip == NULL) return;
    svg_shape_document_destroy(&clip->geometry);
    free(clip->source);
    free(clip);
}
