#include "viewer_timeline.h"

#include <stdio.h>
#include <string.h>

static bool asset_is_svg(const project_bundle_asset *asset)
{
    static const uint8_t media_type[] = "image/svg+xml";
    return asset != NULL && asset->media_type.len == sizeof(media_type) - 1U &&
        memcmp(asset->media_type.ptr, media_type, sizeof(media_type) - 1U) == 0;
}

static bool asset_matches_scene(const viewer_timeline *timeline,
    const project_bundle_asset *asset)
{
    if (timeline == NULL || timeline->document == NULL || asset == NULL ||
        !asset_is_svg(asset)) return false;
    svg_shape_document drawing = {0};
    svg_shape_diagnostic diagnostic = {SVG_SHAPE_ERROR_NONE, 0U};
    if (svg_shape_decode(asset->data, &drawing, &diagnostic) !=
            SVG_SHAPE_ERROR_NONE) {
        svg_shape_document_destroy(&drawing);
        return false;
    }
    bool matches = drawing.width == timeline->document->width &&
        drawing.height == timeline->document->height &&
        drawing.view_x == timeline->document->view_x &&
        drawing.view_y == timeline->document->view_y &&
        drawing.view_width == timeline->document->view_width &&
        drawing.view_height == timeline->document->view_height;
    svg_shape_document_destroy(&drawing);
    return matches;
}

size_t viewer_timeline_native_svg_asset_count(viewer_timeline *timeline)
{
    if (timeline == NULL || !timeline->native_project ||
        timeline->native_bundle == NULL) return 0U;
    if (timeline->native_asset_cache_valid &&
        timeline->native_asset_bundle_count ==
            timeline->native_bundle->asset_count)
        return timeline->native_svg_asset_count;
    timeline->native_svg_asset_count = 0U;
    for (size_t index = 0U; index < timeline->native_bundle->asset_count &&
            index < PROJECT_BUNDLE_MAX_ASSET_COUNT; ++index) {
        project_bundle_asset asset = {0};
        if (project_bundle_asset_at(timeline->native_bundle, index, &asset) &&
            asset_is_svg(&asset))
            timeline->native_svg_asset_indices[
                timeline->native_svg_asset_count++] = index;
    }
    timeline->native_asset_bundle_count = timeline->native_bundle->asset_count;
    timeline->native_asset_cache_valid = true;
    if (timeline->native_svg_asset_count > 0U)
        timeline->native_selected_asset %= timeline->native_svg_asset_count;
    return timeline->native_svg_asset_count;
}

bool viewer_timeline_native_selected_asset(viewer_timeline *timeline,
    project_bundle_asset *out_asset)
{
    if (out_asset != NULL) *out_asset = (project_bundle_asset){0};
    if (timeline == NULL || out_asset == NULL || !timeline->native_project ||
        timeline->native_bundle == NULL) return false;
    size_t count = viewer_timeline_native_svg_asset_count(timeline);
    if (count == 0U) return false;
    size_t selected = timeline->native_selected_asset % count;
    return project_bundle_asset_at(timeline->native_bundle,
        timeline->native_svg_asset_indices[selected], out_asset) &&
        asset_is_svg(out_asset);
}

bool viewer_timeline_cycle_native_asset(viewer_timeline *timeline,
    viewer_editor *editor)
{
    size_t count = viewer_timeline_native_svg_asset_count(timeline);
    if (count == 0U) {
        if (editor != NULL) viewer_editor_set_status(editor,
            "This native project has no SVG drawing assets");
        return false;
    }
    for (size_t step = 0U; step < count; ++step) {
        timeline->native_selected_asset =
            (timeline->native_selected_asset + 1U) % count;
        project_bundle_asset asset = {0};
        if (!viewer_timeline_native_selected_asset(timeline, &asset) ||
            !asset_matches_scene(timeline, &asset)) continue;
        char message[192U];
        int amount = snprintf(message, sizeof(message), "Selected drawing asset: %.*s",
            (int)asset.name.len, (const char *)asset.name.ptr);
        if (editor != NULL) viewer_editor_set_status(editor, amount > 0 &&
            (size_t)amount < sizeof(message) ? message : "Drawing asset selected");
        return true;
    }
    if (editor != NULL) viewer_editor_set_status(editor,
        "No project SVG drawing matches this animation canvas");
    return false;
}

bool viewer_timeline_selected_drawing_track(const viewer_timeline *timeline)
{
    project_timeline_track_view track = {0};
    return timeline != NULL && timeline->native_project &&
        timeline->selected_track < timeline->native_view.track_count &&
        viewer_timeline_native_track_at(timeline, timeline->selected_track, &track) &&
        track.kind.len == 7U && memcmp(track.kind.ptr, "drawing", 7U) == 0;
}

const char *viewer_timeline_native_drawing_action_label(
    const viewer_timeline *timeline)
{
    return viewer_timeline_selected_drawing_track(timeline) ? "Add Cel" :
        "Add Track";
}

static uint64_t target_hash(base_bytes target)
{
    uint64_t hash = UINT64_C(1469598103934665603);
    for (size_t index = 0U; index < target.len; ++index) {
        hash ^= target.ptr[index];
        hash *= UINT64_C(1099511628211);
    }
    return hash;
}

static bool add_native_drawing_track(viewer_timeline *timeline,
    viewer_editor *editor, const project_bundle_asset *asset)
{
    uint8_t target_storage[257U];
    size_t target_length = 0U;
    double opacity = 1.0;
    if (!viewer_editor_selected_opacity_target(editor, target_storage,
            sizeof(target_storage), &target_length, &opacity)) {
        viewer_editor_set_status(editor,
            "Select one identified SVG object with a supported opacity value");
        return false;
    }
    (void)opacity;
    base_bytes target = {target_storage, target_length};
    char track_id[64U];
    size_t track_id_length = 0U;
    bool unique = false;
    for (size_t suffix = 0U; suffix <= timeline->native_view.track_count; ++suffix) {
        int amount = snprintf(track_id, sizeof(track_id), "ui_draw_%016llx_%03zu",
            (unsigned long long)target_hash(target), suffix);
        if (amount <= 0 || (size_t)amount >= sizeof(track_id)) break;
        track_id_length = (size_t)amount;
        unique = true;
        for (size_t index = 0U; index < timeline->native_view.track_count; ++index) {
            project_timeline_track_view track = {0};
            uint8_t existing_id[257U];
            size_t existing_length = 0U;
            if (!viewer_timeline_native_track_at(timeline, index, &track) ||
                !project_timeline_string_copy(track.id,
                    (base_mut_bytes){existing_id, sizeof(existing_id)},
                    &existing_length)) {
                unique = false;
                break;
            }
            if (existing_length == track_id_length &&
                memcmp(existing_id, track_id, track_id_length) == 0) {
                unique = false;
                break;
            }
        }
        if (unique) break;
    }
    if (!unique) {
        viewer_editor_set_status(editor, "Could not create a unique drawing track ID");
        return false;
    }
    char drawing_name[PROJECT_TIMELINE_MAX_STRING_BYTES + 1U];
    int name_amount = snprintf(drawing_name, sizeof(drawing_name),
        "Drawing %.*s", (int)target.len, (const char *)target.ptr);
    if (name_amount <= 0 || (size_t)name_amount >= sizeof(drawing_name)) {
        viewer_editor_set_status(editor, "Could not create drawing track label");
        return false;
    }
    project_timeline_edit_result changed = project_timeline_session_add_drawing_track(
        timeline->native_session, (base_bytes){(const uint8_t *)track_id,
            track_id_length}, target, asset->name,
        (base_bytes){(const uint8_t *)drawing_name, (size_t)name_amount});
    if (changed.error != PROJECT_TIMELINE_OK) {
        char message[160U];
        int amount = snprintf(message, sizeof(message), "Add Track failed: %s",
            project_timeline_error_name(changed.error));
        viewer_editor_set_status(editor, amount > 0 &&
            (size_t)amount < sizeof(message) ? message : "Add Track failed");
        return false;
    }
    timeline->native_mutation_pending = true;
    viewer_timeline_invalidate_preview(timeline);
    if (!viewer_timeline_refresh_native(timeline)) {
        viewer_editor_set_status(editor,
            "Track created; timeline rows need to be refreshed");
        return false;
    }
    timeline->selected_track = timeline->native_view.track_count - 1U;
    char message[192U];
    int amount = snprintf(message, sizeof(message),
        "Added drawing track for %.*s with %.*s at tick 0",
        (int)target.len, (const char *)target.ptr,
        (int)asset->name.len, (const char *)asset->name.ptr);
    viewer_editor_set_status(editor, amount > 0 &&
        (size_t)amount < sizeof(message) ? message : "Drawing track added");
    return true;
}

bool viewer_timeline_add_native_cel(viewer_timeline *timeline,
    viewer_editor *editor)
{
    if (timeline == NULL || editor == NULL || !timeline->native_project ||
        !timeline->native_context_valid || timeline->native_session == NULL) {
        if (editor != NULL) viewer_editor_set_status(editor,
            "Add Cel requires an editable native project");
        return false;
    }
    project_bundle_asset asset = {0};
    if (!viewer_timeline_native_selected_asset(timeline, &asset)) {
        viewer_editor_set_status(editor, "Choose an SVG drawing asset first");
        return false;
    }
    if (!asset_matches_scene(timeline, &asset)) {
        viewer_editor_set_status(editor,
            "Selected drawing asset does not match the animation canvas");
        return false;
    }
    if (!viewer_timeline_selected_drawing_track(timeline))
        return add_native_drawing_track(timeline, editor, &asset);
    project_timeline_track_view track = {0};
    if (!viewer_timeline_native_track_at(timeline, timeline->selected_track, &track))
        return false;
    uint32_t tick = 0U;
    if (!viewer_timeline_seconds_to_native_tick(timeline,
            timeline->time_seconds, &tick) || tick >= timeline->native_duration_ticks) {
        viewer_editor_set_status(editor, "Playhead is outside the project duration");
        return false;
    }
    uint8_t track_id[257U];
    size_t track_id_length = 0U;
    if (!project_timeline_string_copy(track.id,
            (base_mut_bytes){track_id, sizeof(track_id)}, &track_id_length) ||
        track_id_length == 0U) {
        viewer_editor_set_status(editor, "Selected drawing track has an invalid ID");
        return false;
    }
    project_timeline_edit_result changed = project_timeline_session_set_cel(
        timeline->native_session, (base_bytes){track_id, track_id_length}, tick,
        asset.name, asset.name);
    if (changed.error != PROJECT_TIMELINE_OK ||
        !viewer_timeline_refresh_native(timeline)) {
        const char *reason = changed.error == PROJECT_TIMELINE_OK ?
            "timeline refresh failed" : project_timeline_error_name(changed.error);
        char message[160U];
        int amount = snprintf(message, sizeof(message), "Add Cel failed: %s", reason);
        viewer_editor_set_status(editor, amount > 0 &&
            (size_t)amount < sizeof(message) ? message : "Add Cel failed");
        return false;
    }
    viewer_timeline_invalidate_preview(timeline);
    timeline->native_mutation_pending = true;
    char message[192U];
    int amount = snprintf(message, sizeof(message), "Added %.*s at tick %u",
        (int)asset.name.len, (const char *)asset.name.ptr, tick);
    viewer_editor_set_status(editor, amount > 0 &&
        (size_t)amount < sizeof(message) ? message : "Drawing cel added");
    return true;
}
