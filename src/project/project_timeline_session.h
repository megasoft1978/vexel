#ifndef VEXEL_PROJECT_PROJECT_TIMELINE_SESSION_H
#define VEXEL_PROJECT_PROJECT_TIMELINE_SESSION_H

#include "project_timeline_edit.h"
#include "project_timeline_integration.h"
#include "project_animation_render.h"

typedef struct project_timeline_session project_timeline_session;

/* On success, consumes `context`; source bytes are copied by the edit model. */
project_timeline_edit_result project_timeline_session_create(
    base_bytes source, project_timeline_context_owner *context,
    project_timeline_edit_limits limits, project_timeline_session **out_session);
void project_timeline_session_destroy(project_timeline_session *session);
base_bytes project_timeline_session_source(const project_timeline_session *session);
project_timeline_edit_result project_timeline_session_source_lease_create(
    const project_timeline_session *session,
    project_timeline_source_lease **out_lease);
bool project_timeline_session_is_dirty(const project_timeline_session *session);
size_t project_timeline_session_undo_count(const project_timeline_session *session);
size_t project_timeline_session_redo_count(const project_timeline_session *session);
bool project_timeline_session_mark_saved(project_timeline_session *session);

/* Decodes the current edited bytes against the session's retained context.
 * The view borrows session storage and is invalidated by edit, undo, redo, or
 * session destruction. */
project_timeline_result project_timeline_session_decode(
    const project_timeline_session *session, project_timeline_view *out_view);

/* Renders the current timeline at an integer tick. Scene and bundle are
 * borrowed for the call and must represent the scene/assets used to create or
 * most recently refresh this session. `out_tile` must be empty; on success it
 * owns its RGBA storage. The call never mutates session or project bytes. */
project_animation_render_result project_timeline_session_render_frame(
    const project_timeline_session *session,
    const svg_shape_document *scene, const project_bundle_view *bundle,
    uint32_t tick, raster_tile *out_tile);

/* Refresh validates the current edited timeline against the latest scene and
 * bundle assets. Failure leaves the prior context usable. Bundle bytes remain
 * borrowed until the next refresh or session destruction. */
project_timeline_edit_result project_timeline_session_refresh_context(
    project_timeline_session *session, const project_bundle_view *bundle,
    base_bytes scene_svg, svg_xml_cancel_check_fn canceled,
    void *cancel_context);

project_timeline_edit_result project_timeline_session_add_drawing_track(
    project_timeline_session *session, base_bytes track_id,
    base_bytes target_id, base_bytes asset_name, base_bytes track_name);
project_timeline_edit_result project_timeline_session_add_property_track(
    project_timeline_session *session, base_bytes track_id,
    base_bytes target_id, base_bytes property, base_bytes value_type,
    base_bytes interpolation, base_bytes value_json, base_bytes track_name);
project_timeline_edit_result project_timeline_session_add_property_track_key(
    project_timeline_session *session, base_bytes track_id,
    base_bytes target_id, base_bytes property, base_bytes value_type,
    base_bytes interpolation, base_bytes initial_json, uint32_t tick,
    base_bytes value_json, base_bytes track_name);
project_timeline_edit_result project_timeline_session_set_property_key(
    project_timeline_session *session, base_bytes track_id, uint32_t tick,
    base_bytes value_json);
project_timeline_edit_result project_timeline_session_set_cel(
    project_timeline_session *session, base_bytes track_id, uint32_t tick,
    base_bytes asset_name, base_bytes label);
project_timeline_edit_result project_timeline_session_remove_cel(
    project_timeline_session *session, base_bytes track_id, uint32_t tick);
project_timeline_edit_result project_timeline_session_move_item(
    project_timeline_session *session, base_bytes track_id,
    uint32_t from_tick, uint32_t to_tick);
project_timeline_edit_result project_timeline_session_set_loop(
    project_timeline_session *session, project_timeline_loop_mode mode);
project_timeline_edit_result project_timeline_session_set_onion(
    project_timeline_session *session, uint32_t before, uint32_t after,
    double opacity);
project_timeline_edit_result project_timeline_session_undo(
    project_timeline_session *session);
project_timeline_edit_result project_timeline_session_redo(
    project_timeline_session *session);

#endif
