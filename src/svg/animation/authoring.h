#ifndef VEXEL_SVG_ANIMATION_AUTHORING_H
#define VEXEL_SVG_ANIMATION_AUTHORING_H

#include "animation.h"

typedef enum svg_anim_authoring_error {
    SVG_ANIM_AUTHORING_OK = 0,
    SVG_ANIM_AUTHORING_INVALID_ARGUMENT,
    SVG_ANIM_AUTHORING_UNSUPPORTED,
    SVG_ANIM_AUTHORING_LIMIT,
    SVG_ANIM_AUTHORING_OUT_OF_MEMORY,
    SVG_ANIM_AUTHORING_NO_OP
} svg_anim_authoring_error;

/* A patch borrows no source bytes; destroy it after document-edit applies it. */
typedef struct svg_anim_authoring_patch {
    size_t offset;
    size_t remove_length;
    uint8_t *replacement;
    size_t replacement_length;
} svg_anim_authoring_patch;

bool svg_anim_authoring_track_editable(const svg_anim_document *document,
                                      size_t track_index);
bool svg_anim_authoring_track_removable(const svg_anim_document *document,
                                       size_t track_index);
svg_anim_authoring_error svg_anim_authoring_set_key_value(
    const svg_anim_document *document, size_t track_index, size_t key_index,
    base_bytes value, svg_anim_authoring_patch *out_patch);
svg_anim_authoring_error svg_anim_authoring_add_key(
    const svg_anim_document *document, size_t track_index, double time_seconds,
    base_bytes value, svg_anim_authoring_patch *out_patch);
svg_anim_authoring_error svg_anim_authoring_move_key(
    const svg_anim_document *document, size_t track_index, size_t key_index,
    double time_seconds, svg_anim_authoring_patch *out_patch);
svg_anim_authoring_error svg_anim_authoring_delete_key(
    const svg_anim_document *document, size_t track_index, size_t key_index,
    svg_anim_authoring_patch *out_patch);
svg_anim_authoring_error svg_anim_authoring_set_calc_mode(
    const svg_anim_document *document, size_t track_index,
    svg_anim_calc_mode mode, svg_anim_authoring_patch *out_patch);
svg_anim_authoring_error svg_anim_authoring_set_timing(
    const svg_anim_document *document, size_t track_index, double begin_seconds,
    double duration_seconds, svg_anim_authoring_patch *out_patch);
svg_anim_authoring_error svg_anim_authoring_set_spline(
    const svg_anim_document *document, size_t track_index, size_t interval,
    const double spline[4], svg_anim_authoring_patch *out_patch);
svg_anim_authoring_error svg_anim_authoring_create_track(
    base_bytes source, size_t target_offset, svg_anim_attribute attribute,
    svg_anim_kind kind, svg_anim_transform_type transform_type,
    base_bytes initial_value, base_bytes first_value, double time_seconds,
    double duration_seconds, uint8_t **out_fragment, size_t *out_length);
void svg_anim_authoring_patch_destroy(svg_anim_authoring_patch *patch);

#endif
