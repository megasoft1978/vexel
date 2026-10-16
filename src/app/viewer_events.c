#include "viewer_internal.h"

#include "frame_compose.h"
#include "platform/macos/platform_clipboard.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static bool replacement_load_active(const viewer_app *app)
{
    return app != NULL && (app->pending_open || (app->job != NULL &&
        app->job->kind == VIEWER_JOB_LOAD));
}

static bool process_home_pointer_down(viewer_app *app, const platform_event *event)
{
    if (app == NULL || event == NULL || !isfinite(app->backing_scale) ||
        !(app->backing_scale > 0.0)) return false;
    double px = floor((double)event->logical_x * app->backing_scale);
    double py = floor((double)event->logical_y * app->backing_scale);
    if (!isfinite(px) || !isfinite(py) || px < 0.0 || py < 0.0 ||
        px > (double)UINT32_MAX || py > (double)UINT32_MAX) return false;
    viewer_home_state state;
    viewer_home_state_from_app(app, &state);
    viewer_home_action action = viewer_home_pointer_down(&state, app->width,
        app->height, app->backing_scale, (uint32_t)px, (uint32_t)py);
    if (action.kind == VIEWER_HOME_ACTION_OPEN) {
        (void)platform_window_open_panel(app->window);
    } else if (action.kind == VIEWER_HOME_ACTION_RETRY) {
        if (app->failed_path[0] != '\0')
            (void)viewer_request_open(app, app->failed_path, false);
    } else if (action.kind == VIEWER_HOME_ACTION_RECENT &&
        action.recent_index < app->recent_file_count) {
        (void)viewer_request_open(app,
            app->recent_files[action.recent_index].path, false);
    }
    if (action.kind != VIEWER_HOME_ACTION_NONE) app->frame_dirty = true;
    return action.kind != VIEWER_HOME_ACTION_NONE;
}

static bool navigation_differs(const ui_viewer_navigation *left,
                               const ui_viewer_navigation *right)
{
    return left->scale != right->scale || left->pan_x != right->pan_x ||
        left->pan_y != right->pan_y || left->viewport_width != right->viewport_width ||
        left->viewport_height != right->viewport_height;
}

void navigation_changed(viewer_app *app, const ui_viewer_navigation *previous)
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

static bool canvas_contains(const viewer_app *app, double x, double y)
{
    const ui_rect canvas = app->navigation_canvas;
    return x >= (double)canvas.x && y >= (double)canvas.y &&
        x - (double)canvas.x < (double)canvas.width &&
        y - (double)canvas.y < (double)canvas.height;
}

static bool event_canvas_point(const viewer_app *app, const platform_event *event,
                               double *out_x, double *out_y)
{
    if (app == NULL || event == NULL || out_x == NULL || out_y == NULL ||
        !isfinite(app->backing_scale) || !(app->backing_scale > 0.0)) return false;
    double screen_x = (double)event->logical_x * app->backing_scale;
    double screen_y = (double)event->logical_y * app->backing_scale;
    *out_x = screen_x - (double)app->navigation_canvas.x;
    *out_y = screen_y - (double)app->navigation_canvas.y;
    return isfinite(*out_x) && isfinite(*out_y);
}

static void update_editor_cursor(viewer_app *app, const platform_event *event)
{
    if (app == NULL || app->window == NULL) return;
    platform_cursor_kind cursor = PLATFORM_CURSOR_ARROW;
    if (app->navigation_ready && app->navigation.dragging) {
        cursor = PLATFORM_CURSOR_CLOSED_HAND;
    } else if (app->editor != NULL && app->document_visible &&
        app->navigation_ready && event != NULL) {
        double pixel_x = (double)event->logical_x * app->backing_scale;
        double pixel_y = (double)event->logical_y * app->backing_scale;
        double local_x, local_y;
        if (canvas_contains(app, pixel_x, pixel_y) &&
            event_canvas_point(app, event, &local_x, &local_y)) {
            if (app->space_down ||
                viewer_editor_get_tool(app->editor) == UI_ICON_PAN)
                cursor = PLATFORM_CURSOR_OPEN_HAND;
            else cursor = viewer_editor_canvas_cursor(app->editor,
                &app->document.shape, &app->navigation, local_x, local_y);
        }
    }
    (void)platform_window_set_cursor(app->window, cursor);
}

void apply_navigation_fit(viewer_app *app, platform_viewer_action action)
{
    if (!app->navigation_ready || !app->document_visible) return;
    ui_viewer_navigation previous = app->navigation;
    bool changed = false;
    if (action == PLATFORM_ACTION_FIT_WINDOW) {
        changed = ui_viewer_navigation_fit(&app->navigation);
        if (changed) app->navigation_user_zoomed = false;
    } else if (action == PLATFORM_ACTION_FIT_WIDTH) {
        changed = ui_viewer_navigation_fit_width(&app->navigation);
        if (changed) app->navigation_user_zoomed = true;
    } else if (action == PLATFORM_ACTION_ACTUAL_SIZE) {
        changed = ui_viewer_navigation_actual_size(&app->navigation);
        if (changed) app->navigation_user_zoomed = true;
    }
    if (changed) navigation_changed(app, &previous);
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

void viewer_apply_navigation_key(viewer_app *app, ui_viewer_key key)
{
    if (!app->navigation_ready || !app->document_visible) return;
    ui_viewer_navigation previous = app->navigation;
    if (ui_viewer_navigation_key(&app->navigation, key)) {
        if (key == UI_VIEWER_KEY_FIT) app->navigation_user_zoomed = false;
        else if (key == UI_VIEWER_KEY_ZOOM_IN || key == UI_VIEWER_KEY_ZOOM_OUT)
            app->navigation_user_zoomed = true;
        navigation_changed(app, &previous);
    }
}

void viewer_toggle_checkerboard(viewer_app *app)
{
    app->checkerboard = !app->checkerboard;
    (void)platform_window_set_option_state(app->window,
        PLATFORM_ACTION_TOGGLE_CHECKERBOARD, app->checkerboard);
    app->frame_dirty = true;
}

void viewer_cycle_background(viewer_app *app)
{
    app->background_index = (uint8_t)((app->background_index + 1U) % 3U);
    if (app->checkerboard) viewer_toggle_checkerboard(app);
    app->frame_dirty = true;
}

void viewer_toggle_pixel_grid(viewer_app *app)
{
    app->pixel_grid = !app->pixel_grid;
    (void)platform_window_set_option_state(app->window,
        PLATFORM_ACTION_TOGGLE_PIXEL_GRID, app->pixel_grid);
    app->frame_dirty = true;
}

static bool editor_alignment_shortcut(const platform_event *event,
                                     viewer_editor_alignment *out_alignment)
{
    const uint32_t required = PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_OPTION;
    if (event == NULL || out_alignment == NULL ||
        (event->modifiers & required) != required ||
        (event->modifiers & PLATFORM_MODIFIER_CONTROL) != 0U) return false;
    switch (event->key_code) {
        case 123U: *out_alignment = VIEWER_EDITOR_ALIGN_LEFT; return true;
        case 124U: *out_alignment = VIEWER_EDITOR_ALIGN_RIGHT; return true;
        case 126U: *out_alignment = VIEWER_EDITOR_ALIGN_TOP; return true;
        case 125U: *out_alignment = VIEWER_EDITOR_ALIGN_BOTTOM; return true;
        case 4U: *out_alignment = VIEWER_EDITOR_ALIGN_HORIZONTAL_CENTER; return true;
        case 9U: *out_alignment = VIEWER_EDITOR_ALIGN_VERTICAL_CENTER; return true;
        default: return false;
    }
}

static void apply_key(viewer_app *app, const platform_event *event)
{
    bool loading_replacement = app->pending_new_document ||
        (app->job != NULL && app->job->kind == VIEWER_JOB_NEW_DOCUMENT) ||
        (replacement_load_active(app) && app->document_visible);
    bool allowed_command = event->modifiers == PLATFORM_MODIFIER_COMMAND &&
        (strcmp(event->text, "q") == 0 || strcmp(event->text, "Q") == 0 ||
         strcmp(event->text, "o") == 0 || strcmp(event->text, "O") == 0 ||
         strcmp(event->text, "r") == 0 || strcmp(event->text, "R") == 0);
    if (loading_replacement && !allowed_command) return;
    viewer_editor_alignment alignment;
    if (app->document_visible && app->editor != NULL &&
        !viewer_editor_text_input_active(app->editor) &&
        editor_alignment_shortcut(event, &alignment)) {
        uint32_t result = viewer_editor_align_selection(app->editor,
            &app->document.shape, alignment);
        viewer_process_editor_result(app, result);
        return;
    }
    if (app->document_visible && app->editor != NULL &&
        !viewer_editor_text_input_active(app->editor) &&
        viewer_editor_node_edit_active(app->editor)) {
        uint32_t node_result = viewer_editor_node_edit_key(app->editor,
            &app->document.shape, event->key_code, event->modifiers);
        if (node_result != 0U) {
            viewer_process_editor_result(app, node_result);
            return;
        }
    }
    if (event->key_code == 53U && app->navigation_ready &&
        app->navigation.dragging) {
        (void)ui_viewer_navigation_drag_end(&app->navigation);
        app->frame_dirty = true;
    }
    uint32_t editor_result = loading_replacement ? 0U :
        viewer_editor_key(app->editor, event->key_code,
            event->modifiers, event->text, event->text_len);
    if ((editor_result & VIEWER_EDITOR_DELETE_PATH_NODE) != 0U) {
        editor_result &= ~(uint32_t)VIEWER_EDITOR_DELETE_PATH_NODE;
        editor_result |= viewer_editor_delete_selected_path_node(app->editor,
            &app->document.shape);
    }
    uint32_t png_flags = (uint32_t)VIEWER_EDITOR_EXPORT_PNG |
        (uint32_t)VIEWER_EDITOR_COPY_PNG;
    if ((editor_result & png_flags) != 0U) {
        bool copy_png = (editor_result & VIEWER_EDITOR_COPY_PNG) != 0U;
        editor_result &= ~png_flags;
        (void)viewer_export_png(app, copy_png);
        editor_result |= VIEWER_EDITOR_HANDLED;
    }
    uint32_t clipboard_flags = (uint32_t)VIEWER_EDITOR_COPY |
        (uint32_t)VIEWER_EDITOR_CUT | (uint32_t)VIEWER_EDITOR_PASTE |
        (uint32_t)VIEWER_EDITOR_PASTE_IN_PLACE;
    if ((editor_result & clipboard_flags) != 0U) {
        uint32_t request = editor_result & clipboard_flags;
        editor_result &= ~clipboard_flags;
        uint8_t *clipboard = malloc(PLATFORM_CLIPBOARD_SVG_LIMIT);
        if (clipboard == NULL) {
            viewer_editor_set_status(app->editor, "Clipboard allocation failed");
        } else if ((request & VIEWER_EDITOR_COPY) != 0U ||
                   (request & VIEWER_EDITOR_CUT) != 0U) {
            size_t copied_length = 0U;
            if (!viewer_editor_copy_selection(app->editor, clipboard,
                    PLATFORM_CLIPBOARD_SVG_LIMIT, &copied_length) ||
                !platform_clipboard_write_svg(clipboard, copied_length)) {
                viewer_editor_set_status(app->editor,
                    "Could not copy the selected SVG fragment");
                editor_result |= VIEWER_EDITOR_HANDLED;
            } else if ((request & VIEWER_EDITOR_CUT) != 0U) {
                editor_result |= viewer_editor_cut_selection(app->editor);
            } else {
                viewer_editor_set_status(app->editor, "SVG copied to clipboard");
                editor_result |= VIEWER_EDITOR_HANDLED;
            }
        } else {
            size_t pasted_length = 0U;
            platform_clipboard_result clipboard_result = platform_clipboard_read_svg(
                clipboard, PLATFORM_CLIPBOARD_SVG_LIMIT, &pasted_length);
            if (clipboard_result == PLATFORM_CLIPBOARD_OK) {
                editor_result |= viewer_editor_paste_svg_from_clipboard(
                    app->editor, clipboard, pasted_length,
                    (request & VIEWER_EDITOR_PASTE_IN_PLACE) != 0U);
            } else {
                viewer_editor_set_status(app->editor,
                    clipboard_result == PLATFORM_CLIPBOARD_LIMIT ?
                        "Clipboard SVG exceeds the 64 KiB limit" :
                        "Clipboard does not contain an SVG fragment");
                editor_result |= VIEWER_EDITOR_HANDLED;
            }
        }
        free(clipboard);
    }
    if (editor_result != 0U) {
        viewer_process_editor_result(app, editor_result);
        return;
    }
    bool command_only = event->modifiers == PLATFORM_MODIFIER_COMMAND;
    bool plain_key = event->modifiers == 0U;
    if (command_only && (strcmp(event->text, "q") == 0 || strcmp(event->text, "Q") == 0)) {
        app->quit_all_requested = true;
        return;
    }
    if (command_only && (strcmp(event->text, "r") == 0 || strcmp(event->text, "R") == 0)) {
        viewer_request_reload_or_open(app);
        return;
    }
    if (command_only && (strcmp(event->text, "o") == 0 || strcmp(event->text, "O") == 0)) {
        (void)platform_window_open_panel(app->window);
        return;
    }
    if (command_only && (strcmp(event->text, "b") == 0 || strcmp(event->text, "B") == 0)) {
        viewer_cycle_background(app);
        return;
    }
    if (command_only && (strcmp(event->text, "t") == 0 || strcmp(event->text, "T") == 0)) {
        viewer_toggle_checkerboard(app);
        return;
    }
    if (command_only && strcmp(event->text, "0") == 0) {
        viewer_apply_action(app, PLATFORM_ACTION_FIT_WINDOW);
        return;
    }
    if (command_only && strcmp(event->text, "1") == 0) {
        viewer_apply_action(app, PLATFORM_ACTION_ACTUAL_SIZE);
        return;
    }
    if (command_only && (strcmp(event->text, "+") == 0 ||
            strcmp(event->text, "=") == 0)) {
        viewer_apply_navigation_key(app, UI_VIEWER_KEY_ZOOM_IN);
        return;
    }
    if (command_only && strcmp(event->text, "-") == 0) {
        viewer_apply_navigation_key(app, UI_VIEWER_KEY_ZOOM_OUT);
        return;
    }
    if (plain_key && (strcmp(event->text, "b") == 0 || strcmp(event->text, "B") == 0)) {
        viewer_cycle_background(app);
        return;
    }
    if (plain_key && (strcmp(event->text, "g") == 0 || strcmp(event->text, "G") == 0)) {
        viewer_toggle_pixel_grid(app);
        return;
    }
    if (plain_key && (strcmp(event->text, "w") == 0 || strcmp(event->text, "W") == 0)) {
        viewer_apply_action(app, PLATFORM_ACTION_FIT_WIDTH);
        return;
    }
    if (plain_key && strcmp(event->text, "1") == 0) {
        viewer_apply_action(app, PLATFORM_ACTION_ACTUAL_SIZE);
        return;
    }
    if (plain_key && event->key_code == 123U) { viewer_apply_navigation_key(app, UI_VIEWER_KEY_LEFT); return; }
    if (plain_key && event->key_code == 124U) { viewer_apply_navigation_key(app, UI_VIEWER_KEY_RIGHT); return; }
    if (plain_key && event->key_code == 125U) { viewer_apply_navigation_key(app, UI_VIEWER_KEY_DOWN); return; }
    if (plain_key && event->key_code == 126U) { viewer_apply_navigation_key(app, UI_VIEWER_KEY_UP); return; }
    if (plain_key && (strcmp(event->text, "+") == 0 || strcmp(event->text, "=") == 0)) {
        viewer_apply_navigation_key(app, UI_VIEWER_KEY_ZOOM_IN);
    } else if (plain_key && strcmp(event->text, "-") == 0) {
        viewer_apply_navigation_key(app, UI_VIEWER_KEY_ZOOM_OUT);
    } else if (plain_key && strcmp(event->text, "0") == 0) {
        viewer_apply_action(app, PLATFORM_ACTION_FIT_WINDOW);
    }
}

void viewer_apply_synthetic_key(viewer_app *app, uint16_t key_code,
                                uint32_t modifiers, const char *text,
                                size_t text_len)
{
    if (text_len >= sizeof(((platform_event *)0)->text)) return;
    platform_event event = {.type = PLATFORM_EVENT_KEY_DOWN,
        .key_code = key_code, .modifiers = modifiers, .text_len = text_len};
    if (text_len > 0U && text != NULL) memcpy(event.text, text, text_len);
    apply_key(app, &event);
}

static void update_pointer_status(viewer_app *app, const platform_event *event)
{
    app->pointer_valid = false;
    if (!app->navigation_ready || !app->document_visible ||
        !isfinite(app->backing_scale) || !(app->backing_scale > 0.0)) return;
    double screen_x = (double)event->logical_x * app->backing_scale;
    double screen_y = (double)event->logical_y * app->backing_scale;
    double x = (screen_x - (double)app->navigation_canvas.x -
        app->navigation.pan_x) / app->navigation.scale;
    double y = (screen_y - (double)app->navigation_canvas.y -
        app->navigation.pan_y) / app->navigation.scale;
    if (!isfinite(x) || !isfinite(y) || x < 0.0 || y < 0.0 ||
        x >= app->navigation.document_width || y >= app->navigation.document_height) return;
    app->pointer_doc_x = x;
    app->pointer_doc_y = y;
    app->pointer_valid = true;
}

void viewer_process_event(viewer_app *app, const platform_event *event)
{
    if (app->document_visible && app->editor != NULL)
        viewer_editor_set_motion_time(app->editor, event->timestamp_seconds);
    if (app->document_visible && viewer_timeline_process_event(
            &app->timeline, event, app->editor,
            app->width, app->height, app->backing_scale)) {
        if (viewer_timeline_take_editor_mutation(&app->timeline))
            viewer_process_editor_result(app,
                VIEWER_EDITOR_HANDLED | VIEWER_EDITOR_MUTATED);
        if (viewer_timeline_take_native_mutation(&app->timeline)) {
            (void)viewer_set_window_title(app);
            app->frame_dirty = true;
        }
        app->frame_dirty = true;
        (void)viewer_sync_canvas_geometry(app, false);
        return;
    }
    switch (event->type) {
        case PLATFORM_EVENT_ANIMATION_TICK:
            if (app->editor != NULL && viewer_editor_motion_tick(app->editor,
                    event->timestamp_seconds)) app->frame_dirty = true;
            if (app->editor != NULL)
                platform_window_request_animation_ticks(app->window,
                    viewer_editor_motion_active(app->editor));
            return;
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
            }
            viewer_editor_resize(app->editor, app->width, app->height, app->backing_scale);
            (void)viewer_sync_canvas_geometry(app, !app->navigation_user_zoomed);
            if (prior_scale != app->backing_scale) app->frame_dirty = true;
            return;
        }
        case PLATFORM_EVENT_POINTER_DOWN: {
            if (process_home_pointer_down(app, event)) return;
            if (replacement_load_active(app) && app->document_visible) return;
            double pixel_x = (double)event->logical_x * app->backing_scale;
            double pixel_y = (double)event->logical_y * app->backing_scale;
            if (!app->space_down && app->editor != NULL && app->navigation_ready &&
                app->document_visible &&
                isfinite(pixel_x) && isfinite(pixel_y)) {
                viewer_editor_guide_pointer_result guide_result =
                    viewer_editor_guide_pointer_down(app->editor,
                        &app->document.shape, &app->navigation,
                        app->navigation_canvas, pixel_x, pixel_y, event->button);
                if (guide_result == VIEWER_EDITOR_GUIDE_POINTER_CONTEXT_HORIZONTAL ||
                    guide_result == VIEWER_EDITOR_GUIDE_POINTER_CONTEXT_VERTICAL) {
                    (void)platform_window_show_ruler_context_menu(app->window,
                        guide_result == VIEWER_EDITOR_GUIDE_POINTER_CONTEXT_HORIZONTAL,
                        event->logical_x, event->logical_y);
                    return;
                }
                if (guide_result == VIEWER_EDITOR_GUIDE_POINTER_HANDLED) {
                    app->frame_dirty = true;
                    return;
                }
            }
            if (app->editor != NULL)
                viewer_editor_set_snap_bypass(app->editor,
                    (event->modifiers & PLATFORM_MODIFIER_SHIFT) != 0U);
            if (event->button == 0U && app->editor != NULL &&
                app->document_visible && isfinite(pixel_x) && isfinite(pixel_y) &&
                pixel_x >= 0.0 && pixel_y >= 0.0 &&
                pixel_x <= (double)UINT32_MAX && pixel_y <= (double)UINT32_MAX) {
                uint32_t action = viewer_editor_context_action_at(app->editor,
                    (uint32_t)pixel_x, (uint32_t)pixel_y);
                if (action != 0U) {
                    viewer_process_editor_result(app, viewer_editor_context_action_run(
                        app->editor, &app->document.shape, action));
                    return;
                }
            }
            if (app->editor != NULL && isfinite(pixel_x) && isfinite(pixel_y) &&
                pixel_x >= 0.0 && pixel_y >= 0.0 && pixel_x <= (double)UINT32_MAX &&
                pixel_y <= (double)UINT32_MAX) {
                uint32_t result = viewer_editor_pointer_down_click_count(app->editor,
                    (uint32_t)pixel_x, (uint32_t)pixel_y, event->button,
                    event->click_count);
                if (result != 0U) {
                    viewer_process_editor_result(app, result);
                    return;
                }
            }
            double local_x, local_y;
            if (event->button != 0U || !app->navigation_ready ||
                !canvas_contains(app, pixel_x, pixel_y) ||
                !event_canvas_point(app, event, &local_x, &local_y)) return;
            if (app->space_down || viewer_editor_get_tool(app->editor) == UI_ICON_PAN) {
                (void)ui_viewer_navigation_drag_begin(&app->navigation, local_x, local_y);
                update_editor_cursor(app, event);
                return;
            }
            if (viewer_editor_get_tool(app->editor) == UI_ICON_ZOOM) {
                ui_viewer_navigation previous = app->navigation;
                if (ui_viewer_navigation_zoom(&app->navigation, 1.25, local_x, local_y)) {
                    app->navigation_user_zoomed = true;
                    navigation_changed(app, &previous);
                }
                return;
            }
            uint32_t result = viewer_editor_canvas_pointer_down_click_count(app->editor,
                &app->document.shape, &app->navigation, local_x, local_y,
                event->modifiers, event->click_count);
            if (result != 0U) viewer_process_editor_result(app, result);
            return;
        }
        case PLATFORM_EVENT_POINTER_MOVE:
            update_pointer_status(app, event);
            app->frame_dirty = true;
            if (replacement_load_active(app) && app->document_visible) return;
            if (app->editor != NULL && app->navigation_ready && app->document_visible) {
                double pixel_x = (double)event->logical_x * app->backing_scale;
                double pixel_y = (double)event->logical_y * app->backing_scale;
                viewer_editor_set_snap_bypass(app->editor,
                    (event->modifiers & PLATFORM_MODIFIER_SHIFT) != 0U);
                if (viewer_editor_guide_pointer_move(app->editor,
                        &app->document.shape, &app->navigation,
                        app->navigation_canvas, pixel_x, pixel_y)) return;
            }
            if (app->editor != NULL) {
                double px = (double)event->logical_x * app->backing_scale;
                double py = (double)event->logical_y * app->backing_scale;
                if (isfinite(px) && isfinite(py) && px >= 0.0 && py >= 0.0 &&
                    px <= (double)UINT32_MAX && py <= (double)UINT32_MAX)
                    viewer_editor_pointer_move(app->editor, (uint32_t)px, (uint32_t)py);
                platform_window_request_animation_ticks(app->window,
                    viewer_editor_motion_active(app->editor));
            }
            if (app->navigation_ready && app->navigation.dragging) {
                ui_viewer_navigation previous = app->navigation;
                double px = (double)event->logical_x * app->backing_scale -
                    (double)app->navigation_canvas.x;
                double py = (double)event->logical_y * app->backing_scale -
                    (double)app->navigation_canvas.y;
                if (ui_viewer_navigation_drag(&app->navigation, px, py))
                    navigation_changed(app, &previous);
            } else if (app->navigation_ready && app->editor != NULL) {
                double local_x, local_y;
                if (event_canvas_point(app, event, &local_x, &local_y)) {
                    uint32_t result = viewer_editor_canvas_pointer_move_modifiers(
                        app->editor, &app->document.shape, &app->navigation,
                        local_x, local_y, event->modifiers);
                    if (result != 0U) viewer_process_editor_result(app, result);
                }
            }
            update_editor_cursor(app, event);
            return;
        case PLATFORM_EVENT_POINTER_UP:
            if (replacement_load_active(app) && app->document_visible) return;
            if (viewer_editor_guide_pointer_up(app->editor)) {
                app->frame_dirty = true;
                return;
            }
            viewer_editor_pointer_up(app->editor);
            if (app->navigation_ready && app->navigation.dragging) {
                (void)ui_viewer_navigation_drag_end(&app->navigation);
                app->frame_dirty = true;
                update_editor_cursor(app, event);
                return;
            }
            if (app->editor != NULL) {
                uint32_t result = viewer_editor_canvas_pointer_up(app->editor,
                    &app->document.shape);
                if (result != 0U) viewer_process_editor_result(app, result);
                else app->frame_dirty = true;
            }
            update_editor_cursor(app, event);
            return;
        case PLATFORM_EVENT_SCROLL: {
            if (app->editor != NULL) {
                double scale = isfinite(event->backing_scale) && event->backing_scale > 0.0 ?
                    event->backing_scale : app->backing_scale;
                double pixel_x = event->precise_x * scale;
                double pixel_y = event->precise_y * scale;
                if (isfinite(pixel_x) && isfinite(pixel_y) && pixel_x >= 0.0 &&
                    pixel_y >= 0.0 && pixel_x <= (double)UINT32_MAX &&
                    pixel_y <= (double)UINT32_MAX && viewer_editor_scroll_at(app->editor,
                        (uint32_t)pixel_x, (uint32_t)pixel_y, event->delta_y)) {
                    app->frame_dirty = true;
                    return;
                }
            }
            if (!app->navigation_ready) return;
            double scale = isfinite(event->backing_scale) && event->backing_scale > 0.0 ?
                event->backing_scale : app->backing_scale;
            double screen_x = event->precise_x * scale;
            double screen_y = event->precise_y * scale;
            if (!canvas_contains(app, screen_x, screen_y)) return;
            double local_x = event->precise_x -
                (double)app->navigation_canvas.x / scale;
            double local_y = event->precise_y -
                (double)app->navigation_canvas.y / scale;
            if ((event->modifiers & (PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_CONTROL)) != 0U ||
                viewer_editor_get_tool(app->editor) == UI_ICON_ZOOM) {
                ui_viewer_navigation previous = app->navigation;
                if (ui_viewer_navigation_wheel_scaled(&app->navigation, event->delta_y,
                        local_x, local_y, scale)) {
                    app->navigation_user_zoomed = true;
                    navigation_changed(app, &previous);
                }
            } else apply_pan_delta(app, event->delta_x, event->delta_y, scale);
            return;
        }
        case PLATFORM_EVENT_MAGNIFY:
            if (app->navigation_ready) {
                double scale = isfinite(event->backing_scale) && event->backing_scale > 0.0 ?
                    event->backing_scale : app->backing_scale;
                double screen_x = event->precise_x * scale;
                double screen_y = event->precise_y * scale;
                if (!canvas_contains(app, screen_x, screen_y)) return;
                double local_x = event->precise_x -
                    (double)app->navigation_canvas.x / scale;
                double local_y = event->precise_y -
                    (double)app->navigation_canvas.y / scale;
                ui_viewer_navigation previous = app->navigation;
                if (ui_viewer_navigation_pinch_scaled(&app->navigation,
                        event->magnification, local_x, local_y, scale)) {
                    app->navigation_user_zoomed = true;
                    navigation_changed(app, &previous);
                }
            }
            return;
        case PLATFORM_EVENT_KEY_DOWN:
            if (event->key_code == 49U &&
                !viewer_editor_text_input_active(app->editor)) {
                app->space_down = true;
                viewer_editor_set_navigation_overlays_hidden(app->editor, true);
                app->frame_dirty = true;
            }
            apply_key(app, event);
            return;
        case PLATFORM_EVENT_ACTION: viewer_apply_action(app, event->action); return;
        case PLATFORM_EVENT_COMMAND:
            viewer_apply_command(app, (viewer_command_id)event->command_id);
            return;
        case PLATFORM_EVENT_FILE_OPEN: {
            size_t length = viewer_event_path_length(event->path,
                PLATFORM_PATH_CAP - 1U);
            if (length > 0U && length < PLATFORM_PATH_CAP &&
                viewer_request_open(app, event->path, false)) {
                platform_window_hide_welcome(app->window);
                (void)platform_window_set_visible(app->window, true);
            }
            return;
        }
        case PLATFORM_EVENT_CLOSE_REQUEST: viewer_request_close(app); return;
        case PLATFORM_EVENT_DROP_ENTER:
            app->drop_hover = true;
            app->frame_dirty = true;
            return;
        case PLATFORM_EVENT_DROP_EXIT:
            app->drop_hover = false;
            app->frame_dirty = true;
            return;
        case PLATFORM_EVENT_APPEARANCE:
            app->theme = platform_window_is_dark_appearance(app->window) ?
                UI_THEME_DARK : UI_THEME_LIGHT;
            app->frame_dirty = true;
            return;
        case PLATFORM_EVENT_FOCUS:
        case PLATFORM_EVENT_OPEN:
        case PLATFORM_EVENT_KEY_UP:
            if (event->key_code == 49U) {
                app->space_down = false;
                viewer_editor_set_navigation_overlays_hidden(app->editor, false);
                app->frame_dirty = true;
            }
            return;
        case PLATFORM_EVENT_NONE: default: return;
    }
}

