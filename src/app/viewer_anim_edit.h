#ifndef VEXEL_APP_VIEWER_ANIM_EDIT_H
#define VEXEL_APP_VIEWER_ANIM_EDIT_H

#include "svg/animation/animation.h"
#include "viewer_editor.h"

/* Commands compare the borrowed decoded source with the current edit snapshot.
 * A stale preview is rejected before any source or history mutation. */
uint32_t viewer_editor_animation_add_track(viewer_editor *editor,
    const svg_anim_document *animation, svg_anim_attribute attribute,
    svg_anim_kind kind, svg_anim_transform_type transform_type,
    base_bytes initial_value, base_bytes first_value,
    double time_seconds, double duration_seconds);
uint32_t viewer_editor_animation_remove_track(viewer_editor *editor,
    const svg_anim_document *animation, size_t track_index);
uint32_t viewer_editor_animation_set_key_value(viewer_editor *editor,
    const svg_anim_document *animation, size_t track_index, size_t key_index,
    base_bytes value);
uint32_t viewer_editor_animation_add_key(viewer_editor *editor,
    const svg_anim_document *animation, size_t track_index,
    double time_seconds, base_bytes value);
uint32_t viewer_editor_animation_move_key(viewer_editor *editor,
    const svg_anim_document *animation, size_t track_index, size_t key_index,
    double time_seconds);
uint32_t viewer_editor_animation_delete_key(viewer_editor *editor,
    const svg_anim_document *animation, size_t track_index, size_t key_index);
uint32_t viewer_editor_animation_set_mode(viewer_editor *editor,
    const svg_anim_document *animation, size_t track_index,
    svg_anim_calc_mode mode);
uint32_t viewer_editor_animation_set_timing(viewer_editor *editor,
    const svg_anim_document *animation, size_t track_index,
    double begin_seconds, double duration_seconds);
uint32_t viewer_editor_animation_set_spline(viewer_editor *editor,
    const svg_anim_document *animation, size_t track_index, size_t interval,
    const double spline[4]);

#endif
