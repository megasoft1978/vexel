#include "reference.h"

#include <stdlib.h>
#include <string.h>

#define REFERENCE_MAX_IDS 65536U

typedef struct id_entry {
    svg_xml_span name;
    size_t element;
    bool occupied;
} id_entry;

static bool name_is(base_bytes source, svg_xml_span name, const char *literal);

static uint64_t span_hash(base_bytes source, svg_xml_span span)
{
    uint64_t hash = UINT64_C(1469598103934665603);
    for (size_t i = 0; i < span.length; ++i) {
        hash ^= source.ptr[span.offset + i];
        hash *= UINT64_C(1099511628211);
    }
    return hash;
}

static bool spans_equal(base_bytes source, svg_xml_span left, svg_xml_span right)
{
    return left.length == right.length && left.offset <= source.len &&
           right.offset <= source.len && left.length <= source.len - left.offset &&
           right.length <= source.len - right.offset &&
           memcmp(source.ptr + left.offset, source.ptr + right.offset, left.length) == 0;
}

static bool attr_value(base_bytes source, const svg_xml_token *tokens, size_t count,
                       size_t element, const char *local, const char *prefix,
                       svg_xml_span *value)
{
    for (size_t i = element + 1U; i < count && tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE; ++i) {
        if (name_is(source, tokens[i].local_name, local) &&
            ((prefix == NULL && tokens[i].prefix.length == 0U) ||
             (prefix != NULL && name_is(source, tokens[i].prefix, prefix)))) {
            *value = tokens[i].value;
            return true;
        }
    }
    return false;
}

static bool xlink_is_bound(base_bytes source, const svg_xml_token *tokens, size_t count)
{
    for (size_t i = 0; i < count; ++i) {
        if (tokens[i].kind != SVG_XML_TOKEN_START_ELEMENT || tokens[i].depth != 0U) continue;
        for (size_t j = i + 1U; j < count && tokens[j].kind == SVG_XML_TOKEN_ATTRIBUTE; ++j) {
            if (name_is(source, tokens[j].name, "xmlns:xlink") &&
                name_is(source, tokens[j].value, "http://www.w3.org/1999/xlink")) return true;
        }
        return false;
    }
    return false;
}

static void set_failure(svg_shape_diagnostic *diagnostic, svg_shape_error category,
                        size_t offset)
{
    if (diagnostic != NULL) *diagnostic = (svg_shape_diagnostic){category, offset};
}

svg_shape_error svg_reference_validate(base_bytes source, const svg_xml_token *tokens,
                                       size_t count, svg_shape_diagnostic *diagnostic)
{
    size_t id_count = 0U;
    for (size_t i = 0; i < count; ++i) {
        if ((tokens[i].kind == SVG_XML_TOKEN_START_ELEMENT ||
             tokens[i].kind == SVG_XML_TOKEN_EMPTY_ELEMENT) &&
            attr_value(source, tokens, count, i, "id", NULL, &(svg_xml_span){0})) {
            if (++id_count > REFERENCE_MAX_IDS) {
                set_failure(diagnostic, SVG_SHAPE_ERROR_LIMIT, tokens[i].name.offset);
                return SVG_SHAPE_ERROR_LIMIT;
            }
        }
    }
    size_t capacity = 8U;
    while (capacity < id_count * 2U) {
        if (capacity > SIZE_MAX / 2U) {
            set_failure(diagnostic, SVG_SHAPE_ERROR_LIMIT, 0U);
            return SVG_SHAPE_ERROR_LIMIT;
        }
        capacity *= 2U;
    }
    id_entry *table = calloc(capacity, sizeof(*table));
    if (table == NULL) {
        set_failure(diagnostic, SVG_SHAPE_ERROR_OOM, 0U);
        return SVG_SHAPE_ERROR_OOM;
    }
    for (size_t i = 0; i < count; ++i) {
        if (tokens[i].kind != SVG_XML_TOKEN_START_ELEMENT &&
            tokens[i].kind != SVG_XML_TOKEN_EMPTY_ELEMENT) continue;
        svg_xml_span id;
        if (!attr_value(source, tokens, count, i, "id", NULL, &id)) continue;
        if (id.length == 0U) {
            free(table);
            set_failure(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, id.offset);
            return SVG_SHAPE_ERROR_UNSUPPORTED;
        }
        size_t slot = (size_t)span_hash(source, id) & (capacity - 1U);
        while (table[slot].occupied) {
            if (spans_equal(source, table[slot].name, id)) {
                free(table);
                set_failure(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, id.offset);
                return SVG_SHAPE_ERROR_UNSUPPORTED;
            }
            slot = (slot + 1U) & (capacity - 1U);
        }
        table[slot] = (id_entry){id, i, true};
    }

    for (size_t i = 0; i < count; ++i) {
        size_t definition_end;
        if ((tokens[i].kind == SVG_XML_TOKEN_START_ELEMENT ||
             tokens[i].kind == SVG_XML_TOKEN_EMPTY_ELEMENT) &&
            svg_reference_definition_end(source, tokens, count, i, &definition_end)) {
            i = definition_end;
            continue;
        }
        if (tokens[i].kind != SVG_XML_TOKEN_START_ELEMENT &&
            tokens[i].kind != SVG_XML_TOKEN_EMPTY_ELEMENT) continue;
        if (!name_is(source, tokens[i].local_name, "use") || tokens[i].prefix.length != 0U) continue;
        svg_xml_span href;
        bool plain = attr_value(source, tokens, count, i, "href", NULL, &href);
        bool xlink = attr_value(source, tokens, count, i, "href", "xlink", &href);
        if (plain == xlink || (xlink && !xlink_is_bound(source, tokens, count)) ||
            href.length < 2U || source.ptr[href.offset] != '#') {
            free(table);
            set_failure(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, tokens[i].name.offset);
            return SVG_SHAPE_ERROR_UNSUPPORTED;
        }
        svg_xml_span key = {href.offset + 1U, href.length - 1U};
        size_t slot = (size_t)span_hash(source, key) & (capacity - 1U);
        while (table[slot].occupied && !spans_equal(source, table[slot].name, key))
            slot = (slot + 1U) & (capacity - 1U);
        if (!table[slot].occupied) {
            free(table);
            set_failure(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, href.offset);
            return SVG_SHAPE_ERROR_UNSUPPORTED;
        }
        if (table[slot].element == i) {
            free(table);
            set_failure(diagnostic, SVG_SHAPE_ERROR_UNSUPPORTED, href.offset);
            return SVG_SHAPE_ERROR_UNSUPPORTED;
        }
        /* The source expander resolves this validated local reference before
         * the shape traversal. */
    }
    free(table);
    return SVG_SHAPE_ERROR_NONE;
}

static bool name_is(base_bytes source, svg_xml_span name, const char *literal)
{
    size_t length = strlen(literal);
    return name.offset <= source.len && length <= source.len - name.offset &&
           name.length == length && memcmp(source.ptr + name.offset, literal, length) == 0;
}

bool svg_reference_definition_end(base_bytes source, const svg_xml_token *tokens,
                                  size_t count, size_t start, size_t *end)
{
    if (!base_bytes_valid(source) || tokens == NULL || end == NULL || start >= count ||
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

bool svg_reference_attributes_supported(base_bytes source, const svg_xml_token *tokens,
                                        size_t count, size_t element, bool root,
                                        svg_xml_span kind)
{
    for (size_t i = element + 1U; i < count && tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE; ++i) {
        bool namespace_attribute = root &&
            name_is(source, tokens[i].prefix, "xmlns") &&
            name_is(source, tokens[i].local_name, "xlink");
        if (tokens[i].prefix.length != 0U && !namespace_attribute) return false;
        svg_xml_span n = tokens[i].local_name;
        bool common = name_is(source,n,"fill") || name_is(source,n,"fill-rule") || name_is(source,n,"transform") ||
            name_is(source,n,"clip-path") ||
            name_is(source,n,"stroke") || name_is(source,n,"stroke-width") || name_is(source,n,"stroke-linecap") ||
            name_is(source,n,"stroke-linejoin") || name_is(source,n,"stroke-miterlimit") ||
            name_is(source,n,"stroke-dasharray") || name_is(source,n,"stroke-dashoffset");
        bool shape = name_is(source,kind,"rect") ? (name_is(source,n,"x")||name_is(source,n,"y")||name_is(source,n,"width")||name_is(source,n,"height")) :
            name_is(source,kind,"path") ? name_is(source,n,"d") :
            name_is(source,kind,"circle") ? (name_is(source,n,"cx")||name_is(source,n,"cy")||name_is(source,n,"r")) :
            name_is(source,kind,"ellipse") ? (name_is(source,n,"cx")||name_is(source,n,"cy")||name_is(source,n,"rx")||name_is(source,n,"ry")) :
            name_is(source,kind,"line") ? (name_is(source,n,"x1")||name_is(source,n,"y1")||name_is(source,n,"x2")||name_is(source,n,"y2")) :
            name_is(source,n,"points");
        bool metadata = name_is(source,n,"id") || name_is(source,n,"class") || name_is(source,n,"style");
        bool paint = name_is(source,n,"fill") || name_is(source,n,"fill-rule") || name_is(source,n,"stroke") ||
            name_is(source,n,"stroke-width") || name_is(source,n,"stroke-linecap") || name_is(source,n,"stroke-linejoin") ||
            name_is(source,n,"stroke-miterlimit") || name_is(source,n,"stroke-dasharray") ||
            name_is(source,n,"stroke-dashoffset") || name_is(source,n,"clip-path");
        bool ok = root ? (metadata || paint || name_is(source,n,"xmlns") || namespace_attribute || name_is(source,n,"version") ||
                          name_is(source,n,"width") || name_is(source,n,"height") || name_is(source,n,"viewBox") || name_is(source,n,"transform")) :
            name_is(source,kind,"g") ? (metadata || paint || name_is(source,n,"transform")) : (metadata || common || shape);
        if (!ok) return false;
    }
    return true;
}
