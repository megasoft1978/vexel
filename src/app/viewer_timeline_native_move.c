#include "viewer_timeline.h"

#include <stdio.h>

bool viewer_timeline_select_track(viewer_timeline *timeline,
    viewer_editor *editor, size_t track_index)
{
    if (timeline == NULL || track_index >= viewer_timeline_track_count(timeline))
        return false;
    if (timeline->native_project) {
        timeline->selected_track = track_index;
        return true;
    }
    const svg_anim_track *track = &timeline->document->animation.tracks[track_index];
    if (!track->supported) {
        if (editor != NULL) viewer_editor_set_status(editor,
            "Unsupported animation track · preview only");
        return true;
    }
    timeline->selected_track = track_index;
    double start = viewer_timeline_track_start_time(track);
    (void)viewer_timeline_seek(timeline, start);
    timeline->selected_track = track_index;
    if (editor != NULL)
        (void)viewer_editor_select_at_source_offset(editor, track->target_offset);
    return true;
}

bool viewer_timeline_native_move_finish(viewer_timeline *timeline,
    viewer_editor *editor)
{
    if (timeline == NULL || editor == NULL || !timeline->native_dragging_item)
        return false;
    size_t track_index = timeline->native_drag_track;
    uint32_t from_tick = timeline->native_drag_from_tick;
    uint32_t to_tick = timeline->native_drag_to_tick;
    timeline->native_dragging_item = false;
    if (track_index >= timeline->native_view.track_count ||
        timeline->native_session == NULL) return true;
    if (from_tick == to_tick) return true;
    project_timeline_track_view track;
    char track_id[PROJECT_TIMELINE_MAX_STRING_BYTES + 1U];
    size_t track_id_length = 0U;
    if (!viewer_timeline_native_track_at(timeline, track_index, &track) ||
        !project_timeline_string_copy(track.id,
            (base_mut_bytes){(uint8_t *)track_id, sizeof(track_id)},
            &track_id_length)) {
        viewer_editor_set_status(editor, "Could not move timeline item: invalid track");
        return true;
    }
    project_timeline_edit_result moved = project_timeline_session_move_item(
        timeline->native_session,
        (base_bytes){(const uint8_t *)track_id, track_id_length},
        from_tick, to_tick);
    if (moved.error == PROJECT_TIMELINE_DUPLICATE) {
        viewer_editor_set_status(editor, "Could not move item: destination tick is occupied");
        return true;
    }
    if (moved.error == PROJECT_TIMELINE_INVALID_VALUE) {
        viewer_editor_set_status(editor,
            from_tick == 0U ? "Could not move item: tick zero is the required baseline" :
                "Could not move item: destination is outside the timeline");
        return true;
    }
    if (moved.error != PROJECT_TIMELINE_OK) {
        char message[128];
        (void)snprintf(message, sizeof(message), "Could not move timeline item: %s",
            project_timeline_error_name(moved.error));
        viewer_editor_set_status(editor, message);
        return true;
    }
    if (!viewer_timeline_refresh_native(timeline)) {
        viewer_editor_set_status(editor, "Timeline changed, but its preview could not refresh");
        return true;
    }
    viewer_timeline_invalidate_preview(timeline);
    timeline->native_mutation_pending = true;
    viewer_editor_set_status(editor, "Moved timeline item");
    return true;
}
