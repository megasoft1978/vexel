#include "authoring.h"

#include "authoring_internal.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define AUTHORING_TAG_LIMIT (64U * 1024U)
#define AUTHORING_ATTRIBUTE_CAP 128U
#define AUTHORING_MOD_CAP 8U

typedef struct authoring_attribute {
    size_t whitespace_start;
    size_t name_offset;
    size_t name_length;
    size_t value_offset;
    size_t value_length;
    size_t end;
    uint8_t quote;
} authoring_attribute;

typedef struct authoring_modification {
    const char *name;
    base_bytes value;
    bool remove;
    bool found;
} authoring_modification;

static bool checked_range(base_bytes source, size_t offset, size_t length)
{
    return base_bytes_valid(source) && offset <= source.len &&
        length <= source.len - offset;
}
static bool builder_reserve(authoring_builder *builder, size_t additional)
{
    size_t needed = 0U;
    if (builder == NULL || !base_size_add(builder->length, additional, &needed) ||
        needed > AUTHORING_PATCH_LIMIT) return false;
    if (needed <= builder->capacity) return true;
    size_t capacity = builder->capacity == 0U ? 256U : builder->capacity;
    while (capacity < needed) {
        if (capacity > AUTHORING_PATCH_LIMIT / 2U) {
            capacity = AUTHORING_PATCH_LIMIT;
            break;
        }
        capacity *= 2U;
    }
    if (capacity < needed) return false;
    uint8_t *grown = realloc(builder->bytes, capacity);
    if (grown == NULL) return false;
    builder->bytes = grown;
    builder->capacity = capacity;
    return true;
}

bool authoring_builder_append(authoring_builder *builder,
                           const uint8_t *bytes, size_t length)
{
    if (length > 0U && bytes == NULL) return false;
    if (!builder_reserve(builder, length)) return false;
    if (length > 0U) memcpy(builder->bytes + builder->length, bytes, length);
    builder->length += length;
    return true;
}
bool authoring_builder_byte(authoring_builder *builder, uint8_t byte)
{
    return authoring_builder_append(builder, &byte, 1U);
}
bool authoring_builder_cstr(authoring_builder *builder, const char *text)
{
    return text != NULL && authoring_builder_append(builder, (const uint8_t *)text, strlen(text));
}
bool authoring_builder_attribute_value(authoring_builder *builder, base_bytes value,
                                    uint8_t quote)
{
    if (!base_bytes_valid(value) || value.len > AUTHORING_VALUE_LIMIT ||
        (quote != '\'' && quote != '"')) return false;
    for (size_t index = 0U; index < value.len; ++index) {
        const char *escaped = NULL;
        size_t escaped_length = 0U;
        uint8_t byte = value.ptr[index];
        if (byte == '&') { escaped = "&amp;"; escaped_length = 5U; }
        else if (byte == '<') { escaped = "&lt;"; escaped_length = 4U; }
        else if (byte == quote && quote == '"') {
            escaped = "&quot;"; escaped_length = 6U;
        } else if (byte == quote && quote == '\'') {
            escaped = "&apos;"; escaped_length = 6U;
        }
        if (escaped != NULL) {
            if (!authoring_builder_append(builder, (const uint8_t *)escaped, escaped_length))
                return false;
        } else if (!authoring_builder_byte(builder, byte)) return false;
    }
    return true;
}
static bool source_space(uint8_t byte)
{
    return byte == ' ' || byte == '\t' || byte == '\r' || byte == '\n';
}
static bool source_name_byte(uint8_t byte)
{
    return (byte >= 'a' && byte <= 'z') || (byte >= 'A' && byte <= 'Z') ||
        (byte >= '0' && byte <= '9') || byte == '_' || byte == '-' ||
        byte == ':' || byte == '.';
}
static bool parse_open_tag(base_bytes source, size_t offset, size_t limit,
    authoring_attribute attributes[AUTHORING_ATTRIBUTE_CAP], size_t *out_count,
    size_t *out_length)
{
    if (out_count == NULL || out_length == NULL || offset >= limit ||
        limit > source.len || source.ptr[offset] != '<') return false;
    size_t cursor = offset + 1U;
    while (cursor < limit && source_name_byte(source.ptr[cursor])) cursor++;
    if (cursor == offset + 1U) return false;
    size_t count = 0U;
    while (cursor < limit) {
        size_t whitespace_start = cursor;
        while (cursor < limit && source_space(source.ptr[cursor])) cursor++;
        if (cursor >= limit) return false;
        if (source.ptr[cursor] == '>') {
            *out_count = count;
            *out_length = cursor + 1U - offset;
            return *out_length <= AUTHORING_TAG_LIMIT;
        }
        if (source.ptr[cursor] == '/') {
            if (cursor + 1U >= limit || source.ptr[cursor + 1U] != '>') return false;
            *out_count = count;
            *out_length = cursor + 2U - offset;
            return *out_length <= AUTHORING_TAG_LIMIT;
        }
        if (count >= AUTHORING_ATTRIBUTE_CAP) return false;
        authoring_attribute *attribute = &attributes[count];
        attribute->whitespace_start = whitespace_start;
        attribute->name_offset = cursor;
        while (cursor < limit && source_name_byte(source.ptr[cursor])) cursor++;
        if (cursor == attribute->name_offset) return false;
        attribute->name_length = cursor - attribute->name_offset;
        while (cursor < limit && source_space(source.ptr[cursor])) cursor++;
        if (cursor >= limit || source.ptr[cursor++] != '=') return false;
        while (cursor < limit && source_space(source.ptr[cursor])) cursor++;
        if (cursor >= limit || (source.ptr[cursor] != '\'' &&
                source.ptr[cursor] != '"')) return false;
        attribute->quote = source.ptr[cursor++];
        attribute->value_offset = cursor;
        while (cursor < limit && source.ptr[cursor] != attribute->quote) cursor++;
        if (cursor >= limit) return false;
        attribute->value_length = cursor - attribute->value_offset;
        attribute->end = ++cursor;
        count++;
    }
    return false;
}
static bool attr_name_is(base_bytes source, const authoring_attribute *attribute,
                         const char *name)
{
    size_t length = strlen(name);
    return attribute->name_length == length &&
        memcmp(source.ptr + attribute->name_offset, name, length) == 0;
}
static bool find_attribute(base_bytes source,
    const authoring_attribute attributes[AUTHORING_ATTRIBUTE_CAP], size_t count,
    const char *name, authoring_attribute *out)
{
    for (size_t index = 0U; index < count; ++index) {
        if (attr_name_is(source, &attributes[index], name)) {
            if (out != NULL) *out = attributes[index];
            return true;
        }
    }
    return false;
}
static bool apply_modifications(base_bytes source, size_t tag_offset,
    size_t tag_length, const authoring_attribute attributes[AUTHORING_ATTRIBUTE_CAP],
    size_t attribute_count, authoring_modification *mods, size_t mod_count,
    svg_anim_authoring_patch *out_patch)
{
    if (mod_count > AUTHORING_MOD_CAP || out_patch == NULL ||
        !checked_range(source, tag_offset, tag_length)) return false;
    for (size_t i = 0U; i < mod_count; ++i) {
        if (mods[i].name == NULL || (!mods[i].remove &&
                (!base_bytes_valid(mods[i].value) || mods[i].value.len > AUTHORING_VALUE_LIMIT)))
            return false;
        for (size_t j = i + 1U; j < mod_count; ++j)
            if (strcmp(mods[i].name, mods[j].name) == 0) return false;
    }
    authoring_builder output = {0};
    size_t cursor = tag_offset;
    size_t insertion = tag_offset + tag_length - 1U;
    if (tag_length >= 2U && source.ptr[tag_offset + tag_length - 2U] == '/')
        insertion--;
    for (size_t index = 0U; index < attribute_count; ++index) {
        const authoring_attribute *attribute = &attributes[index];
        authoring_modification *mod = NULL;
        for (size_t mod_index = 0U; mod_index < mod_count; ++mod_index) {
            if (attr_name_is(source, attribute, mods[mod_index].name)) {
                mod = &mods[mod_index];
                mod->found = true;
                break;
            }
        }
        size_t prefix_end = attribute->whitespace_start;
        if (prefix_end < cursor || !authoring_builder_append(&output,
                source.ptr + cursor, prefix_end - cursor)) goto fail;
        if (mod != NULL && mod->remove) {
            cursor = attribute->end;
            continue;
        }
        size_t value_start = attribute->value_offset;
        size_t value_end = value_start + attribute->value_length;
        if (value_start < prefix_end || value_end > tag_offset + tag_length ||
            !authoring_builder_append(&output, source.ptr + prefix_end,
                value_start - prefix_end)) goto fail;
        if (mod != NULL) {
            if (!authoring_builder_attribute_value(&output, mod->value, attribute->quote)) goto fail;
        } else if (!authoring_builder_append(&output, source.ptr + value_start,
                attribute->value_length)) goto fail;
        if (!authoring_builder_append(&output, source.ptr + value_end,
                attribute->end - value_end)) goto fail;
        cursor = attribute->end;
    }
    if (cursor > insertion || !authoring_builder_append(&output,
            source.ptr + cursor, insertion - cursor)) goto fail;
    for (size_t index = 0U; index < mod_count; ++index) {
        if (mods[index].remove || mods[index].found) continue;
        if (!authoring_builder_byte(&output, ' ') ||
            !authoring_builder_append(&output, (const uint8_t *)mods[index].name,
                strlen(mods[index].name)) || !authoring_builder_append(&output,
                (const uint8_t *)"=\"", 2U) ||
            !authoring_builder_attribute_value(&output, mods[index].value, '"') ||
            !authoring_builder_byte(&output, '"')) goto fail;
    }
    if (!authoring_builder_append(&output, source.ptr + insertion,
            tag_offset + tag_length - insertion)) goto fail;
    out_patch->replacement = output.bytes;
    out_patch->replacement_length = output.length;
    return true;
fail:
    free(output.bytes);
    return false;
}
static bool make_track_patch(const svg_anim_document *document,
    const svg_anim_track *track, authoring_modification *mods, size_t mod_count,
    svg_anim_authoring_patch *out_patch)
{
    authoring_attribute attrs[AUTHORING_ATTRIBUTE_CAP] = {0};
    size_t count = 0U, tag_length = 0U;
    if (track->element_offset > document->source.len ||
        track->element_length > document->source.len - track->element_offset ||
        !parse_open_tag(document->source, track->element_offset,
            track->element_offset + track->element_length, attrs, &count,
            &tag_length)) return false;
    if (!apply_modifications(document->source, track->element_offset, tag_length,
            attrs, count, mods, mod_count, out_patch)) return false;
    out_patch->offset = track->element_offset;
    out_patch->remove_length = tag_length;
    return true;
}
bool authoring_append_number(authoring_builder *builder, double value)
{
    char text[64];
    int amount = snprintf(text, sizeof(text), "%.17g", value);
    return isfinite(value) && amount > 0 && (size_t)amount < sizeof(text) &&
        authoring_builder_append(builder, (const uint8_t *)text, (size_t)amount);
}
static bool append_value(authoring_builder *builder,
    const svg_anim_document *document, const svg_anim_keyframe *key)
{
    const svg_anim_value *value = &key->value;
    if (value->raw_length > 0U) {
        if (!checked_range(document->source, value->raw_offset, value->raw_length)) return false;
        return authoring_builder_append(builder, document->source.ptr + value->raw_offset,
            value->raw_length);
    }
    if (value->kind == SVG_ANIM_VALUE_NUMBER && key->source_length > 0U) {
        double raw_number = 0.0;
        if (checked_range(document->source, key->source_offset, key->source_length) &&
            source_number(document->source, (svg_xml_span){key->source_offset,
                key->source_length}, &raw_number) && raw_number == value->number[0])
            return authoring_builder_append(builder, document->source.ptr + key->source_offset,
                key->source_length);
    }
    if (value->kind == SVG_ANIM_VALUE_NUMBER) return authoring_append_number(builder, value->number[0]);
    if (value->kind == SVG_ANIM_VALUE_COLOR) {
        char color[10];
        int amount = value->rgba[3] == 255U ?
            snprintf(color, sizeof(color), "#%02X%02X%02X", value->rgba[0],
                value->rgba[1], value->rgba[2]) :
            snprintf(color, sizeof(color), "#%02X%02X%02X%02X", value->rgba[0],
                value->rgba[1], value->rgba[2], value->rgba[3]);
        return amount > 0 && (size_t)amount < sizeof(color) &&
            authoring_builder_append(builder, (const uint8_t *)color, (size_t)amount);
    }
    if (value->kind == SVG_ANIM_VALUE_TRANSFORM) {
        if (!isfinite(value->number[3]) || value->number[3] < 1.0 ||
            value->number[3] > 3.0) return false;
        size_t count = (size_t)value->number[3];
        for (size_t index = 0U; index < count; ++index) {
            if (index > 0U && !authoring_builder_byte(builder, ' ')) return false;
            if (!authoring_append_number(builder, value->number[index])) return false;
        }
        return true;
    }
    return false;
}
static bool editable_kind(const svg_anim_track *track)
{
    bool kind_supported = track != NULL && track->supported &&
        (track->kind == SVG_ANIM_KIND_ANIMATE || track->kind == SVG_ANIM_KIND_TRANSFORM);
    bool attribute_supported = track != NULL &&
        (track->kind == SVG_ANIM_KIND_TRANSFORM ?
            (track->attribute == SVG_ANIM_ATTR_TRANSFORM ||
             track->attribute == SVG_ANIM_ATTR_MOTION) :
            (track->attribute != SVG_ANIM_ATTR_D &&
             track->attribute != SVG_ANIM_ATTR_MOTION &&
             track->attribute != SVG_ANIM_ATTR_TRANSFORM));
    return kind_supported && attribute_supported &&
        !track->additive_sum && !track->accumulate &&
        !track->begin_indefinite && !track->duration_indefinite &&
        !track->repeat_indefinite &&
        !track->repeat_duration_indefinite && !track->has_begin_sync &&
        !track->end_is_sync && track->repeat_count == 1.0 &&
        isfinite(track->duration_seconds) && track->duration_seconds > 0.0 &&
        track->duration_seconds <= SVG_ANIM_MAX_TIMELINE_SECONDS &&
        track->key_count > 0U && track->key_count <= SVG_ANIM_MAX_VALUES;
}
static bool track_and_source(const svg_anim_document *document, size_t track_index,
    const svg_anim_track **out_track)
{
    if (document == NULL || out_track == NULL || document->tracks == NULL ||
        track_index >= document->track_count || !base_bytes_valid(document->source)) return false;
    const svg_anim_track *track = &document->tracks[track_index];
    if (!editable_kind(track) || track->calc_mode == SVG_ANIM_CALC_PACED ||
        track->element_offset >= document->source.len || track->element_length == 0U ||
        track->element_length > document->source.len - track->element_offset)
        return false;
    *out_track = track;
    return true;
}
bool svg_anim_authoring_track_editable(const svg_anim_document *document,
                                      size_t track_index)
{
    const svg_anim_track *track = NULL;
    return track_and_source(document, track_index, &track);
}
bool svg_anim_authoring_track_removable(const svg_anim_document *document,
                                       size_t track_index)
{
    const svg_anim_track *track = NULL;
    if (!track_and_source(document, track_index, &track)) return false;
    authoring_attribute attrs[AUTHORING_ATTRIBUTE_CAP] = {0};
    size_t count = 0U, open_length = 0U;
    if (!parse_open_tag(document->source, track->element_offset,
            track->element_offset + track->element_length, attrs, &count,
            &open_length)) return false;
    size_t span_length = track->element_length;
    if (open_length == span_length) return true;
    size_t name_start = track->element_offset + 1U;
    size_t name_end = name_start;
    while (name_end < document->source.len &&
           source_name_byte(document->source.ptr[name_end])) name_end++;
    size_t name_length = name_end - name_start;
    size_t close_length = 0U;
    if (!base_size_add(name_length, 3U, &close_length) ||
        open_length + close_length != span_length ||
        !checked_range(document->source, track->element_offset + open_length,
            close_length)) return false;
    const uint8_t *close = document->source.ptr + track->element_offset + open_length;
    return close[0] == '<' && close[1] == '/' &&
        memcmp(close + 2U, document->source.ptr + name_start, name_length) == 0 &&
        close[2U + name_length] == '>';
}
void svg_anim_authoring_patch_destroy(svg_anim_authoring_patch *patch)
{
    if (patch == NULL) return;
    free(patch->replacement);
    *patch = (svg_anim_authoring_patch){0};
}
static svg_anim_authoring_error set_key_sequence(const svg_anim_document *document,
    const svg_anim_track *track, const svg_anim_keyframe *keys, size_t key_count,
    size_t override_index, base_bytes override_value, base_bytes splines,
    svg_anim_authoring_patch *out_patch)
{
    if (key_count == 0U || key_count > SVG_ANIM_MAX_VALUES)
        return SVG_ANIM_AUTHORING_UNSUPPORTED;
    if (splines.len > AUTHORING_VALUE_LIMIT) return SVG_ANIM_AUTHORING_LIMIT;
    authoring_builder values = {0}, times = {0};
    for (size_t index = 0U; index < key_count; ++index) {
        if (index > 0U && !authoring_builder_byte(&values, ';')) goto limit;
        if (index == override_index) {
            if (!authoring_builder_append(&values, override_value.ptr, override_value.len)) goto limit;
        } else if (!append_value(&values, document, &keys[index])) goto unsupported;
        if (key_count > 1U) {
            if (index > 0U && !authoring_builder_byte(&times, ';')) goto limit;
            if (!authoring_append_number(&times, keys[index].time)) goto limit;
        }
    }
    if (values.length > AUTHORING_VALUE_LIMIT) goto limit;
    {
        /* A spline track keeps its tuples; dropping to one key reverts to linear. */
        bool revert = track->calc_mode == SVG_ANIM_CALC_SPLINE && splines.len == 0U;
        authoring_modification mods[] = {
            {"values", {values.bytes, values.length}, false, false},
            {"keyTimes", {times.bytes, times.length}, key_count == 1U, false},
            {"from", {0}, true, false}, {"to", {0}, true, false},
            {"by", {0}, true, false},
            {"keySplines", splines, splines.len == 0U, false},
            {"calcMode", {(const uint8_t *)"linear", 6U}, false, false}
        };
        if (!make_track_patch(document, track, mods, revert ? 7U : 6U,
                out_patch)) goto unsupported;
    }
    free(values.bytes); free(times.bytes);
    return SVG_ANIM_AUTHORING_OK;
limit:
    free(values.bytes); free(times.bytes);
    return SVG_ANIM_AUTHORING_LIMIT;
unsupported:
    free(values.bytes); free(times.bytes);
    return SVG_ANIM_AUTHORING_UNSUPPORTED;
}
/* Splits the raw keySplines tuples of a spline track; spans are untrimmed. */
static bool collect_spline_pieces(const svg_anim_document *document,
    const svg_anim_track *track, svg_xml_span pieces[SVG_ANIM_MAX_VALUES],
    size_t *out_count)
{
    authoring_attribute attrs[AUTHORING_ATTRIBUTE_CAP] = {0}, attr;
    size_t count = 0U, tag_length = 0U, piece = 0U, start = 0U;
    if (!parse_open_tag(document->source, track->element_offset,
            track->element_offset + track->element_length, attrs, &count,
            &tag_length) || !find_attribute(document->source, attrs, count,
            "keySplines", &attr)) return false;
    const uint8_t *raw = document->source.ptr + attr.value_offset;
    for (size_t cursor = 0U; cursor <= attr.value_length; ++cursor) {
        if (cursor < attr.value_length && raw[cursor] != ';') continue;
        size_t left = start, right = cursor;
        while (left < right && source_space(raw[left])) left++;
        while (right > left && source_space(raw[right - 1U])) right--;
        if (piece >= track->key_count - 1U || left == right) return false;
        pieces[piece++] = (svg_xml_span){attr.value_offset + start, cursor - start};
        start = cursor + 1U;
    }
    *out_count = piece;
    return piece == track->key_count - 1U;
}

/* Rewrites a key sequence of the track. Spline tuples follow the keys: tuple
 * `dup` (SIZE_MAX for none) is repeated and tuple `drop` removed. */
static svg_anim_authoring_error commit_keys(const svg_anim_document *document,
    const svg_anim_track *track, const svg_anim_keyframe *keys, size_t key_count,
    size_t override_index, base_bytes override_value, size_t dup, size_t drop,
    svg_anim_authoring_patch *out_patch)
{
    authoring_builder splines = {0};
    svg_xml_span pieces[SVG_ANIM_MAX_VALUES];
    size_t count = 0U;
    if (track->calc_mode == SVG_ANIM_CALC_SPLINE) {
        if (!collect_spline_pieces(document, track, pieces, &count))
            return SVG_ANIM_AUTHORING_UNSUPPORTED;
        for (size_t index = 0U; index < count && key_count > 1U; ++index) {
            for (size_t copy = 0U; copy < (index == dup ? 2U : 1U); ++copy) {
                if (index == drop) break;
                if ((splines.length > 0U && !authoring_builder_byte(&splines, ';')) ||
                    !authoring_builder_append(&splines,
                        document->source.ptr + pieces[index].offset,
                        pieces[index].length)) {
                    free(splines.bytes);
                    return SVG_ANIM_AUTHORING_LIMIT;
                }
            }
        }
    }
    svg_anim_authoring_error error = set_key_sequence(document, track, keys,
        key_count, override_index, override_value,
        (base_bytes){splines.bytes, splines.length}, out_patch);
    free(splines.bytes);
    return error;
}
static bool parse_typed_value(const svg_anim_track *track, base_bytes text,
                              svg_anim_value *parsed)
{
    if (track == NULL || parsed == NULL || !base_bytes_valid(text) || text.len == 0U ||
        text.len > AUTHORING_VALUE_LIMIT) return false;
    for (size_t index = 0U; index < text.len; ++index)
        if (text.ptr[index] == ';' || text.ptr[index] == '\0') return false;
    const char *type = "translate";
    if (track->kind == SVG_ANIM_KIND_TRANSFORM) {
        switch (track->transform_type) {
            case SVG_ANIM_TRANSFORM_TRANSLATE: type = "translate"; break;
            case SVG_ANIM_TRANSFORM_SCALE: type = "scale"; break;
            case SVG_ANIM_TRANSFORM_ROTATE: type = "rotate"; break;
            case SVG_ANIM_TRANSFORM_SKEW_X: type = "skewX"; break;
            case SVG_ANIM_TRANSFORM_SKEW_Y: type = "skewY"; break;
        }
    }
    return svg_anim_parse_value(text, (svg_xml_span){0U, text.len},
        track->attribute, track->kind, type, parsed);
}
svg_anim_authoring_error svg_anim_authoring_set_key_value(
    const svg_anim_document *document, size_t track_index, size_t key_index,
    base_bytes value, svg_anim_authoring_patch *out_patch)
{
    if (out_patch == NULL) return SVG_ANIM_AUTHORING_INVALID_ARGUMENT;
    *out_patch = (svg_anim_authoring_patch){0};
    const svg_anim_track *track = NULL;
    if (!track_and_source(document, track_index, &track) || key_index >= track->key_count)
        return SVG_ANIM_AUTHORING_UNSUPPORTED;
    svg_anim_value requested;
    if (!parse_typed_value(track, value, &requested)) return SVG_ANIM_AUTHORING_INVALID_ARGUMENT;
    const svg_anim_value *current = &track->keys[key_index].value;
    if (current->raw_length == value.len &&
        checked_range(document->source, current->raw_offset, current->raw_length) &&
        (value.len == 0U || memcmp(document->source.ptr + current->raw_offset,
            value.ptr, value.len) == 0)) return SVG_ANIM_AUTHORING_NO_OP;
    if (current->kind == requested.kind &&
        ((current->kind == SVG_ANIM_VALUE_NUMBER &&
          current->number[0] == requested.number[0]) ||
         (current->kind == SVG_ANIM_VALUE_COLOR &&
          memcmp(current->rgba, requested.rgba, sizeof(current->rgba)) == 0) ||
         (current->kind == SVG_ANIM_VALUE_TRANSFORM &&
          memcmp(current->number, requested.number, sizeof(current->number)) == 0)))
        return SVG_ANIM_AUTHORING_NO_OP;
    return commit_keys(document, track, track->keys, track->key_count,
        key_index, value, SIZE_MAX, SIZE_MAX, out_patch);
}

static bool key_time_from_seconds(const svg_anim_track *track, double seconds,
                                  double *out_time)
{
    double start = track->begin_indefinite ? NAN : track->begin_offset;
    double end = start + track->duration_seconds;
    if (!isfinite(seconds) || !isfinite(start) || !isfinite(end) ||
        seconds <= start || seconds >= end) return false;
    *out_time = (seconds - start) / track->duration_seconds;
    return isfinite(*out_time) && *out_time > 0.0 && *out_time < 1.0;
}

svg_anim_authoring_error svg_anim_authoring_add_key(
    const svg_anim_document *document, size_t track_index, double time_seconds,
    base_bytes value, svg_anim_authoring_patch *out_patch)
{
    if (out_patch == NULL) return SVG_ANIM_AUTHORING_INVALID_ARGUMENT;
    *out_patch = (svg_anim_authoring_patch){0};
    const svg_anim_track *track = NULL;
    double normalized = 0.0;
    if (!track_and_source(document, track_index, &track) ||
        !key_time_from_seconds(track, time_seconds, &normalized))
        return SVG_ANIM_AUTHORING_UNSUPPORTED;
    if (track->key_count >= SVG_ANIM_MAX_VALUES) return SVG_ANIM_AUTHORING_LIMIT;
    if (!parse_typed_value(track, value, &(svg_anim_value){0}))
        return SVG_ANIM_AUTHORING_INVALID_ARGUMENT;
    size_t insertion = 0U;
    while (insertion < track->key_count && track->keys[insertion].time < normalized) insertion++;
    if ((insertion > 0U && track->keys[insertion - 1U].time >= normalized) ||
        (insertion < track->key_count && track->keys[insertion].time <= normalized))
        return SVG_ANIM_AUTHORING_INVALID_ARGUMENT;
    svg_anim_keyframe keys[SVG_ANIM_MAX_VALUES];
    for (size_t index = 0U; index < insertion; ++index) keys[index] = track->keys[index];
    keys[insertion] = (svg_anim_keyframe){.time = normalized};
    for (size_t index = insertion; index < track->key_count; ++index)
        keys[index + 1U] = track->keys[index];
    return commit_keys(document, track, keys, track->key_count + 1U, insertion,
        value, insertion - 1U, SIZE_MAX, out_patch);
}

svg_anim_authoring_error svg_anim_authoring_move_key(
    const svg_anim_document *document, size_t track_index, size_t key_index,
    double time_seconds, svg_anim_authoring_patch *out_patch)
{
    if (out_patch == NULL) return SVG_ANIM_AUTHORING_INVALID_ARGUMENT;
    *out_patch = (svg_anim_authoring_patch){0};
    const svg_anim_track *track = NULL;
    double normalized = 0.0;
    if (!track_and_source(document, track_index, &track) || key_index == 0U ||
        key_index >= track->key_count - 1U ||
        !key_time_from_seconds(track, time_seconds, &normalized))
        return SVG_ANIM_AUTHORING_UNSUPPORTED;
    if (normalized <= track->keys[key_index - 1U].time ||
        normalized >= track->keys[key_index + 1U].time)
        return SVG_ANIM_AUTHORING_INVALID_ARGUMENT;
    if (normalized == track->keys[key_index].time) return SVG_ANIM_AUTHORING_NO_OP;
    svg_anim_keyframe keys[SVG_ANIM_MAX_VALUES];
    memcpy(keys, track->keys, track->key_count * sizeof(*keys));
    keys[key_index].time = normalized;
    return commit_keys(document, track, keys, track->key_count,
        SIZE_MAX, (base_bytes){0}, SIZE_MAX, SIZE_MAX, out_patch);
}

svg_anim_authoring_error svg_anim_authoring_delete_key(
    const svg_anim_document *document, size_t track_index, size_t key_index,
    svg_anim_authoring_patch *out_patch)
{
    if (out_patch == NULL) return SVG_ANIM_AUTHORING_INVALID_ARGUMENT;
    *out_patch = (svg_anim_authoring_patch){0};
    const svg_anim_track *track = NULL;
    if (!track_and_source(document, track_index, &track) || key_index >= track->key_count)
        return SVG_ANIM_AUTHORING_UNSUPPORTED;
    if (track->key_count == 1U) return SVG_ANIM_AUTHORING_UNSUPPORTED;
    svg_anim_keyframe keys[SVG_ANIM_MAX_VALUES];
    size_t count = 0U;
    for (size_t index = 0U; index < track->key_count; ++index)
        if (index != key_index) keys[count++] = track->keys[index];
    if (count > 1U) {
        double start = keys[0].time;
        double span = keys[count - 1U].time - start;
        if (!(span > 0.0) || !isfinite(span)) return SVG_ANIM_AUTHORING_UNSUPPORTED;
        for (size_t index = 0U; index < count; ++index)
            keys[index].time = (keys[index].time - start) / span;
        keys[0].time = 0.0;
        keys[count - 1U].time = 1.0;
    }
    return commit_keys(document, track, keys, count, SIZE_MAX, (base_bytes){0},
        SIZE_MAX, key_index == track->key_count - 1U ? key_index - 1U : key_index,
        out_patch);
}

static bool spline_values_valid(const double spline[4])
{
    if (spline == NULL) return false;
    for (size_t index = 0U; index < 4U; ++index)
        if (!isfinite(spline[index]) || spline[index] < 0.0 || spline[index] > 1.0)
            return false;
    return true;
}

static bool append_spline(authoring_builder *builder, const double spline[4])
{
    for (size_t index = 0U; index < 4U; ++index) {
        if (index > 0U && !authoring_builder_byte(builder, ' ')) return false;
        if (!authoring_append_number(builder, spline[index])) return false;
    }
    return true;
}

static bool spline_attribute_value(const svg_anim_document *document,
    const svg_anim_track *track, size_t interval, const double selected[4],
    authoring_builder *value)
{
    size_t expected = track->key_count - 1U;
    authoring_attribute attrs[AUTHORING_ATTRIBUTE_CAP] = {0};
    size_t count = 0U, tag_length = 0U;
    if (!parse_open_tag(document->source, track->element_offset,
            track->element_offset + track->element_length, attrs, &count,
            &tag_length)) return false;
    (void)tag_length;
    authoring_attribute spline_attr;
    bool has_source = find_attribute(document->source, attrs, count,
        "keySplines", &spline_attr);
    if (track->calc_mode == SVG_ANIM_CALC_SPLINE && has_source) {
        base_bytes raw = {document->source.ptr + spline_attr.value_offset,
                          spline_attr.value_length};
        size_t start = 0U, piece = 0U;
        for (size_t cursor = 0U; cursor <= raw.len; ++cursor) {
            if (cursor < raw.len && raw.ptr[cursor] != ';') continue;
            size_t left = start, right = cursor;
            while (left < right && source_space(raw.ptr[left])) left++;
            while (right > left && source_space(raw.ptr[right - 1U])) right--;
            if (piece >= expected || left == right) return false;
            if (piece > 0U && !authoring_builder_byte(value, ';')) return false;
            if (piece == interval) {
                if (!append_spline(value, selected)) return false;
            } else if (!authoring_builder_append(value, raw.ptr + start, cursor - start)) return false;
            start = cursor + 1U;
            piece++;
        }
        return piece == expected;
    }
    static const double ease_in_out[4] = {0.42, 0.0, 0.58, 1.0};
    for (size_t index = 0U; index < expected; ++index) {
        if (index > 0U && !authoring_builder_byte(value, ';')) return false;
        if (!append_spline(value, index == interval ? selected : ease_in_out)) return false;
    }
    return expected > 0U;
}

svg_anim_authoring_error svg_anim_authoring_set_spline(
    const svg_anim_document *document, size_t track_index, size_t interval,
    const double spline[4], svg_anim_authoring_patch *out_patch)
{
    if (out_patch == NULL || !spline_values_valid(spline))
        return SVG_ANIM_AUTHORING_INVALID_ARGUMENT;
    *out_patch = (svg_anim_authoring_patch){0};
    const svg_anim_track *track = NULL;
    if (!track_and_source(document, track_index, &track) ||
        track->calc_mode == SVG_ANIM_CALC_PACED ||
        track->key_count < 2U || interval >= track->key_count - 1U)
        return SVG_ANIM_AUTHORING_UNSUPPORTED;
    if (track->calc_mode == SVG_ANIM_CALC_SPLINE &&
        track->spline_count > interval &&
        memcmp(track->splines[interval], spline, sizeof(track->splines[interval])) == 0)
        return SVG_ANIM_AUTHORING_NO_OP;
    authoring_builder encoded = {0};
    if (!spline_attribute_value(document, track, interval, spline, &encoded)) {
        free(encoded.bytes);
        return SVG_ANIM_AUTHORING_UNSUPPORTED;
    }
    authoring_modification mods[] = {
        {"keySplines", {encoded.bytes, encoded.length}, false, false},
        {"calcMode", {(const uint8_t *)"spline", 6U}, false, false}
    };
    bool okay = make_track_patch(document, track, mods, 2U, out_patch);
    free(encoded.bytes);
    return okay ? SVG_ANIM_AUTHORING_OK : SVG_ANIM_AUTHORING_UNSUPPORTED;
}

svg_anim_authoring_error svg_anim_authoring_set_calc_mode(
    const svg_anim_document *document, size_t track_index,
    svg_anim_calc_mode mode, svg_anim_authoring_patch *out_patch)
{
    if (out_patch == NULL) return SVG_ANIM_AUTHORING_INVALID_ARGUMENT;
    *out_patch = (svg_anim_authoring_patch){0};
    const svg_anim_track *track = NULL;
    if (!track_and_source(document, track_index, &track) ||
        (unsigned)mode >= (unsigned)SVG_ANIM_CALC_PACED)
        return SVG_ANIM_AUTHORING_UNSUPPORTED;
    if (mode == track->calc_mode) return SVG_ANIM_AUTHORING_NO_OP;
    if (mode == SVG_ANIM_CALC_SPLINE) {
        return svg_anim_authoring_set_spline(document, track_index, 0U,
            (double[4]){0.42, 0.0, 0.58, 1.0}, out_patch);
    }
    const char *value = mode == SVG_ANIM_CALC_DISCRETE ? "discrete" : "linear";
    authoring_modification mods[] = {{"calcMode",
        {(const uint8_t *)value, strlen(value)}, false, false},
        {"keySplines", {0}, true, false}};
    return make_track_patch(document, track, mods, 2U, out_patch) ?
        SVG_ANIM_AUTHORING_OK : SVG_ANIM_AUTHORING_UNSUPPORTED;
}
svg_anim_authoring_error svg_anim_authoring_set_timing(
    const svg_anim_document *document, size_t track_index, double begin_seconds,
    double duration_seconds, svg_anim_authoring_patch *out_patch)
{
    if (out_patch == NULL) return SVG_ANIM_AUTHORING_INVALID_ARGUMENT;
    *out_patch = (svg_anim_authoring_patch){0};
    const svg_anim_track *track = NULL;
    if (!track_and_source(document, track_index, &track) || track->begin_indefinite)
        return SVG_ANIM_AUTHORING_UNSUPPORTED;
    if (!isfinite(begin_seconds) || begin_seconds < 0.0 ||
        !isfinite(duration_seconds) || duration_seconds <= 0.0 ||
        begin_seconds + duration_seconds > SVG_ANIM_MAX_TIMELINE_SECONDS)
        return SVG_ANIM_AUTHORING_INVALID_ARGUMENT;
    authoring_modification mods[2];
    char begin_text[64], duration_text[64];
    size_t count = 0U;
    int amount;
    if (begin_seconds != track->begin_offset) {
        amount = snprintf(begin_text, sizeof(begin_text), "%.17gs", begin_seconds);
        if (amount <= 0 || (size_t)amount >= sizeof(begin_text))
            return SVG_ANIM_AUTHORING_INVALID_ARGUMENT;
        mods[count++] = (authoring_modification){"begin", {(const uint8_t *)begin_text,
            (size_t)amount}, false, false};
    }
    if (duration_seconds != track->duration_seconds) {
        amount = snprintf(duration_text, sizeof(duration_text), "%.17gs", duration_seconds);
        if (amount <= 0 || (size_t)amount >= sizeof(duration_text))
            return SVG_ANIM_AUTHORING_INVALID_ARGUMENT;
        mods[count++] = (authoring_modification){"dur", {(const uint8_t *)duration_text,
            (size_t)amount}, false, false};
    }
    return count == 0U ? SVG_ANIM_AUTHORING_NO_OP :
        make_track_patch(document, track, mods, count, out_patch) ?
            SVG_ANIM_AUTHORING_OK : SVG_ANIM_AUTHORING_UNSUPPORTED;
}
