#include "viewer_timeline.h"

#include "viewer_editor_internal.h"

#include <math.h>
#include <locale.h>
#include <pthread.h>
#include <stdio.h>
#include <string.h>

static bool timeline_bytes_equal(base_bytes value, const char *text)
{
    size_t length = strlen(text);
    return value.len == length && memcmp(value.ptr, text, length) == 0;
}

static bool paint_track_matches(project_timeline_track_view *track,
    base_bytes target, const char *property)
{
    uint8_t decoded[257U]; size_t length = 0U;
    return timeline_bytes_equal(track->kind, "property") &&
        timeline_bytes_equal(track->property, property) &&
        timeline_bytes_equal(track->value_type,
            strcmp(property, "visibility") == 0 ? "boolean" : "rgba") &&
        project_timeline_string_copy(track->target,
            (base_mut_bytes){decoded, sizeof(decoded)}, &length) &&
        length == target.len && memcmp(decoded, target.ptr, length) == 0;
}

static bool paint_track_has_baseline(viewer_timeline *timeline, size_t index)
{
    project_timeline_iterator iterator = {0};
    project_timeline_item_view item = {0};
    return project_timeline_item_next(&timeline->native_view, index,
        &iterator, &item) && item.tick == 0U;
}

static bool encode_rgba(const double rgba[4], char output[128U], size_t *length)
{
    int count = snprintf(output, 128U, "[%.17g,%.17g,%.17g,%.17g]",
        rgba[0], rgba[1], rgba[2], rgba[3]);
    if (count <= 0 || count >= 128) return false;
    *length = (size_t)count;
    return true;
}

bool viewer_timeline_key_selected_paint(viewer_timeline *timeline,
    viewer_editor *editor, const char *property)
{
    if (timeline == NULL || editor == NULL || !timeline->native_project ||
        timeline->native_session == NULL || !timeline->native_context_valid ||
        property == NULL || !(strcmp(property, "fill") == 0 ||
            strcmp(property, "stroke") == 0)) {
        if (editor != NULL) viewer_editor_set_status(editor,
            "Color keying requires an editable native project");
        return false;
    }
    uint8_t target_storage[257U]; size_t target_length = 0U;
    double rgba[4] = {0};
    if (!viewer_editor_selected_paint_target(editor, property, target_storage,
            sizeof(target_storage), &target_length, rgba)) {
        viewer_editor_set_status(editor,
            "Select an identified object with a supported explicit color attribute");
        return false;
    }
    base_bytes target = {target_storage, target_length};
    uint32_t tick = 0U;
    if (!viewer_timeline_seconds_to_native_tick(timeline, timeline->time_seconds,
            &tick) || tick >= timeline->native_duration_ticks) {
        viewer_editor_set_status(editor, "Playhead is outside the project duration");
        return false;
    }
    size_t matching = SIZE_MAX;
    for (size_t i = 0U; i < timeline->native_view.track_count; ++i)
        if (paint_track_matches(&timeline->native_tracks[i], target, property)) {
            matching = i; break;
        }
    if (matching == SIZE_MAX && tick != 0U) {
        viewer_editor_set_status(editor, "Create the color baseline at tick zero first");
        return false;
    }
    if (matching != SIZE_MAX && tick != 0U &&
        !paint_track_has_baseline(timeline, matching)) {
        viewer_editor_set_status(editor, "Create the color baseline at tick zero first");
        return false;
    }
    char value_text[128U]; size_t value_length = 0U;
    if (!encode_rgba(rgba, value_text, &value_length)) {
        viewer_editor_set_status(editor, "Could not encode selected color");
        return false;
    }
    base_bytes value = {(const uint8_t *)value_text, value_length};
    project_timeline_edit_result changed = {PROJECT_TIMELINE_OK, 0U};
    bool unchanged = false;
    if (matching != SIZE_MAX) {
        uint8_t track_id[PROJECT_TIMELINE_MAX_STRING_BYTES]; size_t id_length = 0U;
        if (!project_timeline_string_copy(timeline->native_tracks[matching].id,
                (base_mut_bytes){track_id, sizeof(track_id)}, &id_length)) return false;
        project_timeline_iterator iterator = {0};
        project_timeline_item_view item = {0};
        while (project_timeline_item_next(&timeline->native_view, matching,
                &iterator, &item)) {
            if (item.tick == tick) {
                unchanged = item.value_json.len == value.len &&
                    memcmp(item.value_json.ptr, value.ptr, value.len) == 0;
                break;
            }
            if (item.tick > tick) break;
        }
        if (!unchanged) changed = project_timeline_session_set_property_key(
            timeline->native_session, (base_bytes){track_id, id_length}, tick, value);
    } else {
        uint64_t hash = UINT64_C(1469598103934665603);
        for (size_t i = 0U; i < target.len; ++i) { hash ^= target.ptr[i]; hash *= UINT64_C(1099511628211); }
        char id[64U]; size_t id_length = 0U; bool unique = false;
        for (uint32_t suffix = 0U; suffix < PROJECT_TIMELINE_MAX_TRACKS; ++suffix) {
            int amount = suffix == 0U ? snprintf(id, sizeof(id), "ui_%s_%016llx",
                property, (unsigned long long)hash) : snprintf(id, sizeof(id),
                "ui_%s_%016llx_%u", property, (unsigned long long)hash, suffix);
            if (amount <= 0 || (size_t)amount >= sizeof(id)) return false;
            id_length = (size_t)amount; unique = true;
            for (size_t i = 0U; i < timeline->native_view.track_count; ++i) {
                uint8_t existing[PROJECT_TIMELINE_MAX_STRING_BYTES]; size_t length = 0U;
                if (!project_timeline_string_copy(timeline->native_tracks[i].id,
                        (base_mut_bytes){existing, sizeof(existing)}, &length))
                    return false;
                if (length == id_length && memcmp(existing, id, length) == 0) {
                    unique = false; break;
                }
            }
            if (unique) break;
        }
        if (!unique) return false;
        changed = project_timeline_session_add_property_track(
            timeline->native_session, (base_bytes){(const uint8_t *)id, id_length},
            target, (base_bytes){(const uint8_t *)property, strlen(property)},
            (base_bytes){(const uint8_t *)"rgba", 4U},
            (base_bytes){(const uint8_t *)"linear", 6U}, value,
            (base_bytes){(const uint8_t *)(strcmp(property, "fill") == 0 ? "Fill" : "Stroke"),
                strcmp(property, "fill") == 0 ? 4U : 6U});
    }
    if (changed.error != PROJECT_TIMELINE_OK || !viewer_timeline_refresh_native(timeline)) {
        viewer_editor_set_status(editor, "Color key could not be committed");
        return false;
    }
    for (size_t i = 0U; i < timeline->native_view.track_count; ++i)
        if (paint_track_matches(&timeline->native_tracks[i], target, property)) {
            timeline->selected_track = i; break;
        }
    if (!unchanged) {
        viewer_timeline_invalidate_preview(timeline);
        timeline->native_mutation_pending = true;
    }
    viewer_editor_set_status(editor, unchanged ? "Color already keyed at current frame" :
        "Color keyed at current frame");
    return true;
}

bool viewer_timeline_key_selected_visibility(viewer_timeline *timeline,
    viewer_editor *editor)
{
    if (timeline == NULL || editor == NULL || !timeline->native_project ||
        timeline->native_session == NULL || !timeline->native_context_valid) {
        if (editor != NULL) viewer_editor_set_status(editor,
            "Visibility keying requires an editable native project");
        return false;
    }
    uint8_t target_storage[257U]; size_t target_length = 0U; bool visible = true;
    if (!viewer_editor_selected_visibility_target(editor, target_storage,
            sizeof(target_storage), &target_length, &visible)) {
        viewer_editor_set_status(editor,
            "Select an identified object with a supported, unambiguous visibility value");
        return false;
    }
    base_bytes target = {target_storage, target_length};
    uint32_t tick = 0U;
    if (!viewer_timeline_seconds_to_native_tick(timeline, timeline->time_seconds,
            &tick) || tick >= timeline->native_duration_ticks) {
        viewer_editor_set_status(editor, "Playhead is outside the project duration");
        return false;
    }
    size_t matching = SIZE_MAX;
    for (size_t i = 0U; i < timeline->native_view.track_count; ++i)
        if (paint_track_matches(&timeline->native_tracks[i], target, "visibility")) {
            matching = i; break;
        }
    if (matching == SIZE_MAX && tick != 0U) {
        viewer_editor_set_status(editor,
            "Create the visibility baseline at tick zero first");
        return false;
    }
    if (matching != SIZE_MAX && tick != 0U &&
        !paint_track_has_baseline(timeline, matching)) {
        viewer_editor_set_status(editor,
            "Create the visibility baseline at tick zero first");
        return false;
    }
    base_bytes value = {(const uint8_t *)(visible ? "true" : "false"),
        visible ? 4U : 5U};
    project_timeline_edit_result changed = {PROJECT_TIMELINE_OK, 0U};
    bool unchanged = false;
    if (matching != SIZE_MAX) {
        uint8_t track_id[PROJECT_TIMELINE_MAX_STRING_BYTES]; size_t id_length = 0U;
        if (!project_timeline_string_copy(timeline->native_tracks[matching].id,
                (base_mut_bytes){track_id, sizeof(track_id)}, &id_length)) return false;
        project_timeline_iterator iterator = {0}; project_timeline_item_view item = {0};
        while (project_timeline_item_next(&timeline->native_view, matching,
                &iterator, &item)) {
            if (item.tick == tick) {
                unchanged = item.value_json.len == value.len &&
                    memcmp(item.value_json.ptr, value.ptr, value.len) == 0;
                break;
            }
            if (item.tick > tick) break;
        }
        if (!unchanged) changed = project_timeline_session_set_property_key(
            timeline->native_session, (base_bytes){track_id, id_length}, tick, value);
    } else {
        uint64_t hash = UINT64_C(1469598103934665603);
        for (size_t i = 0U; i < target.len; ++i) { hash ^= target.ptr[i]; hash *= UINT64_C(1099511628211); }
        char id[64U]; size_t id_length = 0U; bool unique = false;
        for (uint32_t suffix = 0U; suffix < PROJECT_TIMELINE_MAX_TRACKS; ++suffix) {
            int amount = suffix == 0U ? snprintf(id, sizeof(id),
                "ui_visibility_%016llx", (unsigned long long)hash) :
                snprintf(id, sizeof(id), "ui_visibility_%016llx_%u",
                    (unsigned long long)hash, suffix);
            if (amount <= 0 || (size_t)amount >= sizeof(id)) return false;
            id_length = (size_t)amount; unique = true;
            for (size_t i = 0U; i < timeline->native_view.track_count; ++i) {
                uint8_t existing[PROJECT_TIMELINE_MAX_STRING_BYTES]; size_t length = 0U;
                if (!project_timeline_string_copy(timeline->native_tracks[i].id,
                        (base_mut_bytes){existing, sizeof(existing)}, &length))
                    return false;
                if (length == id_length && memcmp(existing, id, length) == 0) {
                    unique = false; break;
                }
            }
            if (unique) break;
        }
        if (!unique) return false;
        changed = project_timeline_session_add_property_track(
            timeline->native_session, (base_bytes){(const uint8_t *)id, id_length},
            target, (base_bytes){(const uint8_t *)"visibility", 10U},
            (base_bytes){(const uint8_t *)"boolean", 7U},
            (base_bytes){(const uint8_t *)"step", 4U}, value,
            (base_bytes){(const uint8_t *)"Visibility", 10U});
    }
    if (changed.error != PROJECT_TIMELINE_OK || !viewer_timeline_refresh_native(timeline)) {
        viewer_editor_set_status(editor, "Visibility key could not be committed");
        return false;
    }
    for (size_t i = 0U; i < timeline->native_view.track_count; ++i)
        if (paint_track_matches(&timeline->native_tracks[i], target, "visibility")) {
            timeline->selected_track = i; break;
        }
    if (!unchanged) {
        viewer_timeline_invalidate_preview(timeline);
        timeline->native_mutation_pending = true;
    }
    viewer_editor_set_status(editor, unchanged ?
        "Visibility already keyed at current frame" :
        "Visibility keyed at current frame");
    return true;
}

bool viewer_timeline_key_selected_opacity(viewer_timeline *timeline,
    viewer_editor *editor)
{
    if (timeline == NULL || editor == NULL || !timeline->native_project ||
        timeline->native_session == NULL || !timeline->native_context_valid) {
        if (editor != NULL) viewer_editor_set_status(editor,
            "Key Opacity requires an editable native project");
        return false;
    }
    uint8_t target_storage[257U];
    size_t target_length = 0U;
    double opacity = 1.0;
    if (!viewer_editor_selected_opacity_target(editor, target_storage,
            sizeof(target_storage), &target_length, &opacity)) {
        viewer_editor_set_status(editor,
            "Select one identified object with a supported opacity value");
        return false;
    }
    base_bytes target = {target_storage, target_length};
    uint32_t tick = 0U;
    if (!viewer_timeline_seconds_to_native_tick(timeline, timeline->time_seconds,
            &tick) || tick >= timeline->native_duration_ticks) {
        viewer_editor_set_status(editor, "Playhead is outside the project duration");
        return false;
    }
    char value_text[32U];
    int value_length = snprintf(value_text, sizeof(value_text), "%.17g", opacity);
    if (value_length <= 0 || (size_t)value_length >= sizeof(value_text)) {
        viewer_editor_set_status(editor, "Could not encode opacity value");
        return false;
    }
    base_bytes value = {(const uint8_t *)value_text, (size_t)value_length};
    size_t matching = SIZE_MAX;
    for (size_t index = 0U; index < timeline->native_view.track_count; ++index) {
        project_timeline_track_view *track = &timeline->native_tracks[index];
        uint8_t decoded[257U];
        size_t length = 0U;
        if (track->kind.len != 8U || memcmp(track->kind.ptr, "property", 8U) != 0 ||
            track->property.len != 7U ||
            memcmp(track->property.ptr, "opacity", 7U) != 0 ||
            track->value_type.len != 6U ||
            memcmp(track->value_type.ptr, "scalar", 6U) != 0 ||
            !project_timeline_string_copy(track->target,
                (base_mut_bytes){decoded, sizeof(decoded)}, &length)) continue;
        if (length == target.len && memcmp(decoded, target.ptr, length) == 0) {
            matching = index;
            break;
        }
    }
    project_timeline_edit_result changed = {PROJECT_TIMELINE_OK, 0U};
    if (matching != SIZE_MAX) {
        uint8_t track_id[PROJECT_TIMELINE_MAX_STRING_BYTES];
        size_t track_id_length = 0U;
        if (!project_timeline_string_copy(timeline->native_tracks[matching].id,
                (base_mut_bytes){track_id, sizeof(track_id)}, &track_id_length)) {
            viewer_editor_set_status(editor, "Could not read opacity track id");
            return false;
        }
        changed = project_timeline_session_set_property_key(
            timeline->native_session,
            (base_bytes){track_id, track_id_length}, tick, value);
    } else {
        uint64_t hash = UINT64_C(1469598103934665603);
        for (size_t index = 0U; index < target.len; ++index) {
            hash ^= target.ptr[index];
            hash *= UINT64_C(1099511628211);
        }
        char id_text[40U];
        int id_length = snprintf(id_text, sizeof(id_text), "ui_opacity_%016llx",
            (unsigned long long)hash);
        if (id_length <= 0 || (size_t)id_length >= sizeof(id_text)) {
            viewer_editor_set_status(editor, "Could not create opacity track id");
            return false;
        }
        base_bytes id = {(const uint8_t *)id_text, (size_t)id_length};
        if (tick == 0U)
            changed = project_timeline_session_add_property_track(
                timeline->native_session, id, target,
                (base_bytes){(const uint8_t *)"opacity", 7U},
                (base_bytes){(const uint8_t *)"scalar", 6U},
                (base_bytes){(const uint8_t *)"linear", 6U}, value,
                (base_bytes){(const uint8_t *)"Opacity", 7U});
        else changed = project_timeline_session_add_property_track_key(
            timeline->native_session, id, target,
            (base_bytes){(const uint8_t *)"opacity", 7U},
            (base_bytes){(const uint8_t *)"scalar", 6U},
            (base_bytes){(const uint8_t *)"linear", 6U}, value, tick,
            value, (base_bytes){(const uint8_t *)"Opacity", 7U});
    }
    if (changed.error != PROJECT_TIMELINE_OK ||
        !viewer_timeline_refresh_native(timeline)) {
        char message[128U];
        const char *reason = changed.error == PROJECT_TIMELINE_OK ?
            "timeline refresh failed" : project_timeline_error_name(changed.error);
        int amount = snprintf(message, sizeof(message), "Key Opacity failed: %s",
            reason);
        viewer_editor_set_status(editor, amount > 0 &&
            (size_t)amount < sizeof(message) ? message : "Key Opacity failed");
        return false;
    }
    viewer_timeline_invalidate_preview(timeline);
    for (size_t index = 0U; index < timeline->native_view.track_count; ++index) {
        project_timeline_track_view *track = &timeline->native_tracks[index];
        uint8_t decoded[257U];
        size_t length = 0U;
        if (track->property.len == 7U &&
            memcmp(track->property.ptr, "opacity", 7U) == 0 &&
            project_timeline_string_copy(track->target,
                (base_mut_bytes){decoded, sizeof(decoded)}, &length) &&
            length == target.len && memcmp(decoded, target.ptr, length) == 0) {
            timeline->selected_track = index;
            break;
        }
    }
    timeline->native_mutation_pending = true;
    viewer_editor_set_status(editor, "Opacity keyed at current frame");
    return true;
}


static bool bytes_equal(base_bytes value, const char *text)
{
    size_t length = strlen(text);
    return value.len == length && memcmp(value.ptr, text, length) == 0;
}

static locale_t transform_json_locale;
static pthread_once_t transform_json_locale_once = PTHREAD_ONCE_INIT;

static void initialize_transform_json_locale(void)
{
    transform_json_locale = newlocale(LC_NUMERIC_MASK, "C", (locale_t)0);
}

static bool encode_transform(const double matrix[6], char output[192],
                             size_t *out_length)
{
    (void)pthread_once(&transform_json_locale_once,
        initialize_transform_json_locale);
    if (transform_json_locale == (locale_t)0) return false;
    locale_t prior = uselocale(transform_json_locale);
    int count = snprintf(output, 192U, "[%.17g,%.17g,%.17g,%.17g,%.17g,%.17g]",
        matrix[0], matrix[1], matrix[2], matrix[3], matrix[4], matrix[5]);
    (void)uselocale(prior);
    if (count <= 0 || count >= 192) return false;
    *out_length = (size_t)count;
    return true;
}

static bool transform_target_equal(project_timeline_track_view *track,
    base_bytes target)
{
    uint8_t decoded[257U];
    size_t length = 0U;
    return bytes_equal(track->kind, "property") &&
        bytes_equal(track->property, "transform") &&
        bytes_equal(track->value_type, "transform2d") &&
        project_timeline_string_copy(track->target,
            (base_mut_bytes){decoded, sizeof(decoded)}, &length) &&
        length == target.len && memcmp(decoded, target.ptr, length) == 0;
}

static bool transform_has_baseline(viewer_timeline *timeline, size_t track_index)
{
    project_timeline_iterator iterator = {0};
    project_timeline_item_view item = {0};
    return project_timeline_item_next(&timeline->native_view, track_index,
        &iterator, &item) && item.tick == 0U;
}

static bool transform_key_matches(viewer_timeline *timeline, size_t track_index,
    uint32_t tick, base_bytes value)
{
    project_timeline_iterator iterator = {0};
    project_timeline_item_view item = {0};
    while (project_timeline_item_next(&timeline->native_view, track_index,
            &iterator, &item)) {
        if (item.tick == tick)
            return item.value_json.len == value.len &&
                memcmp(item.value_json.ptr, value.ptr, value.len) == 0;
        if (item.tick > tick) return false;
    }
    return false;
}

static bool make_transform_track_id(viewer_timeline *timeline, base_bytes target,
    char buffer[64U], size_t *out_length)
{
    uint64_t hash = UINT64_C(1469598103934665603);
    for (size_t i = 0U; i < target.len; ++i) { hash ^= target.ptr[i]; hash *= UINT64_C(1099511628211); }
    for (uint32_t suffix = 0U; suffix < PROJECT_TIMELINE_MAX_TRACKS; ++suffix) {
        int count = suffix == 0U ? snprintf(buffer, 64U, "ui_transform_%016llx",
            (unsigned long long)hash) : snprintf(buffer, 64U, "ui_transform_%016llx_%u",
            (unsigned long long)hash, suffix);
        if (count <= 0 || count >= 64) return false;
        base_bytes candidate = {(const uint8_t *)buffer, (size_t)count};
        bool used = false;
        for (size_t i = 0U; i < timeline->native_view.track_count; ++i) {
            uint8_t existing[PROJECT_TIMELINE_MAX_STRING_BYTES]; size_t length = 0U;
            if (!project_timeline_string_copy(timeline->native_tracks[i].id,
                    (base_mut_bytes){existing, sizeof(existing)}, &length)) return false;
            if (length == candidate.len && memcmp(existing, candidate.ptr, length) == 0) { used = true; break; }
        }
        if (!used) { *out_length = (size_t)count; return true; }
    }
    return false;
}

bool viewer_timeline_key_selected_transform(viewer_timeline *timeline,
    viewer_editor *editor)
{
    if (timeline == NULL || editor == NULL || !timeline->native_project ||
        timeline->native_session == NULL || !timeline->native_context_valid) {
        if (editor != NULL) viewer_editor_set_status(editor,
            "Key Transform requires an editable native project");
        return false;
    }
    uint8_t target_storage[257U]; size_t target_length = 0U;
    double matrix[6] = {0};
    if (!viewer_editor_selected_transform_target(editor, target_storage,
            sizeof(target_storage), &target_length, matrix)) {
        viewer_editor_set_status(editor,
            "Select one identified object with no transform or a matrix(...) transform");
        return false;
    }
    base_bytes target = {target_storage, target_length};
    uint32_t tick = 0U;
    if (!viewer_timeline_seconds_to_native_tick(timeline, timeline->time_seconds,
            &tick) || tick >= timeline->native_duration_ticks) {
        viewer_editor_set_status(editor, "Playhead is outside the project duration");
        return false;
    }
    size_t matching = SIZE_MAX;
    for (size_t i = 0U; i < timeline->native_view.track_count; ++i)
        if (transform_target_equal(&timeline->native_tracks[i], target)) { matching = i; break; }
    if (matching == SIZE_MAX && tick != 0U) {
        viewer_editor_set_status(editor, "Create the transform baseline at tick zero first");
        return false;
    }
    if (matching != SIZE_MAX && tick != 0U &&
        !transform_has_baseline(timeline, matching)) {
        viewer_editor_set_status(editor, "Create the transform baseline at tick zero first");
        return false;
    }
    char value_text[192U]; size_t value_length = 0U;
    if (!encode_transform(matrix, value_text, &value_length)) {
        viewer_editor_set_status(editor, "Could not encode transform matrix");
        return false;
    }
    base_bytes value = {(const uint8_t *)value_text, value_length};
    project_timeline_edit_result changed = {PROJECT_TIMELINE_OK, 0U};
    bool unchanged = matching != SIZE_MAX &&
        transform_key_matches(timeline, matching, tick, value);
    if (matching != SIZE_MAX && !unchanged) {
        uint8_t id[PROJECT_TIMELINE_MAX_STRING_BYTES]; size_t id_length = 0U;
        if (!project_timeline_string_copy(timeline->native_tracks[matching].id,
                (base_mut_bytes){id, sizeof(id)}, &id_length)) return false;
        changed = project_timeline_session_set_property_key(timeline->native_session,
            (base_bytes){id, id_length}, tick, value);
    } else if (matching == SIZE_MAX) {
        char id[64U]; size_t id_length = 0U;
        if (!make_transform_track_id(timeline, target, id, &id_length)) {
            viewer_editor_set_status(editor, "Could not allocate a transform track id");
            return false;
        }
        changed = project_timeline_session_add_property_track(
            timeline->native_session, (base_bytes){(const uint8_t *)id, id_length},
            target, (base_bytes){(const uint8_t *)"transform", 9U},
            (base_bytes){(const uint8_t *)"transform2d", 11U},
            (base_bytes){(const uint8_t *)"linear", 6U}, value,
            (base_bytes){(const uint8_t *)"Transform", 9U});
    }
    if (changed.error != PROJECT_TIMELINE_OK ||
        !viewer_timeline_refresh_native(timeline)) {
        const char *reason = changed.error == PROJECT_TIMELINE_OK ?
            "timeline refresh failed" : project_timeline_error_name(changed.error);
        char message[128U]; int amount = snprintf(message, sizeof(message),
            "Key Transform failed: %s", reason);
        viewer_editor_set_status(editor, amount > 0 && (size_t)amount < sizeof(message) ?
            message : "Key Transform failed");
        return false;
    }
    for (size_t i = 0U; i < timeline->native_view.track_count; ++i)
        if (transform_target_equal(&timeline->native_tracks[i], target)) {
            timeline->selected_track = i; break;
        }
    if (!unchanged) {
        viewer_timeline_invalidate_preview(timeline);
        timeline->native_mutation_pending = true;
        viewer_editor_set_status(editor, "Transform keyed at current frame");
    } else viewer_editor_set_status(editor, "Transform already keyed at current frame");
    return true;
}
