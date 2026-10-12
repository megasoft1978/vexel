#include "authoring.h"

#include "authoring_internal.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static const char *transform_name(svg_anim_transform_type type)
{
    switch (type) {
        case SVG_ANIM_TRANSFORM_TRANSLATE: return "translate";
        case SVG_ANIM_TRANSFORM_SCALE: return "scale";
        case SVG_ANIM_TRANSFORM_ROTATE: return "rotate";
        case SVG_ANIM_TRANSFORM_SKEW_X: return "skewX";
        case SVG_ANIM_TRANSFORM_SKEW_Y: return "skewY";
    }
    return NULL;
}

static bool validate_new_value(base_bytes text, svg_anim_attribute attribute,
    svg_anim_kind kind, const char *type)
{
    if (!base_bytes_valid(text) || text.len == 0U || text.len > AUTHORING_VALUE_LIMIT)
        return false;
    for (size_t index = 0U; index < text.len; ++index)
        if (text.ptr[index] == ';' || text.ptr[index] == '\0') return false;
    svg_anim_value parsed;
    return svg_anim_parse_value(text, (svg_xml_span){0U, text.len}, attribute,
        kind, type, &parsed);
}

svg_anim_authoring_error svg_anim_authoring_create_track(
    base_bytes source, size_t target_offset, svg_anim_attribute attribute,
    svg_anim_kind kind, svg_anim_transform_type transform_type,
    base_bytes initial_value, base_bytes first_value, double time_seconds,
    double duration_seconds, uint8_t **out_fragment, size_t *out_length)
{
    if (out_fragment == NULL || out_length == NULL) return SVG_ANIM_AUTHORING_INVALID_ARGUMENT;
    *out_fragment = NULL; *out_length = 0U;
    if (!base_bytes_valid(source) || target_offset >= source.len ||
        !isfinite(time_seconds) || time_seconds <= 0.0 ||
        !isfinite(duration_seconds) || duration_seconds <= 0.0 ||
        duration_seconds > SVG_ANIM_MAX_TIMELINE_SECONDS || time_seconds > duration_seconds)
        return SVG_ANIM_AUTHORING_INVALID_ARGUMENT;
    const char *attribute_text = svg_anim_attribute_name(attribute);
    const char *type_text = transform_name(transform_type);
    if (attribute == SVG_ANIM_ATTR_D || attribute == SVG_ANIM_ATTR_MOTION ||
        (kind != SVG_ANIM_KIND_ANIMATE && kind != SVG_ANIM_KIND_TRANSFORM) ||
        (kind == SVG_ANIM_KIND_TRANSFORM && (attribute != SVG_ANIM_ATTR_TRANSFORM ||
            type_text == NULL)) ||
        (kind == SVG_ANIM_KIND_ANIMATE && attribute == SVG_ANIM_ATTR_TRANSFORM))
        return SVG_ANIM_AUTHORING_UNSUPPORTED;
    if (!validate_new_value(initial_value, attribute, kind,
            type_text == NULL ? "translate" : type_text) ||
        !validate_new_value(first_value, attribute, kind,
            type_text == NULL ? "translate" : type_text))
        return SVG_ANIM_AUTHORING_INVALID_ARGUMENT;
    double position = time_seconds / duration_seconds;
    if (!isfinite(position) || position <= 0.0 || position > 1.0)
        return SVG_ANIM_AUTHORING_INVALID_ARGUMENT;
    authoring_builder fragment = {0};
    const char *element = kind == SVG_ANIM_KIND_TRANSFORM ? "animateTransform" : "animate";
    char duration_text[64];
    int duration_amount = snprintf(duration_text, sizeof(duration_text), "%.17gs", duration_seconds);
    if (duration_amount <= 0 || (size_t)duration_amount >= sizeof(duration_text) ||
        !authoring_builder_byte(&fragment, '<') || !authoring_builder_cstr(&fragment, element) ||
        !authoring_builder_cstr(&fragment, " attributeName=\"") ||
        !authoring_builder_cstr(&fragment, attribute_text) ||
        !authoring_builder_byte(&fragment, '"')) goto oom;
    if (kind == SVG_ANIM_KIND_TRANSFORM &&
        (!authoring_builder_cstr(&fragment, " type=\"") || !authoring_builder_cstr(&fragment, type_text) ||
         !authoring_builder_byte(&fragment, '"'))) goto oom;
    if (!authoring_builder_cstr(&fragment, " values=\"") ||
        !authoring_builder_attribute_value(&fragment, initial_value, '"') ||
        !authoring_builder_byte(&fragment, ';') ||
        !authoring_builder_attribute_value(&fragment, first_value, '"')) goto oom;
    if (position < 1.0 &&
        (!authoring_builder_byte(&fragment, ';') ||
         !authoring_builder_attribute_value(&fragment, first_value, '"'))) goto oom;
    if (!authoring_builder_cstr(&fragment, "\" begin=\"0s\" keyTimes=\"0;") ||
        !authoring_append_number(&fragment, position)) goto oom;
    if (position < 1.0 && !authoring_builder_append(&fragment,
            (const uint8_t *)";1", 2U)) goto oom;
    if (!authoring_builder_cstr(&fragment, "\" dur=\"") ||
        !authoring_builder_append(&fragment, (const uint8_t *)duration_text,
            (size_t)duration_amount) ||
        !authoring_builder_cstr(&fragment, "\" calcMode=\"linear\" fill=\"freeze\"/>"))
        goto oom;
    *out_fragment = fragment.bytes;
    *out_length = fragment.length;
    return SVG_ANIM_AUTHORING_OK;
oom:
    free(fragment.bytes);
    return SVG_ANIM_AUTHORING_OUT_OF_MEMORY;
}
