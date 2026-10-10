#include "viewer_internal.h"

#include "frame_compose.h"
#include "platform/macos/platform_clipboard.h"
#include "platform/macos/platform_dialog.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
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

static void apply_navigation_fit(viewer_app *app, platform_viewer_action action)
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

static void apply_navigation_key(viewer_app *app, ui_viewer_key key)
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

static void show_save_error(viewer_app *app, const char *title, const char *message)
{
    if (app->editor != NULL) viewer_editor_set_status(app->editor, message);
    (void)platform_dialog_show_error(title, strlen(title), message, strlen(message));
    app->frame_dirty = true;
}

static bool save_document(viewer_app *app, bool force_save_as)
{
    if (app == NULL || app->editor == NULL) return false;
    char chosen_path[PLATFORM_DIALOG_PATH_CAP] = {0};
    size_t chosen_length = 0U;
    const char *path = app->path;
    size_t path_length = event_path_length(path, PLATFORM_PATH_CAP - 1U);
    bool save_as = force_save_as || path_length == 0U || path_length >= PLATFORM_PATH_CAP;
    if (save_as) {
        size_t name_length = event_path_length(app->display_name, sizeof(app->display_name) - 1U);
        platform_dialog_status choice = platform_dialog_save_svg(app->display_name,
            name_length, chosen_path, sizeof(chosen_path), &chosen_length);
        if (choice != PLATFORM_DIALOG_ACCEPT) {
            if (choice == PLATFORM_DIALOG_ERROR)
                show_save_error(app, "Save Failed", "The save panel could not provide a valid SVG path.");
            return false;
        }
        path = chosen_path;
        path_length = chosen_length;
        size_t current_length = event_path_length(app->path, PLATFORM_PATH_CAP - 1U);
        if (app->path[0] != '\0' && path_length == current_length &&
            memcmp(path, app->path, path_length) == 0 && viewer_file_changed_on_disk(app)) {
            show_save_error(app, "File Changed on Disk",
                "The selected file changed outside Vexel. Save As to a different path to keep both versions.");
            return false;
        }
    } else if (viewer_file_changed_on_disk(app)) {
        show_save_error(app, "File Changed on Disk",
            "The file changed outside Vexel. Use Save As to keep both versions.");
        return false;
    }
    document_edit_save_result saved = viewer_editor_save(app->editor,
        (base_bytes){(const uint8_t *)path, path_length});
    if (saved.error != DOCUMENT_EDIT_OK) {
        const char *status = viewer_editor_status(app->editor);
        show_save_error(app, "Save Failed",
            status == NULL ? "The SVG could not be saved." : status);
        return false;
    }
    if (save_as && !viewer_update_active_path(app, path, path_length)) {
        show_save_error(app, "Save Failed",
            "The file was saved, but Vexel could not update its document path.");
        return false;
    }
    viewer_update_file_stamp(app);
    app->frame_dirty = true;
    (void)viewer_set_window_title(app);
    return true;
}

bool viewer_confirm_unsaved_changes(viewer_app *app, const char *title,
                                    const char *message)
{
    if (app == NULL || !viewer_editor_is_dirty(app->editor)) return true;
    platform_unsaved_choice choice = platform_dialog_unsaved_changes(title,
        strlen(title), message, strlen(message));
    if (choice == PLATFORM_UNSAVED_DONT_SAVE) return true;
    if (choice == PLATFORM_UNSAVED_SAVE) return save_document(app, false);
    if (app->editor != NULL) {
        viewer_editor_set_status(app->editor, choice == PLATFORM_UNSAVED_CANCEL ?
            "Action canceled" : "Could not show the unsaved-changes dialog");
    }
    app->frame_dirty = true;
    return false;
}

static void process_editor_result(viewer_app *app, uint32_t result)
{
    if ((result & VIEWER_EDITOR_SELECT_ALL) != 0U && app->document_visible)
        (void)viewer_editor_select_all(app->editor, &app->document.shape);
    if ((result & VIEWER_EDITOR_MUTATED) != 0U) {
        if (!viewer_bump_generation(app)) {
            app->fatal_error = true;
            app->running = false;
            return;
        }
        app->editor_render_requested = true;
        app->frame_dirty = true;
        (void)viewer_set_window_title(app);
    }
    if ((result & VIEWER_EDITOR_SAVE) != 0U) {
        (void)save_document(app, false);
    }
    if ((result & VIEWER_EDITOR_SAVE_AS) != 0U) {
        (void)save_document(app, true);
    }
    if ((result & VIEWER_EDITOR_OPEN) != 0U) {
        (void)platform_window_open_panel(app->window);
    }
    if ((result & VIEWER_EDITOR_RELOAD) != 0U) request_reload_or_open(app);
    if ((result & VIEWER_EDITOR_QUIT) != 0U) viewer_request_close(app);
    if ((result & VIEWER_EDITOR_HANDLED) != 0U) {
        app->frame_dirty = true;
        (void)viewer_sync_canvas_geometry(app, !app->navigation_user_zoomed);
        (void)viewer_set_window_title(app);
    }
}

static void apply_action(viewer_app *app, platform_viewer_action action)
{
    switch (action) {
        case PLATFORM_ACTION_RELOAD: request_reload_or_open(app); break;
        case PLATFORM_ACTION_TOGGLE_CHECKERBOARD: toggle_checkerboard(app); break;
        case PLATFORM_ACTION_CYCLE_BACKGROUND: cycle_background(app); break;
        case PLATFORM_ACTION_FIT_WIDTH:
        case PLATFORM_ACTION_FIT_WINDOW:
        case PLATFORM_ACTION_ACTUAL_SIZE: apply_navigation_fit(app, action); break;
        case PLATFORM_ACTION_TOGGLE_PIXEL_GRID: toggle_pixel_grid(app); break;
        case PLATFORM_ACTION_QUIT: viewer_request_close(app); break;
        case PLATFORM_ACTION_NONE: default: break;
    }
}

static void apply_key(viewer_app *app, const platform_event *event)
{
    if (event->key_code == 53U && app->navigation_ready &&
        app->navigation.dragging) {
        (void)ui_viewer_navigation_drag_end(&app->navigation);
        app->frame_dirty = true;
    }
    uint32_t editor_result = viewer_editor_key(app->editor, event->key_code,
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
                editor_result |= viewer_editor_paste_svg(app->editor, clipboard,
                    pasted_length,
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
        process_editor_result(app, editor_result);
        return;
    }
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
    if (command && strcmp(event->text, "0") == 0) {
        apply_action(app, PLATFORM_ACTION_FIT_WINDOW);
        return;
    }
    if (command && strcmp(event->text, "1") == 0) {
        apply_action(app, PLATFORM_ACTION_ACTUAL_SIZE);
        return;
    }
    if (command && (strcmp(event->text, "+") == 0 ||
            strcmp(event->text, "=") == 0)) {
        apply_navigation_key(app, UI_VIEWER_KEY_ZOOM_IN);
        return;
    }
    if (command && strcmp(event->text, "-") == 0) {
        apply_navigation_key(app, UI_VIEWER_KEY_ZOOM_OUT);
        return;
    }
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
    if (viewer_timeline_process_event(&app->timeline, event, app->editor,
            app->width, app->height, app->backing_scale)) {
        app->frame_dirty = true;
        return;
    }
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
            }
            viewer_editor_resize(app->editor, app->width, app->height, app->backing_scale);
            (void)viewer_sync_canvas_geometry(app, !app->navigation_user_zoomed);
            if (prior_scale != app->backing_scale) app->frame_dirty = true;
            return;
        }
        case PLATFORM_EVENT_POINTER_DOWN: {
            double pixel_x = (double)event->logical_x * app->backing_scale;
            double pixel_y = (double)event->logical_y * app->backing_scale;
            if (app->editor != NULL && isfinite(pixel_x) && isfinite(pixel_y) &&
                pixel_x >= 0.0 && pixel_y >= 0.0 && pixel_x <= (double)UINT32_MAX &&
                pixel_y <= (double)UINT32_MAX) {
                uint32_t result = viewer_editor_pointer_down(app->editor,
                    (uint32_t)pixel_x, (uint32_t)pixel_y, event->button);
                if (result != 0U) {
                    process_editor_result(app, result);
                    return;
                }
            }
            double local_x, local_y;
            if (event->button != 0U || !app->navigation_ready ||
                !canvas_contains(app, pixel_x, pixel_y) ||
                !event_canvas_point(app, event, &local_x, &local_y)) return;
            if (app->space_down || viewer_editor_get_tool(app->editor) == UI_ICON_PAN) {
                (void)ui_viewer_navigation_drag_begin(&app->navigation, local_x, local_y);
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
            if (result != 0U) process_editor_result(app, result);
            return;
        }
        case PLATFORM_EVENT_POINTER_MOVE:
            update_pointer_status(app, event);
            app->frame_dirty = true;
            if (app->editor != NULL) {
                double px = (double)event->logical_x * app->backing_scale;
                double py = (double)event->logical_y * app->backing_scale;
                if (isfinite(px) && isfinite(py) && px >= 0.0 && py >= 0.0 &&
                    px <= (double)UINT32_MAX && py <= (double)UINT32_MAX)
                    viewer_editor_pointer_move(app->editor, (uint32_t)px, (uint32_t)py);
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
                    uint32_t result = viewer_editor_canvas_pointer_move(app->editor,
                        &app->document.shape, &app->navigation, local_x, local_y);
                    if (result != 0U) process_editor_result(app, result);
                }
            }
            return;
        case PLATFORM_EVENT_POINTER_UP:
            viewer_editor_pointer_up(app->editor);
            if (app->navigation_ready && app->navigation.dragging) {
                (void)ui_viewer_navigation_drag_end(&app->navigation);
                app->frame_dirty = true;
                return;
            }
            if (app->editor != NULL) {
                uint32_t result = viewer_editor_canvas_pointer_up(app->editor,
                    &app->document.shape);
                if (result != 0U) process_editor_result(app, result);
                else app->frame_dirty = true;
            }
            return;
        case PLATFORM_EVENT_SCROLL: {
            if (app->editor != NULL) {
                double pixel_x = event->precise_x * event->backing_scale;
                if (isfinite(pixel_x) && pixel_x >= 0.0 && pixel_x <= (double)UINT32_MAX &&
                    viewer_editor_scroll(app->editor, (uint32_t)pixel_x, event->delta_y)) {
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
                !viewer_editor_text_input_active(app->editor)) app->space_down = true;
            apply_key(app, event);
            return;
        case PLATFORM_EVENT_ACTION: apply_action(app, event->action); return;
        case PLATFORM_EVENT_FILE_OPEN:
            if (event_path_length(event->path, PLATFORM_PATH_CAP - 1U) < PLATFORM_PATH_CAP)
                (void)viewer_request_open(app, event->path, false);
            return;
        case PLATFORM_EVENT_CLOSE_REQUEST: viewer_request_close(app); return;
        case PLATFORM_EVENT_OPEN:
        case PLATFORM_EVENT_KEY_UP:
            if (event->key_code == 49U) app->space_down = false;
            return;
        case PLATFORM_EVENT_NONE: default: return;
    }
}

