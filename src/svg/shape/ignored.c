#include "traverse_internal.h"

#include "reference.h"
#include "../css/css.h"

#include <string.h>

const char *svg_shape_ignored_kind_name(svg_shape_ignored_kind kind)
{
    static const char *const names[SVG_SHAPE_IGNORED_KIND_COUNT] = {
        "unknown-attribute", "meta", "doctype", "writing-mode"};
    return (unsigned)kind < SVG_SHAPE_IGNORED_KIND_COUNT ? names[kind] : "invalid";
}

static void note(svg_shape_document *document, svg_shape_ignored_kind kind,
                 size_t offset)
{
    svg_shape_ignored *entry = &document->ignored[kind];
    if (entry->count == 0U || offset < entry->first_offset)
        entry->first_offset = offset;
    if (entry->count != SIZE_MAX) entry->count++;
}

static bool ci_equal(base_bytes source, size_t at, const char *literal)
{
    size_t n = strlen(literal);
    if (at > source.len || n > source.len - at) return false;
    for (size_t i = 0U; i < n; ++i) {
        uint8_t a = source.ptr[at + i];
        if (a >= 'A' && a <= 'Z') a = (uint8_t)(a + ('a' - 'A'));
        if (a != (uint8_t)literal[i]) return false;
    }
    return true;
}

static bool ident_byte(uint8_t c)
{
    return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') ||
           (c >= '0' && c <= '9') || c == '-' || c == '_';
}

/* Notes every `writing-mode :` declaration name inside a CSS text span. */
static void scan_css_text(base_bytes source, svg_xml_span span,
                          svg_shape_document *document)
{
    static const char name[] = "writing-mode";
    size_t n = sizeof(name) - 1U;
    if (span.offset > source.len || span.length > source.len - span.offset) return;
    for (size_t i = span.offset; i + n <= span.offset + span.length; ++i) {
        if (!ci_equal(source, i, name)) continue;
        if (i > span.offset && ident_byte(source.ptr[i - 1U])) continue;
        size_t j = i + n;
        while (j < span.offset + span.length &&
               (source.ptr[j] == ' ' || source.ptr[j] == '\t' ||
                source.ptr[j] == '\r' || source.ptr[j] == '\n')) ++j;
        if (j < span.offset + span.length && source.ptr[j] == ':')
            note(document, SVG_SHAPE_IGNORED_WRITING_MODE, i);
    }
}

void svg_shape_ignored_scan(base_bytes source, const svg_xml_token *tokens,
                            size_t token_count, svg_shape_document *document)
{
    for (size_t i = 0U; i < token_count; ++i) {
        const svg_xml_token *token = &tokens[i];
        if (token->kind == SVG_XML_TOKEN_DOCTYPE) {
            note(document, SVG_SHAPE_IGNORED_DOCTYPE, token->source.offset);
        } else if (token->kind == SVG_XML_TOKEN_START_ELEMENT ||
                   token->kind == SVG_XML_TOKEN_EMPTY_ELEMENT) {
            if (svg_shape_traverse_element_name(source, token, "meta"))
                note(document, SVG_SHAPE_IGNORED_META, token->source.offset);
            if (token->kind == SVG_XML_TOKEN_START_ELEMENT &&
                svg_css_is_style_element(source, token)) {
                for (size_t j = i + 1U; j < token_count &&
                     !(tokens[j].kind == SVG_XML_TOKEN_END_ELEMENT &&
                       tokens[j].depth == token->depth); ++j)
                    if (tokens[j].kind == SVG_XML_TOKEN_TEXT ||
                        tokens[j].kind == SVG_XML_TOKEN_CDATA)
                        scan_css_text(source, tokens[j].value, document);
            }
        } else if (token->kind == SVG_XML_TOKEN_ATTRIBUTE &&
                   token->prefix.length == 0U) {
            if (svg_shape_span_eq(source, token->local_name, "writing-mode"))
                note(document, SVG_SHAPE_IGNORED_WRITING_MODE, token->name.offset);
            else if (svg_shape_span_eq(source, token->local_name, "style"))
                scan_css_text(source, token->value, document);
        }
    }
}

/* SVG attributes that change rendering. If the renderer does not support one of
 * these it still fails closed; anything else that is not namespaced has no
 * defined effect and is ignored. */
static bool affects_rendering(base_bytes source, svg_xml_span name)
{
    static const char *const known[] = {
        "alignment-baseline", "baseline-shift", "clip", "clip-rule",
        "color-interpolation", "color-rendering", "cursor", "direction",
        "dominant-baseline", "enable-background", "flood-color",
        "flood-opacity", "font", "font-size-adjust", "font-stretch",
        "font-variant", "glyph-orientation-horizontal",
        "glyph-orientation-vertical", "image-rendering", "kerning",
        "lighting-color", "overflow", "stop-color", "stop-opacity",
        "text-decoration", "text-rendering", "unicode-bidi", "transform-origin",
        "transform-box", "d", "x", "y", "width", "height", "rx", "ry", "cx",
        "cy", "r", "x1", "y1", "x2", "y2", "points", "dx", "dy", "rotate",
        "textLength", "lengthAdjust", "href", "viewBox", "pathLength",
        "preserveAspectRatio", "clipPathUnits", "maskUnits",
        "maskContentUnits", "patternUnits", "patternContentUnits",
        "patternTransform", "gradientUnits", "gradientTransform", "offset",
        "requiredFeatures", "requiredExtensions", "systemLanguage"};
    for (size_t i = 0U; i < sizeof(known) / sizeof(known[0]); ++i)
        if (svg_shape_span_eq(source, name, known[i])) return true;
    return false;
}

static bool first_visit(shape_traversal *traversal, size_t token)
{
    svg_shape_ignored_state *state = traversal->ignored;
    if (state->seen == NULL) {
        state->seen = svg_shape_calloc((traversal->token_count + 7U) / 8U, 1U);
        if (state->seen == NULL) return true;
    }
    uint8_t bit = (uint8_t)(1U << (token & 7U));
    if ((state->seen[token >> 3U] & bit) != 0U) return false;
    state->seen[token >> 3U] |= bit;
    return true;
}

bool svg_shape_traverse_attributes_supported(shape_traversal *traversal,
                                             size_t element, bool root)
{
    svg_xml_token *tokens = traversal->tokens;
    base_bytes source = traversal->source;
    size_t count = traversal->token_count;
    svg_xml_span kind = tokens[element].local_name;
    if (svg_reference_attributes_supported(source, tokens, count, element, root,
                                           kind)) return true;
    for (size_t i = element + 1U;
         i < count && tokens[i].kind == SVG_XML_TOKEN_ATTRIBUTE; ++i) {
        svg_xml_token single[2] = {tokens[element], tokens[i]};
        if (svg_reference_attributes_supported(source, single, 2U, 0U, root, kind))
            continue;
        if (tokens[i].prefix.length != 0U) return false;
        svg_xml_span name = tokens[i].local_name;
        bool writing_mode = svg_shape_span_eq(source, name, "writing-mode");
        if (!writing_mode && affects_rendering(source, name)) return false;
        if (svg_shape_span_eq(source, name, "xmlns")) continue;
        /* writing-mode is already reported by the document scan. */
        if (!writing_mode && first_visit(traversal, i))
            note(traversal->ignored->document, SVG_SHAPE_IGNORED_UNKNOWN_ATTRIBUTE,
                 tokens[i].name.offset);
    }
    return true;
}
