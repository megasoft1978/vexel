#ifndef VEXEL_APP_VIEWER_TIMELINE_H
#define VEXEL_APP_VIEWER_TIMELINE_H

#include "platform/platform_window.h"
#include "raster/tile.h"
#include "shape.h"
#include "ui/ui_core.h"
#include "ui/ui_widgets.h"
#include "viewer_editor.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define VIEWER_TIMELINE_VISIBLE_TRACK_CAP 5U

typedef struct viewer_timeline {
    const svg_shape_document *document; /* borrowed while the source document lives */
    raster_tile preview;                /* owned cached animation sample */
    double duration_seconds;
    double time_seconds;
    double preview_time_seconds;
    double last_tick_seconds;
    double last_frame_tick_seconds;
    size_t first_track;
    size_t selected_track;
    size_t selected_key;
    size_t easing_focused_field;
    size_t easing_drag_handle;
    size_t easing_drag_field;
    size_t dragging_key_track;
    size_t dragging_key_index;
    ui_widget_number_field easing_fields[4];
    svg_anim_calc_mode easing_mode;
    bool playing;
    bool loop;
    bool dragging_scrubber;
    bool preview_valid;
    bool preview_failed;
    bool has_tick_time;
    bool easing_initialized;
    bool easing_has_focus;
    bool easing_graph_focused;
    bool easing_pending;
    bool easing_mode_override;
    bool easing_graph_dragging;
    bool easing_field_dragging;
    bool dragging_key;
    bool edit_mutation_pending;
} viewer_timeline;

typedef struct viewer_timeline_layout {
    double backing_scale;
    ui_rect panel;
    ui_rect play_button;
    ui_rect time_readout;
    ui_rect loop_toggle;
    ui_rect scrubber;
    ui_rect track_rows[VIEWER_TIMELINE_VISIBLE_TRACK_CAP];
    size_t visible_track_count;
    size_t track_count;
} viewer_timeline_layout;

typedef struct viewer_timeline_easing_layout {
    ui_rect mode_buttons[3];
    ui_rect graph;
    ui_rect fields[4];
    ui_rect presets[4];
    bool visible;
} viewer_timeline_easing_layout;

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
bool viewer_timeline_accessibility_snapshot(viewer_timeline *timeline,
    const viewer_timeline_layout *layout, ui_theme theme,
    bool increased_contrast, ui_semantics_snapshot *out_snapshot);
bool viewer_timeline_easing_layout_compute(const viewer_timeline_layout *timeline_layout,
    viewer_timeline_easing_layout *out_layout);
bool viewer_timeline_easing_enabled(const viewer_timeline *timeline);
void viewer_timeline_easing_sync(viewer_timeline *timeline);
void viewer_timeline_easing_paint(viewer_timeline *timeline,
    const viewer_timeline_layout *timeline_layout,
    const viewer_timeline_easing_layout *easing_layout,
    ui_widget_canvas *canvas);
void viewer_timeline_easing_paint_key_markers(const viewer_timeline *timeline,
    const viewer_timeline_layout *layout, size_t track_index, size_t row,
    ui_widget_canvas *canvas);
bool viewer_timeline_easing_process_event(viewer_timeline *timeline,
    viewer_editor *editor, const platform_event *event,
    const viewer_timeline_layout *layout);
bool viewer_timeline_take_editor_mutation(viewer_timeline *timeline);

#endif
