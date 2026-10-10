#include "animation_internal.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

static bool parse_track(base_bytes source, const svg_xml_token *tokens,
                        size_t token_count, size_t element, size_t parent,
                        svg_anim_document *document, svg_anim_track *out)
{
    memset(out, 0, sizeof(*out));
    out->begin_sync_track = SIZE_MAX;
    out->end_sync_track = SIZE_MAX;
    out->repeat_duration = INFINITY;
    out->begin_indefinite = false;
    out->supported = true;
    const svg_xml_token *tag = &tokens[element];
    if (tag->prefix.length != 0U) {
        add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_ELEMENT, tag->name.offset);
        return false;
    }
    if (token_name(source, tag, "animate")) out->kind = SVG_ANIM_KIND_ANIMATE;
    else if (token_name(source, tag, "animateTransform")) out->kind = SVG_ANIM_KIND_TRANSFORM;
    else if (token_name(source, tag, "set")) out->kind = SVG_ANIM_KIND_SET;
    else if (token_name(source, tag, "animateMotion")) out->kind = SVG_ANIM_KIND_MOTION;
    else {
        add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_ELEMENT, tag->name.offset);
        return false;
    }
    if (parent >= token_count) {
        add_diagnostic(document, SVG_ANIM_DIAG_INVALID_TRACK, tag->name.offset);
        return false;
    }
    if (!element_span(tokens, token_count, parent, &out->target_offset,
                      &out->target_length) ||
        !element_span(tokens, token_count, element, &out->element_offset,
                      &out->element_length)) {
        add_diagnostic(document, SVG_ANIM_DIAG_INVALID_TRACK, tag->name.offset);
        return false;
    }
    svg_xml_span id = attr_or_empty(source, tokens, token_count, element, "id");
    out->id = id;
    if (out->kind == SVG_ANIM_KIND_MOTION) {
        out->attribute = SVG_ANIM_ATTR_MOTION;
    } else {
        svg_xml_span attribute_name = attr_or_empty(source, tokens, token_count,
                                                     element, "attributeName");
        if (attribute_name.length == 0U) {
            add_diagnostic(document, SVG_ANIM_DIAG_INVALID_TRACK, tag->name.offset);
            return false;
        }
        bool transform = false, motion = false;
        out->attribute = attribute_kind(source, attribute_name, &transform, &motion);
        if (out->kind == SVG_ANIM_KIND_TRANSFORM) {
            if (!transform) {
                add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_ATTRIBUTE,
                               attribute_name.offset);
                return false;
            }
        } else if (transform || out->attribute == SVG_ANIM_ATTR_MOTION) {
            add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_ATTRIBUTE,
                           attribute_name.offset);
            return false;
        }
    }
    svg_xml_span transform_type_span = attr_or_empty(source, tokens, token_count,
                                                       element, "type");
    svg_xml_span attribute_type = attr_or_empty(source, tokens, token_count,
                                                 element, "attributeType");
    if (attribute_type.length != 0U && !span_eq(source, attribute_type, "XML")) {
        add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_ATTRIBUTE,
                       attribute_type.offset);
        return false;
    }
    char transform_type[16] = "translate";
    if (out->kind == SVG_ANIM_KIND_TRANSFORM) {
        if (transform_type_span.length == 0U || transform_type_span.length >= sizeof(transform_type)) {
            add_diagnostic(document, SVG_ANIM_DIAG_INVALID_TRACK, tag->name.offset);
            return false;
        }
        memcpy(transform_type, source.ptr + transform_type_span.offset,
               transform_type_span.length);
        transform_type[transform_type_span.length] = '\0';
        if (strcmp(transform_type, "translate") != 0 && strcmp(transform_type, "scale") != 0 &&
            strcmp(transform_type, "rotate") != 0 && strcmp(transform_type, "skewX") != 0 &&
            strcmp(transform_type, "skewY") != 0) {
            add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_ATTRIBUTE,
                           transform_type_span.offset);
            return false;
        }
        if (strcmp(transform_type, "translate") == 0)
            out->transform_type = SVG_ANIM_TRANSFORM_TRANSLATE;
        else if (strcmp(transform_type, "scale") == 0)
            out->transform_type = SVG_ANIM_TRANSFORM_SCALE;
        else if (strcmp(transform_type, "rotate") == 0)
            out->transform_type = SVG_ANIM_TRANSFORM_ROTATE;
        else if (strcmp(transform_type, "skewX") == 0)
            out->transform_type = SVG_ANIM_TRANSFORM_SKEW_X;
        else out->transform_type = SVG_ANIM_TRANSFORM_SKEW_Y;
    }
    size_t ignored_offset = 0U;
    if (!parse_calc_fill(source, tokens, token_count, element, out,
                         &ignored_offset, document)) return false;
    svg_xml_span additive = attr_or_empty(source, tokens, token_count, element, "additive");
    if (additive.length != 0U && !span_eq(source, additive, "replace")) {
        if (span_eq(source, additive, "sum")) out->additive_sum = true;
        else {
            add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_COMPOSITION,
                           additive.offset);
            return false;
        }
    }
    svg_xml_span accumulate = attr_or_empty(source, tokens, token_count, element,
                                             "accumulate");
    if (accumulate.length != 0U && !span_eq(source, accumulate, "none")) {
        if (span_eq(source, accumulate, "sum")) out->accumulate = true;
        else {
            add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_COMPOSITION,
                           accumulate.offset);
            return false;
        }
    }
    if (out->kind == SVG_ANIM_KIND_MOTION &&
        (out->additive_sum || out->accumulate)) {
        add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_COMPOSITION,
                       tag->name.offset);
        return false;
    }
    if ((out->kind == SVG_ANIM_KIND_TRANSFORM || out->attribute == SVG_ANIM_ATTR_D) &&
        (out->additive_sum || out->accumulate)) {
        add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_COMPOSITION,
                       tag->name.offset);
        return false;
    }
    svg_xml_span begin = attr_or_empty(source, tokens, token_count, element, "begin");
    if (begin.length != 0U && !parse_sync_time(source, begin, out, true)) {
        add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_TIMING, begin.offset);
        return false;
    }
    svg_xml_span end = attr_or_empty(source, tokens, token_count, element, "end");
    if (end.length != 0U) {
        if (!parse_sync_time(source, end, out, false)) {
            add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_TIMING, end.offset);
            return false;
        }
        if (out->end_is_sync && out->key_count == 0U) { /* sync ID saved after keys exist */ }
    }
    svg_xml_span duration = attr_or_empty(source, tokens, token_count, element, "dur");
    if (duration.length == 0U && out->kind != SVG_ANIM_KIND_SET) {
        add_diagnostic(document, SVG_ANIM_DIAG_INVALID_TRACK, tag->name.offset);
        return false;
    }
    if (span_eq(source, duration, "indefinite")) out->duration_indefinite = true;
    else if (duration.length != 0U &&
             !time_value(source, duration, &out->duration_seconds)) {
        add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_TIMING, duration.offset);
        return false;
    }
    if (!out->duration_indefinite && out->kind != SVG_ANIM_KIND_SET &&
        !(out->duration_seconds > 0.0)) {
        add_diagnostic(document, SVG_ANIM_DIAG_INVALID_TRACK, duration.offset);
        return false;
    }
    if (out->kind == SVG_ANIM_KIND_SET && duration.length == 0U)
        out->duration_indefinite = true;
    if (!parse_repeat(source, attr_or_empty(source, tokens, token_count,
                                           element, "repeatCount"), out)) {
        add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_TIMING, tag->name.offset);
        return false;
    }
    svg_xml_span repeat_dur = attr_or_empty(source, tokens, token_count, element,
                                             "repeatDur");
    if (span_eq(source, repeat_dur, "indefinite")) out->repeat_duration_indefinite = true;
    else if (repeat_dur.length != 0U) {
        if (!time_value(source, repeat_dur, &out->repeat_duration)) {
            add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_TIMING,
                           repeat_dur.offset);
            return false;
        }
    }
    if (out->kind == SVG_ANIM_KIND_SET) out->repeat_count = 1.0;
    out->base_value = (svg_anim_value){0};
    if (out->kind != SVG_ANIM_KIND_MOTION &&
        !base_attribute_value(source, tokens, token_count, parent,
            out->attribute, out->kind, transform_type, &out->base_value)) {
        add_diagnostic(document, SVG_ANIM_DIAG_INVALID_TRACK, tag->name.offset);
        return false;
    }
    svg_xml_span values = attr_or_empty(source, tokens, token_count, element, "values");
    svg_xml_span from = attr_or_empty(source, tokens, token_count, element, "from");
    svg_xml_span to = attr_or_empty(source, tokens, token_count, element, "to");
    svg_xml_span by = attr_or_empty(source, tokens, token_count, element, "by");
    if (values.length != 0U && (from.length != 0U || to.length != 0U || by.length != 0U)) {
        add_diagnostic(document, SVG_ANIM_DIAG_INVALID_TRACK, values.offset);
        return false;
    }
    svg_xml_span pieces[SVG_ANIM_MAX_VALUES];
    size_t count = 0U;
    bool implicit_from = false;
    bool by_value = false;
    if (out->kind == SVG_ANIM_KIND_MOTION) {
        svg_xml_span path = {0U, 0U};
        if (!svg_anim_resolve_motion_path(source, tokens, token_count,
                                          element, &path) ||
            !svg_anim_parse_motion_path(source, path, out)) {
            add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_PATH, tag->name.offset);
            return false;
        }
        svg_xml_span rotate = attr_or_empty(source, tokens, token_count,
                                            element, "rotate");
        if (span_eq(source, rotate, "auto")) out->motion_rotate_auto = true;
        else if (span_eq(source, rotate, "auto-reverse")) {
            out->motion_rotate_auto = true;
            out->motion_rotate_reverse = true;
        } else if (rotate.length != 0U &&
                   !source_number(source, rotate, &out->motion_fixed_rotation)) {
            add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_ATTRIBUTE,
                           rotate.offset);
            return false;
        }
        pieces[count++] = path;
    } else if (values.length != 0U) {
        if (!split_value_spans(source, values, pieces, SVG_ANIM_MAX_VALUES, &count)) {
            add_diagnostic(document, SVG_ANIM_DIAG_INVALID_TRACK, values.offset);
            return false;
        }
    } else if (out->kind == SVG_ANIM_KIND_SET) {
        if (to.length == 0U) {
            add_diagnostic(document, SVG_ANIM_DIAG_INVALID_TRACK, tag->name.offset);
            return false;
        }
        pieces[count++] = to;
    } else if (from.length != 0U && to.length != 0U) {
        pieces[count++] = from;
        pieces[count++] = to;
    } else if (to.length != 0U) {
        if (out->kind != SVG_ANIM_KIND_SET) {
            implicit_from = true;
            pieces[count++] = to;
        } else pieces[count++] = to;
    } else if (by.length != 0U) {
        implicit_from = true;
        by_value = true;
        pieces[count++] = by;
    } else {
        add_diagnostic(document, SVG_ANIM_DIAG_INVALID_TRACK, tag->name.offset);
        return false;
    }
    if (count == 0U || count > SVG_ANIM_MAX_VALUES) {
        add_diagnostic(document, SVG_ANIM_DIAG_INVALID_TRACK, tag->name.offset);
        return false;
    }
    size_t source_value_count = count;
    if (implicit_from) {
        if (count >= SVG_ANIM_MAX_VALUES) {
            add_diagnostic(document, SVG_ANIM_DIAG_INVALID_TRACK, tag->name.offset);
            return false;
        }
        count++;
    }
    out->keys = calloc(count, sizeof(*out->keys));
    if (out->keys == NULL) return false;
    out->key_count = count;
    if (implicit_from) {
        out->keys[0].value = out->base_value;
        out->keys[0].source_offset = out->base_value.raw_offset;
        out->keys[0].source_length = out->base_value.raw_length;
        out->keys[0].time = 0.0;
    }
    for (size_t index = 0U; index < source_value_count; ++index) {
        svg_anim_value value;
        if (!parse_value(source, pieces[index], out->attribute, out->kind,
                         transform_type, &value)) {
            add_diagnostic(document, out->attribute == SVG_ANIM_ATTR_D ?
                SVG_ANIM_DIAG_UNSUPPORTED_PATH : SVG_ANIM_DIAG_INVALID_TRACK,
                pieces[index].offset);
            return false;
        }
        size_t key_index = index + (implicit_from ? 1U : 0U);
        if (by_value) {
            if (value.kind != SVG_ANIM_VALUE_NUMBER ||
                out->base_value.kind != SVG_ANIM_VALUE_NUMBER) {
                add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_COMPOSITION,
                               pieces[index].offset);
                return false;
            }
            value.number[0] += out->base_value.number[0];
        }
        out->keys[key_index].value = value;
        out->keys[key_index].source_offset = pieces[index].offset;
        out->keys[key_index].source_length = pieces[index].length;
        out->keys[key_index].time = count == 1U ? 0.0 :
            (double)key_index / (double)(count - 1U);
    }
    if (out->kind == SVG_ANIM_KIND_ANIMATE && out->attribute == SVG_ANIM_ATTR_D) {
        svg_xml_span base_path = attr_or_empty(source, tokens, token_count, parent, "d");
        if (base_path.length == 0U) {
            add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_PATH, tag->name.offset);
            return false;
        }
        for (size_t index = 0U; index < count; ++index) {
            svg_xml_span current = {out->keys[index].value.raw_offset,
                                    out->keys[index].value.raw_length};
            if (!path_structures_match(source, base_path, current)) {
                add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_PATH,
                               current.offset);
                return false;
            }
        }
    }
    svg_xml_span key_times = attr_or_empty(source, tokens, token_count, element,
                                            "keyTimes");
    if (!parse_key_times(source, key_times, out)) {
        add_diagnostic(document, SVG_ANIM_DIAG_INVALID_TRACK,
                       key_times.length == 0U ? tag->name.offset : key_times.offset);
        return false;
    }
    if (out->calc_mode == SVG_ANIM_CALC_PACED && key_times.length != 0U) {
        add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_ATTRIBUTE,
                       key_times.offset);
        return false;
    }
    svg_xml_span splines = attr_or_empty(source, tokens, token_count, element,
                                          "keySplines");
    if (!parse_splines(source, splines, out)) {
        add_diagnostic(document, SVG_ANIM_DIAG_INVALID_TRACK,
                       splines.length == 0U ? tag->name.offset : splines.offset);
        return false;
    }
    return true;
}

static bool span_id_eq(base_bytes source, svg_xml_span left, svg_xml_span right)
{
    return left.length != 0U && left.length == right.length &&
           left.offset <= source.len && right.offset <= source.len &&
           left.length <= source.len - left.offset &&
           right.length <= source.len - right.offset &&
           memcmp(source.ptr + left.offset, source.ptr + right.offset, left.length) == 0;
}

static bool resolve_syncbases(svg_anim_document *document)
{
    for (size_t index = 0U; index < document->track_count; ++index) {
        svg_anim_track *track = &document->tracks[index];
        if (track->has_begin_sync) {
            svg_xml_span sync_id = track->begin_sync_id;
            size_t found = SIZE_MAX, matches = 0U;
            for (size_t candidate = 0U; candidate < document->track_count; ++candidate) {
                if (span_id_eq(document->source, sync_id,
                               document->tracks[candidate].id)) {
                    found = candidate;
                    matches++;
                }
            }
            if (matches != 1U) {
                add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_TIMING,
                               sync_id.offset);
                track->supported = false;
            } else track->begin_sync_track = found;
        }
        if (track->end_is_sync) {
            svg_xml_span sync_id = track->end_sync_id;
            size_t found = SIZE_MAX, matches = 0U;
            for (size_t candidate = 0U; candidate < document->track_count; ++candidate) {
                if (span_id_eq(document->source, sync_id,
                               document->tracks[candidate].id)) {
                    found = candidate;
                    matches++;
                }
            }
            if (matches != 1U) {
                add_diagnostic(document, SVG_ANIM_DIAG_UNSUPPORTED_TIMING,
                               sync_id.offset);
                track->supported = false;
            } else track->end_sync_track = found;
        }
    }
    return true;
}

static bool contains_bytes(base_bytes source, svg_xml_span span, const char *needle)
{
    size_t length = strlen(needle);
    if (span.offset > source.len || span.length > source.len - span.offset ||
        span.length < length) return false;
    for (size_t index = 0U; index + length <= span.length; ++index)
        if (memcmp(source.ptr + span.offset + index, needle, length) == 0) return true;
    return false;
}

bool svg_anim_parse(base_bytes source, const svg_xml_token *tokens,
                    size_t token_count, svg_anim_document *out,
                    size_t *error_offset)
{
    if (error_offset != NULL) *error_offset = 0U;
    if (out == NULL || tokens == NULL || !base_bytes_valid(source)) return false;
    *out = (svg_anim_document){.source = source};
    out->tracks = calloc(SVG_ANIM_MAX_TRACKS, sizeof(*out->tracks));
    if (out->tracks == NULL) return false;
    for (size_t index = 0U; index < token_count; ++index) {
        const svg_xml_token *token = &tokens[index];
        if (token->kind == SVG_XML_TOKEN_START_ELEMENT ||
            token->kind == SVG_XML_TOKEN_EMPTY_ELEMENT) {
            if (token_name(source, token, "style")) {
                size_t end = index + 1U;
                while (end < token_count &&
                       !(tokens[end].kind == SVG_XML_TOKEN_END_ELEMENT &&
                         tokens[end].depth == token->depth)) {
                    if (tokens[end].kind == SVG_XML_TOKEN_TEXT &&
                        (contains_bytes(source, tokens[end].value, "@keyframes") ||
                         contains_bytes(source, tokens[end].value, "animation:") ||
                         contains_bytes(source, tokens[end].value, "transition:")))
                        add_diagnostic(out, SVG_ANIM_DIAG_CSS_ANIMATION,
                                       tokens[end].value.offset);
                    end++;
                }
            }
            bool is_animation = token_name(source, token, "animate") ||
                token_name(source, token, "animateTransform") ||
                token_name(source, token, "set") || token_name(source, token, "animateMotion") ||
                token_name(source, token, "animateColor");
            if (!is_animation) continue;
            if (out->track_count >= SVG_ANIM_MAX_TRACKS) {
                if (error_offset != NULL) *error_offset = token->name.offset;
                svg_anim_document_destroy(out);
                return false;
            }
            size_t parent = SIZE_MAX;
            if (token->depth > 0U) {
                for (size_t candidate = index; candidate > 0U; --candidate) {
                    const svg_xml_token *possible = &tokens[candidate - 1U];
                    if ((possible->kind == SVG_XML_TOKEN_START_ELEMENT ||
                         possible->kind == SVG_XML_TOKEN_EMPTY_ELEMENT) &&
                        possible->depth + 1U == token->depth) {
                        parent = candidate - 1U;
                        break;
                    }
                }
            }
            svg_anim_track track;
            if (!parse_track(source, tokens, token_count, index, parent, out,
                             &track)) {
                free(track.keys);
                free(track.splines);
                free(track.motion_points);
                continue;
            }
            out->tracks[out->track_count++] = track;
        }
    }
    (void)resolve_syncbases(out);
    return true;
}

void svg_anim_document_destroy(svg_anim_document *document)
{
    if (document == NULL) return;
    for (size_t index = 0U; index < document->track_count; ++index) {
        free(document->tracks[index].keys);
        free(document->tracks[index].splines);
        free(document->tracks[index].motion_points);
    }
    free(document->tracks);
    *document = (svg_anim_document){0};
}
