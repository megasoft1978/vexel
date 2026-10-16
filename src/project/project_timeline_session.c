#include "project_timeline_session.h"

#include <stdlib.h>

struct project_timeline_session {
    project_timeline_edit *edit;
    project_timeline_context_owner context;
    uint8_t *saved_source;
    size_t saved_source_length;
};

static project_timeline_edit_result session_result(project_timeline_error error,
                                                    size_t offset)
{
    return (project_timeline_edit_result){error, offset};
}

project_timeline_edit_result project_timeline_session_create(
    base_bytes source, project_timeline_context_owner *context,
    project_timeline_edit_limits limits, project_timeline_session **out_session)
{
    if (out_session != NULL) *out_session = NULL;
    if (context == NULL || out_session == NULL)
        return session_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    project_timeline_session *session = calloc(1U, sizeof(*session));
    if (session == NULL) return session_result(PROJECT_TIMELINE_LIMIT, 0U);
    project_timeline_edit_result created = project_timeline_edit_create(
        source, &context->context, limits, &session->edit);
    if (created.error != PROJECT_TIMELINE_OK) {
        free(session);
        return created;
    }
    if (source.len != 0U) {
        session->saved_source = malloc(source.len);
        if (session->saved_source == NULL) {
            project_timeline_edit_destroy(session->edit);
            free(session);
            return session_result(PROJECT_TIMELINE_LIMIT, 0U);
        }
        for (size_t index = 0U; index < source.len; ++index)
            session->saved_source[index] = source.ptr[index];
    }
    session->saved_source_length = source.len;
    session->context = *context;
    *context = (project_timeline_context_owner){0};
    *out_session = session;
    return session_result(PROJECT_TIMELINE_OK, 0U);
}

void project_timeline_session_destroy(project_timeline_session *session)
{
    if (session == NULL) return;
    project_timeline_edit_destroy(session->edit);
    project_timeline_context_owner_destroy(&session->context);
    free(session->saved_source);
    free(session);
}

bool project_timeline_session_is_dirty(const project_timeline_session *session)
{
    if (session == NULL) return false;
    base_bytes current = project_timeline_edit_source(session->edit);
    if (current.len != session->saved_source_length) return true;
    for (size_t index = 0U; index < current.len; ++index)
        if (current.ptr[index] != session->saved_source[index]) return true;
    return false;
}

size_t project_timeline_session_undo_count(const project_timeline_session *session)
{
    return session == NULL ? 0U : project_timeline_edit_undo_count(session->edit);
}

size_t project_timeline_session_redo_count(const project_timeline_session *session)
{
    return session == NULL ? 0U : project_timeline_edit_redo_count(session->edit);
}

bool project_timeline_session_mark_saved(project_timeline_session *session)
{
    if (session == NULL) return false;
    base_bytes current = project_timeline_edit_source(session->edit);
    uint8_t *copy = NULL;
    if (current.len != 0U) {
        copy = malloc(current.len);
        if (copy == NULL) return false;
        for (size_t index = 0U; index < current.len; ++index)
            copy[index] = current.ptr[index];
    }
    free(session->saved_source);
    session->saved_source = copy;
    session->saved_source_length = current.len;
    return true;
}

base_bytes project_timeline_session_source(const project_timeline_session *session)
{
    return session == NULL ? (base_bytes){NULL, 0U} :
        project_timeline_edit_source(session->edit);
}

project_timeline_edit_result project_timeline_session_source_lease_create(
    const project_timeline_session *session,
    project_timeline_source_lease **out_lease)
{
    if (out_lease != NULL) *out_lease = NULL;
    if (session == NULL || out_lease == NULL)
        return session_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    return project_timeline_edit_source_lease_create(session->edit, out_lease);
}

project_timeline_result project_timeline_session_decode(
    const project_timeline_session *session, project_timeline_view *out_view)
{
    if (out_view != NULL) *out_view = (project_timeline_view){0};
    if (session == NULL || out_view == NULL)
        return (project_timeline_result){PROJECT_TIMELINE_INVALID_ARGUMENT, 0U};
    return project_timeline_decode(project_timeline_session_source(session),
        &session->context.context, out_view);
}

project_animation_render_result project_timeline_session_render_frame(
    const project_timeline_session *session,
    const svg_shape_document *scene, const project_bundle_view *bundle,
    uint32_t tick, raster_tile *out_tile)
{
    project_animation_render_result invalid = {
        .error = PROJECT_ANIMATION_RENDER_INVALID_ARGUMENT
    };
    if (session == NULL || scene == NULL || bundle == NULL || out_tile == NULL)
        return invalid;
    if (scene->source.ptr == NULL || scene->source.len == 0U) return invalid;
    project_bundle_view current_bundle = *bundle;
    current_bundle.scene_svg = scene->source;
    current_bundle.timeline_json = project_timeline_session_source(session);
    project_timeline_view view = {0};
    project_timeline_context_owner current_context = {0};
    project_timeline_integration_result checked =
        project_timeline_validate_bundle_owned(&current_bundle,
            current_bundle.scene_svg, NULL, NULL, &view, &current_context);
    project_timeline_context_owner_destroy(&current_context);
    if (checked.compatibility != PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1 ||
        checked.validation.error != PROJECT_TIMELINE_OK) {
        invalid.error = checked.validation.error ==
                PROJECT_TIMELINE_INVALID_REFERENCE ?
            PROJECT_ANIMATION_RENDER_INVALID_REFERENCE :
            PROJECT_ANIMATION_RENDER_INVALID_TIMELINE;
        invalid.source_offset = checked.validation.error_offset;
        return invalid;
    }
    return project_animation_render_frame(scene, &current_bundle, &view, tick,
        out_tile);
}

project_timeline_edit_result project_timeline_session_refresh_context(
    project_timeline_session *session, const project_bundle_view *bundle,
    base_bytes scene_svg, svg_xml_cancel_check_fn canceled,
    void *cancel_context)
{
    if (session == NULL || bundle == NULL)
        return session_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    project_bundle_view candidate_bundle = *bundle;
    candidate_bundle.timeline_json = project_timeline_session_source(session);
    project_timeline_context_owner candidate_context = {0};
    project_timeline_view view = {0};
    project_timeline_integration_result checked =
        project_timeline_validate_bundle_owned(&candidate_bundle, scene_svg,
            canceled, cancel_context, &view, &candidate_context);
    if (checked.canceled || checked.compatibility !=
            PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1 ||
        checked.validation.error != PROJECT_TIMELINE_OK) {
        project_timeline_context_owner_destroy(&candidate_context);
        return session_result(checked.validation.error == PROJECT_TIMELINE_OK ?
            PROJECT_TIMELINE_INVALID_ARGUMENT : checked.validation.error,
            checked.validation.error_offset);
    }
    project_timeline_context_owner_destroy(&session->context);
    session->context = candidate_context;
    return session_result(PROJECT_TIMELINE_OK, 0U);
}

project_timeline_edit_result project_timeline_session_add_drawing_track(
    project_timeline_session *session, base_bytes track_id,
    base_bytes target_id, base_bytes asset_name, base_bytes track_name)
{
    if (session == NULL) return session_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    return project_timeline_edit_add_drawing_track(session->edit,
        &session->context.context, track_id, target_id, asset_name, track_name);
}

project_timeline_edit_result project_timeline_session_add_property_track(
    project_timeline_session *session, base_bytes track_id,
    base_bytes target_id, base_bytes property, base_bytes value_type,
    base_bytes interpolation, base_bytes value_json, base_bytes track_name)
{
    if (session == NULL) return session_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    return project_timeline_edit_add_property_track(session->edit,
        &session->context.context, track_id, target_id, property, value_type,
        interpolation, value_json, track_name);
}

project_timeline_edit_result project_timeline_session_add_property_track_key(
    project_timeline_session *session, base_bytes track_id,
    base_bytes target_id, base_bytes property, base_bytes value_type,
    base_bytes interpolation, base_bytes initial_json, uint32_t tick,
    base_bytes value_json, base_bytes track_name)
{
    if (session == NULL) return session_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    return project_timeline_edit_add_property_track_key(session->edit,
        &session->context.context, track_id, target_id, property, value_type,
        interpolation, initial_json, tick, value_json, track_name);
}

project_timeline_edit_result project_timeline_session_set_property_key(
    project_timeline_session *session, base_bytes track_id, uint32_t tick,
    base_bytes value_json)
{
    if (session == NULL) return session_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    return project_timeline_edit_set_property_key(session->edit,
        &session->context.context, track_id, tick, value_json);
}

project_timeline_edit_result project_timeline_session_set_cel(
    project_timeline_session *session, base_bytes track_id, uint32_t tick,
    base_bytes asset_name, base_bytes label)
{
    if (session == NULL) return session_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    return project_timeline_edit_set_cel(session->edit,
        &session->context.context, track_id, tick, asset_name, label);
}

project_timeline_edit_result project_timeline_session_remove_cel(
    project_timeline_session *session, base_bytes track_id, uint32_t tick)
{
    if (session == NULL) return session_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    return project_timeline_edit_remove_cel(session->edit,
        &session->context.context, track_id, tick);
}

project_timeline_edit_result project_timeline_session_move_item(
    project_timeline_session *session, base_bytes track_id,
    uint32_t from_tick, uint32_t to_tick)
{
    if (session == NULL) return session_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    return project_timeline_edit_move_item(session->edit,
        &session->context.context, track_id, from_tick, to_tick);
}

project_timeline_edit_result project_timeline_session_set_loop(
    project_timeline_session *session, project_timeline_loop_mode mode)
{
    if (session == NULL) return session_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    return project_timeline_edit_set_loop(session->edit,
        &session->context.context, mode);
}

project_timeline_edit_result project_timeline_session_set_onion(
    project_timeline_session *session, uint32_t before, uint32_t after,
    double opacity)
{
    if (session == NULL) return session_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    return project_timeline_edit_set_onion(session->edit,
        &session->context.context, before, after, opacity);
}

project_timeline_edit_result project_timeline_session_undo(
    project_timeline_session *session)
{
    if (session == NULL) return session_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    return project_timeline_edit_undo(session->edit, &session->context.context);
}

project_timeline_edit_result project_timeline_session_redo(
    project_timeline_session *session)
{
    if (session == NULL) return session_result(PROJECT_TIMELINE_INVALID_ARGUMENT, 0U);
    return project_timeline_edit_redo(session->edit, &session->context.context);
}
