#ifndef VEXEL_PROJECT_PROJECT_TIMELINE_H
#define VEXEL_PROJECT_PROJECT_TIMELINE_H

#include "project_bundle.h"

#define PROJECT_TIMELINE_MAX_TRACKS 256U
#define PROJECT_TIMELINE_MAX_ITEMS 1000000U
#define PROJECT_TIMELINE_MAX_ITEMS_PER_TRACK 100000U
#define PROJECT_TIMELINE_MAX_STRING_BYTES 4096U

typedef enum project_timeline_error {
    PROJECT_TIMELINE_OK = 0,
    PROJECT_TIMELINE_INVALID_ARGUMENT,
    PROJECT_TIMELINE_MALFORMED_JSON,
    PROJECT_TIMELINE_UNSUPPORTED_VERSION,
    PROJECT_TIMELINE_INVALID_SCHEMA,
    PROJECT_TIMELINE_INVALID_REFERENCE,
    PROJECT_TIMELINE_DUPLICATE,
    PROJECT_TIMELINE_LIMIT,
    PROJECT_TIMELINE_INVALID_VALUE
} project_timeline_error;

typedef enum project_timeline_loop_mode {
    PROJECT_TIMELINE_LOOP_ONCE = 0,
    PROJECT_TIMELINE_LOOP_REPEAT,
    PROJECT_TIMELINE_LOOP_PING_PONG
} project_timeline_loop_mode;

typedef struct project_timeline_asset_ref {
    base_bytes name;
    base_bytes media_type;
} project_timeline_asset_ref;

typedef struct project_timeline_context {
    /* Scene IDs are borrowed, unique, and sorted by unsigned byte order. */
    const base_bytes *scene_ids;
    size_t scene_id_count;
    const project_timeline_asset_ref *assets;
    size_t asset_count;
} project_timeline_context;

typedef struct project_timeline_view {
    /* Treat as immutable. Initialize only through project_timeline_decode;
     * accessors require a decoded view and its original borrowed source. */
    base_bytes source;
    base_bytes track_spans[PROJECT_TIMELINE_MAX_TRACKS];
    base_bytes item_spans[PROJECT_TIMELINE_MAX_TRACKS];
    bool drawing_tracks[PROJECT_TIMELINE_MAX_TRACKS];
    size_t item_counts[PROJECT_TIMELINE_MAX_TRACKS];
    size_t tracks_offset;
    size_t tracks_length;
    uint32_t ticks_per_second;
    uint32_t duration_ticks;
    uint32_t fps_numerator;
    uint32_t fps_denominator;
    project_timeline_loop_mode loop_mode;
    uint32_t onion_before;
    uint32_t onion_after;
    double onion_opacity;
    size_t track_count;
    bool empty;
} project_timeline_view;

typedef struct project_timeline_result {
    project_timeline_error error;
    size_t error_offset;
} project_timeline_result;

typedef struct project_timeline_track_view {
    /* String slices are raw JSON string contents; decode through
     * project_timeline_string_copy before interpreting escaped characters. */
    base_bytes id;
    base_bytes kind;
    base_bytes target;
    base_bytes name;
    bool visible;
    base_bytes payload_json;
    size_t item_count;
    /* Decoded property metadata; empty for drawing tracks. Slices borrow source. */
    base_bytes property;
    base_bytes value_type;
    base_bytes interpolation;
} project_timeline_track_view;

typedef struct project_timeline_item_view {
    uint32_t tick;
    base_bytes asset;
    base_bytes value_json;
    size_t source_offset;
    size_t source_length;
} project_timeline_item_view;

typedef struct project_timeline_iterator {
    /* Zero-initialize before the first item_next call; state is bound to one
     * track and source view until exhausted or reset to {0}. */
    size_t track_index;
    size_t next_index;
    size_t source_cursor;
    bool initialized;
} project_timeline_iterator;

project_timeline_result project_timeline_decode(base_bytes source,
    const project_timeline_context *context, project_timeline_view *out_view);
project_timeline_result project_timeline_decode_cancellable(base_bytes source,
    const project_timeline_context *context, project_timeline_view *out_view,
    bool (*cancel_check)(void *), void *cancel_context, bool *out_canceled);
bool project_timeline_track_at(const project_timeline_view *view, size_t index,
                               project_timeline_track_view *out_track);
bool project_timeline_item_at(const project_timeline_view *view, size_t track_index,
                              size_t item_index, project_timeline_item_view *out_item);
bool project_timeline_item_next(const project_timeline_view *view, size_t track_index,
                               project_timeline_iterator *iterator,
                               project_timeline_item_view *out_item);
bool project_timeline_string_copy(base_bytes encoded_string, base_mut_bytes output,
                                  size_t *out_length);
const char *project_timeline_error_name(project_timeline_error error);

#endif
