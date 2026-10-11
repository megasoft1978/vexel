#ifndef VEXEL_SVG_ANIMATION_H
#define VEXEL_SVG_ANIMATION_H

#include "base.h"
#include "xml_tokenizer.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define SVG_ANIM_MAX_TRACKS 128U
#define SVG_ANIM_MAX_VALUES 256U
#define SVG_ANIM_MAX_DIAGNOSTICS 128U
#define SVG_ANIM_MAX_TIMELINE_SECONDS 3600.0

typedef enum svg_anim_attribute {
    SVG_ANIM_ATTR_X,
    SVG_ANIM_ATTR_Y,
    SVG_ANIM_ATTR_WIDTH,
    SVG_ANIM_ATTR_HEIGHT,
    SVG_ANIM_ATTR_CX,
    SVG_ANIM_ATTR_CY,
    SVG_ANIM_ATTR_R,
    SVG_ANIM_ATTR_RX,
    SVG_ANIM_ATTR_RY,
    SVG_ANIM_ATTR_OPACITY,
    SVG_ANIM_ATTR_FILL_OPACITY,
    SVG_ANIM_ATTR_STROKE_OPACITY,
    SVG_ANIM_ATTR_FILL,
    SVG_ANIM_ATTR_STROKE,
    SVG_ANIM_ATTR_STROKE_WIDTH,
    SVG_ANIM_ATTR_STROKE_DASHOFFSET,
    SVG_ANIM_ATTR_D,
    SVG_ANIM_ATTR_TRANSFORM,
    SVG_ANIM_ATTR_MOTION
} svg_anim_attribute;

typedef enum svg_anim_kind {
    SVG_ANIM_KIND_ANIMATE,
    SVG_ANIM_KIND_TRANSFORM,
    SVG_ANIM_KIND_SET,
    SVG_ANIM_KIND_MOTION
} svg_anim_kind;

typedef enum svg_anim_value_kind {
    SVG_ANIM_VALUE_NUMBER,
    SVG_ANIM_VALUE_COLOR,
    SVG_ANIM_VALUE_TRANSFORM,
    SVG_ANIM_VALUE_MOTION,
    SVG_ANIM_VALUE_PATH
} svg_anim_value_kind;

typedef enum svg_anim_calc_mode {
    SVG_ANIM_CALC_LINEAR,
    SVG_ANIM_CALC_SPLINE,
    SVG_ANIM_CALC_DISCRETE,
    SVG_ANIM_CALC_PACED
} svg_anim_calc_mode;

typedef enum svg_anim_fill_mode {
    SVG_ANIM_FILL_REMOVE,
    SVG_ANIM_FILL_FREEZE
} svg_anim_fill_mode;

typedef enum svg_anim_transform_type {
    SVG_ANIM_TRANSFORM_TRANSLATE,
    SVG_ANIM_TRANSFORM_SCALE,
    SVG_ANIM_TRANSFORM_ROTATE,
    SVG_ANIM_TRANSFORM_SKEW_X,
    SVG_ANIM_TRANSFORM_SKEW_Y
} svg_anim_transform_type;

typedef enum svg_anim_diagnostic_kind {
    SVG_ANIM_DIAG_UNSUPPORTED_ELEMENT,
    SVG_ANIM_DIAG_UNSUPPORTED_ATTRIBUTE,
    SVG_ANIM_DIAG_UNSUPPORTED_TIMING,
    SVG_ANIM_DIAG_UNSUPPORTED_COMPOSITION,
    SVG_ANIM_DIAG_UNSUPPORTED_PATH,
    SVG_ANIM_DIAG_CSS_ANIMATION,
    SVG_ANIM_DIAG_INVALID_TRACK
} svg_anim_diagnostic_kind;

typedef struct svg_anim_value {
    svg_anim_value_kind kind;
    double number[4];
    uint8_t rgba[4];
    size_t raw_offset;
    size_t raw_length;
} svg_anim_value;

typedef struct svg_anim_keyframe {
    double time;
    svg_anim_value value;
    size_t source_offset;
    size_t source_length;
} svg_anim_keyframe;

typedef struct svg_anim_motion_point {
    double x;
    double y;
    double distance;
} svg_anim_motion_point;

typedef struct svg_anim_track {
    svg_anim_kind kind;
    svg_anim_attribute attribute;
    svg_anim_transform_type transform_type;
    svg_anim_calc_mode calc_mode;
    svg_anim_fill_mode fill_mode;
    svg_anim_value base_value;
    svg_anim_keyframe *keys;
    svg_anim_motion_point *motion_points;
    size_t key_count;
    size_t motion_point_count;
    double motion_length;
    double motion_fixed_rotation;
    double (*splines)[4];
    size_t spline_count;
    double begin_seconds;
    double begin_offset;
    double duration_seconds;
    double end_seconds;
    double repeat_count;
    double repeat_duration;
    size_t begin_sync_track;
    size_t end_sync_track;
    svg_xml_span begin_sync_id;
    svg_xml_span end_sync_id;
    bool begin_indefinite;
    bool begin_sync_end;
    bool has_begin_sync;
    bool has_end;
    bool end_is_sync;
    bool repeat_indefinite;
    bool duration_indefinite;
    bool repeat_duration_indefinite;
    bool additive_sum;
    bool accumulate;
    bool motion_rotate_auto;
    bool motion_rotate_reverse;
    bool supported;
    size_t target_offset;
    size_t target_length;
    size_t element_offset;
    size_t element_length;
    svg_xml_span id;
} svg_anim_track;

typedef struct svg_anim_diagnostic {
    svg_anim_diagnostic_kind kind;
    size_t offset;
} svg_anim_diagnostic;

typedef struct svg_anim_document {
    base_bytes source;
    svg_anim_track *tracks;
    size_t track_count;
    svg_anim_diagnostic diagnostics[SVG_ANIM_MAX_DIAGNOSTICS];
    size_t diagnostic_count;
} svg_anim_document;

typedef struct svg_anim_overlay_item {
    size_t track_index;
    svg_anim_value value;
    size_t iteration;
    bool active;
} svg_anim_overlay_item;

typedef struct svg_anim_overlay {
    svg_anim_overlay_item *items;
    size_t count;
    double time_seconds;
} svg_anim_overlay;

/* Track values and byte spans borrow source; document destruction frees only arrays. */
bool svg_anim_parse(base_bytes source, const svg_xml_token *tokens,
                    size_t token_count, svg_anim_document *out,
                    size_t *error_offset);
void svg_anim_document_destroy(svg_anim_document *document);
bool svg_anim_eval(const svg_anim_document *document, double time_seconds,
                   svg_anim_overlay *out);
void svg_anim_overlay_destroy(svg_anim_overlay *overlay);
const char *svg_anim_attribute_name(svg_anim_attribute attribute);
const char *svg_anim_diagnostic_name(svg_anim_diagnostic_kind kind);

#endif
