#include "viewer_anim_edit.h"

#include "svg/animation/authoring.h"
#include "viewer_editor_internal.h"

#include <stdlib.h>
#include <string.h>

#define VIEWER_ANIM_EDIT_SOURCE_LIMIT (64U * 1024U * 1024U)

static bool source_is_current(viewer_editor *editor,
                              const svg_anim_document *animation)
{
    if (editor == NULL || editor->edit == NULL || animation == NULL ||
        !base_bytes_valid(animation->source) ||
        animation->source.len == 0U ||
        animation->source.len > VIEWER_ANIM_EDIT_SOURCE_LIMIT ||
        document_edit_byte_length(editor->edit) != animation->source.len)
        return false;
    uint8_t *current = malloc(animation->source.len);
    if (current == NULL) return false;
    size_t length = 0U;
    bool okay = document_edit_materialize(editor->edit, current,
        animation->source.len, &length) && length == animation->source.len &&
        memcmp(current, animation->source.ptr, length) == 0;
    free(current);
    return okay;
}

static uint32_t apply_patch(viewer_editor *editor,
    const svg_anim_document *animation, svg_anim_authoring_patch *patch)
{
    if (patch == NULL || patch->replacement == NULL ||
        patch->replacement_length == 0U) return VIEWER_EDITOR_HANDLED;
    size_t selected_offset = patch->offset;
    document_edit_error error = document_edit_replace_source_span(editor->edit,
        (document_source_span){patch->offset, patch->remove_length},
        (base_bytes){patch->replacement, patch->replacement_length});
    svg_anim_authoring_patch_destroy(patch);
    if (error != DOCUMENT_EDIT_OK) {
        viewer_editor_set_status(editor, "Animation edit rejected; source is unchanged");
        return VIEWER_EDITOR_HANDLED;
    }
    (void)animation;
    (void)viewer_editor_select_at_source_offset(editor, selected_offset);
    viewer_editor_set_status(editor, "Animation key edited");
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_MUTATED;
}

static uint32_t apply_authoring_result(viewer_editor *editor,
    const svg_anim_document *animation, svg_anim_authoring_error result,
    svg_anim_authoring_patch *patch)
{
    if (result == SVG_ANIM_AUTHORING_NO_OP) {
        svg_anim_authoring_patch_destroy(patch);
        return VIEWER_EDITOR_HANDLED;
    }
    if (result != SVG_ANIM_AUTHORING_OK) {
        svg_anim_authoring_patch_destroy(patch);
        viewer_editor_set_status(editor,
            result == SVG_ANIM_AUTHORING_UNSUPPORTED ?
                "This animation track is preview-only" : "Animation edit rejected");
        return VIEWER_EDITOR_HANDLED;
    }
    return apply_patch(editor, animation, patch);
}

static uint32_t apply_track_patch(viewer_editor *editor,
    size_t target_offset,
    const uint8_t *fragment, size_t fragment_length)
{
    document_handle inserted;
    document_edit_error error = document_edit_insert_xml_fragment(editor->edit,
        editor->selected, (base_bytes){fragment, fragment_length}, &inserted);
    if (error != DOCUMENT_EDIT_OK) {
        viewer_editor_set_status(editor, "Animation track could not be inserted");
        return VIEWER_EDITOR_HANDLED;
    }
    (void)viewer_editor_select_at_source_offset(editor, target_offset);
    viewer_editor_set_status(editor, "Animation track added");
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_MUTATED;
}

uint32_t viewer_editor_animation_add_track(viewer_editor *editor,
    const svg_anim_document *animation, svg_anim_attribute attribute,
    svg_anim_kind kind, svg_anim_transform_type transform_type,
    base_bytes initial_value, base_bytes first_value,
    double time_seconds, double duration_seconds)
{
    if (!source_is_current(editor, animation)) return VIEWER_EDITOR_HANDLED;
    document_source_span target;
    if (!document_edit_source_span(editor->edit, editor->selected, &target))
        return VIEWER_EDITOR_HANDLED;
    uint8_t *fragment = NULL;
    size_t fragment_length = 0U;
    svg_anim_authoring_error result = svg_anim_authoring_create_track(
        animation->source, target.offset, attribute, kind, transform_type,
        initial_value, first_value, time_seconds, duration_seconds,
        &fragment, &fragment_length);
    if (result != SVG_ANIM_AUTHORING_OK) {
        free(fragment);
        viewer_editor_set_status(editor, "Animation track could not be created");
        return VIEWER_EDITOR_HANDLED;
    }
    uint32_t flags = apply_track_patch(editor, target.offset,
        fragment, fragment_length);
    free(fragment);
    return flags;
}

uint32_t viewer_editor_animation_remove_track(viewer_editor *editor,
    const svg_anim_document *animation, size_t track_index)
{
    if (!source_is_current(editor, animation) ||
        !svg_anim_authoring_track_removable(animation, track_index))
        return VIEWER_EDITOR_HANDLED;
    const svg_anim_track *track = &animation->tracks[track_index];
    document_handle node;
    if (!document_edit_element_at_offset(editor->edit, track->element_offset, &node))
        return VIEWER_EDITOR_HANDLED;
    document_edit_error error = document_edit_delete_element(editor->edit, node);
    if (error != DOCUMENT_EDIT_OK) return VIEWER_EDITOR_HANDLED;
    (void)viewer_editor_select_at_source_offset(editor, track->target_offset);
    viewer_editor_set_status(editor, "Animation track removed");
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_MUTATED;
}

static uint32_t run_patch(viewer_editor *editor,
    const svg_anim_document *animation, svg_anim_authoring_error result,
    svg_anim_authoring_patch *patch)
{
    return apply_authoring_result(editor, animation, result, patch);
}

uint32_t viewer_editor_animation_set_key_value(viewer_editor *editor,
    const svg_anim_document *animation, size_t track_index, size_t key_index,
    base_bytes value)
{
    if (!source_is_current(editor, animation)) return VIEWER_EDITOR_HANDLED;
    svg_anim_authoring_patch patch = {0};
    svg_anim_authoring_error error = svg_anim_authoring_set_key_value(animation,
        track_index, key_index, value, &patch);
    return run_patch(editor, animation, error, &patch);
}

uint32_t viewer_editor_animation_add_key(viewer_editor *editor,
    const svg_anim_document *animation, size_t track_index, double time_seconds,
    base_bytes value)
{
    if (!source_is_current(editor, animation)) return VIEWER_EDITOR_HANDLED;
    svg_anim_authoring_patch patch = {0};
    svg_anim_authoring_error error = svg_anim_authoring_add_key(animation,
        track_index, time_seconds, value, &patch);
    return run_patch(editor, animation, error, &patch);
}

uint32_t viewer_editor_animation_move_key(viewer_editor *editor,
    const svg_anim_document *animation, size_t track_index, size_t key_index,
    double time_seconds)
{
    if (!source_is_current(editor, animation)) return VIEWER_EDITOR_HANDLED;
    svg_anim_authoring_patch patch = {0};
    svg_anim_authoring_error error = svg_anim_authoring_move_key(animation,
        track_index, key_index, time_seconds, &patch);
    return run_patch(editor, animation, error, &patch);
}

uint32_t viewer_editor_animation_delete_key(viewer_editor *editor,
    const svg_anim_document *animation, size_t track_index, size_t key_index)
{
    if (!source_is_current(editor, animation)) return VIEWER_EDITOR_HANDLED;
    svg_anim_authoring_patch patch = {0};
    svg_anim_authoring_error error = svg_anim_authoring_delete_key(animation,
        track_index, key_index, &patch);
    return run_patch(editor, animation, error, &patch);
}

uint32_t viewer_editor_animation_set_mode(viewer_editor *editor,
    const svg_anim_document *animation, size_t track_index,
    svg_anim_calc_mode mode)
{
    if (!source_is_current(editor, animation)) return VIEWER_EDITOR_HANDLED;
    svg_anim_authoring_patch patch = {0};
    svg_anim_authoring_error error = svg_anim_authoring_set_calc_mode(animation,
        track_index, mode, &patch);
    return run_patch(editor, animation, error, &patch);
}

uint32_t viewer_editor_animation_set_timing(viewer_editor *editor,
    const svg_anim_document *animation, size_t track_index,
    double begin_seconds, double duration_seconds)
{
    if (!source_is_current(editor, animation)) return VIEWER_EDITOR_HANDLED;
    svg_anim_authoring_patch patch = {0};
    svg_anim_authoring_error error = svg_anim_authoring_set_timing(animation,
        track_index, begin_seconds, duration_seconds, &patch);
    return run_patch(editor, animation, error, &patch);
}

uint32_t viewer_editor_animation_set_spline(viewer_editor *editor,
    const svg_anim_document *animation, size_t track_index, size_t interval,
    const double spline[4])
{
    if (!source_is_current(editor, animation)) return VIEWER_EDITOR_HANDLED;
    svg_anim_authoring_patch patch = {0};
    svg_anim_authoring_error error = svg_anim_authoring_set_spline(animation,
        track_index, interval, spline, &patch);
    return run_patch(editor, animation, error, &patch);
}
