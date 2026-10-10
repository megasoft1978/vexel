#include "reference.h"
#include "../shape/shape_alloc.h"

#include <stdlib.h>
#include <string.h>

static bool name_is(base_bytes source, svg_xml_span name, const char *literal)
{
    size_t length = strlen(literal);
    return name.offset <= source.len && name.length == length &&
           length <= source.len - name.offset &&
           memcmp(source.ptr + name.offset, literal, length) == 0;
}

static uint64_t span_hash(base_bytes source, svg_xml_span span)
{
    uint64_t hash = UINT64_C(1469598103934665603);
    for (size_t i = 0U; i < span.length; ++i) {
        hash ^= source.ptr[span.offset + i];
        hash *= UINT64_C(1099511628211);
    }
    return hash;
}

static bool spans_equal(base_bytes source, svg_xml_span left,
                        svg_xml_span right)
{
    return left.length == right.length && left.offset <= source.len &&
           right.offset <= source.len && left.length <= source.len - left.offset &&
           right.length <= source.len - right.offset &&
           memcmp(source.ptr + left.offset, source.ptr + right.offset,
                  left.length) == 0;
}

static bool attr_value(base_bytes source, const svg_xml_token *tokens,
                       size_t count, size_t element, const char *local,
                       const char *prefix, svg_xml_span *value)
{
    for (size_t i = element + 1U;
         i < count && tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE; ++i) {
        if (name_is(source, tokens[i].local_name, local) &&
            ((prefix == NULL && tokens[i].prefix.length == 0U) ||
             (prefix != NULL && name_is(source, tokens[i].prefix, prefix)))) {
            if (value != NULL) *value = tokens[i].value;
            return true;
        }
    }
    return false;
}

static void set_failure(svg_shape_diagnostic *diagnostic,
                        svg_shape_error category, size_t offset)
{
    if (diagnostic != NULL)
        *diagnostic = (svg_shape_diagnostic){category, offset};
}

bool svg_reference_xlink_is_bound(base_bytes source,
                                  const svg_xml_token *tokens, size_t count)
{
    for (size_t i = 0U; i < count; ++i) {
        if (tokens[i].kind != SVG_XML_TOKEN_START_ELEMENT ||
            tokens[i].depth != 0U) continue;
        for (size_t j = i + 1U;
             j < count && tokens[j].kind == SVG_XML_TOKEN_ATTRIBUTE; ++j) {
            if (name_is(source, tokens[j].name, "xmlns:xlink") &&
                name_is(source, tokens[j].value,
                        "http://www.w3.org/1999/xlink")) return true;
        }
        return false;
    }
    return false;
}

svg_shape_error svg_reference_index_create(base_bytes source,
                                           const svg_xml_token *tokens,
                                           size_t count,
                                           svg_reference_index *index,
                                           svg_shape_diagnostic *diagnostic)
{
    if (index == NULL || !base_bytes_valid(source) ||
        (tokens == NULL && count != 0U)) {
        set_failure(diagnostic, SVG_SHAPE_ERROR_INVALID_ARGUMENT, 0U);
        return SVG_SHAPE_ERROR_INVALID_ARGUMENT;
    }
    *index = (svg_reference_index){0};
    if (diagnostic != NULL)
        *diagnostic = (svg_shape_diagnostic){SVG_SHAPE_ERROR_NONE, 0U};

    size_t id_count = 0U;
    for (size_t i = 0U; i < count; ++i) {
        if (tokens[i].kind != SVG_XML_TOKEN_START_ELEMENT &&
            tokens[i].kind != SVG_XML_TOKEN_EMPTY_ELEMENT) continue;
        if (!attr_value(source, tokens, count, i, "id", NULL, NULL)) continue;
        if (++id_count > SVG_REFERENCE_MAX_IDS) {
            set_failure(diagnostic, SVG_SHAPE_ERROR_LIMIT,
                        tokens[i].name.offset);
            return SVG_SHAPE_ERROR_LIMIT;
        }
    }
    if (id_count == 0U) return SVG_SHAPE_ERROR_NONE;

    /* The slot count is a compile-time constant. It never tracks source or ID
     * count, and the 16 MiB source limit keeps token indices within uint32_t. */
    index->slots = svg_shape_calloc(SVG_REFERENCE_ID_TABLE_CAPACITY,
                          sizeof(*index->slots));
    if (index->slots == NULL) {
        set_failure(diagnostic, SVG_SHAPE_ERROR_OOM, 0U);
        return SVG_SHAPE_ERROR_OOM;
    }
    (void)svg_shape_alloc_set_tag(index->slots,
                                  SVG_SHAPE_ALLOC_TAG_REFERENCE_INDEX);
    index->id_count = id_count;

    for (size_t i = 0U; i < count; ++i) {
        if (tokens[i].kind != SVG_XML_TOKEN_START_ELEMENT &&
            tokens[i].kind != SVG_XML_TOKEN_EMPTY_ELEMENT) continue;
        svg_xml_span id;
        if (!attr_value(source, tokens, count, i, "id", NULL, &id)) continue;
        if (id.length == 0U || id.offset > source.len ||
            id.length > source.len - id.offset || i >= UINT32_MAX) {
            set_failure(diagnostic, id.length == 0U ? SVG_SHAPE_ERROR_UNSUPPORTED :
                        SVG_SHAPE_ERROR_LIMIT, id.offset);
            svg_reference_index_destroy(index);
            return diagnostic != NULL ? diagnostic->category :
                (id.length == 0U ? SVG_SHAPE_ERROR_UNSUPPORTED : SVG_SHAPE_ERROR_LIMIT);
        }
        size_t slot = (size_t)span_hash(source, id) &
                      (SVG_REFERENCE_ID_TABLE_CAPACITY - 1U);
        while (index->slots[slot] != 0U) {
            size_t previous = (size_t)index->slots[slot] - 1U;
            svg_xml_span old_id;
            if (!attr_value(source, tokens, count, previous, "id", NULL,
                            &old_id)) {
                set_failure(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED,
                            tokens[i].name.offset);
                svg_reference_index_destroy(index);
                return SVG_SHAPE_ERROR_UNSUPPORTED;
            }
            if (spans_equal(source, old_id, id)) {
                set_failure(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, id.offset);
                svg_reference_index_destroy(index);
                return SVG_SHAPE_ERROR_UNSUPPORTED;
            }
            slot = (slot + 1U) & (SVG_REFERENCE_ID_TABLE_CAPACITY - 1U);
        }
        index->slots[slot] = (uint32_t)(i + 1U);
    }
    return SVG_SHAPE_ERROR_NONE;
}

void svg_reference_index_destroy(svg_reference_index *index)
{
    if (index == NULL) return;
    svg_shape_free(index->slots);
    *index = (svg_reference_index){0};
}

bool svg_reference_index_find(base_bytes source, const svg_xml_token *tokens,
                              size_t count, svg_reference_index *index,
                              svg_xml_span id, size_t *element)
{
    if (!base_bytes_valid(source) || tokens == NULL || index == NULL ||
        index->slots == NULL || element == NULL || id.length == 0U ||
        id.offset > source.len || id.length > source.len - id.offset) return false;
    size_t slot = (size_t)span_hash(source, id) &
                  (SVG_REFERENCE_ID_TABLE_CAPACITY - 1U);
    while (index->slots[slot] != 0U) {
        if (index->lookup_probes != SIZE_MAX) index->lookup_probes++;
        size_t candidate = (size_t)index->slots[slot] - 1U;
        if (candidate >= count) return false;
        svg_xml_span candidate_id;
        if (!attr_value(source, tokens, count, candidate, "id", NULL,
                        &candidate_id)) return false;
        if (spans_equal(source, candidate_id, id)) {
            *element = candidate;
            return true;
        }
        slot = (slot + 1U) & (SVG_REFERENCE_ID_TABLE_CAPACITY - 1U);
    }
    if (index->lookup_probes != SIZE_MAX) index->lookup_probes++;
    return false;
}

size_t svg_reference_index_storage_bytes(const svg_reference_index *index)
{
    return index != NULL && index->slots != NULL ?
        SVG_REFERENCE_ID_TABLE_CAPACITY * sizeof(*index->slots) : 0U;
}

size_t svg_reference_index_lookup_probes(const svg_reference_index *index)
{
    return index == NULL ? 0U : index->lookup_probes;
}

bool svg_reference_definition_end(base_bytes source,
                                  const svg_xml_token *tokens, size_t count,
                                  size_t start, size_t *end)
{
    if (!base_bytes_valid(source) || tokens == NULL || end == NULL ||
        start >= count ||
        (tokens[start].kind != SVG_XML_TOKEN_START_ELEMENT &&
         tokens[start].kind != SVG_XML_TOKEN_EMPTY_ELEMENT) ||
        tokens[start].prefix.length != 0U ||
        (!name_is(source, tokens[start].local_name, "defs") &&
         !name_is(source, tokens[start].local_name, "symbol"))) return false;
    if (tokens[start].kind == SVG_XML_TOKEN_EMPTY_ELEMENT) {
        *end = start;
        return true;
    }
    for (size_t i = start + 1U; i < count; ++i) {
        if (tokens[i].kind == SVG_XML_TOKEN_END_ELEMENT &&
            tokens[i].depth == tokens[start].depth) {
            *end = i;
            return true;
        }
    }
    return false;
}

bool svg_reference_attributes_supported(base_bytes source,
                                        const svg_xml_token *tokens,
                                        size_t count, size_t element, bool root,
                                        svg_xml_span kind)
{
    for (size_t i = element + 1U;
         i < count && tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE; ++i) {
        bool namespace_attribute = root &&
            name_is(source, tokens[i].prefix, "xmlns") &&
            name_is(source, tokens[i].local_name, "xlink");
        if (tokens[i].prefix.length != 0U && !namespace_attribute) return false;
        svg_xml_span n = tokens[i].local_name;
        bool common = name_is(source,n,"fill") || name_is(source,n,"fill-rule") ||
            name_is(source,n,"transform") || name_is(source,n,"clip-path") ||
            name_is(source,n,"fill-opacity") || name_is(source,n,"stroke-opacity") ||
            name_is(source,n,"opacity") || name_is(source,n,"visibility") ||
            name_is(source,n,"display") || name_is(source,n,"color") ||
            name_is(source,n,"stroke") || name_is(source,n,"stroke-width") ||
            name_is(source,n,"stroke-linecap") || name_is(source,n,"stroke-linejoin") ||
            name_is(source,n,"stroke-miterlimit") || name_is(source,n,"stroke-dasharray") ||
            name_is(source,n,"stroke-dashoffset");
        bool shape = name_is(source,kind,"rect") ?
            (name_is(source,n,"x")||name_is(source,n,"y")||name_is(source,n,"width")||name_is(source,n,"height")||name_is(source,n,"rx")||name_is(source,n,"ry")) :
            name_is(source,kind,"path") ? name_is(source,n,"d") :
            name_is(source,kind,"circle") ? (name_is(source,n,"cx")||name_is(source,n,"cy")||name_is(source,n,"r")) :
            name_is(source,kind,"ellipse") ? (name_is(source,n,"cx")||name_is(source,n,"cy")||name_is(source,n,"rx")||name_is(source,n,"ry")) :
            name_is(source,kind,"line") ? (name_is(source,n,"x1")||name_is(source,n,"y1")||name_is(source,n,"x2")||name_is(source,n,"y2")) :
            name_is(source,n,"points");
        bool metadata = name_is(source,n,"id") || name_is(source,n,"class") ||
                        name_is(source,n,"style");
        bool ok = root ? (metadata || common || name_is(source,n,"xmlns") ||
                          namespace_attribute || name_is(source,n,"version") ||
                          name_is(source,n,"width") || name_is(source,n,"height") ||
                          name_is(source,n,"viewBox") ||
                          name_is(source,n,"preserveAspectRatio")) :
            name_is(source,kind,"g") ? (metadata || common) :
            name_is(source,kind,"svg") ? (metadata || common ||
                name_is(source,n,"x") || name_is(source,n,"y") ||
                name_is(source,n,"width") || name_is(source,n,"height") ||
                name_is(source,n,"viewBox") ||
                name_is(source,n,"preserveAspectRatio")) :
            (metadata || common || shape);
        if (!ok) return false;
    }
    return true;
}

bool svg_reference_use_attributes_supported(base_bytes source,
                                            const svg_xml_token *tokens,
                                            size_t count, size_t element)
{
    for (size_t i = element + 1U;
         i < count && tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE; ++i) {
        svg_xml_span name = tokens[i].local_name;
        bool plain = tokens[i].prefix.length == 0U;
        if (!plain) {
            if (!name_is(source, tokens[i].prefix, "xlink") ||
                !name_is(source, name, "href") ||
                !svg_reference_xlink_is_bound(source, tokens, count)) return false;
            continue;
        }
        bool allowed = name_is(source,name,"href") || name_is(source,name,"id") ||
            name_is(source,name,"class") || name_is(source,name,"style") ||
            name_is(source,name,"x") || name_is(source,name,"y") ||
            name_is(source,name,"width") || name_is(source,name,"height") ||
            name_is(source,name,"preserveAspectRatio") ||
            name_is(source,name,"fill") || name_is(source,name,"fill-rule") ||
            name_is(source,name,"fill-opacity") ||
            name_is(source,name,"stroke-opacity") ||
            name_is(source,name,"opacity") || name_is(source,name,"visibility") ||
            name_is(source,name,"display") || name_is(source,name,"color") ||
            name_is(source,name,"transform") || name_is(source,name,"clip-path") ||
            name_is(source,name,"stroke") || name_is(source,name,"stroke-width") ||
            name_is(source,name,"stroke-linecap") || name_is(source,name,"stroke-linejoin") ||
            name_is(source,name,"stroke-miterlimit") || name_is(source,name,"stroke-dasharray") ||
            name_is(source,name,"stroke-dashoffset");
        if (!allowed) return false;
    }
    return true;
}
