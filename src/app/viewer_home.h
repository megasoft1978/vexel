#ifndef VEXEL_APP_VIEWER_HOME_H
#define VEXEL_APP_VIEWER_HOME_H

#include "ui/ui_core.h"

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define VIEWER_HOME_RECENT_MAX 5U
#define VIEWER_HOME_RECENT_LABEL_CAP 96U

typedef struct viewer_home_state {
    bool has_document;
    bool loading;
    bool load_failed;
    bool drop_hover;
    const char *load_error;
    const char *recent_labels[VIEWER_HOME_RECENT_MAX];
    size_t recent_count;
    ui_rect canvas;
} viewer_home_state;

typedef struct viewer_home_layout {
    ui_rect notice;
    ui_rect open_button;
    ui_rect retry_button;
    ui_rect recent_rows[VIEWER_HOME_RECENT_MAX];
    ui_rect drop_zone;
    size_t recent_count;
} viewer_home_layout;

typedef enum viewer_home_action_kind {
    VIEWER_HOME_ACTION_NONE,
    VIEWER_HOME_ACTION_OPEN,
    VIEWER_HOME_ACTION_RETRY,
    VIEWER_HOME_ACTION_RECENT
} viewer_home_action_kind;

typedef struct viewer_home_action {
    viewer_home_action_kind kind;
    size_t recent_index;
} viewer_home_action;

bool viewer_home_layout_get(const viewer_home_state *state, uint32_t width,
                            uint32_t height, double backing_scale,
                            viewer_home_layout *out_layout);
viewer_home_action viewer_home_pointer_down(const viewer_home_state *state,
    uint32_t width, uint32_t height, double backing_scale, uint32_t x, uint32_t y);
void viewer_home_paint(const viewer_home_state *state, ui_theme theme,
    uint8_t *rgba, size_t capacity, size_t stride, uint32_t width, uint32_t height,
    double backing_scale);

#endif
