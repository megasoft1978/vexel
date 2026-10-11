#include "viewer_editor_internal.h"

#include "viewer_chrome.h"

#include <math.h>
#include <stdint.h>
#include <string.h>

void viewer_editor_guides_init(viewer_editor_guides *guides)
{
    if (guides == NULL) return;
    memset(guides, 0, sizeof(*guides));
    guides->rulers_visible = true;
    guides->guides_visible = true;
    guides->selected_index = SIZE_MAX;
    guides->drag_index = SIZE_MAX;
}

static viewer_editor_guide_axis *guide_axis_mut(viewer_editor_guides *guides,
    viewer_editor_guide_axis_kind axis)
{
    if (guides == NULL) return NULL;
    if (axis == VIEWER_EDITOR_GUIDE_HORIZONTAL) return &guides->horizontal;
    if (axis == VIEWER_EDITOR_GUIDE_VERTICAL) return &guides->vertical;
    return NULL;
}

static const viewer_editor_guide_axis *guide_axis(const viewer_editor_guides *guides,
    viewer_editor_guide_axis_kind axis)
{
    if (guides == NULL) return NULL;
    if (axis == VIEWER_EDITOR_GUIDE_HORIZONTAL) return &guides->horizontal;
    if (axis == VIEWER_EDITOR_GUIDE_VERTICAL) return &guides->vertical;
    return NULL;
}

static bool guide_value_valid(double value)
{
    return isfinite(value) && fabs(value) <= 1000000.0;
}

size_t viewer_editor_guides_count(const viewer_editor_guides *guides,
                                  viewer_editor_guide_axis_kind axis)
{
    const viewer_editor_guide_axis *selected = guide_axis(guides, axis);
    if (selected == NULL || selected->count > VIEWER_EDITOR_GUIDE_CAP_PER_AXIS)
        return 0U;
    return selected->count;
}

bool viewer_editor_guides_get(const viewer_editor_guides *guides,
    viewer_editor_guide_axis_kind axis, size_t index, double *out_value)
{
    const viewer_editor_guide_axis *selected = guide_axis(guides, axis);
    if (selected == NULL || out_value == NULL ||
        selected->count > VIEWER_EDITOR_GUIDE_CAP_PER_AXIS || index >= selected->count)
        return false;
    *out_value = selected->values[index];
    return true;
}

bool viewer_editor_guides_add(viewer_editor_guides *guides,
    viewer_editor_guide_axis_kind axis, double value)
{
    viewer_editor_guide_axis *selected = guide_axis_mut(guides, axis);
    if (selected == NULL || guides->locked || !guide_value_valid(value) ||
        selected->count >= VIEWER_EDITOR_GUIDE_CAP_PER_AXIS) return false;
    selected->values[selected->count] = value;
    selected->count++;
    guides->selected_axis = axis;
    guides->selected_index = selected->count - 1U;
    guides->selected = true;
    return true;
}

bool viewer_editor_guides_move(viewer_editor_guides *guides,
    viewer_editor_guide_axis_kind axis, size_t index, double value)
{
    viewer_editor_guide_axis *selected = guide_axis_mut(guides, axis);
    if (selected == NULL || guides->locked || !guide_value_valid(value) ||
        selected->count > VIEWER_EDITOR_GUIDE_CAP_PER_AXIS || index >= selected->count)
        return false;
    selected->values[index] = value;
    guides->selected_axis = axis;
    guides->selected_index = index;
    guides->selected = true;
    return true;
}

bool viewer_editor_guides_remove(viewer_editor_guides *guides,
    viewer_editor_guide_axis_kind axis, size_t index)
{
    viewer_editor_guide_axis *selected = guide_axis_mut(guides, axis);
    if (selected == NULL || guides->locked ||
        selected->count > VIEWER_EDITOR_GUIDE_CAP_PER_AXIS ||
        index >= selected->count) return false;
    size_t trailing = selected->count - index - 1U;
    if (trailing > 0U)
        memmove(&selected->values[index], &selected->values[index + 1U],
            trailing * sizeof(selected->values[0]));
    selected->count--;
    if (guides->selected && guides->selected_axis == axis) {
        if (guides->selected_index == index) {
            guides->selected = false;
            guides->selected_index = SIZE_MAX;
        } else if (guides->selected_index > index) guides->selected_index--;
    }
    return true;
}

bool viewer_editor_guides_clear(viewer_editor_guides *guides,
    viewer_editor_guide_axis_kind axis)
{
    viewer_editor_guide_axis *selected = guide_axis_mut(guides, axis);
    if (selected == NULL || selected->count == 0U) return false;
    selected->count = 0U;
    if (guides->selected && guides->selected_axis == axis) {
        guides->selected = false;
        guides->selected_index = SIZE_MAX;
    }
    guides->dragging = false;
    return true;
}

bool viewer_editor_guides_delete_selected(viewer_editor_guides *guides)
{
    if (guides == NULL || guides->locked || !guides->selected) return false;
    return viewer_editor_guides_remove(guides, guides->selected_axis,
        guides->selected_index);
}

bool viewer_editor_guides_nearest(const viewer_editor_guides *guides,
    viewer_editor_guide_axis_kind axis, double value, double tolerance,
    double *out_value, size_t *out_index)
{
    const viewer_editor_guide_axis *selected = guide_axis(guides, axis);
    if (selected == NULL || !guide_value_valid(value) || !isfinite(tolerance) ||
        !(tolerance > 0.0) || selected->count > VIEWER_EDITOR_GUIDE_CAP_PER_AXIS)
        return false;
    double best_distance = tolerance;
    size_t best_index = SIZE_MAX;
    for (size_t index = 0U; index < selected->count; ++index) {
        double distance = fabs(selected->values[index] - value);
        if (distance <= best_distance) {
            if (best_index == SIZE_MAX || distance < best_distance) {
                best_distance = distance;
                best_index = index;
            }
        }
    }
    if (best_index == SIZE_MAX) return false;
    if (out_value != NULL) *out_value = selected->values[best_index];
    if (out_index != NULL) *out_index = best_index;
    return true;
}

bool viewer_editor_guides_tick_step(double pixels_per_unit, double backing_scale,
    double desired_spacing_points, double *out_step)
{
    if (out_step == NULL || !isfinite(pixels_per_unit) || !(pixels_per_unit > 0.0) ||
        pixels_per_unit > 1000000000.0 || !isfinite(backing_scale) ||
        backing_scale < 0.5 || backing_scale > 4.0 ||
        !isfinite(desired_spacing_points) || !(desired_spacing_points > 0.0) ||
        desired_spacing_points > 512.0) return false;
    double target = desired_spacing_points * backing_scale / pixels_per_unit;
    if (!isfinite(target) || !(target > 0.0)) return false;
    double exponent = floor(log10(target));
    if (!isfinite(exponent) || exponent < -300.0 || exponent > 300.0) return false;
    double decade = pow(10.0, exponent);
    if (!isfinite(decade) || !(decade > 0.0)) return false;
    double ratio = target / decade;
    double multiple = ratio <= 1.0 ? 1.0 : ratio <= 2.0 ? 2.0 :
        ratio <= 5.0 ? 5.0 : 10.0;
    double step = multiple * decade;
    if (!isfinite(step) || !(step > 0.0)) return false;
    *out_step = step;
    return true;
}

bool viewer_editor_set_rulers_visible(viewer_editor *editor, bool visible)
{
    if (editor == NULL || editor->guides.rulers_visible == visible) return false;
    editor->guides.rulers_visible = visible;
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return true;
}

bool viewer_editor_rulers_visible(const viewer_editor *editor)
{
    return editor != NULL && editor->guides.rulers_visible;
}

bool viewer_editor_set_guides_visible(viewer_editor *editor, bool visible)
{
    if (editor == NULL || editor->guides.guides_visible == visible) return false;
    editor->guides.guides_visible = visible;
    if (!visible) editor->guides.selected = false;
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return true;
}

bool viewer_editor_guides_visible(const viewer_editor *editor)
{
    return editor != NULL && editor->guides.guides_visible;
}

bool viewer_editor_set_guides_locked(viewer_editor *editor, bool locked)
{
    if (editor == NULL || editor->guides.locked == locked) return false;
    editor->guides.locked = locked;
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return true;
}

bool viewer_editor_guides_locked(const viewer_editor *editor)
{
    return editor != NULL && editor->guides.locked;
}

bool viewer_editor_clear_guides(viewer_editor *editor,
                                viewer_editor_guide_axis_kind axis)
{
    if (editor == NULL || !viewer_editor_guides_clear(&editor->guides, axis)) return false;
    editor_status_ok(editor, axis == VIEWER_EDITOR_GUIDE_HORIZONTAL ?
        "Horizontal guides cleared" : "Vertical guides cleared");
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return true;
}

bool viewer_editor_get_navigation_canvas_rect(viewer_editor *editor,
    struct viewer_chrome_layout *out_layout, ui_rect *out_canvas)
{
    viewer_chrome_layout layout;
    if (editor == NULL || out_canvas == NULL ||
        !viewer_editor_get_chrome_layout(editor, &layout)) return false;
    if (out_layout != NULL) *out_layout = layout;
    ui_rect canvas = layout.canvas;
    if (editor->guides.rulers_visible && editor->modern_style) {
        double scale = editor->backing_scale;
        if (!isfinite(scale) || scale < 0.5 || scale > 4.0) return false;
        uint32_t inset = (uint32_t)ceil(36.0 * scale);
        if (canvas.width <= inset || canvas.height <= inset) return false;
        canvas.x += inset;
        canvas.y += inset;
        canvas.width -= inset;
        canvas.height -= inset;
    }
    if (canvas.width == 0U || canvas.height == 0U) return false;
    *out_canvas = canvas;
    return true;
}

typedef struct guide_view_transform {
    double scale_x;
    double scale_y;
    double offset_x;
    double offset_y;
} guide_view_transform;

static bool guide_view_transform_get(const svg_shape_document *document,
                                    guide_view_transform *out)
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
        ox = ((double)document->width - document->view_width * scale) * aspect.align_x;
        oy = ((double)document->height - document->view_height * scale) * aspect.align_y;
    }
    if (!isfinite(sx) || !(sx > 0.0) || !isfinite(sy) || !(sy > 0.0) ||
        !isfinite(ox) || !isfinite(oy)) return false;
    *out = (guide_view_transform){sx, sy, ox, oy};
    return true;
}

static bool guide_user_at_screen(const svg_shape_document *document,
    const ui_viewer_navigation *navigation, ui_rect canvas, double screen_x,
    double screen_y, geom_point *out_user)
{
    guide_view_transform transform;
    if (navigation == NULL || out_user == NULL || !isfinite(screen_x) ||
        !isfinite(screen_y) || !isfinite(navigation->scale) ||
        !(navigation->scale > 0.0) || !guide_view_transform_get(document, &transform))
        return false;
    double device_x = (screen_x - (double)canvas.x - navigation->pan_x) /
        navigation->scale;
    double device_y = (screen_y - (double)canvas.y - navigation->pan_y) /
        navigation->scale;
    double user_x = document->view_x + (device_x - transform.offset_x) /
        transform.scale_x;
    double user_y = document->view_y + (device_y - transform.offset_y) /
        transform.scale_y;
    if (!isfinite(user_x) || !isfinite(user_y) || fabs(user_x) > 1000000.0 ||
        fabs(user_y) > 1000000.0) return false;
    *out_user = (geom_point){user_x, user_y};
    return true;
}

static bool guide_screen_at_user(const svg_shape_document *document,
    const ui_viewer_navigation *navigation, ui_rect canvas, geom_point user,
    geom_point *out_screen)
{
    guide_view_transform transform;
    if (navigation == NULL || out_screen == NULL || !isfinite(user.x) ||
        !isfinite(user.y) || !isfinite(navigation->scale) ||
        !(navigation->scale > 0.0) || !guide_view_transform_get(document, &transform))
        return false;
    double device_x = transform.offset_x + (user.x - document->view_x) * transform.scale_x;
    double device_y = transform.offset_y + (user.y - document->view_y) * transform.scale_y;
    double x = (double)canvas.x + navigation->pan_x + device_x * navigation->scale;
    double y = (double)canvas.y + navigation->pan_y + device_y * navigation->scale;
    if (!isfinite(x) || !isfinite(y)) return false;
    *out_screen = (geom_point){x, y};
    return true;
}

static bool guide_top_ruler_contains(const viewer_editor *editor, ui_rect canvas,
    double screen_x, double screen_y)
{
    double scale = editor->backing_scale > 0.0 ? editor->backing_scale : 1.0;
    double inset = ceil(36.0 * scale);
    return screen_x >= (double)canvas.x - inset &&
        screen_x < (double)canvas.x + canvas.width &&
        screen_y >= (double)canvas.y - inset && screen_y < (double)canvas.y;
}

static bool guide_left_ruler_contains(const viewer_editor *editor, ui_rect canvas,
    double screen_x, double screen_y)
{
    double scale = editor->backing_scale > 0.0 ? editor->backing_scale : 1.0;
    double inset = ceil(36.0 * scale);
    return screen_x >= (double)canvas.x - inset && screen_x < (double)canvas.x &&
        screen_y >= (double)canvas.y && screen_y < (double)canvas.y + canvas.height;
}

static bool guide_line_hit(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    ui_rect canvas, double screen_x, double screen_y,
    viewer_editor_guide_axis_kind *out_axis, size_t *out_index)
{
    if (editor == NULL || document == NULL || out_axis == NULL || out_index == NULL ||
        !editor->guides.guides_visible) return false;
    double tolerance = 6.0 * (editor->backing_scale > 0.0 ? editor->backing_scale : 1.0);
    double best = tolerance + 1.0;
    bool found = false;
    const viewer_editor_guide_axis_kind axes[2] = {
        VIEWER_EDITOR_GUIDE_HORIZONTAL, VIEWER_EDITOR_GUIDE_VERTICAL};
    for (size_t axis_index = 0U; axis_index < 2U; ++axis_index) {
        viewer_editor_guide_axis_kind axis = axes[axis_index];
        size_t count = viewer_editor_guides_count(&editor->guides, axis);
        for (size_t index = 0U; index < count; ++index) {
            double value = 0.0;
            if (!viewer_editor_guides_get(&editor->guides, axis, index, &value)) continue;
            geom_point line;
            geom_point user = axis == VIEWER_EDITOR_GUIDE_HORIZONTAL ?
                (geom_point){document->view_x, value} :
                (geom_point){value, document->view_y};
            if (!guide_screen_at_user(document, navigation, canvas, user, &line)) continue;
            double distance = axis == VIEWER_EDITOR_GUIDE_HORIZONTAL ?
                fabs(screen_y - line.y) : fabs(screen_x - line.x);
            if (distance <= tolerance && (!found || distance < best)) {
                best = distance;
                found = true;
                *out_axis = axis;
                *out_index = index;
            }
        }
    }
    return found;
}

viewer_editor_guide_pointer_result viewer_editor_guide_pointer_down(
    viewer_editor *editor, const svg_shape_document *document,
    const ui_viewer_navigation *navigation, ui_rect canvas,
    double screen_x, double screen_y, uint8_t button)
{
    if (editor == NULL || document == NULL || navigation == NULL ||
        !isfinite(screen_x) || !isfinite(screen_y))
        return VIEWER_EDITOR_GUIDE_POINTER_NONE;
    bool top = editor->guides.rulers_visible &&
        guide_top_ruler_contains(editor, canvas, screen_x, screen_y);
    bool left = editor->guides.rulers_visible &&
        guide_left_ruler_contains(editor, canvas, screen_x, screen_y);
    if (button == 1U && (top || left))
        return top ? VIEWER_EDITOR_GUIDE_POINTER_CONTEXT_HORIZONTAL :
            VIEWER_EDITOR_GUIDE_POINTER_CONTEXT_VERTICAL;
    if (button != 0U) return VIEWER_EDITOR_GUIDE_POINTER_NONE;
    if (top || left) {
        if (editor->guides.locked) {
            editor_status_ok(editor, "Guides are locked");
            return VIEWER_EDITOR_GUIDE_POINTER_HANDLED;
        }
        viewer_editor_guides *guides = &editor->guides;
        guides->selected = false;
        guides->dragging = true;
        guides->drag_creating = true;
        guides->drag_moved = false;
        guides->drag_axis = top ? VIEWER_EDITOR_GUIDE_HORIZONTAL :
            VIEWER_EDITOR_GUIDE_VERTICAL;
        guides->drag_index = SIZE_MAX;
        guides->drag_value_valid = false;
        guides->drag_start_x = screen_x;
        guides->drag_start_y = screen_y;
        geom_point user;
        if (!guide_user_at_screen(document, navigation, canvas, screen_x,
                screen_y, &user)) {
            guides->dragging = false;
            return VIEWER_EDITOR_GUIDE_POINTER_HANDLED;
        }
        guides->drag_value = guides->drag_axis == VIEWER_EDITOR_GUIDE_HORIZONTAL ?
            user.y : user.x;
        guides->drag_value_valid = guide_value_valid(guides->drag_value);
        editor->redraw_requested = true;
        return VIEWER_EDITOR_GUIDE_POINTER_HANDLED;
    }
    viewer_editor_guide_axis_kind axis;
    size_t index;
    if (guide_line_hit(editor, document, navigation, canvas, screen_x, screen_y,
            &axis, &index)) {
        viewer_editor_guides *guides = &editor->guides;
        guides->selected = true;
        guides->selected_axis = axis;
        guides->selected_index = index;
        guides->dragging = !guides->locked;
        guides->drag_creating = false;
        guides->drag_moved = false;
        guides->drag_axis = axis;
        guides->drag_index = index;
        guides->drag_value_valid = true;
        guides->drag_start_x = screen_x;
        guides->drag_start_y = screen_y;
        double current = 0.0;
        if (viewer_editor_guides_get(guides, axis, index, &current))
            guides->drag_value = current;
        editor->redraw_requested = true;
        return VIEWER_EDITOR_GUIDE_POINTER_HANDLED;
    }
    if (editor->guides.selected) {
        editor->guides.selected = false;
        editor->redraw_requested = true;
    }
    return VIEWER_EDITOR_GUIDE_POINTER_NONE;
}

bool viewer_editor_guide_pointer_move(viewer_editor *editor,
    const svg_shape_document *document, const ui_viewer_navigation *navigation,
    ui_rect canvas, double screen_x, double screen_y)
{
    if (editor == NULL || document == NULL || navigation == NULL ||
        !editor->guides.dragging || !isfinite(screen_x) || !isfinite(screen_y))
        return false;
    viewer_editor_guides *guides = &editor->guides;
    if (hypot(screen_x - guides->drag_start_x, screen_y - guides->drag_start_y) >= 1.0)
        guides->drag_moved = true;
    geom_point user;
    if (guide_user_at_screen(document, navigation, canvas, screen_x, screen_y, &user)) {
        double value = guides->drag_axis == VIEWER_EDITOR_GUIDE_HORIZONTAL ?
            user.y : user.x;
        guides->drag_value_valid = guide_value_valid(value);
        if (guides->drag_value_valid) guides->drag_value = value;
    } else guides->drag_value_valid = false;
    editor->redraw_requested = true;
    return true;
}

bool viewer_editor_guide_pointer_up(viewer_editor *editor)
{
    if (editor == NULL || !editor->guides.dragging) return false;
    viewer_editor_guides *guides = &editor->guides;
    bool creating = guides->drag_creating;
    bool moved = guides->drag_moved;
    bool value_valid = guides->drag_value_valid;
    viewer_editor_guide_axis_kind axis = guides->drag_axis;
    size_t index = guides->drag_index;
    double value = guides->drag_value;
    guides->dragging = false;
    guides->drag_creating = false;
    guides->drag_moved = false;
    guides->drag_value_valid = false;
    if (moved && value_valid) {
        bool changed = creating ? viewer_editor_guides_add(guides, axis, value) :
            viewer_editor_guides_move(guides, axis, index, value);
        if (!changed && creating && viewer_editor_guides_count(guides, axis) >=
                VIEWER_EDITOR_GUIDE_CAP_PER_AXIS)
            editor_status_ok(editor, "Guide limit reached (128 per axis)");
        else if (changed) editor_status_ok(editor, creating ? "Guide created" : "Guide moved");
    }
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return true;
}

uint32_t viewer_editor_delete_selected_guide(viewer_editor *editor)
{
    if (editor == NULL || !editor->guides.selected) return 0U;
    if (editor->guides.locked) {
        editor_status_ok(editor, "Guides are locked");
        editor->redraw_requested = true;
        return VIEWER_EDITOR_HANDLED;
    }
    if (!viewer_editor_guides_delete_selected(&editor->guides)) return 0U;
    editor_status_ok(editor, "Guide removed");
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return VIEWER_EDITOR_HANDLED;
}

void viewer_editor_set_navigation_overlays_hidden(viewer_editor *editor, bool hidden)
{
    if (editor == NULL || editor->navigation_overlays_hidden == hidden) return;
    editor->navigation_overlays_hidden = hidden;
    editor->redraw_requested = true;
}

void viewer_editor_set_snap_bypass(viewer_editor *editor, bool bypass)
{
    if (editor != NULL) editor->snap_bypass = bypass;
}

static const double zoom_presets[] = {0.25, 0.5, 1.0, 2.0, 4.0, 8.0};

double viewer_editor_zoom_preset_scale(size_t index)
{
    return index < sizeof(zoom_presets) / sizeof(zoom_presets[0]) ?
        zoom_presets[index] : 0.0;
}

bool viewer_editor_take_zoom_preset(viewer_editor *editor, double *out_scale)
{
    if (editor == NULL || out_scale == NULL || !editor->zoom_preset_pending ||
        editor->zoom_preset_request >= sizeof(zoom_presets) / sizeof(zoom_presets[0]))
        return false;
    *out_scale = zoom_presets[editor->zoom_preset_request];
    editor->zoom_preset_pending = false;
    return true;
}
