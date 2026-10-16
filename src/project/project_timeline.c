#include "project_timeline_json.h"

#include <math.h>
#include <stdlib.h>
#include <string.h>

static bool valid_ascii_id(base_bytes id)
{
    if (id.len == 0U || id.len > 128U) return false;
    for (size_t i = 0U; i < id.len; i++) {
        uint8_t b = id.ptr[i];
        if (!((b >= 'a' && b <= 'z') || (b >= 'A' && b <= 'Z') ||
              (b >= '0' && b <= '9') || b == '_' || b == '-')) return false;
    }
    return true;
}

static bool find_asset(const project_timeline_context *context, base_bytes name)
{
    for (size_t i = 0U; i < context->asset_count; i++) {
        const project_timeline_asset_ref *asset = &context->assets[i];
        if (bytes_equal(asset->name, name))
            return field_equal(asset->media_type, "image/svg+xml");
    }
    return false;
}

static bool valid_asset_name(base_bytes name)
{
    if (!base_bytes_valid(name) || name.len == 0U ||
        name.len > PROJECT_BUNDLE_MAX_ASSET_NAME_BYTES ||
        (name.len == 1U && name.ptr[0] == '.') ||
        (name.len == 2U && name.ptr[0] == '.' && name.ptr[1] == '.')) return false;
    for (size_t i = 0U; i < name.len; i++) {
        uint8_t b = name.ptr[i];
        if (!((b >= 'a' && b <= 'z') || (b >= 'A' && b <= 'Z') ||
              (b >= '0' && b <= '9') || b == '.' || b == '_' || b == '-')) return false;
    }
    return true;
}

static bool valid_media_type(base_bytes type)
{
    if (!base_bytes_valid(type) || type.len < 3U ||
        type.len > PROJECT_BUNDLE_MAX_MEDIA_TYPE_BYTES) return false;
    bool slash = false;
    for (size_t i = 0U; i < type.len; i++) {
        uint8_t byte = type.ptr[i];
        if (byte == '/') {
            if (slash || i == 0U || i + 1U == type.len) return false;
            slash = true;
        } else if (byte < 0x21U || byte > 0x7eU) return false;
    }
    return slash;
}

static bool find_scene_id(const project_timeline_context *context, base_bytes id)
{
    size_t low = 0U, high = context->scene_id_count;
    while (low < high) {
        size_t middle = low + (high - low) / 2U;
        int order = bytes_compare(context->scene_ids[middle], id);
        if (order == 0) return true;
        if (order < 0) low = middle + 1U;
        else high = middle;
    }
    return false;
}

static bool parse_tick_item(json_reader *reader, json_span span, bool drawing,
                            base_bytes value_type, uint32_t duration,
                            uint32_t *out_tick, base_bytes *out_asset)
{
    object_fields fields;
    if (!parse_object_fields(reader, span, &fields)) return false;
    const char *const drawing_fields[] = {"tick", "asset", "label"};
    const char *const key_fields[] = {"tick", "value"};
    if ((!drawing && !only_fields(&fields, key_fields, 2U)) ||
        (drawing && (!only_fields(&fields, drawing_fields, 3U) &&
                     !(fields.count == 2U && get_field(&fields, "tick") != NULL &&
                       get_field(&fields, "asset") != NULL))))
        return fail_at(reader, PROJECT_TIMELINE_INVALID_SCHEMA, span.start);
    const json_field *tick = get_field(&fields, "tick");
    const json_field *payload = get_field(&fields, drawing ? "asset" : "value");
    if (tick == NULL || payload == NULL || !span_uint(reader, tick->value, out_tick)) return false;
    if (*out_tick >= duration)
        return fail_at(reader, PROJECT_TIMELINE_INVALID_VALUE, tick->value.start);
    if (drawing) {
        if (!span_string(reader, payload->value, out_asset)) return false;
        const json_field *label = get_field(&fields, "label");
        if (label != NULL) {
            base_bytes label_value;
            if (!span_string(reader, label->value, &label_value)) return false;
            (void)label_value;
        }
    } else {
        if (json_string_equal(value_type, "boolean")) {
            bool ignored;
            if (!span_bool(reader, payload->value, &ignored)) return false;
        } else {
            json_reader child = *reader;
            child.cursor = payload->value.start;
            skip_space(&child);
            if (json_string_equal(value_type, "scalar")) {
                if (child.cursor >= child.source.len ||
                    !(child.source.ptr[child.cursor] == '-' ||
                      (child.source.ptr[child.cursor] >= '0' && child.source.ptr[child.cursor] <= '9')) ||
                    !scan_number(&child)) { *reader = child; return fail_at(reader, PROJECT_TIMELINE_INVALID_VALUE, payload->value.start); }
            } else {
                json_span components[6];
                size_t count = 0U;
                size_t expected = json_string_equal(value_type, "rgba") ? 4U : 6U;
                if (!span_array(&child, payload->value, components, 6U, &count) || count != expected)
                    return fail_at(reader, PROJECT_TIMELINE_INVALID_VALUE, payload->value.start);
                for (size_t i = 0U; i < count; i++) {
                    json_reader component_reader = *reader;
                    component_reader.cursor = components[i].start;
                    if (component_reader.cursor >= component_reader.source.len ||
                        !(component_reader.source.ptr[component_reader.cursor] == '-' ||
                          (component_reader.source.ptr[component_reader.cursor] >= '0' && component_reader.source.ptr[component_reader.cursor] <= '9')) ||
                          !scan_number(&component_reader)) return fail_at(reader, PROJECT_TIMELINE_INVALID_VALUE, components[i].start);
                    if (json_string_equal(value_type, "rgba")) {
                        double component_value = 0.0;
                        if (!span_number_value(reader, components[i], &component_value) ||
                            component_value < 0.0 || component_value > 1.0)
                            return fail_at(reader, PROJECT_TIMELINE_INVALID_VALUE, components[i].start);
                    }
                }
            }
        }
    }
    return true;
}

typedef struct item_validation {
    bool drawing;
    base_bytes value_type;
    uint32_t duration;
    uint32_t previous_tick;
    const project_timeline_context *context;
} item_validation;

typedef bool (*array_visitor_fn)(json_reader *, json_span, size_t, void *);

static bool array_visit(json_reader *reader, json_span span, size_t capacity,
                        array_visitor_fn visitor, void *context, size_t *out_count)
{
    json_reader child = *reader;
    child.cursor = span.start;
    skip_space(&child);
    if (child.cursor >= child.source.len || child.source.ptr[child.cursor++] != '[') {
        *reader = child; return fail_at(reader, PROJECT_TIMELINE_INVALID_SCHEMA, span.start);
    }
    size_t count = 0U;
    skip_space(&child);
    if (child.cursor < child.source.len && child.source.ptr[child.cursor] == ']') child.cursor++;
    else for (;;) {
        if (count >= capacity) { *reader = child; return fail(reader, PROJECT_TIMELINE_LIMIT); }
        size_t start = child.cursor;
        if (!scan_value(&child, 1U)) { *reader = child; return false; }
        json_span item = {start, child.cursor};
        if (visitor != NULL && !visitor(&child, item, count, context)) {
            *reader = child;
            return false;
        }
        count++;
        skip_space(&child);
        if (child.cursor >= child.source.len) { *reader = child; return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON); }
        uint8_t byte = child.source.ptr[child.cursor++];
        if (byte == ']') break;
        if (byte != ',') { *reader = child; return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON); }
        skip_space(&child);
    }
    if (child.cursor != span.end) { *reader = child; return fail(reader, PROJECT_TIMELINE_MALFORMED_JSON); }
    *out_count = count;
    return true;
}

static bool validate_item_visit(json_reader *reader, json_span span,
                                size_t index, void *opaque)
{
    item_validation *state = opaque;
    uint32_t tick = 0U;
    base_bytes asset = {NULL, 0U};
    if (!parse_tick_item(reader, span, state->drawing, state->value_type,
                         state->duration, &tick, &asset)) return false;
    if ((index == 0U && tick != 0U) || (index != 0U && tick <= state->previous_tick)) {
        reader->error = span.start;
        reader->error_kind = PROJECT_TIMELINE_DUPLICATE;
        return false;
    }
    if (state->drawing) {
        char asset_name[PROJECT_BUNDLE_MAX_ASSET_NAME_BYTES];
        size_t asset_length = 0U;
        if (!project_timeline_string_copy(asset,
                (base_mut_bytes){(uint8_t *)asset_name, sizeof(asset_name)}, &asset_length) ||
            !find_asset(state->context,
                (base_bytes){(const uint8_t *)asset_name, asset_length})) {
            reader->error = span.start;
            reader->error_kind = PROJECT_TIMELINE_INVALID_REFERENCE;
            return false;
        }
    }
    state->previous_tick = tick;
    return true;
}

static project_timeline_result result(project_timeline_error error, size_t offset)
{
    return (project_timeline_result){error, offset};
}

project_timeline_result project_timeline_decode_cancellable(base_bytes source,
    const project_timeline_context *context, project_timeline_view *out_view,
    bool (*cancel_check)(void *), void *cancel_context, bool *out_canceled)
{
    if (out_canceled != NULL) *out_canceled = false;
    if (out_view == NULL || !base_bytes_valid(source) || context == NULL ||
        (context->scene_ids == NULL && context->scene_id_count != 0U) ||
        (context->assets == NULL && context->asset_count != 0U))
        return result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    memset(out_view, 0, sizeof(*out_view));
    if (context->scene_id_count > 1000000U || context->asset_count > PROJECT_BUNDLE_MAX_ASSET_COUNT)
        return result(PROJECT_TIMELINE_LIMIT, 0U);
    for (size_t i = 0U; i < context->scene_id_count; i++) {
        if (!base_bytes_valid(context->scene_ids[i]) || context->scene_ids[i].len == 0U ||
            context->scene_ids[i].len > 256U) return result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
        if (i > 0U) {
            int order = bytes_compare(context->scene_ids[i - 1U], context->scene_ids[i]);
            if (order == 0) return result(PROJECT_TIMELINE_DUPLICATE, 0U);
            if (order > 0) return result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
        }
    }
    for (size_t i = 0U; i < context->asset_count; i++) {
        if (!valid_asset_name(context->assets[i].name) ||
            !valid_media_type(context->assets[i].media_type))
            return result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
        for (size_t prior = 0U; prior < i; prior++)
            if (bytes_equal(context->assets[prior].name, context->assets[i].name))
                return result(PROJECT_TIMELINE_DUPLICATE, 0U);
    }
    if (source.len == 0U) { out_view->empty = true; return result(PROJECT_TIMELINE_OK, 0U); }
    if (source.len > PROJECT_BUNDLE_MAX_TIMELINE_BYTES)
        return result(PROJECT_TIMELINE_LIMIT, 0U);

    json_reader reader = {.source = source, .error_kind = PROJECT_TIMELINE_OK,
        .cancel_check = cancel_check, .cancel_context = cancel_context};
    size_t root_start = 0U;
    if (!scan_value(&reader, 0U)) {
        if (out_canceled != NULL) *out_canceled = reader.canceled;
        return result(reader.error_kind, reader.error);
    }
    root_start = reader.cursor;
    skip_space(&reader);
    if (reader.cursor != source.len) return result(PROJECT_TIMELINE_MALFORMED_JSON, reader.cursor);
    object_fields root;
    if (!parse_object_fields(&reader, (json_span){0U, root_start}, &root))
        return result(reader.error_kind, reader.error);
    const char *const root_names[] = {"schema", "version", "ticks_per_second", "duration_ticks", "fps", "loop", "onion", "tracks"};
    base_bytes schema;
    uint32_t version, tps, duration;
    const json_field *f_schema = get_field(&root, "schema");
    const json_field *f_version = get_field(&root, "version");
    if (f_schema == NULL)
        return result(PROJECT_TIMELINE_UNSUPPORTED_VERSION, 0U);
    if (!span_string(&reader, f_schema->value, &schema))
        return result(PROJECT_TIMELINE_INVALID_SCHEMA, reader.error);
    if (!json_string_equal(schema, "vexel.animation")) return result(PROJECT_TIMELINE_UNSUPPORTED_VERSION, f_schema->value.start);
    if (f_version == NULL) return result(PROJECT_TIMELINE_INVALID_SCHEMA, f_schema->value.start);
    if (!span_uint(&reader, f_version->value, &version)) return result(reader.error_kind, reader.error);
    if (version != 1U) return result(PROJECT_TIMELINE_UNSUPPORTED_VERSION, f_version->value.start);
    if (!only_fields(&root, root_names, 8U)) return result(PROJECT_TIMELINE_INVALID_SCHEMA, 0U);
    const json_field *f_tps = get_field(&root, "ticks_per_second");
    const json_field *f_duration = get_field(&root, "duration_ticks");
    const json_field *f_fps = get_field(&root, "fps");
    const json_field *f_loop = get_field(&root, "loop");
    const json_field *f_onion = get_field(&root, "onion");
    const json_field *f_tracks = get_field(&root, "tracks");
    if (f_tps == NULL || f_duration == NULL || f_fps == NULL || f_loop == NULL ||
        f_onion == NULL || f_tracks == NULL)
        return result(PROJECT_TIMELINE_INVALID_SCHEMA, 0U);
    if (!span_uint(&reader, f_tps->value, &tps)) return result(reader.error_kind, f_tps->value.start);
    if (tps == 0U || tps > 240U) return result(PROJECT_TIMELINE_INVALID_VALUE, f_tps->value.start);
    if (!span_uint(&reader, f_duration->value, &duration)) return result(reader.error_kind, f_duration->value.start);
    if (duration == 0U || duration > 14400000U)
        return result(PROJECT_TIMELINE_INVALID_VALUE, f_duration->value.start);
    object_fields fps;
    if (!parse_object_fields(&reader, f_fps->value, &fps)) return result(reader.error_kind, reader.error);
    const char *const fps_names[] = {"numerator", "denominator"};
    uint32_t numerator = 0U, denominator = 0U;
    const json_field *fps_numerator = get_field(&fps, "numerator");
    const json_field *fps_denominator = get_field(&fps, "denominator");
    if (!only_fields(&fps, fps_names, 2U) || fps_numerator == NULL || fps_denominator == NULL)
        return result(PROJECT_TIMELINE_INVALID_VALUE, f_fps->value.start);
    if (!span_uint(&reader, fps_numerator->value, &numerator) ||
        !span_uint(&reader, fps_denominator->value, &denominator))
        return result(reader.error_kind, reader.error);
    if (numerator == 0U || denominator == 0U ||
        ((uint64_t)tps * denominator) % numerator != 0U)
        return result(PROJECT_TIMELINE_INVALID_VALUE, f_fps->value.start);
    base_bytes loop;
    if (!span_string(&reader, f_loop->value, &loop))
        return result(PROJECT_TIMELINE_INVALID_VALUE, f_loop->value.start);
    project_timeline_loop_mode loop_mode;
    if (json_string_equal(loop, "once")) loop_mode = PROJECT_TIMELINE_LOOP_ONCE;
    else if (json_string_equal(loop, "loop")) loop_mode = PROJECT_TIMELINE_LOOP_REPEAT;
    else if (json_string_equal(loop, "ping_pong"))
        loop_mode = PROJECT_TIMELINE_LOOP_PING_PONG;
    else return result(PROJECT_TIMELINE_INVALID_VALUE, f_loop->value.start);
    object_fields onion;
    if (!parse_object_fields(&reader, f_onion->value, &onion)) return result(reader.error_kind, reader.error);
    const char *const onion_names[] = {"before", "after", "opacity"};
    uint32_t before = 0U, after = 0U;
    const json_field *onion_before = get_field(&onion, "before");
    const json_field *onion_after = get_field(&onion, "after");
    if (!only_fields(&onion, onion_names, 3U) ||
        onion_before == NULL || onion_after == NULL ||
        !span_uint(&reader, onion_before->value, &before) ||
        !span_uint(&reader, onion_after->value, &after) || before > 8U || after > 8U)
        return result(PROJECT_TIMELINE_INVALID_VALUE, f_onion->value.start);
    const json_field *opacity = get_field(&onion, "opacity");
    json_reader opacity_reader = reader;
    opacity_reader.cursor = opacity->value.start;
    if (opacity_reader.cursor >= source.len ||
        !(source.ptr[opacity_reader.cursor] == '-' ||
          (source.ptr[opacity_reader.cursor] >= '0' && source.ptr[opacity_reader.cursor] <= '9')) ||
        !scan_number(&opacity_reader)) return result(PROJECT_TIMELINE_INVALID_VALUE, opacity->value.start);
    double opacity_value = 0.0;
    if (!span_number_value(&reader, opacity->value, &opacity_value) ||
        opacity_value < 0.0 || opacity_value > 1.0)
        return result(PROJECT_TIMELINE_INVALID_VALUE, opacity->value.start);

    json_span track_spans[PROJECT_TIMELINE_MAX_TRACKS];
    size_t track_count = 0U;
    if (!span_array(&reader, f_tracks->value, track_spans, PROJECT_TIMELINE_MAX_TRACKS, &track_count))
        return result(reader.error_kind, reader.error);
    char track_id_storage[PROJECT_TIMELINE_MAX_TRACKS][129];
    base_bytes track_ids[PROJECT_TIMELINE_MAX_TRACKS];
    size_t total_items = 0U;
    for (size_t ti = 0U; ti < track_count; ti++) {
        object_fields track;
        if (!parse_object_fields(&reader, track_spans[ti], &track)) return result(reader.error_kind, reader.error);
        const json_field *kind_field = get_field(&track, "kind");
        base_bytes kind;
        if (kind_field == NULL || !span_string(&reader, kind_field->value, &kind)) return result(PROJECT_TIMELINE_INVALID_SCHEMA, track_spans[ti].start);
        bool drawing = json_string_equal(kind, "drawing");
        if (!drawing && !json_string_equal(kind, "property")) return result(PROJECT_TIMELINE_INVALID_VALUE, kind_field->value.start);
        const char *const common_names[] = {"id", "kind", "target", "name", "visible", drawing ? "cels" : "property", drawing ? "unused" : "value_type", drawing ? "unused2" : "interpolation", drawing ? "unused3" : "keys"};
        /* Kind payloads have distinct exact fields; optional presentation fields are accepted. */
        base_bytes id_raw, target_raw, ignored;
        char id_storage[129], target_storage[257];
        size_t id_length = 0U, target_length = 0U;
        base_bytes id, target;
        bool visible = false;
        const json_field *id_field = get_field(&track, "id"), *target_field = get_field(&track, "target");
        const json_field *visible_field = get_field(&track, "visible");
        if (id_field == NULL || target_field == NULL || visible_field == NULL ||
            !span_string(&reader, id_field->value, &id_raw) ||
            !project_timeline_string_copy(id_raw,
                (base_mut_bytes){(uint8_t *)id_storage, sizeof(id_storage)}, &id_length) ||
            !span_string(&reader, target_field->value, &target_raw) ||
            !project_timeline_string_copy(target_raw,
                (base_mut_bytes){(uint8_t *)target_storage, sizeof(target_storage)}, &target_length) ||
            !span_bool(&reader, visible_field->value, &visible))
            return result(PROJECT_TIMELINE_INVALID_SCHEMA, track_spans[ti].start);
        id = (base_bytes){(const uint8_t *)id_storage, id_length};
        target = (base_bytes){(const uint8_t *)target_storage, target_length};
        if (!valid_ascii_id(id) || target.len == 0U)
            return result(PROJECT_TIMELINE_INVALID_SCHEMA, track_spans[ti].start);
        for (size_t prior = 0U; prior < ti; prior++)
            if (bytes_equal(track_ids[prior], id)) return result(PROJECT_TIMELINE_DUPLICATE, id_field->value.start);
        memcpy(track_id_storage[ti], id_storage, id_length);
        track_ids[ti] = (base_bytes){(const uint8_t *)track_id_storage[ti], id_length};
        if (!find_scene_id(context, target)) return result(PROJECT_TIMELINE_INVALID_REFERENCE, target_field->value.start);
        const json_field *name_field = get_field(&track, "name");
        if (name_field != NULL && !span_string(&reader, name_field->value, &ignored)) return result(reader.error_kind, reader.error);
        (void)common_names;
        const json_field *items_field = get_field(&track, drawing ? "cels" : "keys");
        if (drawing) {
            const char *const names_with_name[] = {"id", "kind", "target", "name", "visible", "cels"};
            const char *const names_without_name[] = {"id", "kind", "target", "visible", "cels"};
            if (!only_fields(&track, name_field == NULL ? names_without_name : names_with_name,
                             name_field == NULL ? 5U : 6U) || items_field == NULL)
                return result(PROJECT_TIMELINE_INVALID_SCHEMA, track_spans[ti].start);
        } else {
            const char *const names_with_name[] = {"id", "kind", "target", "name", "visible", "property", "value_type", "interpolation", "keys"};
            const char *const names_without_name[] = {"id", "kind", "target", "visible", "property", "value_type", "interpolation", "keys"};
            if (!only_fields(&track, name_field == NULL ? names_without_name : names_with_name,
                             name_field == NULL ? 8U : 9U)) return result(PROJECT_TIMELINE_INVALID_SCHEMA, track_spans[ti].start);
        }
        size_t item_count = 0U;
        if (items_field == NULL) return result(PROJECT_TIMELINE_INVALID_SCHEMA, track_spans[ti].start);
        base_bytes value_type = {NULL, 0U};
        if (!drawing) {
            const json_field *property_field = get_field(&track, "property");
            const json_field *type_field = get_field(&track, "value_type");
            const json_field *interpolation_field = get_field(&track, "interpolation");
            base_bytes property, interpolation;
            if (property_field == NULL || type_field == NULL || interpolation_field == NULL ||
                !span_string(&reader, property_field->value, &property) || property.len == 0U ||
                !span_string(&reader, type_field->value, &value_type) ||
                !span_string(&reader, interpolation_field->value, &interpolation) ||
                !(json_string_equal(interpolation, "step") || json_string_equal(interpolation, "linear") || json_string_equal(interpolation, "cubic")) ||
                !(json_string_equal(value_type, "scalar") || json_string_equal(value_type, "transform2d") ||
                  json_string_equal(value_type, "rgba") || json_string_equal(value_type, "boolean")))
                return result(PROJECT_TIMELINE_INVALID_VALUE, track_spans[ti].start);
            bool property_matches =
                (json_string_equal(property, "x") || json_string_equal(property, "y") ||
                 json_string_equal(property, "width") || json_string_equal(property, "height") ||
                 json_string_equal(property, "cx") || json_string_equal(property, "cy") ||
                 json_string_equal(property, "r") || json_string_equal(property, "rx") ||
                 json_string_equal(property, "ry") || json_string_equal(property, "opacity") ||
                 json_string_equal(property, "fill-opacity") || json_string_equal(property, "stroke-opacity") ||
                 json_string_equal(property, "stroke-width") || json_string_equal(property, "stroke-dashoffset")) &&
                json_string_equal(value_type, "scalar");
            property_matches = property_matches ||
                ((json_string_equal(property, "fill") || json_string_equal(property, "stroke")) &&
                 json_string_equal(value_type, "rgba")) ||
                (json_string_equal(property, "transform") && json_string_equal(value_type, "transform2d")) ||
                (json_string_equal(property, "visibility") && json_string_equal(value_type, "boolean"));
            if (!property_matches) return result(PROJECT_TIMELINE_INVALID_VALUE, property_field->value.start);
            if (json_string_equal(value_type, "boolean") && !json_string_equal(interpolation, "step"))
                return result(PROJECT_TIMELINE_INVALID_VALUE, interpolation_field->value.start);
        }
        item_validation item_state = {drawing, value_type, duration, 0U, context};
        if (!array_visit(&reader, items_field->value, PROJECT_TIMELINE_MAX_ITEMS_PER_TRACK,
                         validate_item_visit, &item_state, &item_count))
            return result(reader.error_kind, reader.error);
        if (item_count == 0U) return result(PROJECT_TIMELINE_INVALID_VALUE, items_field->value.start);
        if (!base_size_add(total_items, item_count, &total_items) || total_items > PROJECT_TIMELINE_MAX_ITEMS)
            return result(PROJECT_TIMELINE_LIMIT, items_field->value.start);
    }
    project_timeline_view decoded = {0};
    decoded.source = source;
    decoded.tracks_offset = f_tracks->value.start;
    decoded.tracks_length = f_tracks->value.end - f_tracks->value.start;
    decoded.ticks_per_second = tps;
    decoded.duration_ticks = duration;
    decoded.fps_numerator = numerator;
    decoded.fps_denominator = denominator;
    decoded.loop_mode = loop_mode;
    decoded.onion_before = before;
    decoded.onion_after = after;
    decoded.onion_opacity = opacity_value;
    decoded.track_count = track_count;
    for (size_t i = 0U; i < track_count; i++) {
        decoded.track_spans[i] = (base_bytes){source.ptr + track_spans[i].start,
            track_spans[i].end - track_spans[i].start};
        object_fields fields;
        if (!parse_object_fields(&reader, track_spans[i], &fields))
            return result(reader.error_kind, reader.error);
        const json_field *kind_field = get_field(&fields, "kind");
        const json_field *items_field = NULL;
        base_bytes kind;
        if (kind_field == NULL || !span_string(&reader, kind_field->value, &kind))
            return result(PROJECT_TIMELINE_INVALID_SCHEMA, track_spans[i].start);
        decoded.drawing_tracks[i] = json_string_equal(kind, "drawing");
        items_field = get_field(&fields, decoded.drawing_tracks[i] ? "cels" : "keys");
        if (items_field == NULL) return result(PROJECT_TIMELINE_INVALID_SCHEMA, track_spans[i].start);
        decoded.item_spans[i] = (base_bytes){source.ptr + items_field->value.start,
            items_field->value.end - items_field->value.start};
        size_t item_count = 0U;
        if (!array_count(&reader, items_field->value, PROJECT_TIMELINE_MAX_ITEMS_PER_TRACK,
                         &item_count)) return result(reader.error_kind, reader.error);
        decoded.item_counts[i] = item_count;
    }
    *out_view = decoded;
    return result(PROJECT_TIMELINE_OK, 0U);
}

project_timeline_result project_timeline_decode(base_bytes source,
    const project_timeline_context *context, project_timeline_view *out_view)
{
    return project_timeline_decode_cancellable(source, context, out_view,
        NULL, NULL, NULL);
}

bool project_timeline_track_at(const project_timeline_view *view, size_t index,
                               project_timeline_track_view *out_track)
{
    if (view == NULL || out_track == NULL ||
        view->track_count > PROJECT_TIMELINE_MAX_TRACKS || index >= view->track_count ||
        !base_bytes_valid(view->source) || !base_bytes_valid(view->track_spans[index]) ||
        !base_bytes_valid(view->item_spans[index])) return false;
    json_reader reader = {.source = view->source, .error_kind = PROJECT_TIMELINE_OK};
    size_t start = (size_t)(view->track_spans[index].ptr - view->source.ptr);
    json_span track_span = {start, start + view->track_spans[index].len};
    object_fields track;
    if (!parse_object_fields(&reader, track_span, &track)) return false;
    const json_field *id = get_field(&track, "id"), *kind = get_field(&track, "kind"),
        *target = get_field(&track, "target"), *name = get_field(&track, "name"),
        *visible = get_field(&track, "visible");
    if (id == NULL || kind == NULL || target == NULL || visible == NULL) return false;
    base_bytes id_bytes, kind_bytes, target_bytes, name_bytes = {NULL, 0U};
    bool is_visible = false;
    if (!span_string(&reader, id->value, &id_bytes) || !span_string(&reader, kind->value, &kind_bytes) ||
        !span_string(&reader, target->value, &target_bytes) || !span_bool(&reader, visible->value, &is_visible)) return false;
    if (name != NULL && !span_string(&reader, name->value, &name_bytes)) return false;
    base_bytes property_bytes = {NULL, 0U};
    base_bytes value_type_bytes = {NULL, 0U};
    base_bytes interpolation_bytes = {NULL, 0U};
    if (!json_string_equal(kind_bytes, "drawing")) {
        const json_field *property = get_field(&track, "property");
        const json_field *value_type = get_field(&track, "value_type");
        const json_field *interpolation = get_field(&track, "interpolation");
        if (property == NULL || value_type == NULL || interpolation == NULL ||
            !span_string(&reader, property->value, &property_bytes) ||
            !span_string(&reader, value_type->value, &value_type_bytes) ||
            !span_string(&reader, interpolation->value, &interpolation_bytes))
            return false;
    }
    *out_track = (project_timeline_track_view){
        .id = id_bytes, .kind = kind_bytes, .target = target_bytes,
        .name = name_bytes, .visible = is_visible,
        .payload_json = view->track_spans[index],
        .item_count = view->item_counts[index], .property = property_bytes,
        .value_type = value_type_bytes, .interpolation = interpolation_bytes};
    return true;
}

bool project_timeline_item_at(const project_timeline_view *view, size_t track_index,
                              size_t item_index, project_timeline_item_view *out_item)
{
    if (view == NULL || out_item == NULL || track_index >= view->track_count ||
        view->track_count > PROJECT_TIMELINE_MAX_TRACKS ||
        !base_bytes_valid(view->source) || !base_bytes_valid(view->item_spans[track_index])) return false;
    json_reader reader = {.source = view->source, .error_kind = PROJECT_TIMELINE_OK};
    json_span item_span;
    json_span item_array = {
        (size_t)(view->item_spans[track_index].ptr - view->source.ptr),
        (size_t)(view->item_spans[track_index].ptr - view->source.ptr) +
            view->item_spans[track_index].len};
    if (!array_item_at(&reader, item_array, item_index, &item_span)) return false;
    bool drawing = view->drawing_tracks[track_index];
    object_fields item;
    if (!parse_object_fields(&reader, item_span, &item)) return false;
    const json_field *tick_field = get_field(&item, "tick");
    uint32_t tick = 0U;
    if (tick_field == NULL || !span_uint(&reader, tick_field->value, &tick)) return false;
    project_timeline_item_view output = {tick, {NULL, 0U}, {NULL, 0U},
        item_span.start, item_span.end - item_span.start};
    if (drawing) {
        const json_field *asset = get_field(&item, "asset");
        if (asset == NULL || !span_string(&reader, asset->value, &output.asset)) return false;
    } else {
        const json_field *value = get_field(&item, "value");
        if (value == NULL) return false;
        output.value_json = (base_bytes){view->source.ptr + value->value.start,
            value->value.end - value->value.start};
    }
    *out_item = output;
    return true;
}

bool project_timeline_item_next(const project_timeline_view *view, size_t track_index,
                               project_timeline_iterator *iterator,
                               project_timeline_item_view *out_item)
{
    if (view == NULL || iterator == NULL || out_item == NULL || track_index >= view->track_count ||
        view->track_count > PROJECT_TIMELINE_MAX_TRACKS ||
        !base_bytes_valid(view->item_spans[track_index])) return false;
    if (iterator->initialized && iterator->track_index != track_index) return false;
    size_t array_start = (size_t)(view->item_spans[track_index].ptr - view->source.ptr);
    size_t array_end = array_start + view->item_spans[track_index].len;
    if (!iterator->initialized) {
        iterator->track_index = track_index;
        iterator->next_index = 0U;
        iterator->source_cursor = array_start;
        iterator->initialized = true;
    }
    if (iterator->source_cursor < array_start || iterator->source_cursor > array_end ||
        iterator->source_cursor == SIZE_MAX) return false;
    json_reader reader = {.source = view->source, .cursor = iterator->source_cursor,
        .error_kind = PROJECT_TIMELINE_OK};
    json_span item_span;
    if (iterator->next_index == 0U) {
        reader.cursor = array_start;
        skip_space(&reader);
        if (reader.cursor >= reader.source.len || reader.source.ptr[reader.cursor++] != '[') return false;
        skip_space(&reader);
    }
    size_t start = reader.cursor;
    if (reader.cursor >= reader.source.len || reader.source.ptr[reader.cursor] == ']') return false;
    if (!scan_value(&reader, 1U)) return false;
    item_span = (json_span){start, reader.cursor};
    skip_space(&reader);
    if (reader.cursor >= reader.source.len) return false;
    uint8_t delimiter = reader.source.ptr[reader.cursor++];
    if (delimiter != ',' && delimiter != ']') return false;
    if (delimiter == ',') skip_space(&reader);
    iterator->source_cursor = delimiter == ']' ? SIZE_MAX : reader.cursor;
    object_fields item;
    if (!parse_object_fields(&reader, item_span, &item)) return false;
    const json_field *tick_field = get_field(&item, "tick");
    uint32_t tick = 0U;
    if (tick_field == NULL || !span_uint(&reader, tick_field->value, &tick)) return false;
    project_timeline_item_view output = {tick, {NULL, 0U}, {NULL, 0U},
        item_span.start, item_span.end - item_span.start};
    if (view->drawing_tracks[track_index]) {
        const json_field *asset = get_field(&item, "asset");
        if (asset == NULL || !span_string(&reader, asset->value, &output.asset)) return false;
    } else {
        const json_field *value = get_field(&item, "value");
        if (value == NULL) return false;
        output.value_json = (base_bytes){view->source.ptr + value->value.start,
            value->value.end - value->value.start};
    }
    iterator->next_index++;
    *out_item = output;
    return true;
}

const char *project_timeline_error_name(project_timeline_error error)
{
    static const char *const names[] = {"ok", "invalid_argument", "malformed_json",
        "unsupported_version", "invalid_schema", "invalid_reference", "duplicate",
        "limit", "invalid_value"};
    return (unsigned)error < sizeof(names) / sizeof(names[0]) ? names[error] : "unknown";
}

