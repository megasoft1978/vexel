#include "animation_internal.h"

#include <errno.h>
#include <math.h>
#include <stdlib.h>
#include <string.h>

#define MOTION_MAX_POINTS 4096U
#define MOTION_MAX_BYTES (64U * 1024U)

typedef struct path_reader {
    base_bytes source;
    size_t cursor;
    size_t end;
} path_reader;

static void skip_path_space(path_reader *reader)
{
    while (reader->cursor < reader->end) {
        uint8_t byte = reader->source.ptr[reader->cursor];
        if (byte != ' ' && byte != '\t' && byte != '\r' && byte != '\n' && byte != ',')
            break;
        reader->cursor++;
    }
}

static bool path_has_number(path_reader *reader)
{
    skip_path_space(reader);
    if (reader->cursor >= reader->end) return false;
    uint8_t byte = reader->source.ptr[reader->cursor];
    return (byte >= '0' && byte <= '9') || byte == '+' || byte == '-' || byte == '.';
}

static bool read_path_number(path_reader *reader, double *out)
{
    skip_path_space(reader);
    size_t start = reader->cursor;
    if (start >= reader->end) return false;
    if (reader->source.ptr[reader->cursor] == '+' ||
        reader->source.ptr[reader->cursor] == '-') reader->cursor++;
    bool digits = false;
    while (reader->cursor < reader->end &&
           reader->source.ptr[reader->cursor] >= '0' &&
           reader->source.ptr[reader->cursor] <= '9') {
        digits = true;
        reader->cursor++;
    }
    if (reader->cursor < reader->end && reader->source.ptr[reader->cursor] == '.') {
        reader->cursor++;
        while (reader->cursor < reader->end &&
               reader->source.ptr[reader->cursor] >= '0' &&
               reader->source.ptr[reader->cursor] <= '9') {
            digits = true;
            reader->cursor++;
        }
    }
    if (!digits) return false;
    if (reader->cursor < reader->end &&
        (reader->source.ptr[reader->cursor] == 'e' ||
         reader->source.ptr[reader->cursor] == 'E')) {
        reader->cursor++;
        if (reader->cursor < reader->end &&
            (reader->source.ptr[reader->cursor] == '+' ||
             reader->source.ptr[reader->cursor] == '-')) reader->cursor++;
        size_t exponent_start = reader->cursor;
        while (reader->cursor < reader->end &&
               reader->source.ptr[reader->cursor] >= '0' &&
               reader->source.ptr[reader->cursor] <= '9') reader->cursor++;
        if (reader->cursor == exponent_start) return false;
    }
    size_t length = reader->cursor - start;
    char buffer[128];
    if (length == 0U || length >= sizeof(buffer)) return false;
    memcpy(buffer, reader->source.ptr + start, length);
    buffer[length] = '\0';
    errno = 0;
    char *end = NULL;
    double value = strtod(buffer, &end);
    if (errno != 0 || end != buffer + length || !isfinite(value) ||
        fabs(value) > 1000000.0) return false;
    *out = value;
    return true;
}

static bool append_motion_point(svg_anim_track *track, size_t *capacity,
                                double x, double y)
{
    if (!isfinite(x) || !isfinite(y) || fabs(x) > 1000000.0 ||
        fabs(y) > 1000000.0 || track->motion_point_count >= MOTION_MAX_POINTS)
        return false;
    if (track->motion_point_count == *capacity) {
        size_t next = *capacity == 0U ? 16U : *capacity * 2U;
        if (next > MOTION_MAX_POINTS) next = MOTION_MAX_POINTS;
        if (next <= *capacity || next > SIZE_MAX / sizeof(*track->motion_points))
            return false;
        svg_anim_motion_point *grown = realloc(track->motion_points,
                                                next * sizeof(*grown));
        if (grown == NULL) return false;
        track->motion_points = grown;
        *capacity = next;
    }
    double distance = 0.0;
    if (track->motion_point_count != 0U) {
        const svg_anim_motion_point *previous =
            &track->motion_points[track->motion_point_count - 1U];
        distance = previous->distance + hypot(x - previous->x, y - previous->y);
        if (!isfinite(distance) || distance > 1000000000.0) return false;
    }
    track->motion_points[track->motion_point_count++] =
        (svg_anim_motion_point){x, y, distance};
    track->motion_length = distance;
    return true;
}

bool svg_anim_parse_motion_path(base_bytes source, svg_xml_span path,
                                svg_anim_track *track)
{
    if (track == NULL || path.length == 0U || path.length > MOTION_MAX_BYTES ||
        path.offset > source.len || path.length > source.len - path.offset) return false;
    path_reader reader = {source, path.offset, path.offset + path.length};
    size_t capacity = 0U;
    double x = 0.0, y = 0.0, start_x = 0.0, start_y = 0.0;
    char command = '\0';
    bool moved = false;
    while (true) {
        skip_path_space(&reader);
        if (reader.cursor == reader.end) break;
        uint8_t byte = source.ptr[reader.cursor];
        if ((byte >= 'A' && byte <= 'Z') || (byte >= 'a' && byte <= 'z')) {
            command = (char)byte;
            reader.cursor++;
            if (command != 'M' && command != 'm' && command != 'L' && command != 'l' &&
                command != 'H' && command != 'h' && command != 'V' && command != 'v' &&
                command != 'Z' && command != 'z') return false;
            if ((command == 'M' || command == 'm') && moved) return false;
            if (command == 'Z' || command == 'z') {
                if (!moved || !append_motion_point(track, &capacity, start_x, start_y))
                    return false;
                x = start_x;
                y = start_y;
                command = '\0';
                continue;
            }
        }
        if (command == '\0' || !path_has_number(&reader)) return false;
        bool relative = command >= 'a' && command <= 'z';
        if (command == 'M' || command == 'm' || command == 'L' || command == 'l') {
            double next_x = 0.0, next_y = 0.0;
            if (!read_path_number(&reader, &next_x) ||
                !read_path_number(&reader, &next_y)) return false;
            if (relative) {
                next_x += x;
                next_y += y;
            }
            x = next_x;
            y = next_y;
            if (!moved) {
                start_x = x;
                start_y = y;
                moved = true;
            }
            if (!append_motion_point(track, &capacity, x, y)) return false;
            if (command == 'M') command = 'L';
            else if (command == 'm') command = 'l';
        } else if (command == 'H' || command == 'h') {
            double next_x = 0.0;
            if (!read_path_number(&reader, &next_x)) return false;
            x = relative ? x + next_x : next_x;
            if (!moved) return false;
            if (!append_motion_point(track, &capacity, x, y)) return false;
        } else if (command == 'V' || command == 'v') {
            double next_y = 0.0;
            if (!read_path_number(&reader, &next_y)) return false;
            y = relative ? y + next_y : next_y;
            if (!moved) return false;
            if (!append_motion_point(track, &capacity, x, y)) return false;
        }
    }
    return moved && track->motion_point_count >= 2U &&
           track->motion_length > 0.0;
}

static bool attr_with_prefix(base_bytes source, const svg_xml_token *tokens,
                             size_t token_count, size_t element,
                             svg_xml_span *out)
{
    for (size_t index = element + 1U;
         index < token_count && tokens[index].kind == SVG_XML_TOKEN_ATTRIBUTE;
         ++index) {
        if (svg_anim_span_eq(source, tokens[index].local_name, "href") &&
            (tokens[index].prefix.length == 0U ||
             svg_anim_span_eq(source, tokens[index].prefix, "xlink"))) {
            *out = tokens[index].value;
            return true;
        }
    }
    return false;
}

static bool same_bytes(base_bytes source, svg_xml_span left,
                       svg_xml_span right)
{
    return left.length != 0U && left.length == right.length &&
        left.offset <= source.len && right.offset <= source.len &&
        left.length <= source.len - left.offset &&
        right.length <= source.len - right.offset &&
        memcmp(source.ptr + left.offset, source.ptr + right.offset,
               left.length) == 0;
}

bool svg_anim_resolve_motion_path(base_bytes source,
                                  const svg_xml_token *tokens,
                                  size_t token_count, size_t animation,
                                  svg_xml_span *out_path)
{
    if (tokens == NULL || out_path == NULL || animation >= token_count) return false;
    if (svg_anim_attr(source, tokens, token_count, animation, "path", out_path))
        return true;
    size_t mpath = SIZE_MAX;
    for (size_t index = animation + 1U; index < token_count; ++index) {
        if (tokens[index].kind == SVG_XML_TOKEN_END_ELEMENT &&
            tokens[index].depth == tokens[animation].depth) break;
        if (tokens[index].depth == tokens[animation].depth + 1U &&
            (tokens[index].kind == SVG_XML_TOKEN_START_ELEMENT ||
             tokens[index].kind == SVG_XML_TOKEN_EMPTY_ELEMENT) &&
            svg_anim_token_name(source, &tokens[index], "mpath")) {
            mpath = index;
            break;
        }
    }
    if (mpath == SIZE_MAX) return false;
    svg_xml_span href = {0U, 0U};
    if (!attr_with_prefix(source, tokens, token_count, mpath, &href) ||
        href.length < 2U || source.ptr[href.offset] != '#') return false;
    svg_xml_span id = {href.offset + 1U, href.length - 1U};
    size_t target = SIZE_MAX, matches = 0U;
    for (size_t index = 0U; index < token_count; ++index) {
        if ((tokens[index].kind != SVG_XML_TOKEN_START_ELEMENT &&
             tokens[index].kind != SVG_XML_TOKEN_EMPTY_ELEMENT) ||
            !svg_anim_token_name(source, &tokens[index], "path")) continue;
        svg_xml_span candidate_id;
        if (svg_anim_attr(source, tokens, token_count, index, "id", &candidate_id) &&
            same_bytes(source, id, candidate_id)) {
            target = index;
            matches++;
        }
    }
    if (matches != 1U) return false;
    return svg_anim_attr(source, tokens, token_count, target, "d", out_path);
}
