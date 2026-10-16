#ifndef VEXEL_PROJECT_PROJECT_TIMELINE_EDIT_H
#define VEXEL_PROJECT_PROJECT_TIMELINE_EDIT_H

#include "project_timeline.h"

typedef struct project_timeline_edit project_timeline_edit;
typedef struct project_timeline_source_lease project_timeline_source_lease;

typedef struct project_timeline_edit_limits {
    size_t history_commands;
    size_t history_bytes;
} project_timeline_edit_limits;

typedef struct project_timeline_edit_result {
    project_timeline_error error;
    size_t error_offset;
} project_timeline_edit_result;

#define PROJECT_TIMELINE_EDIT_DEFAULT_COMMANDS 16U
#define PROJECT_TIMELINE_EDIT_DEFAULT_BYTES (128U * 1024U * 1024U)
#define PROJECT_TIMELINE_EDIT_MAX_COMMANDS 1024U

project_timeline_edit_limits project_timeline_edit_default_limits(void);
/* Creation copies source bytes; caller storage need not outlive this call. The
 * validation context is borrowed only during each operation. */
project_timeline_edit_result project_timeline_edit_create(
    base_bytes source, const project_timeline_context *context,
    project_timeline_edit_limits limits, project_timeline_edit **out_edit);
void project_timeline_edit_destroy(project_timeline_edit *edit);
/* The returned view borrows edit and is invalidated by a successful edit,
 * undo, redo, or destroy. Rejected operations preserve bytes and history. */
base_bytes project_timeline_edit_source(const project_timeline_edit *edit);
/* Pins the immutable current byte buffer without copying it. Leases may be
 * read on a worker and released independently of history trimming. */
project_timeline_edit_result project_timeline_edit_source_lease_create(
    const project_timeline_edit *edit,
    project_timeline_source_lease **out_lease);
base_bytes project_timeline_source_lease_bytes(
    const project_timeline_source_lease *lease);
void project_timeline_source_lease_destroy(project_timeline_source_lease *lease);
size_t project_timeline_edit_undo_count(const project_timeline_edit *edit);
size_t project_timeline_edit_redo_count(const project_timeline_edit *edit);
project_timeline_edit_result project_timeline_edit_add_drawing_track(
    project_timeline_edit *edit, const project_timeline_context *context,
    base_bytes track_id, base_bytes target_id, base_bytes asset_name,
    base_bytes track_name);
/* Property tracks start with the supplied value at tick zero. value_json is a
 * bounded raw JSON number, boolean, or numeric array and is schema-validated. */
project_timeline_edit_result project_timeline_edit_add_property_track(
    project_timeline_edit *edit, const project_timeline_context *context,
    base_bytes track_id, base_bytes target_id, base_bytes property,
    base_bytes value_type, base_bytes interpolation, base_bytes value_json,
    base_bytes track_name);
project_timeline_edit_result project_timeline_edit_add_property_track_key(
    project_timeline_edit *edit, const project_timeline_context *context,
    base_bytes track_id, base_bytes target_id, base_bytes property,
    base_bytes value_type, base_bytes interpolation, base_bytes initial_json,
    uint32_t tick, base_bytes value_json, base_bytes track_name);
project_timeline_edit_result project_timeline_edit_set_property_key(
    project_timeline_edit *edit, const project_timeline_context *context,
    base_bytes track_id, uint32_t tick, base_bytes value_json);
/* set_cel replaces an exposure at tick or inserts it in order. Empty label
 * omits label (and clears one on replacement). remove_cel of tick zero is
 * rejected when the invariant would leave a track without a tick-zero cel;
 * removing a missing tick returns INVALID_REFERENCE. */
project_timeline_edit_result project_timeline_edit_set_cel(
    project_timeline_edit *edit, const project_timeline_context *context,
    base_bytes track_id, uint32_t tick, base_bytes asset_name,
    base_bytes label);
project_timeline_edit_result project_timeline_edit_remove_cel(
    project_timeline_edit *edit, const project_timeline_context *context,
    base_bytes track_id, uint32_t tick);
/* Moves one existing drawing cel or property key while preserving its payload.
 * Tick zero is the required baseline and cannot be moved. Collisions fail. */
project_timeline_edit_result project_timeline_edit_move_item(
    project_timeline_edit *edit, const project_timeline_context *context,
    base_bytes track_id, uint32_t from_tick, uint32_t to_tick);
project_timeline_edit_result project_timeline_edit_set_loop(
    project_timeline_edit *edit, const project_timeline_context *context,
    project_timeline_loop_mode mode);
project_timeline_edit_result project_timeline_edit_set_onion(
    project_timeline_edit *edit, const project_timeline_context *context,
    uint32_t before, uint32_t after, double opacity);
project_timeline_edit_result project_timeline_edit_undo(
    project_timeline_edit *edit, const project_timeline_context *context);
project_timeline_edit_result project_timeline_edit_redo(
    project_timeline_edit *edit, const project_timeline_context *context);

#endif
