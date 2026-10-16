#define _POSIX_C_SOURCE 200809L

#include "mcp_broker_internal.h"

#include "svg/animation/animation.h"
#include "svg/xml/xml_tokenizer.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MCP_ANIMATION_SOURCE_LIMIT (16U * 1024U * 1024U)
#define MCP_ANIMATION_VALUE_CAP 2048U

typedef struct animation_snapshot {
    uint8_t *source;
    svg_anim_document document;
} animation_snapshot;

static void snapshot_destroy(animation_snapshot *snapshot)
{
    if (snapshot == NULL) return;
    svg_anim_document_destroy(&snapshot->document);
    free(snapshot->source);
    *snapshot = (animation_snapshot){0};
}

static bool snapshot_create(mcp_broker_document *document, animation_snapshot *out)
{
    if (document == NULL || document->edit == NULL || out == NULL) return false;
    *out = (animation_snapshot){0};
    size_t length = document_edit_byte_length(document->edit);
    if (length == 0U || length > MCP_ANIMATION_SOURCE_LIMIT) return false;
    uint8_t *source = malloc(length);
    size_t capacity = length / 2U + 2U;
    if (source == NULL || capacity > SIZE_MAX / sizeof(svg_xml_token)) {
        free(source);
        return false;
    }
    svg_xml_token *tokens = calloc(capacity, sizeof(*tokens));
    if (tokens == NULL || !document_edit_materialize(document->edit, source, length, &length)) {
        free(tokens);
        free(source);
        return false;
    }
    svg_xml_result xml = svg_xml_tokenize((base_bytes){source, length},
        svg_xml_default_options(), tokens, capacity);
    bool okay = xml.diagnostic.category == SVG_XML_ERROR_NONE &&
        svg_anim_parse((base_bytes){source, length}, tokens, xml.token_count,
                       &out->document, NULL);
    free(tokens);
    if (!okay) {
        svg_anim_document_destroy(&out->document);
        free(source);
        return false;
    }
    out->source = source;
    return true;
}

static bool append_number(mcp_json_buffer *out, double value)
{
    if (!isfinite(value)) return mcp_json_buffer_literal(out, "null");
    char text[48];
    int length = snprintf(text, sizeof(text), "%.17g", value);
    return length > 0 && (size_t)length < sizeof(text) &&
        mcp_json_buffer_append(out, text, (size_t)length);
}

static bool append_size(mcp_json_buffer *out, size_t value)
{
    char text[32];
    int length = snprintf(text, sizeof(text), "%zu", value);
    return length > 0 && (size_t)length < sizeof(text) &&
        mcp_json_buffer_append(out, text, (size_t)length);
}

static const char *kind_name(svg_anim_kind kind)
{
    switch (kind) {
        case SVG_ANIM_KIND_ANIMATE: return "animate";
        case SVG_ANIM_KIND_TRANSFORM: return "animateTransform";
        case SVG_ANIM_KIND_SET: return "set";
        case SVG_ANIM_KIND_MOTION: return "animateMotion";
    }
    return "unknown";
}

static const char *value_kind_name(svg_anim_value_kind kind)
{
    switch (kind) {
        case SVG_ANIM_VALUE_NUMBER: return "number";
        case SVG_ANIM_VALUE_COLOR: return "color";
        case SVG_ANIM_VALUE_TRANSFORM: return "transform";
        case SVG_ANIM_VALUE_MOTION: return "motion";
        case SVG_ANIM_VALUE_PATH: return "path";
    }
    return "unknown";
}

static bool append_value(mcp_json_buffer *out, const svg_anim_value *value)
{
    if (value == NULL || !mcp_json_buffer_literal(out, "{\"kind\":" ) ||
        !mcp_json_buffer_quoted(out, value_kind_name(value->kind),
                                strlen(value_kind_name(value->kind)))) return false;
    if (value->kind == SVG_ANIM_VALUE_NUMBER) {
        if (!mcp_json_buffer_literal(out, ",\"value\":" ) ||
            !append_number(out, value->number[0])) return false;
    } else if (value->kind == SVG_ANIM_VALUE_COLOR) {
        if (!mcp_json_buffer_literal(out, ",\"rgba\":[")) return false;
        for (size_t index = 0U; index < 4U; index++) {
            if (index > 0U && !mcp_json_buffer_literal(out, ",")) return false;
            if (!append_size(out, value->rgba[index])) return false;
        }
        if (!mcp_json_buffer_literal(out, "]")) return false;
    } else {
        if (!mcp_json_buffer_literal(out, ",\"components\":[")) return false;
        for (size_t index = 0U; index < 4U; index++) {
            if (index > 0U && !mcp_json_buffer_literal(out, ",")) return false;
            if (!append_number(out, value->number[index])) return false;
        }
        if (!mcp_json_buffer_literal(out, "]")) return false;
    }
    return mcp_json_buffer_literal(out, "}");
}

static bool append_diagnostics(const animation_snapshot *snapshot, mcp_json_buffer *out)
{
    if (!mcp_json_buffer_literal(out, "[")) return false;
    for (size_t index = 0U; index < snapshot->document.diagnostic_count; index++) {
        const svg_anim_diagnostic *diagnostic = &snapshot->document.diagnostics[index];
        const char *name = svg_anim_diagnostic_name(diagnostic->kind);
        if (index > 0U && !mcp_json_buffer_literal(out, ",")) return false;
        if (!mcp_json_buffer_literal(out, "{\"kind\":" ) ||
            !mcp_json_buffer_quoted(out, name, strlen(name)) ||
            !mcp_json_buffer_literal(out, ",\"offset\":" ) ||
            !append_size(out, diagnostic->offset) ||
            !mcp_json_buffer_literal(out, "}")) return false;
    }
    return mcp_json_buffer_literal(out, "]");
}

static bool append_track(mcp_broker_document *document,
                         const animation_snapshot *snapshot,
                         const svg_anim_track *track, mcp_json_buffer *out)
{
    document_handle target;
    if (!document_edit_element_at_offset(document->edit, track->target_offset, &target))
        return false;
    const char *attribute = svg_anim_attribute_name(track->attribute);
    if (!mcp_json_buffer_literal(out, "{\"animationId\":")) return false;
    if (track->id.length > 0U) {
        if (!mcp_json_buffer_quoted(out, (const char *)snapshot->source + track->id.offset,
                                    track->id.length)) return false;
    } else if (!mcp_json_buffer_literal(out, "null")) return false;
    if (!mcp_json_buffer_literal(out, ",\"kind\":" ) ||
        !mcp_json_buffer_quoted(out, kind_name(track->kind), strlen(kind_name(track->kind))) ||
        !mcp_json_buffer_literal(out, ",\"attribute\":" ) ||
        !mcp_json_buffer_quoted(out, attribute, strlen(attribute)) ||
        !mcp_json_buffer_literal(out, ",\"targetId\":" ) ||
        !mcp_broker_element_id(document, target, out) ||
        !mcp_json_buffer_literal(out, ",\"supported\":" ) ||
        !mcp_json_buffer_literal(out, track->supported ? "true" : "false") ||
        !mcp_json_buffer_literal(out, ",\"timing\":{\"begin\":" ) ||
        !append_number(out, track->begin_seconds) ||
        !mcp_json_buffer_literal(out, ",\"duration\":" ) ||
        !append_number(out, track->duration_seconds) ||
        !mcp_json_buffer_literal(out, ",\"end\":" ) ||
        !append_number(out, track->end_seconds) ||
        !mcp_json_buffer_literal(out, ",\"repeatCount\":" ) ||
        !append_number(out, track->repeat_count) ||
        !mcp_json_buffer_literal(out, "},\"sourceSpan\":{\"offset\":" ) ||
        !append_size(out, track->element_offset) ||
        !mcp_json_buffer_literal(out, ",\"length\":" ) ||
        !append_size(out, track->element_length) ||
        !mcp_json_buffer_literal(out, "},\"targetSpan\":{\"offset\":" ) ||
        !append_size(out, track->target_offset) ||
        !mcp_json_buffer_literal(out, ",\"length\":" ) ||
        !append_size(out, track->target_length) ||
        !mcp_json_buffer_literal(out, "},\"keys\":[")) return false;
    for (size_t key_index = 0U; key_index < track->key_count; key_index++) {
        const svg_anim_keyframe *key = &track->keys[key_index];
        if (key_index > 0U && !mcp_json_buffer_literal(out, ",")) return false;
        if (!mcp_json_buffer_literal(out, "{\"time\":" ) ||
            !append_number(out, key->time) ||
            !mcp_json_buffer_literal(out, ",\"value\":" ) ||
            !append_value(out, &key->value) ||
            !mcp_json_buffer_literal(out, ",\"sourceSpan\":{\"offset\":" ) ||
            !append_size(out, key->source_offset) ||
            !mcp_json_buffer_literal(out, ",\"length\":" ) ||
            !append_size(out, key->source_length) ||
            !mcp_json_buffer_literal(out, "}}")) return false;
    }
    return mcp_json_buffer_literal(out, "]}");
}

static bool animation_list(mcp_broker_document *document, mcp_tool_response *out)
{
    animation_snapshot snapshot = {0};
    if (!snapshot_create(document, &snapshot))
        return mcp_broker_set_error(out, "animation_decode_failed", "document could not be decoded within the animation source limit");
    bool okay = mcp_json_buffer_literal(&out->structured_json, "{\"documentId\":" ) &&
        mcp_json_buffer_quoted(&out->structured_json, document->id, strlen(document->id)) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"tracks\":[" );
    for (size_t index = 0U; okay && index < snapshot.document.track_count; index++) {
        if (index > 0U) okay = mcp_json_buffer_literal(&out->structured_json, ",");
        if (okay) okay = append_track(document, &snapshot, &snapshot.document.tracks[index],
                                      &out->structured_json);
    }
    if (okay) okay = mcp_json_buffer_literal(&out->structured_json, "],\"diagnostics\":" ) &&
        append_diagnostics(&snapshot, &out->structured_json) &&
        mcp_json_buffer_literal(&out->structured_json, "}");
    snapshot_destroy(&snapshot);
    if (okay) return true;
    out->structured_json.length = 0U;
    if (out->structured_json.data != NULL) out->structured_json.data[0] = '\0';
    return mcp_broker_set_error(out, "animation_list_failed",
        "animation listing could not be represented within the bounded MCP result buffer");
}

static bool find_track(mcp_broker_document *document, const mcp_json_doc *request,
                       size_t args, animation_snapshot *snapshot,
                       document_handle *out_handle)
{
    char animation_id[256];
    size_t id_length = 0U;
    if (!mcp_broker_string_arg(request, args, "animationId", animation_id,
                               sizeof(animation_id), &id_length) || id_length == 0U ||
        !snapshot_create(document, snapshot)) return false;
    const svg_anim_track *found = NULL;
    for (size_t index = 0U; index < snapshot->document.track_count; index++) {
        const svg_anim_track *track = &snapshot->document.tracks[index];
        if (track->id.length != id_length ||
            memcmp(snapshot->source + track->id.offset, animation_id, id_length) != 0) continue;
        if (found != NULL) return false;
        found = track;
    }
    return found != NULL && document_edit_element_at_offset(document->edit,
        found->element_offset, out_handle);
}

static bool allowed_animation_fragment(const char *xml, size_t length)
{
    if (xml == NULL || length == 0U || length > DOCUMENT_EDIT_XML_FRAGMENT_LIMIT) return false;
    size_t capacity = length / 2U + 2U;
    if (capacity > SIZE_MAX / sizeof(svg_xml_token)) return false;
    svg_xml_token *tokens = calloc(capacity, sizeof(*tokens));
    if (tokens == NULL) return false;
    svg_xml_result parsed = svg_xml_tokenize((base_bytes){(const uint8_t *)xml, length},
        svg_xml_default_options(), tokens, capacity);
    bool allowed = false;
    if (parsed.diagnostic.category == SVG_XML_ERROR_NONE) {
        for (size_t index = 0U; index < parsed.token_count; index++) {
            const svg_xml_token *token = &tokens[index];
            if (token->kind != SVG_XML_TOKEN_START_ELEMENT &&
                token->kind != SVG_XML_TOKEN_EMPTY_ELEMENT) continue;
            if (token->depth != 0U || token->prefix.length != 0U) break;
            const char *name = xml + token->name.offset;
            size_t name_length = token->name.length;
            allowed = (name_length == 7U && memcmp(name, "animate", 7U) == 0) ||
                (name_length == 16U && memcmp(name, "animateTransform", 16U) == 0) ||
                (name_length == 3U && memcmp(name, "set", 3U) == 0) ||
                (name_length == 13U && memcmp(name, "animateMotion", 13U) == 0);
            break;
        }
    }
    free(tokens);
    return allowed;
}

static bool animation_edit(mcp_broker *broker, mcp_broker_document *document,
                           const mcp_json_doc *request, size_t args,
                           const char *tool, mcp_tool_response *out)
{
    document_handle target = {0};
    document_edit_error error = DOCUMENT_EDIT_INVALID_ARGUMENT;
    animation_snapshot snapshot = {0};
    if (strcmp(tool, "animation_add") == 0) {
        char target_id[256], xml[DOCUMENT_EDIT_XML_FRAGMENT_LIMIT + 1U];
        size_t target_length = 0U, xml_length = 0U;
        if (!mcp_broker_string_arg(request, args, "targetId", target_id,
                sizeof(target_id), &target_length) ||
            !mcp_broker_string_arg(request, args, "xml", xml, sizeof(xml), &xml_length) ||
            !allowed_animation_fragment(xml, xml_length) ||
            !mcp_broker_resolve_element(document, target_id, target_length, &target))
            return mcp_broker_set_error(out, "invalid_animation", "target or animation fragment is invalid");
        error = document_edit_insert_xml_fragment(document->edit, target,
            (base_bytes){(const uint8_t *)xml, xml_length}, &target);
    } else {
        if (!find_track(document, request, args, &snapshot, &target)) {
            snapshot_destroy(&snapshot);
            return mcp_broker_set_error(out, "animation_not_found", "animationId must identify one supported track");
        }
        if (strcmp(tool, "animation_remove") == 0) {
            error = document_edit_delete_element(document->edit, target);
        } else {
            char name[128], value[MCP_ANIMATION_VALUE_CAP];
            size_t name_length = 0U, value_length = 0U;
            if (mcp_broker_string_arg(request, args, "name", name, sizeof(name), &name_length) &&
                mcp_broker_string_arg(request, args, "value", value, sizeof(value), &value_length) &&
                name_length > 0U && memchr(name, '\0', name_length) == NULL &&
                memchr(value, '\0', value_length) == NULL) {
                bool present = false;
                size_t count = document_edit_attribute_count(document->edit, target);
                for (size_t index = 0U; index < count; index++) {
                    document_edit_attribute attribute;
                    base_bytes attribute_name;
                    if (document_edit_attribute_at(document->edit, target, index, &attribute) &&
                        document_edit_span_bytes(document->edit, attribute.name, &attribute_name) &&
                        attribute_name.len == name_length &&
                        memcmp(attribute_name.ptr, name, name_length) == 0) {
                        present = true;
                        break;
                    }
                }
                error = present ? document_edit_set_attribute(document->edit, target,
                    (base_bytes){(const uint8_t *)name, name_length},
                    (base_bytes){(const uint8_t *)value, value_length}) :
                    document_edit_add_attribute(document->edit, target,
                    (base_bytes){(const uint8_t *)name, name_length},
                    (base_bytes){(const uint8_t *)value, value_length});
            }
        }
    }
    snapshot_destroy(&snapshot);
    if (error != DOCUMENT_EDIT_OK) {
        const char *name = document_edit_error_name(error);
        return mcp_broker_set_error(out, name == NULL ? "animation_edit_failed" : name,
                                    name == NULL ? "animation edit failed" : name);
    }
    document_handle selection;
    if (!document_edit_root(document->edit, &selection))
        return mcp_broker_set_error(out, "document_invalid", "animation edit succeeded but the root could not be rebound");
    mcp_broker_after_edit(broker, document, selection);
    return mcp_json_buffer_literal(&out->structured_json, "{\"documentId\":" ) &&
        mcp_json_buffer_quoted(&out->structured_json, document->id, strlen(document->id)) &&
        mcp_json_buffer_literal(&out->structured_json, ",\"changed\":true}");
}

bool mcp_broker_animation_call(mcp_broker *broker, const mcp_json_doc *request,
                               size_t args, const char *tool,
                               mcp_tool_response *out)
{
    if (broker == NULL || request == NULL || tool == NULL || out == NULL) return false;
    mcp_broker_document *document = mcp_broker_find(broker, request, args, "documentId");
    if (document == NULL) return mcp_broker_set_error(out, "document_not_found", "documentId is not open");
    if (strcmp(tool, "animation_list") == 0) return animation_list(document, out);
    if (strcmp(tool, "animation_add") == 0 || strcmp(tool, "animation_set") == 0 ||
        strcmp(tool, "animation_remove") == 0)
        return animation_edit(broker, document, request, args, tool, out);
    return mcp_broker_export_animation(broker, document, request, args, out);
}
