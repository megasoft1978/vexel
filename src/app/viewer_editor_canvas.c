#include "viewer_editor_internal.h"

#include "document_model.h"
#include "viewer_chrome.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct canvas_view_transform {
    double scale_x;
    double scale_y;
    double offset_x;
    double offset_y;
} canvas_view_transform;

static bool canvas_view_transform_get(const svg_shape_document *document,
                                      canvas_view_transform *out)
{
    if (document == NULL || out == NULL || document->width == 0U ||
        document->height == 0U || !isfinite(document->view_x) ||
        !isfinite(document->view_y) || !isfinite(document->view_width) ||
        !isfinite(document->view_height) || !(document->view_width > 0.0) ||
        !(document->view_height > 0.0)) return false;
    double sx = (double)document->width / document->view_width;
    double sy = (double)document->height / document->view_height;
    double ox = 0.0, oy = 0.0;
    svg_shape_aspect aspect = document->preserve_aspect;
    if (!aspect.set) aspect = (svg_shape_aspect){0.5, 0.5, false, false, true};
    if (!aspect.none) {
        double scale = aspect.slice ? fmax(sx, sy) : fmin(sx, sy);
        sx = scale;
        sy = scale;
        ox = ((double)document->width - document->view_width * scale) *
            aspect.align_x;
        oy = ((double)document->height - document->view_height * scale) *
            aspect.align_y;
    }
    if (!isfinite(sx) || !isfinite(sy) || !(sx > 0.0) || !(sy > 0.0) ||
        !isfinite(ox) || !isfinite(oy)) return false;
    *out = (canvas_view_transform){sx, sy, ox, oy};
    return true;
}

static bool canvas_user_at(const svg_shape_document *document,
                           const ui_viewer_navigation *navigation,
                           double x, double y, geom_point *out)
{
    canvas_view_transform view;
    if (navigation == NULL || out == NULL || !isfinite(x) || !isfinite(y) ||
        !isfinite(navigation->scale) || !(navigation->scale > 0.0) ||
        !canvas_view_transform_get(document, &view)) return false;
    double device_x = (x - navigation->pan_x) / navigation->scale;
    double device_y = (y - navigation->pan_y) / navigation->scale;
    double user_x = document->view_x + (device_x - view.offset_x) / view.scale_x;
    double user_y = document->view_y + (device_y - view.offset_y) / view.scale_y;
    if (!isfinite(user_x) || !isfinite(user_y) || fabs(user_x) > 1000000.0 ||
        fabs(user_y) > 1000000.0) return false;
    *out = (geom_point){user_x, user_y};
    return true;
}

static bool canvas_device_at(const svg_shape_document *document, geom_point user,
                             geom_point *out)
{
    canvas_view_transform view;
    if (out == NULL || !canvas_view_transform_get(document, &view)) return false;
    double x = view.offset_x + (user.x - document->view_x) * view.scale_x;
    double y = view.offset_y + (user.y - document->view_y) * view.scale_y;
    if (!isfinite(x) || !isfinite(y)) return false;
    *out = (geom_point){x, y};
    return true;
}

static bool editor_selected_shape(viewer_editor *editor,
                                  const svg_shape_document *document,
                                  svg_shape_selection *out)
{
    document_source_span span;
    return editor != NULL && document != NULL && out != NULL &&
        viewer_editor_selection_span(editor, &span) &&
        svg_shape_element_at_offset(document, span.offset, out);
}

static bool rect_valid(svg_shape_rect rect)
{
    return isfinite(rect.x) && isfinite(rect.y) && isfinite(rect.width) &&
        isfinite(rect.height) && rect.width >= 0.0 && rect.height >= 0.0;
}

static void editor_reset_canvas_gesture(viewer_editor *editor)
{
    editor->canvas_gesture = EDITOR_CANVAS_GESTURE_NONE;
    editor->canvas_gesture_moved = false;
    editor->gesture_rotation = 0.0;
    editor->gesture_scale_x = 1.0;
    editor->gesture_scale_y = 1.0;
    editor->gesture_current_x = editor->gesture_start_x;
    editor->gesture_current_y = editor->gesture_start_y;
}

static double handle_distance(double x, double y, geom_point point)
{
    return hypot(x - point.x, y - point.y);
}

static size_t selection_handle_at(const viewer_editor *editor,
                                  const svg_shape_selection *selection,
                                  const ui_viewer_navigation *navigation,
                                  double x, double y)
{
    if (editor == NULL || selection == NULL || navigation == NULL ||
        !rect_valid(selection->device_bounds) || !isfinite(navigation->scale) ||
        !(navigation->scale > 0.0)) return SIZE_MAX;
    double left = navigation->pan_x + selection->device_bounds.x * navigation->scale;
    double top = navigation->pan_y + selection->device_bounds.y * navigation->scale;
    double right = left + selection->device_bounds.width * navigation->scale;
    double bottom = top + selection->device_bounds.height * navigation->scale;
    double middle_x = (left + right) * 0.5;
    double middle_y = (top + bottom) * 0.5;
    geom_point handles[9] = {
        {left, top}, {middle_x, top}, {right, top}, {right, middle_y},
        {right, bottom}, {middle_x, bottom}, {left, bottom}, {left, middle_y},
        {middle_x, top - 22.0 * editor->backing_scale}
    };
    double radius = 4.0 * editor->backing_scale;
    for (size_t handle = 0U; handle < 9U; ++handle)
        if (handle_distance(x, y, handles[handle]) <= radius) return handle;
    return SIZE_MAX;
}

static bool editor_hit_canvas_shape(viewer_editor *editor,
                                    const svg_shape_document *document,
                                    const ui_viewer_navigation *navigation,
                                    double x, double y,
                                    svg_shape_selection *out_selection,
                                    geom_point *out_user)
{
    canvas_view_transform view;
    geom_point user;
    if (!canvas_view_transform_get(document, &view) ||
        !canvas_user_at(document, navigation, x, y, &user)) return false;
    double pixel_scale = navigation->scale * fmin(view.scale_x, view.scale_y);
    if (!isfinite(pixel_scale) || !(pixel_scale > 0.0)) return false;
    double tolerance = 4.0 * editor->backing_scale / pixel_scale;
    if (!isfinite(tolerance) || tolerance > 1000000.0) tolerance = 1000000.0;
    if (!svg_shape_hit_test(document, user.x, user.y, tolerance, out_selection))
        return false;
    *out_user = user;
    return true;
}

static void editor_begin_gesture(viewer_editor *editor,
                                 editor_canvas_gesture_kind kind,
                                 const svg_shape_document *document,
                                 const ui_viewer_navigation *navigation,
                                 geom_point user, double x, double y)
{
    geom_point device = {0.0, 0.0};
    (void)canvas_device_at(document, user, &device);
    editor->canvas_gesture = kind;
    editor->canvas_gesture_moved = false;
    editor->gesture_start_x = user.x;
    editor->gesture_start_y = user.y;
    editor->gesture_current_x = user.x;
    editor->gesture_current_y = user.y;
    editor->gesture_start_device_x = device.x;
    editor->gesture_start_device_y = device.y;
    editor->gesture_scale_x = 1.0;
    editor->gesture_scale_y = 1.0;
    editor->gesture_rotation = 0.0;
    editor->gesture_pivot_x = user.x;
    editor->gesture_pivot_y = user.y;
    editor->resize_handle = 0U;
    editor->snap_guide_x_visible = false;
    editor->snap_guide_y_visible = false;
    editor->snap_bypass = false;
    if (kind == EDITOR_CANVAS_GESTURE_MOVE) {
        editor->canvas_preview_bounds = editor->canvas_selection.device_bounds;
    } else if (kind == EDITOR_CANVAS_GESTURE_DRAW ||
               kind == EDITOR_CANVAS_GESTURE_MARQUEE) {
        editor->canvas_preview_bounds = (svg_shape_rect){device.x, device.y, 0.0, 0.0};
    }
    (void)navigation;
    (void)x;
    (void)y;
}

static bool canvas_update_selection_preview(viewer_editor *editor,
    const svg_shape_document *document,
    const ui_viewer_navigation *navigation, geom_point user)
{
    geom_point device;
    if (!canvas_device_at(document, user, &device)) return false;
    if (editor->canvas_gesture == EDITOR_CANVAS_GESTURE_MOVE ||
        editor->canvas_gesture == EDITOR_CANVAS_GESTURE_RESIZE) {
        canvas_view_transform view;
        if (canvas_view_transform_get(document, &view)) {
            double scale = navigation->scale * fmin(view.scale_x, view.scale_y);
            double tolerance = scale > 0.0 ?
                6.0 * editor->backing_scale / scale : 0.0;
            if (editor->canvas_gesture == EDITOR_CANVAS_GESTURE_MOVE &&
                tolerance > 0.0) {
                double dx = user.x - editor->gesture_start_x;
                double dy = user.y - editor->gesture_start_y;
                editor_canvas_snap_translation(editor, document, tolerance,
                    &dx, &dy);
                user.x = editor->gesture_start_x + dx;
                user.y = editor->gesture_start_y + dy;
            } else if (editor->canvas_gesture == EDITOR_CANVAS_GESTURE_RESIZE &&
                       tolerance > 0.0) {
                user = editor_canvas_snap_point(editor, document, user, tolerance);
            }
            if (!canvas_device_at(document, user, &device)) return false;
        }
    } else {
        editor->snap_guide_x_visible = false;
        editor->snap_guide_y_visible = false;
    }
    editor->gesture_current_x = user.x;
    editor->gesture_current_y = user.y;
    double screen_dx = (device.x - editor->gesture_start_device_x) * navigation->scale;
    double screen_dy = (device.y - editor->gesture_start_device_y) * navigation->scale;
    double threshold = 2.0 * editor->backing_scale;
    editor->canvas_gesture_moved = editor->canvas_gesture_moved ||
        fabs(screen_dx) > threshold || fabs(screen_dy) > threshold;
    if (editor->canvas_gesture == EDITOR_CANVAS_GESTURE_DRAW ||
        editor->canvas_gesture == EDITOR_CANVAS_GESTURE_MARQUEE) {
        editor->canvas_preview_bounds = (svg_shape_rect){
            fmin(editor->gesture_start_device_x, device.x),
            fmin(editor->gesture_start_device_y, device.y),
            fabs(device.x - editor->gesture_start_device_x),
            fabs(device.y - editor->gesture_start_device_y)};
        return rect_valid(editor->canvas_preview_bounds);
    }
    svg_shape_rect start = editor->canvas_selection.device_bounds;
    if (editor->canvas_gesture == EDITOR_CANVAS_GESTURE_MOVE) {
        editor->canvas_preview_bounds = (svg_shape_rect){
            start.x + device.x - editor->gesture_start_device_x,
            start.y + device.y - editor->gesture_start_device_y,
            start.width, start.height};
        editor->gesture_pivot_x = user.x - editor->gesture_start_x;
        editor->gesture_pivot_y = user.y - editor->gesture_start_y;
        return rect_valid(editor->canvas_preview_bounds);
    }
    if (editor->canvas_gesture == EDITOR_CANVAS_GESTURE_RESIZE) {
        svg_shape_rect original = editor->canvas_selection.user_bounds;
        if (!(original.width > 0.0) || !(original.height > 0.0)) return false;
        double left = original.x, right = original.x + original.width;
        double top = original.y, bottom = original.y + original.height;
        double fixed_x = (left + right) * 0.5;
        double fixed_y = (top + bottom) * 0.5;
        double current_x = user.x, current_y = user.y;
        switch (editor->resize_handle) {
            case 0U: fixed_x = right; fixed_y = bottom; break;
            case 1U: fixed_y = bottom; current_x = fixed_x; break;
            case 2U: fixed_x = left; fixed_y = bottom; break;
            case 3U: fixed_x = left; current_y = fixed_y; break;
            case 4U: fixed_x = left; fixed_y = top; break;
            case 5U: fixed_y = top; current_x = fixed_x; break;
            case 6U: fixed_x = right; fixed_y = top; break;
            case 7U: fixed_x = right; current_y = fixed_y; break;
            default: return false;
        }
        double new_left = editor->resize_handle == 1U || editor->resize_handle == 5U ?
            left : fmin(fixed_x, current_x);
        double new_right = editor->resize_handle == 1U || editor->resize_handle == 5U ?
            right : fmax(fixed_x, current_x);
        double new_top = editor->resize_handle == 3U || editor->resize_handle == 7U ?
            top : fmin(fixed_y, current_y);
        double new_bottom = editor->resize_handle == 3U || editor->resize_handle == 7U ?
            bottom : fmax(fixed_y, current_y);
        if (editor->resize_handle == 1U || editor->resize_handle == 5U) {
            new_left = left; new_right = right;
        }
        if (editor->resize_handle == 3U || editor->resize_handle == 7U) {
            new_top = top; new_bottom = bottom;
        }
        double width = new_right - new_left;
        double height = new_bottom - new_top;
        if (!(width > 0.01) || !(height > 0.01)) return false;
        editor->gesture_scale_x = width / original.width;
        editor->gesture_scale_y = height / original.height;
        if (editor->gesture_scale_x < 0.01) editor->gesture_scale_x = 0.01;
        if (editor->gesture_scale_y < 0.01) editor->gesture_scale_y = 0.01;
        editor->gesture_pivot_x = fixed_x;
        editor->gesture_pivot_y = fixed_y;
        geom_point pivot_device;
        if (!canvas_device_at(document, (geom_point){fixed_x, fixed_y}, &pivot_device))
            return false;
        double x0 = pivot_device.x + (start.x - pivot_device.x) * editor->gesture_scale_x;
        double y0 = pivot_device.y + (start.y - pivot_device.y) * editor->gesture_scale_y;
        double x1 = pivot_device.x + (start.x + start.width - pivot_device.x) *
            editor->gesture_scale_x;
        double y1 = pivot_device.y + (start.y + start.height - pivot_device.y) *
            editor->gesture_scale_y;
        editor->canvas_preview_bounds = (svg_shape_rect){fmin(x0, x1), fmin(y0, y1),
            fabs(x1 - x0), fabs(y1 - y0)};
        return rect_valid(editor->canvas_preview_bounds);
    }
    if (editor->canvas_gesture == EDITOR_CANVAS_GESTURE_ROTATE) {
        double center_x = editor->canvas_selection.user_bounds.x +
            editor->canvas_selection.user_bounds.width * 0.5;
        double center_y = editor->canvas_selection.user_bounds.y +
            editor->canvas_selection.user_bounds.height * 0.5;
        double start_angle = atan2(editor->gesture_start_y - center_y,
                                   editor->gesture_start_x - center_x);
        double current_angle = atan2(user.y - center_y, user.x - center_x);
        double angle = current_angle - start_angle;
        if (!isfinite(angle)) return false;
        editor->gesture_rotation = angle;
        editor->gesture_pivot_x = center_x;
        editor->gesture_pivot_y = center_y;
        geom_point pivot_device;
        if (!canvas_device_at(document, (geom_point){center_x, center_y}, &pivot_device))
            return false;
        double c = cos(angle), s = sin(angle);
        double min_x = INFINITY, min_y = INFINITY;
        double max_x = -INFINITY, max_y = -INFINITY;
        geom_point corners[4] = {
            {start.x, start.y}, {start.x + start.width, start.y},
            {start.x + start.width, start.y + start.height},
            {start.x, start.y + start.height}
        };
        for (size_t index = 0U; index < 4U; ++index) {
            double dx = corners[index].x - pivot_device.x;
            double dy = corners[index].y - pivot_device.y;
            double x = pivot_device.x + dx * c - dy * s;
            double y = pivot_device.y + dx * s + dy * c;
            min_x = fmin(min_x, x); min_y = fmin(min_y, y);
            max_x = fmax(max_x, x); max_y = fmax(max_y, y);
        }
        editor->canvas_preview_bounds = (svg_shape_rect){min_x, min_y,
            max_x - min_x, max_y - min_y};
        return rect_valid(editor->canvas_preview_bounds);
    }
    return false;
}


uint32_t viewer_editor_canvas_pointer_down(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    double x, double y, uint32_t modifiers)
{
    return viewer_editor_canvas_pointer_down_click_count(editor, document,
        navigation, x, y, modifiers, 1U);
}

uint32_t viewer_editor_canvas_pointer_down_click_count(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    double x, double y, uint32_t modifiers, uint8_t click_count)
{
    if (editor == NULL || document == NULL || navigation == NULL ||
        !isfinite(x) || !isfinite(y)) return 0U;
    uint32_t prior = 0U;
    if (editor->field_kind != EDITOR_FIELD_NONE) {
        prior = editor_commit_field(editor);
        if (editor->field_kind != EDITOR_FIELD_NONE)
            return prior | VIEWER_EDITOR_HANDLED;
    }
    geom_point user;
    if (!canvas_user_at(document, navigation, x, y, &user)) return prior;
    if (editor->draw_tool == VIEWER_EDITOR_TOOL_TEXT)
        return prior | editor_canvas_text_place(editor, document, user);
    if (editor->draw_tool == VIEWER_EDITOR_TOOL_PEN) {
        if (editor->node_edit_active) {
            canvas_view_transform view;
            double tolerance = 0.0;
            if (canvas_view_transform_get(document, &view)) {
                double scale = navigation->scale * fmin(view.scale_x, view.scale_y);
                if (isfinite(scale) && scale > 0.0)
                    tolerance = 8.0 * editor->backing_scale / scale;
            }
            if (tolerance > 0.0 && editor_canvas_path_node_begin(editor,
                    document, user, tolerance)) {
                editor->redraw_requested = true;
                return prior | VIEWER_EDITOR_HANDLED;
            }
            if (click_count >= 2U && tolerance > 0.0) {
                uint32_t inserted = editor_canvas_path_add_node(editor, document,
                    user, tolerance);
                if (inserted != 0U) return prior | inserted;
            }
            return prior | VIEWER_EDITOR_HANDLED;
        }
    }
    if (editor->draw_tool == VIEWER_EDITOR_TOOL_POLYGON ||
        editor->draw_tool == VIEWER_EDITOR_TOOL_POLYLINE ||
        editor->draw_tool == VIEWER_EDITOR_TOOL_PEN)
        return prior | editor_canvas_path_pointer_down(editor, user, x, y,
            click_count == 0U ? 1U : click_count);
    editor_reset_canvas_gesture(editor);
    if (editor->draw_tool != VIEWER_EDITOR_TOOL_SELECT) {
        editor_begin_gesture(editor, EDITOR_CANVAS_GESTURE_DRAW,
            document, navigation, user, x, y);
        editor->redraw_requested = true;
        return prior | VIEWER_EDITOR_HANDLED;
    }
    if (editor->canvas_selection_count > 0U) {
        svg_shape_selection current_selection;
        if (editor_canvas_selection_bounds(editor, document, &current_selection) ||
            editor_selected_shape(editor, document, &current_selection)) {
            editor->canvas_selection = current_selection;
            editor->canvas_selection_valid = true;
            size_t handle = selection_handle_at(editor, &editor->canvas_selection,
                navigation, x, y);
            if (handle < 9U) {
                editor_begin_gesture(editor,
                    handle == 8U ? EDITOR_CANVAS_GESTURE_ROTATE :
                        EDITOR_CANVAS_GESTURE_RESIZE,
                    document, navigation, user, x, y);
                editor->resize_handle = (uint8_t)handle;
                editor->canvas_preview_bounds = editor->canvas_selection.device_bounds;
                editor->redraw_requested = true;
                return prior | VIEWER_EDITOR_HANDLED;
            }
        }
    }
    svg_shape_selection selection;
    geom_point hit_user;
    if (!editor_hit_canvas_shape(editor, document, navigation, x, y,
            &selection, &hit_user)) {
        bool shift = (modifiers & PLATFORM_MODIFIER_SHIFT) != 0U;
        if (!shift) editor_canvas_selection_clear(editor);
        editor->marquee_preserve_selection = shift && editor->canvas_selection_valid;
        editor_begin_gesture(editor, EDITOR_CANVAS_GESTURE_MARQUEE,
            document, navigation, user, x, y);
        editor->redraw_requested = true;
        return prior | VIEWER_EDITOR_HANDLED;
    }
    if ((modifiers & PLATFORM_MODIFIER_SHIFT) != 0U) {
        if (!editor->canvas_selection_valid) editor_canvas_selection_clear(editor);
        (void)editor_canvas_selection_toggle(editor, selection.source_offset);
        editor_status_ok(editor, editor->canvas_selection_count > 1U ?
            "Elements selected" : "Element selected");
        return prior | VIEWER_EDITOR_HANDLED;
    }
    if (!editor_canvas_selection_contains(editor, selection.source_offset) ||
        !editor->canvas_selection_valid) {
        if (!viewer_editor_select_at_source_offset(editor, selection.source_offset))
            return prior | VIEWER_EDITOR_HANDLED;
    }
    svg_shape_selection selected_bounds;
    editor->canvas_selection = editor_canvas_selection_bounds(editor, document,
        &selected_bounds) ? selected_bounds : selection;
    editor->canvas_selection_valid = true;
    editor_begin_gesture(editor, EDITOR_CANVAS_GESTURE_MOVE,
        document, navigation, hit_user, x, y);
    editor->canvas_preview_bounds = editor->canvas_selection.device_bounds;
    editor_status_ok(editor, "Selected canvas element");
    editor->redraw_requested = true;
    return prior | VIEWER_EDITOR_HANDLED;
}

uint32_t viewer_editor_canvas_pointer_move(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    double x, double y)
{
    return viewer_editor_canvas_pointer_move_modifiers(editor, document,
        navigation, x, y, 0U);
}

uint32_t viewer_editor_canvas_pointer_move_modifiers(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    double x, double y, uint32_t modifiers)
{
    if (editor == NULL || document == NULL || navigation == NULL ||
        !isfinite(x) || !isfinite(y)) return 0U;
    editor->context_pointer_valid = true;
    editor->context_pointer_x = x;
    editor->context_pointer_y = y;
    editor->snap_bypass = (modifiers & PLATFORM_MODIFIER_SHIFT) != 0U;
    if (editor->canvas_gesture == EDITOR_CANVAS_GESTURE_NONE &&
        !editor->node_edit_active) return 0U;
    geom_point user;
    if (!canvas_user_at(document, navigation, x, y, &user)) return 0U;
    if (editor->canvas_gesture == EDITOR_CANVAS_GESTURE_NONE) {
        if (!editor->node_edit_active) return 0U;
        canvas_view_transform view;
        if (!canvas_view_transform_get(document, &view)) return 0U;
        double scale = navigation->scale * fmin(view.scale_x, view.scale_y);
        if (!(scale > 0.0) || !isfinite(scale)) return 0U;
        double tolerance = 8.0 * editor->backing_scale / scale;
        return editor_canvas_path_node_hover(editor, document, user, tolerance);
    }
    if (editor->canvas_gesture == EDITOR_CANVAS_GESTURE_PATH)
        return editor_canvas_path_pointer_move(editor, user, x, y);
    if (editor->canvas_gesture == EDITOR_CANVAS_GESTURE_PATH_NODE)
        return editor_canvas_path_node_pointer_move(editor, document, user);
    if (!canvas_update_selection_preview(editor, document, navigation, user))
        return VIEWER_EDITOR_HANDLED;
    editor->redraw_requested = true;
    return VIEWER_EDITOR_HANDLED;
}

uint32_t viewer_editor_canvas_pointer_up(viewer_editor *editor,
    const svg_shape_document *document)
{
    if (editor == NULL || document == NULL) return 0U;
    editor_canvas_gesture_kind gesture = editor->canvas_gesture;
    bool moved = editor->canvas_gesture_moved;
    if (gesture == EDITOR_CANVAS_GESTURE_PATH)
        return editor_canvas_path_pointer_up(editor);
    if (gesture == EDITOR_CANVAS_GESTURE_PATH_NODE)
        return editor_canvas_path_node_pointer_up(editor, document);
    uint32_t result = VIEWER_EDITOR_HANDLED;
    if (gesture == EDITOR_CANVAS_GESTURE_DRAW && moved) {
        geom_point start = {editor->gesture_start_x, editor->gesture_start_y};
        geom_point end = {editor->gesture_current_x, editor->gesture_current_y};
        result = editor_insert_drawn_shape(editor, document, start, end);
    } else if (gesture == EDITOR_CANVAS_GESTURE_MARQUEE) {
        if (moved)
            (void)editor_canvas_selection_marquee(editor, document,
                editor->canvas_preview_bounds, editor->marquee_preserve_selection);
    } else if (moved && gesture == EDITOR_CANVAS_GESTURE_MOVE) {
        double dx = editor->gesture_current_x - editor->gesture_start_x;
        double dy = editor->gesture_current_y - editor->gesture_start_y;
        result = editor->canvas_selection_count > 1U ?
            editor_canvas_multi_transform(editor, dx, dy, 1.0, 1.0,
                0.0, 0.0, 0.0) :
            editor_commit_transform(editor, dx, dy, 1.0, 1.0, 0.0, 0.0, 0.0);
    } else if (moved && gesture == EDITOR_CANVAS_GESTURE_RESIZE) {
        result = editor->canvas_selection_count > 1U ?
            editor_canvas_multi_transform(editor, 0.0, 0.0,
                editor->gesture_scale_x, editor->gesture_scale_y, 0.0,
                editor->gesture_pivot_x, editor->gesture_pivot_y) :
            editor_commit_transform(editor, 0.0, 0.0,
                editor->gesture_scale_x, editor->gesture_scale_y, 0.0,
                editor->gesture_pivot_x, editor->gesture_pivot_y);
    } else if (moved && gesture == EDITOR_CANVAS_GESTURE_ROTATE) {
        result = editor->canvas_selection_count > 1U ?
            editor_canvas_multi_transform(editor, 0.0, 0.0, 1.0, 1.0,
                editor->gesture_rotation, editor->gesture_pivot_x,
                editor->gesture_pivot_y) :
            editor_commit_transform(editor, 0.0, 0.0, 1.0, 1.0,
                editor->gesture_rotation, editor->gesture_pivot_x,
                editor->gesture_pivot_y);
    }
    editor_reset_canvas_gesture(editor);
    editor->snap_bypass = false;
    editor_canvas_snap_clear(editor);
    editor->redraw_requested = true;
    return result;
}

platform_cursor_kind viewer_editor_canvas_cursor(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    double x, double y)
{
    if (editor == NULL || document == NULL || navigation == NULL ||
        !isfinite(x) || !isfinite(y) || editor->draw_tool != VIEWER_EDITOR_TOOL_SELECT ||
        editor->active_tool != UI_ICON_SELECT || editor->node_edit_active ||
        editor->canvas_selection_count == 0U) return PLATFORM_CURSOR_ARROW;
    svg_shape_selection selection;
    if (!editor_canvas_selection_bounds(editor, document, &selection) &&
        !editor_selected_shape(editor, document, &selection))
        return PLATFORM_CURSOR_ARROW;
    size_t handle = selection_handle_at(editor, &selection, navigation, x, y);
    switch (handle) {
        case 0U:
        case 4U: return PLATFORM_CURSOR_RESIZE_NORTHWEST_SOUTHEAST;
        case 2U:
        case 6U: return PLATFORM_CURSOR_RESIZE_NORTHEAST_SOUTHWEST;
        case 1U:
        case 5U: return PLATFORM_CURSOR_RESIZE_NORTH_SOUTH;
        case 3U:
        case 7U: return PLATFORM_CURSOR_RESIZE_EAST_WEST;
        case 8U: return PLATFORM_CURSOR_ROTATE;
        default: break;
    }
    svg_shape_selection hit;
    geom_point hit_user;
    if (editor_hit_canvas_shape(editor, document, navigation, x, y,
            &hit, &hit_user) &&
        editor_canvas_selection_contains(editor, hit.source_offset))
        return PLATFORM_CURSOR_OPEN_HAND;
    return PLATFORM_CURSOR_ARROW;
}

uint32_t viewer_editor_cancel_canvas_gesture(viewer_editor *editor)
{
    if (editor == NULL || editor->canvas_gesture == EDITOR_CANVAS_GESTURE_NONE)
        return 0U;
    if (editor->canvas_gesture == EDITOR_CANVAS_GESTURE_PATH)
        editor_canvas_path_cancel(editor);
    else if (editor->canvas_gesture == EDITOR_CANVAS_GESTURE_PATH_NODE)
        editor->path_node_segment = SIZE_MAX;
    editor_reset_canvas_gesture(editor);
    editor->snap_bypass = false;
    editor_canvas_snap_clear(editor);
    editor->redraw_requested = true;
    editor_status_ok(editor, "Canvas gesture canceled");
    return VIEWER_EDITOR_HANDLED;
}

uint32_t viewer_editor_nudge_selection(viewer_editor *editor, double dx, double dy)
{
    if (editor == NULL || editor->edit == NULL || !isfinite(dx) || !isfinite(dy) ||
        (dx == 0.0 && dy == 0.0)) return 0U;
    if (!editor->canvas_selection_valid) {
        editor_status_ok(editor, "Select a rendered shape before nudging");
        return VIEWER_EDITOR_HANDLED;
    }
    if (editor->canvas_selection_count > 1U)
        return editor_canvas_multi_transform(editor, dx, dy, 1.0, 1.0,
            0.0, 0.0, 0.0);
    return editor_commit_transform(editor, dx, dy, 1.0, 1.0, 0.0, 0.0, 0.0);
}

bool viewer_editor_get_canvas_overlay(viewer_editor *editor,
    const svg_shape_document *document,
    viewer_editor_canvas_overlay_state *out_overlay)
{
    if (editor == NULL || document == NULL || out_overlay == NULL ||
        editor->navigation_overlays_hidden) return false;
    *out_overlay = (viewer_editor_canvas_overlay_state){0};
    out_overlay->snap_guide_x_visible = editor->snap_guide_x_visible;
    out_overlay->snap_guide_y_visible = editor->snap_guide_y_visible;
    out_overlay->snap_guide_x = editor->snap_guide_x;
    out_overlay->snap_guide_y = editor->snap_guide_y;
    out_overlay->snap_label_visible = editor->snap_label_visible;
    if (editor->snap_label_visible)
        editor_set_text(out_overlay->snap_label, sizeof(out_overlay->snap_label),
            editor->snap_label);
    if (editor->canvas_gesture == EDITOR_CANVAS_GESTURE_PATH &&
        editor->path_anchor_count > 0U) {
        size_t count = editor->path_anchor_count;
        if (!editor->path_anchor_dragging && count < VIEWER_EDITOR_PATH_POINT_CAP &&
            (editor->path_current_x != editor->path_anchors[count - 1U].x ||
             editor->path_current_y != editor->path_anchors[count - 1U].y)) ++count;
        if (count > VIEWER_EDITOR_PATH_POINT_CAP) count = VIEWER_EDITOR_PATH_POINT_CAP;
        out_overlay->path_point_count = count;
        out_overlay->visible = true;
        out_overlay->drawing_preview = true;
        out_overlay->path_closed_preview = editor->path_close_preview;
        for (size_t index = 0U; index < editor->path_anchor_count; ++index) {
            const editor_path_anchor *anchor = &editor->path_anchors[index];
            out_overlay->path_points[index] = (geom_point){anchor->x, anchor->y};
            out_overlay->path_handles[index] = (geom_point){
                anchor->x + anchor->handle_x, anchor->y + anchor->handle_y};
            out_overlay->path_has_handles[index] = anchor->has_handle;
        }
        if (count > editor->path_anchor_count)
            out_overlay->path_points[count - 1U] = (geom_point){
                editor->path_current_x, editor->path_current_y};
        return true;
    }
    if (editor_canvas_path_nodes_overlay(editor, document, out_overlay)) return true;
    if (editor->canvas_gesture == EDITOR_CANVAS_GESTURE_DRAW) {
        if (!rect_valid(editor->canvas_preview_bounds)) return false;
        out_overlay->bounds = editor->canvas_preview_bounds;
        out_overlay->visible = true;
        out_overlay->drawing_preview = true;
        return true;
    }
    if (editor->canvas_gesture == EDITOR_CANVAS_GESTURE_MARQUEE) {
        if (!rect_valid(editor->canvas_preview_bounds)) return false;
        out_overlay->bounds = editor->canvas_preview_bounds;
        out_overlay->visible = true;
        out_overlay->drawing_preview = true;
        return true;
    }
    if (editor->draw_tool != VIEWER_EDITOR_TOOL_SELECT ||
        editor->active_tool != UI_ICON_SELECT || editor->node_edit_active ||
        (editor->canvas_selection_count == 0U && !editor->canvas_selection_valid))
        return false;
    svg_shape_selection selection;
    if (!editor_canvas_selection_bounds(editor, document, &selection) &&
        !editor_selected_shape(editor, document, &selection)) {
        if (editor->canvas_gesture == EDITOR_CANVAS_GESTURE_NONE) return false;
        selection = editor->canvas_selection;
    }
    svg_shape_rect bounds = selection.device_bounds;
    if (editor->canvas_gesture == EDITOR_CANVAS_GESTURE_MOVE ||
        editor->canvas_gesture == EDITOR_CANVAS_GESTURE_RESIZE)
        bounds = editor->canvas_preview_bounds;
    if (!rect_valid(bounds)) return false;
    out_overlay->bounds = bounds;
    out_overlay->rotation_radians =
        editor->canvas_gesture == EDITOR_CANVAS_GESTURE_ROTATE ?
            editor->gesture_rotation : 0.0;
    out_overlay->visible = true;
    return true;
}

void viewer_editor_set_canvas_status(viewer_editor *editor, double zoom,
    bool pointer_valid, double pointer_x, double pointer_y,
    uint32_t document_width, uint32_t document_height,
    size_t tiles_done, size_t tiles_total, bool tiles_active)
{
    if (editor == NULL || !isfinite(zoom) || !(zoom > 0.0) ||
        (pointer_valid && (!isfinite(pointer_x) || !isfinite(pointer_y)))) return;
    if (editor->canvas_zoom == zoom && editor->canvas_pointer_valid == pointer_valid &&
        editor->canvas_pointer_x == pointer_x && editor->canvas_pointer_y == pointer_y &&
        editor->canvas_document_width == document_width &&
        editor->canvas_document_height == document_height &&
        editor->canvas_tiles_done == tiles_done &&
        editor->canvas_tiles_total == tiles_total &&
        editor->canvas_tiles_active == tiles_active) return;
    editor->canvas_zoom = zoom;
    editor->canvas_pointer_valid = pointer_valid;
    editor->canvas_pointer_x = pointer_x;
    editor->canvas_pointer_y = pointer_y;
    editor->canvas_document_width = document_width;
    editor->canvas_document_height = document_height;
    editor->canvas_tiles_done = tiles_done;
    editor->canvas_tiles_total = tiles_total;
    editor->canvas_tiles_active = tiles_active;
    editor->tree_ready = false;
    editor->redraw_requested = true;
}

static bool editor_node_contains_canvas(const ui_node *node, uint32_t x,
                                        uint32_t y)
{
    return node != NULL && x >= node->bounds.x && y >= node->bounds.y &&
        x - node->bounds.x < node->bounds.width &&
        y - node->bounds.y < node->bounds.height;
}

void viewer_editor_pointer_move(viewer_editor *editor, uint32_t x, uint32_t y)
{
    if (editor == NULL) return;
    editor_snap_popover_pointer_move(editor, x, y);
    editor_context_action_pointer_move(editor, x, y);
    if (editor->modern_style) {
        viewer_chrome_layout layout;
        if (viewer_chrome_layout_compute(editor->width, editor->height,
                editor->backing_scale, editor->want_structure,
                editor->want_inspector, &layout) &&
            x >= layout.canvas.x && y >= layout.canvas.y &&
            x - layout.canvas.x < layout.canvas.width &&
            y - layout.canvas.y < layout.canvas.height) {
            editor->context_pointer_valid = true;
            editor->context_pointer_x = x - layout.canvas.x;
            editor->context_pointer_y = y - layout.canvas.y;
        } else editor->context_pointer_valid = false;
    }
    if (!editor->tree_ready) editor_build_tree(editor);
    if (!editor->tree_ready) return;
    uint32_t hovered = 0U;
    for (size_t reverse = editor->tree.count; reverse > 0U; --reverse) {
        const ui_node *node = &editor->tree.nodes[reverse - 1U];
        bool interactive = node->spec.role == UI_ROLE_BUTTON ||
            node->spec.role == UI_ROLE_TEXT_FIELD || node->spec.role == UI_ROLE_ROW;
        if (interactive && editor_node_contains_canvas(node, x, y)) {
            hovered = node->spec.id;
            break;
        }
    }
    if (hovered == editor->hovered_id && x == editor->hovered_x) return;
    editor->hovered_id = hovered;
    editor->hovered_x = x;
    editor->tree_ready = false;
    editor->redraw_requested = true;
}

void viewer_editor_pointer_up(viewer_editor *editor)
{
    if (editor == NULL || editor->pressed_id == 0U) return;
    editor->pressed_id = 0U;
    editor->tree_ready = false;
    editor->redraw_requested = true;
}
