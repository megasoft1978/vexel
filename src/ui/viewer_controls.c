#include "viewer_controls.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

static bool finite_positive(double value)
{
    return isfinite(value) && value > 0.0;
}

static bool finite_point(const ui_viewer_navigation *state, double x, double y)
{
    return state != NULL && isfinite(x) && isfinite(y) &&
        finite_positive(state->scale) && isfinite(state->pan_x) && isfinite(state->pan_y);
}

static double clamp_scale(double scale)
{
    return scale < UI_VIEWER_MIN_SCALE ? UI_VIEWER_MIN_SCALE :
        (scale > UI_VIEWER_MAX_SCALE ? UI_VIEWER_MAX_SCALE : scale);
}

bool ui_viewer_navigation_init(ui_viewer_navigation *state, double width, double height,
                               double document_width, double document_height)
{
    if (state == NULL || !finite_positive(width) || !finite_positive(height) ||
        !finite_positive(document_width) || !finite_positive(document_height)) return false;
    *state = (ui_viewer_navigation){.viewport_width = width, .viewport_height = height,
        .document_width = document_width, .document_height = document_height,
        .scale = 1.0, .pan_x = 0.0, .pan_y = 0.0};
    return true;
}

bool ui_viewer_navigation_zoom(ui_viewer_navigation *state, double factor, double x, double y)
{
    if (!finite_point(state, x, y) || !finite_positive(factor)) return false;
    double scale;
    if (factor > UI_VIEWER_MAX_SCALE / state->scale) scale = UI_VIEWER_MAX_SCALE;
    else if (factor < UI_VIEWER_MIN_SCALE / state->scale) scale = UI_VIEWER_MIN_SCALE;
    else scale = clamp_scale(state->scale * factor);
    double world_x = (x - state->pan_x) / state->scale;
    double world_y = (y - state->pan_y) / state->scale;
    double pan_x = x - world_x * scale, pan_y = y - world_y * scale;
    if (!isfinite(scale) || !isfinite(pan_x) || !isfinite(pan_y)) return false;
    state->scale = scale;
    state->pan_x = pan_x;
    state->pan_y = pan_y;
    return true;
}

bool ui_viewer_navigation_wheel(ui_viewer_navigation *state, double delta_y, double x, double y)
{
    if (!isfinite(delta_y) || fabs(delta_y) > 10000.0) return false;
    return ui_viewer_navigation_zoom(state, exp(-delta_y * 0.001), x, y);
}

bool ui_viewer_navigation_pinch(ui_viewer_navigation *state, double magnification,
                                double x, double y)
{
    if (!isfinite(magnification) || fabs(magnification) > 100.0) return false;
    return ui_viewer_navigation_zoom(state, exp(magnification), x, y);
}

bool ui_viewer_navigation_wheel_scaled(ui_viewer_navigation *state, double delta_y,
                                       double logical_x, double logical_y,
                                       double backing_scale)
{
    if (!finite_positive(backing_scale) || !isfinite(logical_x) || !isfinite(logical_y)) return false;
    double pixel_x = logical_x * backing_scale, pixel_y = logical_y * backing_scale;
    if (!isfinite(pixel_x) || !isfinite(pixel_y)) return false;
    return ui_viewer_navigation_wheel(state, delta_y, pixel_x, pixel_y);
}

bool ui_viewer_navigation_pinch_scaled(ui_viewer_navigation *state, double magnification,
                                        double logical_x, double logical_y,
                                        double backing_scale)
{
    if (!finite_positive(backing_scale) || !isfinite(logical_x) || !isfinite(logical_y)) return false;
    double pixel_x = logical_x * backing_scale, pixel_y = logical_y * backing_scale;
    if (!isfinite(pixel_x) || !isfinite(pixel_y)) return false;
    return ui_viewer_navigation_pinch(state, magnification, pixel_x, pixel_y);
}

bool ui_viewer_navigation_resize(ui_viewer_navigation *state, double width, double height)
{
    if (state == NULL || !finite_positive(width) || !finite_positive(height) ||
        !finite_positive(state->scale)) return false;
    double world_x = (state->viewport_width * 0.5 - state->pan_x) / state->scale;
    double world_y = (state->viewport_height * 0.5 - state->pan_y) / state->scale;
    double pan_x = width * 0.5 - world_x * state->scale;
    double pan_y = height * 0.5 - world_y * state->scale;
    if (!isfinite(pan_x) || !isfinite(pan_y)) return false;
    state->viewport_width = width;
    state->viewport_height = height;
    state->pan_x = pan_x;
    state->pan_y = pan_y;
    return true;
}

bool ui_viewer_navigation_resize_scaled(ui_viewer_navigation *state, double logical_width,
                                        double logical_height, double backing_scale)
{
    if (!finite_positive(logical_width) || !finite_positive(logical_height) ||
        !finite_positive(backing_scale)) return false;
    double pixel_width = logical_width * backing_scale;
    double pixel_height = logical_height * backing_scale;
    if (!finite_positive(pixel_width) || !finite_positive(pixel_height)) return false;
    return ui_viewer_navigation_resize(state, pixel_width, pixel_height);
}

bool ui_viewer_navigation_drag_begin(ui_viewer_navigation *state, double x, double y)
{
    if (!finite_point(state, x, y)) return false;
    state->dragging = true; state->drag_x = x; state->drag_y = y;
    return true;
}

bool ui_viewer_navigation_drag(ui_viewer_navigation *state, double x, double y)
{
    if (!finite_point(state, x, y) || !state->dragging) return false;
    double px = state->pan_x + x - state->drag_x, py = state->pan_y + y - state->drag_y;
    if (!isfinite(px) || !isfinite(py)) return false;
    state->pan_x = px; state->pan_y = py; state->drag_x = x; state->drag_y = y;
    return true;
}

bool ui_viewer_navigation_drag_end(ui_viewer_navigation *state)
{
    if (state == NULL || !state->dragging) return false;
    state->dragging = false;
    return true;
}

bool ui_viewer_navigation_drag_begin_scaled(ui_viewer_navigation *state, double logical_x,
                                            double logical_y, double backing_scale)
{
    if (!finite_positive(backing_scale) || !isfinite(logical_x) || !isfinite(logical_y)) return false;
    double pixel_x = logical_x * backing_scale, pixel_y = logical_y * backing_scale;
    if (!isfinite(pixel_x) || !isfinite(pixel_y)) return false;
    return ui_viewer_navigation_drag_begin(state, pixel_x, pixel_y);
}

bool ui_viewer_navigation_drag_scaled(ui_viewer_navigation *state, double logical_x,
                                       double logical_y, double backing_scale)
{
    if (!finite_positive(backing_scale) || !isfinite(logical_x) || !isfinite(logical_y)) return false;
    double pixel_x = logical_x * backing_scale, pixel_y = logical_y * backing_scale;
    if (!isfinite(pixel_x) || !isfinite(pixel_y)) return false;
    return ui_viewer_navigation_drag(state, pixel_x, pixel_y);
}

bool ui_viewer_navigation_fit(ui_viewer_navigation *state)
{
    if (state == NULL || !finite_positive(state->viewport_width) ||
        !finite_positive(state->viewport_height) || !finite_positive(state->document_width) ||
        !finite_positive(state->document_height)) return false;
    double scale = clamp_scale(fmin(state->viewport_width / state->document_width,
                                    state->viewport_height / state->document_height));
    double px = (state->viewport_width - state->document_width * scale) * 0.5;
    double py = (state->viewport_height - state->document_height * scale) * 0.5;
    if (!isfinite(px) || !isfinite(py)) return false;
    state->scale = scale; state->pan_x = px; state->pan_y = py; state->dragging = false;
    return true;
}

bool ui_viewer_navigation_fit_width(ui_viewer_navigation *state)
{
    if (state == NULL || !finite_positive(state->viewport_width) ||
        !finite_positive(state->viewport_height) || !finite_positive(state->document_width) ||
        !finite_positive(state->document_height)) return false;
    double scale = clamp_scale(state->viewport_width / state->document_width);
    double px = (state->viewport_width - state->document_width * scale) * 0.5;
    double py = (state->viewport_height - state->document_height * scale) * 0.5;
    if (!isfinite(scale) || !isfinite(px) || !isfinite(py)) return false;
    state->scale = scale;
    state->pan_x = px;
    state->pan_y = py;
    state->dragging = false;
    return true;
}

bool ui_viewer_navigation_actual_size(ui_viewer_navigation *state)
{
    if (state == NULL || !finite_positive(state->viewport_width) ||
        !finite_positive(state->viewport_height) || !finite_positive(state->document_width) ||
        !finite_positive(state->document_height)) return false;
    double px = (state->viewport_width - state->document_width) * 0.5;
    double py = (state->viewport_height - state->document_height) * 0.5;
    if (!isfinite(px) || !isfinite(py)) return false;
    state->scale = 1.0;
    state->pan_x = px;
    state->pan_y = py;
    state->dragging = false;
    return true;
}

bool ui_viewer_navigation_key(ui_viewer_navigation *state, ui_viewer_key key)
{
    if (state == NULL || !isfinite(state->pan_x) || !isfinite(state->pan_y)) return false;
    double next_x = state->pan_x, next_y = state->pan_y;
    switch (key) {
        case UI_VIEWER_KEY_LEFT: next_x += 32.0; break;
        case UI_VIEWER_KEY_RIGHT: next_x -= 32.0; break;
        case UI_VIEWER_KEY_UP: next_y += 32.0; break;
        case UI_VIEWER_KEY_DOWN: next_y -= 32.0; break;
        case UI_VIEWER_KEY_ZOOM_IN: return ui_viewer_navigation_zoom(state, 1.25,
            state->viewport_width * 0.5, state->viewport_height * 0.5);
        case UI_VIEWER_KEY_ZOOM_OUT: return ui_viewer_navigation_zoom(state, 0.8,
            state->viewport_width * 0.5, state->viewport_height * 0.5);
        case UI_VIEWER_KEY_FIT: return ui_viewer_navigation_fit(state);
        default: return false;
    }
    if (!isfinite(next_x) || !isfinite(next_y)) return false;
    state->pan_x = next_x;
    state->pan_y = next_y;
    return true;
}

bool ui_viewer_navigation_project(const ui_viewer_navigation *state, double wx, double wy,
                                  double *sx, double *sy)
{
    if (!finite_point(state, wx, wy) || sx == NULL || sy == NULL) return false;
    double x = wx * state->scale + state->pan_x, y = wy * state->scale + state->pan_y;
    if (!isfinite(x) || !isfinite(y)) return false;
    *sx = x; *sy = y;
    return true;
}

bool ui_viewer_progress_update(ui_viewer_progress *state, uint64_t generation,
                               unsigned percent, ui_progress_phase phase,
                               const char *label, size_t length)
{
    if (state == NULL || label == NULL || length >= UI_VIEWER_LABEL_CAP || percent > 100U ||
        phase < UI_PROGRESS_IDLE || phase > UI_PROGRESS_FAILED || memchr(label, '\0', length) != NULL ||
        (state->generation > generation) || (phase == UI_PROGRESS_ACTIVE && percent >= 100U)) return false;
    if (state->generation == generation &&
        (state->phase == UI_PROGRESS_COMPLETE || state->phase == UI_PROGRESS_CANCELED ||
         state->phase == UI_PROGRESS_FAILED) && phase != state->phase) return false;
    if (state->generation != generation) state->cancel_requested = false;
    memcpy(state->label, label, length); state->label[length] = '\0';
    state->generation = generation;
    state->percent = phase == UI_PROGRESS_COMPLETE ? 100U : percent;
    state->phase = phase;
    return true;
}

bool ui_viewer_progress_cancel(ui_viewer_progress *state, bool *out_request)
{
    if (state == NULL || out_request == NULL) return false;
    *out_request = state->phase == UI_PROGRESS_ACTIVE && !state->cancel_requested;
    if (*out_request) state->cancel_requested = true;
    return true;
}

static void node_set(ui_semantic_node *node, const char *id, const char *role,
                     const char *name, int32_t x, int32_t y, int32_t width,
                     int32_t height, unsigned value, bool enabled)
{
    (void)snprintf(node->id, sizeof(node->id), "%s", id);
    (void)snprintf(node->role, sizeof(node->role), "%s", role);
    (void)snprintf(node->name, sizeof(node->name), "%s", name);
    node->x = x; node->y = y; node->width = width; node->height = height;
    node->value = value; node->enabled = enabled;
}

size_t ui_viewer_semantics(const ui_viewer_navigation *nav, const ui_viewer_progress *progress,
                           ui_semantic_node *out, size_t capacity)
{
    if (nav == NULL || progress == NULL || out == NULL || capacity < 2U ||
        !isfinite(nav->scale) || !isfinite(nav->viewport_width) || !isfinite(nav->viewport_height) ||
        nav->viewport_width > INT32_MAX || nav->viewport_height > INT32_MAX) return 0U;
    node_set(&out[0], "canvas", "canvas", "Document canvas", 0, 0,
        (int32_t)nav->viewport_width, (int32_t)nav->viewport_height, 0U, true);
    (void)snprintf(out[0].name, sizeof(out[0].name), "Zoom %.3f%%", nav->scale * 100.0);
    node_set(&out[1], "progress", "progressbar", progress->label, 0, 0,
        (int32_t)nav->viewport_width, 20, progress->percent, false);
    if (capacity < 3U || progress->phase != UI_PROGRESS_ACTIVE) return 2U;
    node_set(&out[2], "cancel", "button", "Cancel indexing", 0, 20, 96, 28, 0U,
        !progress->cancel_requested);
    return 3U;
}

bool ui_viewer_overlay_rgba(const ui_viewer_progress *progress, uint8_t *pixels,
                            size_t length, uint32_t width, uint32_t height, size_t stride)
{
    if (progress == NULL || pixels == NULL || width == 0U || height == 0U ||
        width > 4096U || height > 4096U || stride < (size_t)width * 4U ||
        (size_t)height > length / stride || progress->percent > 100U) return false;
    uint32_t bar_h = height < 20U ? height : 20U;
    uint32_t fill = (uint32_t)(((uint64_t)width * progress->percent) / 100U);
    for (uint32_t y = 0U; y < bar_h; y++) {
        uint8_t *row = pixels + (size_t)y * stride;
        for (uint32_t x = 0U; x < width; x++) {
            uint8_t *pixel = row + (size_t)x * 4U;
            pixel[0] = x < fill ? 34U : 48U; pixel[1] = x < fill ? 128U : 54U;
            pixel[2] = x < fill ? 224U : 64U; pixel[3] = 230U;
        }
    }
    if (progress->phase == UI_PROGRESS_ACTIVE && width >= 48U && height >= 48U) {
        for (uint32_t y = 24U; y < 44U; y++) for (uint32_t x = 8U; x < 40U; x++) {
            uint8_t *p = pixels + (size_t)y * stride + (size_t)x * 4U;
            bool cross = (x == y - 16U + 8U) || (x + y == 48U);
            p[0] = cross ? 255U : 190U; p[1] = cross ? 255U : 42U;
            p[2] = cross ? 255U : 42U; p[3] = 255U;
        }
    }
    return true;
}
