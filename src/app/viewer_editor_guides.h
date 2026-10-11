#ifndef VEXEL_APP_VIEWER_EDITOR_GUIDES_H
#define VEXEL_APP_VIEWER_EDITOR_GUIDES_H

#include <stdbool.h>
#include <stddef.h>

#define VIEWER_EDITOR_GUIDE_CAP_PER_AXIS 128U
#define VIEWER_EDITOR_GUIDE_TICK_CAP 256U
#define VIEWER_EDITOR_RULER_TOP_POINTS 26U
#define VIEWER_EDITOR_RULER_LEFT_POINTS 36U

typedef enum viewer_editor_guide_axis_kind {
    VIEWER_EDITOR_GUIDE_HORIZONTAL = 0,
    VIEWER_EDITOR_GUIDE_VERTICAL = 1
} viewer_editor_guide_axis_kind;

typedef struct viewer_editor_guide_axis {
    double values[VIEWER_EDITOR_GUIDE_CAP_PER_AXIS];
    size_t count;
} viewer_editor_guide_axis;

typedef struct viewer_editor_guides {
    viewer_editor_guide_axis horizontal;
    viewer_editor_guide_axis vertical;
    viewer_editor_guide_axis_kind selected_axis;
    viewer_editor_guide_axis_kind drag_axis;
    viewer_editor_guide_axis_kind context_axis;
    size_t selected_index;
    size_t drag_index;
    double drag_value;
    double drag_start_x;
    double drag_start_y;
    bool rulers_visible;
    bool guides_visible;
    bool locked;
    bool selected;
    bool dragging;
    bool drag_creating;
    bool drag_moved;
    bool drag_value_valid;
    bool context_open;
} viewer_editor_guides;

void viewer_editor_guides_init(viewer_editor_guides *guides);
size_t viewer_editor_guides_count(const viewer_editor_guides *guides,
                                  viewer_editor_guide_axis_kind axis);
bool viewer_editor_guides_get(const viewer_editor_guides *guides,
    viewer_editor_guide_axis_kind axis, size_t index, double *out_value);
bool viewer_editor_guides_add(viewer_editor_guides *guides,
    viewer_editor_guide_axis_kind axis, double value);
bool viewer_editor_guides_move(viewer_editor_guides *guides,
    viewer_editor_guide_axis_kind axis, size_t index, double value);
bool viewer_editor_guides_remove(viewer_editor_guides *guides,
    viewer_editor_guide_axis_kind axis, size_t index);
bool viewer_editor_guides_clear(viewer_editor_guides *guides,
    viewer_editor_guide_axis_kind axis);
bool viewer_editor_guides_delete_selected(viewer_editor_guides *guides);
bool viewer_editor_guides_nearest(const viewer_editor_guides *guides,
    viewer_editor_guide_axis_kind axis, double value, double tolerance,
    double *out_value, size_t *out_index);
bool viewer_editor_guides_tick_step(double pixels_per_unit, double backing_scale,
    double desired_spacing_points, double *out_step);

#endif
