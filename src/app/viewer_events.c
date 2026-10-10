#include "viewer_internal.h"

#include "frame_compose.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

static size_t event_path_length(const char *path, size_t limit)
{
    size_t length = 0U;
    if (path == NULL) return 0U;
    while (length <= limit && path[length] != '\0') length++;
    return length;
}

static bool navigation_differs(const ui_viewer_navigation *left,
                               const ui_viewer_navigation *right)
{
    return left->scale != right->scale || left->pan_x != right->pan_x ||
        left->pan_y != right->pan_y || left->viewport_width != right->viewport_width ||
        left->viewport_height != right->viewport_height;
}

static void navigation_changed(viewer_app *app, const ui_viewer_navigation *previous)
{
    if (!navigation_differs(previous, &app->navigation)) return;
    if (!viewer_bump_generation(app)) {
        app->fatal_error = true;
        app->running = false;
        return;
    }
    viewer_navigation_clamp(&app->navigation);
    viewer_refresh_sharp_plan(app);
    (void)viewer_set_window_title(app);
}

static void apply_pan_delta(viewer_app *app, double dx, double dy, double scale)
{
    if (!app->navigation_ready || !isfinite(dx) || !isfinite(dy) ||
        !isfinite(scale) || !(scale > 0.0)) return;
    ui_viewer_navigation previous = app->navigation;
    double x = app->navigation.pan_x + dx * scale;
    double y = app->navigation.pan_y + dy * scale;
    if (!isfinite(x) || !isfinite(y)) return;
    app->navigation.pan_x = x;
    app->navigation.pan_y = y;
    navigation_changed(app, &previous);
}

static void apply_navigation_key(viewer_app *app, ui_viewer_key key)
{
    if (!app->navigation_ready || !app->document_visible) return;
    ui_viewer_navigation previous = app->navigation;
    if (ui_viewer_navigation_key(&app->navigation, key)) navigation_changed(app, &previous);
}

static void toggle_checkerboard(viewer_app *app)
{
    app->checkerboard = !app->checkerboard;
    (void)platform_window_set_option_state(app->window, PLATFORM_ACTION_TOGGLE_CHECKERBOARD,
                                          app->checkerboard);
    app->frame_dirty = true;
}

static void cycle_background(viewer_app *app)
{
    app->background_index = (uint8_t)((app->background_index + 1U) % 3U);
    if (app->checkerboard) toggle_checkerboard(app);
    app->frame_dirty = true;
}

static void toggle_pixel_grid(viewer_app *app)
{
    app->pixel_grid = !app->pixel_grid;
    (void)platform_window_set_option_state(app->window, PLATFORM_ACTION_TOGGLE_PIXEL_GRID,
                                          app->pixel_grid);
    app->frame_dirty = true;
}

static void request_reload_or_open(viewer_app *app)
{
    if (app->path[0] != '\0') (void)viewer_request_open(app, app->path, false);
    else (void)platform_window_open_panel(app->window);
}

static void apply_action(viewer_app *app, platform_viewer_action action)
{
    switch (action) {
        case PLATFORM_ACTION_RELOAD: request_reload_or_open(app); break;
        case PLATFORM_ACTION_TOGGLE_CHECKERBOARD: toggle_checkerboard(app); break;
        case PLATFORM_ACTION_CYCLE_BACKGROUND: cycle_background(app); break;
        case PLATFORM_ACTION_FIT_WIDTH:
            if (app->navigation_ready) {
                ui_viewer_navigation previous = app->navigation;
                (void)ui_viewer_navigation_fit_width(&app->navigation);
                navigation_changed(app, &previous);
            }
            break;
        case PLATFORM_ACTION_FIT_WINDOW: apply_navigation_key(app, UI_VIEWER_KEY_FIT); break;
        case PLATFORM_ACTION_ACTUAL_SIZE:
            if (app->navigation_ready) {
                ui_viewer_navigation previous = app->navigation;
                (void)ui_viewer_navigation_actual_size(&app->navigation);
                navigation_changed(app, &previous);
            }
            break;
        case PLATFORM_ACTION_TOGGLE_PIXEL_GRID: toggle_pixel_grid(app); break;
        case PLATFORM_ACTION_QUIT: viewer_request_close(app); break;
        case PLATFORM_ACTION_NONE: default: break;
    }
}

static void apply_key(viewer_app *app, const platform_event *event)
{
    bool command = (event->modifiers & PLATFORM_MODIFIER_COMMAND) != 0U;
    if (command && (strcmp(event->text, "q") == 0 || strcmp(event->text, "Q") == 0)) {
        viewer_request_close(app);
        return;
    }
    if (command && (strcmp(event->text, "r") == 0 || strcmp(event->text, "R") == 0)) {
        request_reload_or_open(app);
        return;
    }
    if (command && (strcmp(event->text, "o") == 0 || strcmp(event->text, "O") == 0)) {
        (void)platform_window_open_panel(app->window);
        return;
    }
    if (command) return;
    if (strcmp(event->text, "t") == 0 || strcmp(event->text, "T") == 0) {
        toggle_checkerboard(app);
        return;
    }
    if (strcmp(event->text, "b") == 0 || strcmp(event->text, "B") == 0) {
        cycle_background(app);
        return;
    }
    if (strcmp(event->text, "g") == 0 || strcmp(event->text, "G") == 0) {
        toggle_pixel_grid(app);
        return;
    }
    if (strcmp(event->text, "w") == 0 || strcmp(event->text, "W") == 0) {
        apply_action(app, PLATFORM_ACTION_FIT_WIDTH);
        return;
    }
    if (strcmp(event->text, "1") == 0) {
        apply_action(app, PLATFORM_ACTION_ACTUAL_SIZE);
        return;
    }
    if (event->key_code == 123U) { apply_navigation_key(app, UI_VIEWER_KEY_LEFT); return; }
    if (event->key_code == 124U) { apply_navigation_key(app, UI_VIEWER_KEY_RIGHT); return; }
    if (event->key_code == 125U) { apply_navigation_key(app, UI_VIEWER_KEY_DOWN); return; }
    if (event->key_code == 126U) { apply_navigation_key(app, UI_VIEWER_KEY_UP); return; }
    if (strcmp(event->text, "+") == 0 || strcmp(event->text, "=") == 0) {
        apply_navigation_key(app, UI_VIEWER_KEY_ZOOM_IN);
    } else if (strcmp(event->text, "-") == 0) {
        apply_navigation_key(app, UI_VIEWER_KEY_ZOOM_OUT);
    } else if (strcmp(event->text, "0") == 0) {
        apply_action(app, PLATFORM_ACTION_FIT_WINDOW);
    }
}

static void update_pointer_status(viewer_app *app, const platform_event *event)
{
    app->pointer_valid = false;
    if (!app->navigation_ready || !app->document_visible ||
        !isfinite(app->backing_scale) || !(app->backing_scale > 0.0)) return;
    double screen_x = (double)event->logical_x * app->backing_scale;
    double screen_y = (double)event->logical_y * app->backing_scale;
    double x = (screen_x - app->navigation.pan_x) / app->navigation.scale;
    double y = (screen_y - app->navigation.pan_y) / app->navigation.scale;
    if (!isfinite(x) || !isfinite(y) || x < 0.0 || y < 0.0 ||
        x >= app->navigation.document_width || y >= app->navigation.document_height) return;
    app->pointer_doc_x = x;
    app->pointer_doc_y = y;
    app->pointer_valid = true;
}

void viewer_process_event(viewer_app *app, const platform_event *event)
{
    switch (event->type) {
        case PLATFORM_EVENT_RESIZE:
        case PLATFORM_EVENT_SCALE: {
            if (event->pixel_width == 0U || event->pixel_height == 0U ||
                event->pixel_width > VIEWER_FRAME_MAX_AXIS ||
                event->pixel_height > VIEWER_FRAME_MAX_AXIS ||
                !isfinite(event->backing_scale) || !(event->backing_scale > 0.0)) {
                app->fatal_error = true;
                app->running = false;
                return;
            }
            bool resized = event->pixel_width != app->width || event->pixel_height != app->height;
            double prior_scale = app->backing_scale;
            app->backing_scale = event->backing_scale;
            if (resized) {
                if (!viewer_allocate_frame(app, event->pixel_width, event->pixel_height)) {
                    app->fatal_error = true;
                    app->running = false;
                    return;
                }
                if (app->navigation_ready) {
                    ui_viewer_navigation previous = app->navigation;
                    if (ui_viewer_navigation_resize(&app->navigation,
                            (double)app->width, (double)app->height))
                        navigation_changed(app, &previous);
                }
            }
            if (prior_scale != app->backing_scale) app->frame_dirty = true;
            return;
        }
        case PLATFORM_EVENT_POINTER_DOWN:
            if (app->navigation_ready && event->button == 0U)
                (void)ui_viewer_navigation_drag_begin_scaled(&app->navigation,
                    (double)event->logical_x, (double)event->logical_y, app->backing_scale);
            return;
        case PLATFORM_EVENT_POINTER_MOVE:
            update_pointer_status(app, event);
            app->frame_dirty = true;
            if (app->navigation_ready && app->navigation.dragging) {
                ui_viewer_navigation previous = app->navigation;
                if (ui_viewer_navigation_drag_scaled(&app->navigation,
                        (double)event->logical_x, (double)event->logical_y,
                        app->backing_scale)) navigation_changed(app, &previous);
            }
            return;
        case PLATFORM_EVENT_POINTER_UP:
            if (app->navigation_ready && app->navigation.dragging)
                (void)ui_viewer_navigation_drag_end(&app->navigation);
            return;
        case PLATFORM_EVENT_SCROLL:
            if (!app->navigation_ready) return;
            if ((event->modifiers & (PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_CONTROL)) != 0U) {
                ui_viewer_navigation previous = app->navigation;
                if (ui_viewer_navigation_wheel_scaled(&app->navigation, event->delta_y,
                        event->precise_x, event->precise_y, event->backing_scale))
                    navigation_changed(app, &previous);
            } else apply_pan_delta(app, event->delta_x, event->delta_y, event->backing_scale);
            return;
        case PLATFORM_EVENT_MAGNIFY:
            if (app->navigation_ready) {
                ui_viewer_navigation previous = app->navigation;
                if (ui_viewer_navigation_pinch_scaled(&app->navigation,
                        event->magnification, event->precise_x, event->precise_y,
                        event->backing_scale)) navigation_changed(app, &previous);
            }
            return;
        case PLATFORM_EVENT_KEY_DOWN: apply_key(app, event); return;
        case PLATFORM_EVENT_ACTION: apply_action(app, event->action); return;
        case PLATFORM_EVENT_FILE_OPEN:
            if (event_path_length(event->path, PLATFORM_PATH_CAP - 1U) < PLATFORM_PATH_CAP)
                (void)viewer_request_open(app, event->path, false);
            return;
        case PLATFORM_EVENT_CLOSE_REQUEST: viewer_request_close(app); return;
        case PLATFORM_EVENT_OPEN:
        case PLATFORM_EVENT_NONE:
        case PLATFORM_EVENT_KEY_UP: default: return;
    }
}

