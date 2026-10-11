#ifndef VEXEL_APP_VIEWER_CHROME_H
#define VEXEL_APP_VIEWER_CHROME_H

#include "ui/ui_core.h"

#include <stdbool.h>
#include <stdint.h>

#define VIEWER_CHROME_TOOLBAR_POINTS 40U
#define VIEWER_CHROME_OPTIONS_POINTS 32U
#define VIEWER_CHROME_STATUS_POINTS 24U
#define VIEWER_CHROME_RAIL_POINTS 44U
#define VIEWER_CHROME_STRUCTURE_POINTS 232U
#define VIEWER_CHROME_INSPECTOR_POINTS 320U
#define VIEWER_CHROME_CANVAS_MIN_POINTS 240U

typedef struct viewer_chrome_layout {
    ui_rect toolbar;
    ui_rect options;
    ui_rect workspace;
    ui_rect rail;
    ui_rect structure;
    ui_rect canvas;
    ui_rect timeline;
    ui_rect inspector;
    ui_rect status;
    bool structure_visible;
    bool inspector_visible;
} viewer_chrome_layout;

/* Inputs and returned bounds use backing pixels. The calculation is allocation-free. */
bool viewer_chrome_layout_compute(uint32_t width, uint32_t height,
                                  double backing_scale,
                                  bool want_structure, bool want_inspector,
                                  viewer_chrome_layout *out_layout);
bool viewer_chrome_layout_compute_docked(uint32_t width, uint32_t height,
    double backing_scale, bool want_structure, bool want_inspector,
    bool timeline_visible, uint32_t timeline_height_points,
    viewer_chrome_layout *out_layout);

#endif
