#ifndef VEXEL_UI_VIEWER_CONTROLS_H
#define VEXEL_UI_VIEWER_CONTROLS_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define UI_VIEWER_MIN_SCALE 0.001
#define UI_VIEWER_MAX_SCALE 1000.0
#define UI_VIEWER_LABEL_CAP 96U

typedef struct ui_viewer_navigation {
    double viewport_width, viewport_height;
    double document_width, document_height;
    double scale, pan_x, pan_y;
    double drag_x, drag_y;
    bool dragging;
} ui_viewer_navigation;

typedef enum ui_viewer_key {
    UI_VIEWER_KEY_LEFT, UI_VIEWER_KEY_RIGHT, UI_VIEWER_KEY_UP, UI_VIEWER_KEY_DOWN,
    UI_VIEWER_KEY_ZOOM_IN, UI_VIEWER_KEY_ZOOM_OUT, UI_VIEWER_KEY_FIT
} ui_viewer_key;

typedef enum ui_progress_phase {
    UI_PROGRESS_IDLE, UI_PROGRESS_ACTIVE, UI_PROGRESS_COMPLETE,
    UI_PROGRESS_CANCELED, UI_PROGRESS_FAILED
} ui_progress_phase;

typedef struct ui_viewer_progress {
    uint64_t generation;
    unsigned percent;
    ui_progress_phase phase;
    bool cancel_requested;
    char label[UI_VIEWER_LABEL_CAP];
} ui_viewer_progress;

typedef struct ui_semantic_node {
    char id[24], role[24], name[UI_VIEWER_LABEL_CAP];
    int32_t x, y, width, height;
    unsigned value;
    bool enabled;
} ui_semantic_node;

bool ui_viewer_navigation_init(ui_viewer_navigation *state, double viewport_width,
                               double viewport_height, double document_width,
                               double document_height);
bool ui_viewer_navigation_resize(ui_viewer_navigation *state, double width, double height);
/* Navigation state uses backing pixels. Convert AppKit logical coordinates at
 * the platform boundary with its current backing scale. */
bool ui_viewer_navigation_resize_scaled(ui_viewer_navigation *state, double logical_width,
                                        double logical_height, double backing_scale);
bool ui_viewer_navigation_drag_begin(ui_viewer_navigation *state, double x, double y);
bool ui_viewer_navigation_drag(ui_viewer_navigation *state, double x, double y);
bool ui_viewer_navigation_drag_end(ui_viewer_navigation *state);
bool ui_viewer_navigation_drag_begin_scaled(ui_viewer_navigation *state, double logical_x,
                                            double logical_y, double backing_scale);
bool ui_viewer_navigation_drag_scaled(ui_viewer_navigation *state, double logical_x,
                                       double logical_y, double backing_scale);
bool ui_viewer_navigation_zoom(ui_viewer_navigation *state, double factor,
                               double cursor_x, double cursor_y);
bool ui_viewer_navigation_wheel(ui_viewer_navigation *state, double delta_y,
                                double cursor_x, double cursor_y);
bool ui_viewer_navigation_wheel_scaled(ui_viewer_navigation *state, double delta_y,
                                       double logical_x, double logical_y,
                                       double backing_scale);
bool ui_viewer_navigation_pinch(ui_viewer_navigation *state, double magnification,
                                double cursor_x, double cursor_y);
bool ui_viewer_navigation_pinch_scaled(ui_viewer_navigation *state, double magnification,
                                       double logical_x, double logical_y,
                                       double backing_scale);
bool ui_viewer_navigation_fit(ui_viewer_navigation *state);
bool ui_viewer_navigation_fit_width(ui_viewer_navigation *state);
bool ui_viewer_navigation_actual_size(ui_viewer_navigation *state);
bool ui_viewer_navigation_key(ui_viewer_navigation *state, ui_viewer_key key);
bool ui_viewer_navigation_project(const ui_viewer_navigation *state, double world_x,
                                  double world_y, double *screen_x, double *screen_y);
bool ui_viewer_progress_update(ui_viewer_progress *state, uint64_t generation,
                               unsigned percent, ui_progress_phase phase,
                               const char *label, size_t label_len);
bool ui_viewer_progress_cancel(ui_viewer_progress *state, bool *out_request);
size_t ui_viewer_semantics(const ui_viewer_navigation *navigation,
                           const ui_viewer_progress *progress, ui_semantic_node *out,
                           size_t capacity);
bool ui_viewer_overlay_rgba(const ui_viewer_progress *progress, uint8_t *pixels,
                            size_t length, uint32_t width, uint32_t height,
                            size_t stride);

#endif
