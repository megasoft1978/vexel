#ifndef VEXEL_APP_VIEWER_TIMELINE_H
#define VEXEL_APP_VIEWER_TIMELINE_H

#include "platform/platform_window.h"
#include "raster/tile.h"
#include "shape.h"
#include "ui/ui_core.h"
#include "viewer_editor.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define VIEWER_TIMELINE_VISIBLE_TRACK_CAP 32U
#define VIEWER_TIMELINE_DOCK_MIN_POINTS 184U
#define VIEWER_TIMELINE_DOCK_MAX_POINTS 320U
#define VIEWER_TIMELINE_DOCK_DEFAULT_POINTS 220U
#define VIEWER_TIMELINE_DOCK_COLLAPSED_POINTS 32U

typedef enum viewer_timeline_mode {
    VIEWER_TIMELINE_MODE_ONE_SHOT,
    VIEWER_TIMELINE_MODE_LOOP,
    VIEWER_TIMELINE_MODE_PING_PONG
} viewer_timeline_mode;

typedef struct viewer_timeline {
    const svg_shape_document *document; /* borrowed while the source document lives */
    raster_tile preview;                /* owned cached animation sample */
    double duration_seconds;
    double time_seconds;
    double preview_time_seconds;
    double last_tick_seconds;
    double last_frame_tick_seconds;
    double view_start_seconds;
    double zoom_factor;
    double playback_speed;
    double drag_start_y;
    size_t first_track;
    size_t selected_track;
    uint16_t dock_height_points;
    viewer_timeline_mode mode;
    int8_t playback_direction;
    bool playing;
    bool loop;
    bool dock_open;
    bool dock_state_explicit;
    bool reduced_motion;
    bool dragging_scrubber;
    bool dragging_resize;
    bool preview_valid;
    bool preview_failed;
    bool has_tick_time;
} viewer_timeline;

typedef struct viewer_timeline_layout {
    double backing_scale;
    ui_rect panel;
    ui_rect play_button;
    ui_rect time_readout;
    ui_rect loop_toggle;
    ui_rect scrubber;
    ui_rect header;
    ui_rect collapse_toggle;
    ui_rect mode_button;
    ui_rect speed_button;
    ui_rect fit_button;
    ui_rect resize_handle;
    ui_rect ruler;
    ui_rect track_gutter;
    ui_rect time_grid;
    ui_rect track_rows[VIEWER_TIMELINE_VISIBLE_TRACK_CAP];
    size_t first_track;
    size_t visible_track_count;
    size_t track_count;
    bool collapsed;
    bool compact;
} viewer_timeline_layout;

typedef enum viewer_timeline_pointer_kind {
    VIEWER_TIMELINE_POINTER_DOWN,
    VIEWER_TIMELINE_POINTER_MOVE,
    VIEWER_TIMELINE_POINTER_UP,
    VIEWER_TIMELINE_POINTER_SCROLL
} viewer_timeline_pointer_kind;

void viewer_timeline_init(viewer_timeline *timeline);
void viewer_timeline_reset(viewer_timeline *timeline,
                           const svg_shape_document *document);
void viewer_timeline_destroy(viewer_timeline *timeline);
size_t viewer_timeline_track_count(const viewer_timeline *timeline);
double viewer_timeline_duration(const viewer_timeline *timeline);
bool viewer_timeline_track_label(const viewer_timeline *timeline, size_t track_index,
                                char *buffer, size_t capacity, size_t *out_length);
bool viewer_timeline_seek(viewer_timeline *timeline, double time_seconds);
bool viewer_timeline_set_loop(viewer_timeline *timeline, bool loop);
bool viewer_timeline_set_mode(viewer_timeline *timeline, viewer_timeline_mode mode);
bool viewer_timeline_set_speed(viewer_timeline *timeline, double speed);
bool viewer_timeline_zoom_at(viewer_timeline *timeline, double anchor_seconds,
                             double factor);
bool viewer_timeline_pan(viewer_timeline *timeline, double delta_seconds);
bool viewer_timeline_fit(viewer_timeline *timeline);
bool viewer_timeline_dock_set_open(viewer_timeline *timeline, bool open);
bool viewer_timeline_dock_resize(viewer_timeline *timeline, int32_t delta_points);
void viewer_timeline_set_reduced_motion(viewer_timeline *timeline, bool reduced_motion);
void viewer_timeline_sync_selection(viewer_timeline *timeline, viewer_editor *editor);
uint32_t viewer_timeline_key_hit_radius(double backing_scale);
double viewer_timeline_view_duration(const viewer_timeline *timeline);
bool viewer_timeline_toggle_play(viewer_timeline *timeline, double now_seconds);
bool viewer_timeline_advance(viewer_timeline *timeline, double now_seconds);
bool viewer_timeline_tick(viewer_timeline *timeline);
bool viewer_timeline_handle_key(viewer_timeline *timeline, uint16_t key_code,
                                const char *text, size_t text_length,
                                double now_seconds);
bool viewer_timeline_layout_compute(const viewer_timeline *timeline,
                                    uint32_t width, uint32_t height,
                                    double backing_scale,
                                    viewer_timeline_layout *out_layout);
bool viewer_timeline_pointer(viewer_timeline *timeline,
                             const viewer_timeline_layout *layout,
                             viewer_timeline_pointer_kind kind,
                             double x, double y, double delta_y,
                             double now_seconds);
bool viewer_timeline_pointer_ex(viewer_timeline *timeline,
                                const viewer_timeline_layout *layout,
                                viewer_timeline_pointer_kind kind,
                                double x, double y, double delta_x, double delta_y,
                                uint32_t modifiers, double now_seconds,
                                viewer_editor *editor);
bool viewer_timeline_process_event(viewer_timeline *timeline,
                                   const platform_event *event,
                                   viewer_editor *editor,
                                   uint32_t width, uint32_t height,
                                   double backing_scale);
const raster_tile *viewer_timeline_prepare_frame(viewer_timeline *timeline);
bool viewer_timeline_paint(viewer_timeline *timeline, viewer_editor *editor,
                           uint8_t *rgba, size_t capacity,
                           uint32_t width, uint32_t height, size_t stride,
                           double backing_scale);
bool viewer_timeline_paint_appearance(viewer_timeline *timeline,
                                      viewer_editor *editor, uint8_t *rgba,
                                      size_t capacity, uint32_t width,
                                      uint32_t height, size_t stride,
                                      double backing_scale, ui_theme theme,
                                      bool increased_contrast,
                                      bool reduced_motion);
bool viewer_timeline_paint_scalar_appearance(viewer_timeline *timeline,
                                             viewer_editor *editor,
                                             uint8_t *rgba, size_t capacity,
                                             uint32_t width, uint32_t height,
                                             size_t stride, double backing_scale,
                                             ui_theme theme,
                                             bool increased_contrast,
                                             bool reduced_motion);
bool viewer_timeline_accessibility_snapshot(viewer_timeline *timeline,
    const viewer_timeline_layout *layout, ui_theme theme,
    bool increased_contrast, ui_semantics_snapshot *out_snapshot);

#endif
