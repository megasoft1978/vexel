#include "animation_internal.h"

#include <errno.h>
#include <math.h>
#include <stdlib.h>
#include <string.h>

#define ANIM_MAX_PATH_VALUE (64U * 1024U)
#define ANIM_MAX_PATH_NUMBERS 4096U

typedef struct anim_element {
    size_t token;
    size_t source_offset;
    size_t source_length;
} anim_element;

bool span_eq(base_bytes source, svg_xml_span span, const char *text)
{
    size_t length = strlen(text);
    return span.offset <= source.len && span.length <= source.len - span.offset &&
           span.length == length && memcmp(source.ptr + span.offset, text, length) == 0;
}

bool token_name(base_bytes source, const svg_xml_token *token,
                       const char *name)
{
    return token->prefix.length == 0U && span_eq(source, token->local_name, name);
}

bool attr(base_bytes source, const svg_xml_token *tokens,
                 size_t token_count, size_t element, const char *name,
                 svg_xml_span *out)
{
    if (element >= token_count) return false;
    for (size_t index = element + 1U;
         index < token_count && tokens[index].kind == SVG_XML_TOKEN_ATTRIBUTE;
         ++index) {
        if (tokens[index].prefix.length == 0U &&
            span_eq(source, tokens[index].local_name, name)) {
            if (out != NULL) *out = tokens[index].value;
            return true;
        }
    }
    return false;
}

svg_xml_span attr_or_empty(base_bytes source,
                                  const svg_xml_token *tokens,
                                  size_t token_count, size_t element,
                                  const char *name)
{
    svg_xml_span value = {0U, 0U};
    (void)attr(source, tokens, token_count, element, name, &value);
    return value;
}

bool source_number(base_bytes source, svg_xml_span span, double *out)
{
    char buffer[128];
    if (span.length == 0U || span.length >= sizeof(buffer) ||
        span.offset > source.len || span.length > source.len - span.offset)
        return false;
    memcpy(buffer, source.ptr + span.offset, span.length);
    buffer[span.length] = '\0';
    errno = 0;
    char *end = NULL;
    double number = strtod(buffer, &end);
    if (errno != 0 || end != buffer + span.length || !isfinite(number)) return false;
    *out = number;
    return true;
}

bool time_value(base_bytes source, svg_xml_span span, double *out)
{
    char buffer[128];
    if (span.length == 0U || span.length >= sizeof(buffer) ||
        span.offset > source.len || span.length > source.len - span.offset)
        return false;
    memcpy(buffer, source.ptr + span.offset, span.length);
    buffer[span.length] = '\0';
    size_t length = span.length;
    double multiplier = 1.0;
    if (length >= 2U && buffer[length - 2U] == 'm' && buffer[length - 1U] == 's') {
        buffer[length - 2U] = '\0';
        multiplier = 0.001;
    } else if (length >= 1U && buffer[length - 1U] == 's') {
        buffer[length - 1U] = '\0';
    }
    errno = 0;
    char *end = NULL;
    double value = strtod(buffer, &end);
    if (errno != 0 || end == buffer || *end != '\0' || !isfinite(value) ||
        value < 0.0 || value > SVG_ANIM_MAX_TIMELINE_SECONDS / multiplier)
        return false;
    *out = value * multiplier;
    return isfinite(*out) && *out <= SVG_ANIM_MAX_TIMELINE_SECONDS;
}

static bool color_hex(uint8_t nibble, uint8_t *out)
{
    if (nibble >= '0' && nibble <= '9') *out = (uint8_t)(nibble - '0');
    else if (nibble >= 'a' && nibble <= 'f') *out = (uint8_t)(nibble - 'a' + 10U);
    else if (nibble >= 'A' && nibble <= 'F') *out = (uint8_t)(nibble - 'A' + 10U);
    else return false;
    return true;
}

static bool parse_color(base_bytes source, svg_xml_span span, uint8_t out[4])
{
    if (span.offset > source.len || span.length > source.len - span.offset) return false;
    const uint8_t *text = source.ptr + span.offset;
    size_t length = span.length;
    if (length == 4U && memcmp(text, "none", 4U) == 0) {
        memset(out, 0, 4U);
        return true;
    }
    if (length == 4U && text[0] == '#') {
        for (size_t channel = 0U; channel < 3U; ++channel) {
            uint8_t nibble = 0U;
            if (!color_hex(text[channel + 1U], &nibble)) return false;
            out[channel] = (uint8_t)(nibble * 17U);
        }
        out[3] = 255U;
        return true;
    }
    if ((length == 7U || length == 9U) && text[0] == '#') {
        size_t channels = length == 9U ? 4U : 3U;
        for (size_t channel = 0U; channel < channels; ++channel) {
            uint8_t high = 0U, low = 0U;
            if (!color_hex(text[1U + channel * 2U], &high) ||
                !color_hex(text[2U + channel * 2U], &low)) return false;
            out[channel] = (uint8_t)(high * 16U + low);
        }
        if (channels == 3U) out[3] = 255U;
        return true;
    }
    struct named_color { const char *name; uint8_t rgba[4]; };
    static const struct named_color named[] = {
        {"black", {0U, 0U, 0U, 255U}}, {"white", {255U, 255U, 255U, 255U}},
        {"red", {255U, 0U, 0U, 255U}}, {"green", {0U, 128U, 0U, 255U}},
        {"blue", {0U, 0U, 255U, 255U}}, {"yellow", {255U, 255U, 0U, 255U}},
        {"cyan", {0U, 255U, 255U, 255U}}, {"magenta", {255U, 0U, 255U, 255U}},
        {"gray", {128U, 128U, 128U, 255U}}, {"grey", {128U, 128U, 128U, 255U}},
        {"transparent", {0U, 0U, 0U, 0U}}
    };
    for (size_t index = 0U; index < sizeof(named) / sizeof(named[0]); ++index) {
        size_t name_length = strlen(named[index].name);
        if (length == name_length && memcmp(text, named[index].name, length) == 0) {
            memcpy(out, named[index].rgba, 4U);
            return true;
        }
    }
    return false;
}

static bool parse_transform_value(base_bytes source, svg_xml_span span,
                                  const char *type, svg_anim_value *out)
{
    if (span.offset > source.len || span.length > source.len - span.offset ||
        span.length == 0U || span.length > 256U) return false;
    char buffer[257];
    memcpy(buffer, source.ptr + span.offset, span.length);
    buffer[span.length] = '\0';
    double values[4] = {0.0, 0.0, 0.0, 0.0};
    size_t count = 0U;
    char *cursor = buffer;
    while (*cursor != '\0') {
        while (*cursor == ' ' || *cursor == '\t' || *cursor == '\r' ||
               *cursor == '\n' || *cursor == ',') cursor++;
        if (*cursor == '\0') break;
        if (count >= 4U) return false;
        errno = 0;
        char *end = NULL;
        double number = strtod(cursor, &end);
        if (errno != 0 || end == cursor || !isfinite(number)) return false;
        values[count++] = number;
        cursor = end;
    }
    size_t minimum = strcmp(type, "rotate") == 0 ? 1U : 1U;
    size_t maximum = strcmp(type, "rotate") == 0 ? 3U :
                     (strcmp(type, "translate") == 0 ||
                      strcmp(type, "scale") == 0 ? 2U : 1U);
    if (count < minimum || count > maximum) return false;
    out->kind = SVG_ANIM_VALUE_TRANSFORM;
    memcpy(out->number, values, sizeof(values));
    out->number[3] = (double)count;
    out->raw_offset = span.offset;
    out->raw_length = span.length;
    return true;
}

static bool parse_path_shape(base_bytes source, svg_xml_span span,
                             uint8_t *commands, size_t command_capacity,
                             size_t *command_count, size_t *number_count)
{
    if (span.offset > source.len || span.length > source.len - span.offset ||
        span.length == 0U || span.length > ANIM_MAX_PATH_VALUE) return false;
    size_t c_count = 0U, n_count = 0U;
    size_t cursor = span.offset, limit = span.offset + span.length;
    while (cursor < limit) {
        uint8_t byte = source.ptr[cursor];
        if ((byte >= 'A' && byte <= 'Z') || (byte >= 'a' && byte <= 'z')) {
            if (byte != 'M' && byte != 'm' && byte != 'L' && byte != 'l' &&
                byte != 'H' && byte != 'h' && byte != 'V' && byte != 'v' &&
                byte != 'C' && byte != 'c' && byte != 'S' && byte != 's' &&
                byte != 'Q' && byte != 'q' && byte != 'T' && byte != 't' &&
                byte != 'A' && byte != 'a' && byte != 'Z' && byte != 'z') return false;
            if (c_count >= command_capacity) return false;
            commands[c_count++] = byte;
            cursor++;
            continue;
        }
        if (byte == ' ' || byte == '\t' || byte == '\r' || byte == '\n' || byte == ',') {
            cursor++;
            continue;
        }
        size_t start = cursor;
        if (byte == '+' || byte == '-') cursor++;
        bool digits = false;
        while (cursor < limit && source.ptr[cursor] >= '0' && source.ptr[cursor] <= '9') {
            digits = true;
            cursor++;
        }
        if (cursor < limit && source.ptr[cursor] == '.') {
            cursor++;
            while (cursor < limit && source.ptr[cursor] >= '0' && source.ptr[cursor] <= '9') {
                digits = true;
                cursor++;
            }
        }
        if (!digits) return false;
        if (cursor < limit && (source.ptr[cursor] == 'e' || source.ptr[cursor] == 'E')) {
            cursor++;
            if (cursor < limit && (source.ptr[cursor] == '+' || source.ptr[cursor] == '-')) cursor++;
            size_t exponent = cursor;
            while (cursor < limit && source.ptr[cursor] >= '0' && source.ptr[cursor] <= '9') cursor++;
            if (cursor == exponent) return false;
        }
        if (cursor <= start || ++n_count > ANIM_MAX_PATH_NUMBERS) return false;
    }
    *command_count = c_count;
    *number_count = n_count;
    return c_count != 0U;
}

bool path_structures_match(base_bytes source, svg_xml_span left,
                                  svg_xml_span right)
{
    uint8_t left_commands[1024], right_commands[1024];
    size_t lc = 0U, rc = 0U, ln = 0U, rn = 0U;
    return parse_path_shape(source, left, left_commands, sizeof(left_commands), &lc, &ln) &&
           parse_path_shape(source, right, right_commands, sizeof(right_commands), &rc, &rn) &&
           lc == rc && ln == rn && memcmp(left_commands, right_commands, lc) == 0;
}

svg_anim_attribute attribute_kind(base_bytes source, svg_xml_span name,
                                         bool *is_transform, bool *is_motion)
{
    static const struct { const char *name; svg_anim_attribute value; } attrs[] = {
        {"x", SVG_ANIM_ATTR_X}, {"y", SVG_ANIM_ATTR_Y},
        {"width", SVG_ANIM_ATTR_WIDTH}, {"height", SVG_ANIM_ATTR_HEIGHT},
        {"cx", SVG_ANIM_ATTR_CX}, {"cy", SVG_ANIM_ATTR_CY},
        {"r", SVG_ANIM_ATTR_R}, {"rx", SVG_ANIM_ATTR_RX},
        {"ry", SVG_ANIM_ATTR_RY}, {"opacity", SVG_ANIM_ATTR_OPACITY},
        {"fill-opacity", SVG_ANIM_ATTR_FILL_OPACITY},
        {"stroke-opacity", SVG_ANIM_ATTR_STROKE_OPACITY},
        {"fill", SVG_ANIM_ATTR_FILL}, {"stroke", SVG_ANIM_ATTR_STROKE},
        {"stroke-width", SVG_ANIM_ATTR_STROKE_WIDTH},
        {"stroke-dashoffset", SVG_ANIM_ATTR_STROKE_DASHOFFSET},
        {"d", SVG_ANIM_ATTR_D}
    };
    *is_transform = span_eq(source, name, "transform");
    *is_motion = false;
    for (size_t index = 0U; index < sizeof(attrs) / sizeof(attrs[0]); ++index)
        if (span_eq(source, name, attrs[index].name)) return attrs[index].value;
    return SVG_ANIM_ATTR_MOTION;
}

const char *svg_anim_attribute_name(svg_anim_attribute attribute)
{
    static const char *names[] = {
        "x", "y", "width", "height", "cx", "cy", "r", "rx", "ry",
        "opacity", "fill-opacity", "stroke-opacity", "fill", "stroke",
        "stroke-width", "stroke-dashoffset", "d", "transform", "motion"
    };
    return (size_t)attribute < sizeof(names) / sizeof(names[0]) ?
           names[attribute] : "unknown";
}

const char *svg_anim_diagnostic_name(svg_anim_diagnostic_kind kind)
{
    static const char *names[] = {
        "unsupported-element", "unsupported-attribute", "unsupported-timing",
        "unsupported-composition", "unsupported-path", "css-animation",
        "invalid-track"
    };
    return (size_t)kind < sizeof(names) / sizeof(names[0]) ? names[kind] : "unknown";
}

void add_diagnostic(svg_anim_document *document,
                           svg_anim_diagnostic_kind kind, size_t offset)
{
    if (document->diagnostic_count >= SVG_ANIM_MAX_DIAGNOSTICS) return;
    document->diagnostics[document->diagnostic_count++] =
        (svg_anim_diagnostic){kind, offset};
}

bool element_span(const svg_xml_token *tokens, size_t token_count,
                         size_t element, size_t *offset, size_t *length)
{
    if (element >= token_count) return false;
    size_t start = tokens[element].source.offset;
    size_t end = tokens[element].source.offset + tokens[element].source.length;
    if (tokens[element].kind == SVG_XML_TOKEN_START_ELEMENT) {
        size_t depth = tokens[element].depth;
        bool found = false;
        for (size_t index = element + 1U; index < token_count; ++index) {
            if (tokens[index].kind == SVG_XML_TOKEN_END_ELEMENT &&
                tokens[index].depth == depth) {
                end = tokens[index].source.offset + tokens[index].source.length;
                found = true;
                break;
            }
        }
        if (!found) return false;
    }
    if (end < start) return false;
    *offset = start;
    *length = end - start;
    return true;
}

bool parse_sync_time(base_bytes source, svg_xml_span span,
                            svg_anim_track *track, bool is_begin)
{
    if (span_eq(source, span, "indefinite")) {
        if (is_begin) track->begin_indefinite = true;
        else track->has_end = false;
        return true;
    }
    if (time_value(source, span, is_begin ? &track->begin_offset : &track->end_seconds)) {
        if (!is_begin) track->has_end = true;
        return true;
    }
    size_t limit = span.offset + span.length;
    size_t dot = span.offset;
    while (dot < limit && source.ptr[dot] != '.') dot++;
    if (dot == span.offset || dot == limit) return false;
    size_t suffix_end = dot + 1U;
    while (suffix_end < limit && source.ptr[suffix_end] != '+' &&
           source.ptr[suffix_end] != '-') suffix_end++;
    svg_xml_span suffix = {dot + 1U, suffix_end - dot - 1U};
    bool sync_end = span_eq(source, suffix, "end");
    if (!sync_end && !span_eq(source, suffix, "begin")) return false;
    svg_xml_span id = {span.offset, dot - span.offset};
    double offset = 0.0;
    if (suffix_end < limit) {
        bool negative = source.ptr[suffix_end] == '-';
        svg_xml_span offset_span = {suffix_end + 1U, limit - suffix_end - 1U};
        if (!time_value(source, offset_span, &offset)) return false;
        if (negative) offset = -offset;
    }
    if (id.length == 0U) return false;
    if (is_begin) {
        track->has_begin_sync = true;
        track->begin_sync_end = sync_end;
        track->begin_offset = offset;
        track->begin_sync_track = SIZE_MAX;
    } else {
        track->has_end = true;
        track->end_is_sync = true;
        track->end_sync_track = SIZE_MAX;
        track->end_seconds = offset;
    }
    if (is_begin) track->begin_sync_id = id;
    else track->end_sync_id = id;
    return true;
}

bool parse_value(base_bytes source, svg_xml_span span,
                        svg_anim_attribute attribute, svg_anim_kind kind,
                        const char *transform_type, svg_anim_value *out)
{
    *out = (svg_anim_value){0};
    if (kind == SVG_ANIM_KIND_TRANSFORM) {
        return parse_transform_value(source, span, transform_type, out);
    }
    if (kind == SVG_ANIM_KIND_MOTION) {
        out->kind = SVG_ANIM_VALUE_MOTION;
        out->raw_offset = span.offset;
        out->raw_length = span.length;
        return true;
    }
    if (attribute == SVG_ANIM_ATTR_D) {
        out->kind = SVG_ANIM_VALUE_PATH;
        out->raw_offset = span.offset;
        out->raw_length = span.length;
        return true;
    }
    if (attribute == SVG_ANIM_ATTR_FILL || attribute == SVG_ANIM_ATTR_STROKE) {
        out->kind = SVG_ANIM_VALUE_COLOR;
        out->raw_offset = span.offset;
        out->raw_length = span.length;
        return parse_color(source, span, out->rgba);
    }
    out->kind = SVG_ANIM_VALUE_NUMBER;
    return source_number(source, span, &out->number[0]);
}

bool split_value_spans(base_bytes source, svg_xml_span values,
                              svg_xml_span *out, size_t capacity,
                              size_t *out_count)
{
    if (values.offset > source.len || values.length > source.len - values.offset) return false;
    size_t count = 0U, cursor = values.offset, end = values.offset + values.length;
    while (cursor <= end) {
        size_t item_start = cursor;
        while (cursor < end && source.ptr[cursor] != ';') cursor++;
        size_t item_end = cursor;
        while (item_start < item_end && (source.ptr[item_start] == ' ' ||
               source.ptr[item_start] == '\t' || source.ptr[item_start] == '\r' ||
               source.ptr[item_start] == '\n')) item_start++;
        while (item_end > item_start && (source.ptr[item_end - 1U] == ' ' ||
               source.ptr[item_end - 1U] == '\t' || source.ptr[item_end - 1U] == '\r' ||
               source.ptr[item_end - 1U] == '\n')) item_end--;
        if (item_start == item_end || count >= capacity) return false;
        out[count++] = (svg_xml_span){item_start, item_end - item_start};
        if (cursor == end) break;
        cursor++;
    }
    *out_count = count;
    return count != 0U;
}

static bool is_color_attribute(svg_anim_attribute attribute)
{
    return attribute == SVG_ANIM_ATTR_FILL || attribute == SVG_ANIM_ATTR_STROKE;
}

bool base_attribute_value(base_bytes source, const svg_xml_token *tokens,
                                 size_t token_count, size_t target,
                                 svg_anim_attribute attribute, svg_anim_kind kind,
                                 const char *transform_type, svg_anim_value *out)
{
    if (kind == SVG_ANIM_KIND_TRANSFORM) {
        *out = (svg_anim_value){.kind = SVG_ANIM_VALUE_TRANSFORM};
        if (strcmp(transform_type, "scale") == 0) {
            out->number[0] = 1.0;
            out->number[1] = 1.0;
            out->number[3] = 2.0;
        } else if (strcmp(transform_type, "translate") == 0) {
            out->number[3] = 2.0;
        } else {
            out->number[3] = 1.0;
        }
        return true;
    }
    if (kind == SVG_ANIM_KIND_MOTION) {
        *out = (svg_anim_value){.kind = SVG_ANIM_VALUE_MOTION};
        return true;
    }
    svg_xml_span raw;
    if (!attr(source, tokens, token_count, target,
              svg_anim_attribute_name(attribute), &raw)) {
        if (is_color_attribute(attribute)) {
            *out = (svg_anim_value){.kind = SVG_ANIM_VALUE_COLOR,
                .rgba = {0U, 0U, 0U, 0U}};
            if (attribute == SVG_ANIM_ATTR_FILL) out->rgba[3] = 255U;
            return true;
        }
        *out = (svg_anim_value){.kind = SVG_ANIM_VALUE_NUMBER};
        if (attribute == SVG_ANIM_ATTR_OPACITY ||
            attribute == SVG_ANIM_ATTR_FILL_OPACITY ||
            attribute == SVG_ANIM_ATTR_STROKE_OPACITY)
            out->number[0] = 1.0;
        return true;
    }
    return parse_value(source, raw, attribute, kind, transform_type, out);
}

bool parse_key_times(base_bytes source, svg_xml_span span,
                            svg_anim_track *track)
{
    if (span.length == 0U) return true;
    svg_xml_span pieces[SVG_ANIM_MAX_VALUES];
    size_t count = 0U;
    if (!split_value_spans(source, span, pieces, SVG_ANIM_MAX_VALUES, &count) ||
        count != track->key_count) return false;
    double previous = -1.0;
    for (size_t index = 0U; index < count; ++index) {
        double value = 0.0;
        if (!source_number(source, pieces[index], &value) || value < 0.0 ||
            value > 1.0 || (index != 0U && value <= previous)) return false;
        track->keys[index].time = value;
        previous = value;
    }
    return track->keys[0].time == 0.0 &&
           (track->kind == SVG_ANIM_KIND_SET ||
            track->keys[count - 1U].time == 1.0);
}

bool parse_splines(base_bytes source, svg_xml_span span,
                          svg_anim_track *track)
{
    if (track->calc_mode != SVG_ANIM_CALC_SPLINE) return span.length == 0U;
    size_t expected = track->key_count > 0U ? track->key_count - 1U : 0U;
    if (expected == 0U || expected > SVG_ANIM_MAX_VALUES) return false;
    track->splines = calloc(expected, sizeof(*track->splines));
    if (track->splines == NULL) return false;
    track->spline_count = expected;
    svg_xml_span pieces[SVG_ANIM_MAX_VALUES];
    size_t count = 0U;
    if (!split_value_spans(source, span, pieces, SVG_ANIM_MAX_VALUES, &count) ||
        count != expected) return false;
    for (size_t index = 0U; index < count; ++index) {
        char buffer[256];
        if (pieces[index].length >= sizeof(buffer)) return false;
        memcpy(buffer, source.ptr + pieces[index].offset, pieces[index].length);
        buffer[pieces[index].length] = '\0';
        char *cursor = buffer;
        for (size_t coordinate = 0U; coordinate < 4U; ++coordinate) {
            while (*cursor == ' ' || *cursor == '\t' || *cursor == '\r' ||
                   *cursor == '\n' || *cursor == ',') cursor++;
            errno = 0;
            char *end = NULL;
            double value = strtod(cursor, &end);
            if (errno != 0 || end == cursor || !isfinite(value) || value < 0.0 ||
                value > 1.0) return false;
            track->splines[index][coordinate] = value;
            cursor = end;
        }
        while (*cursor == ' ' || *cursor == '\t' || *cursor == '\r' ||
               *cursor == '\n' || *cursor == ',') cursor++;
        if (*cursor != '\0') return false;
    }
    return true;
}

bool parse_repeat(base_bytes source, svg_xml_span span,
                         svg_anim_track *track)
{
    if (span.length == 0U) {
        track->repeat_count = 1.0;
        return true;
    }
    if (span_eq(source, span, "indefinite")) {
        track->repeat_indefinite = true;
        track->repeat_count = INFINITY;
        return true;
    }
    return source_number(source, span, &track->repeat_count) &&
           track->repeat_count > 0.0 && track->repeat_count <= 100000.0;
}

bool parse_calc_fill(base_bytes source, const svg_xml_token *tokens,
                            size_t token_count, size_t element,
                            svg_anim_track *track, size_t *diagnostic_offset,
                            svg_anim_document *document)
{
    svg_xml_span span = attr_or_empty(source, tokens, token_count, element, "calcMode");
    if (span.length == 0U || span_eq(source, span, "linear"))
        track->calc_mode = SVG_ANIM_CALC_LINEAR;
    else if (span_eq(source, span, "discrete"))
        track->calc_mode = SVG_ANIM_CALC_DISCRETE;
    else if (span_eq(source, span, "spline"))
        track->calc_mode = SVG_ANIM_CALC_SPLINE;
    else if (span_eq(source, span, "paced"))
        track->calc_mode = SVG_ANIM_CALC_PACED;
    else {
        *diagnostic_offset = span.offset;
        add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_ATTRIBUTE, span.offset);
        return false;
    }
    span = attr_or_empty(source, tokens, token_count, element, "fill");
    if (span.length == 0U || span_eq(source, span, "remove"))
        track->fill_mode = SVG_ANIM_FILL_REMOVE;
    else if (span_eq(source, span, "freeze"))
        track->fill_mode = SVG_ANIM_FILL_FREEZE;
    else {
        *diagnostic_offset = span.offset;
        add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_ATTRIBUTE, span.offset);
        return false;
    }
    return true;
}

