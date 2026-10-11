#include "viewer_internal.h"

#include "platform/macos/platform_clipboard.h"

#include <stdio.h>

void viewer_process_editor_result(viewer_app *app, uint32_t result)
{
    if ((result & VIEWER_EDITOR_ZOOM_PRESET_REQUEST) != 0U) {
        result &= ~(uint32_t)VIEWER_EDITOR_ZOOM_PRESET_REQUEST;
        double scale = 0.0;
        if (app->navigation_ready && viewer_editor_take_zoom_preset(app->editor, &scale)) {
            ui_viewer_navigation previous = app->navigation;
            if (ui_viewer_navigation_zoom_to(&app->navigation, scale,
                    app->navigation.viewport_width * 0.5,
                    app->navigation.viewport_height * 0.5)) {
                app->navigation_user_zoomed = true;
                navigation_changed(app, &previous);
            }
        }
    }
    if ((result & VIEWER_EDITOR_DELETE_PATH_NODE) != 0U) {
        result &= ~(uint32_t)VIEWER_EDITOR_DELETE_PATH_NODE;
        result |= app->document_visible ?
            viewer_editor_delete_selected_path_node(app->editor,
                &app->document.shape) : VIEWER_EDITOR_HANDLED;
    }
    uint32_t node_options = result & ((uint32_t)VIEWER_EDITOR_NODE_CORNER |
        (uint32_t)VIEWER_EDITOR_NODE_SMOOTH);
    if (node_options != 0U) {
        result &= ~((uint32_t)VIEWER_EDITOR_NODE_CORNER |
            (uint32_t)VIEWER_EDITOR_NODE_SMOOTH);
        result |= viewer_editor_convert_selected_path_node(app->editor,
            app->document_visible ? &app->document.shape : NULL,
            (node_options & VIEWER_EDITOR_NODE_SMOOTH) != 0U);
    }
    viewer_command_id selected_command = VIEWER_COMMAND_NONE;
    bool run_selected_command = (result & VIEWER_EDITOR_COMMAND_SELECTED) != 0U &&
        viewer_editor_take_pending_command(app->editor, &selected_command);
    result &= ~(uint32_t)VIEWER_EDITOR_COMMAND_SELECTED;
    if ((result & VIEWER_EDITOR_SELECT_ALL) != 0U && app->document_visible)
        (void)viewer_editor_select_all(app->editor, &app->document.shape);
    if (app->editor != NULL && viewer_timeline_track_count(&app->timeline) > 0U) {
        bool requested_open = viewer_editor_timeline_visible(app->editor);
        if (requested_open != app->timeline.dock_open)
            (void)viewer_timeline_dock_set_open(&app->timeline, requested_open);
    }
    bool document_mutated = (result & VIEWER_EDITOR_MUTATED) != 0U;
    bool preview_changed = (result & VIEWER_EDITOR_PREVIEW_CHANGED) != 0U;
    if (document_mutated || preview_changed) {
        if (!viewer_bump_generation(app)) {
            app->fatal_error = true;
            app->running = false;
            return;
        }
        app->editor_render_requested = true;
        app->frame_dirty = true;
        if (document_mutated) (void)viewer_set_window_title(app);
    }
    if ((result & VIEWER_EDITOR_SAVE) != 0U) {
        (void)viewer_save_document(app, false);
    }
    if ((result & VIEWER_EDITOR_SAVE_AS) != 0U) {
        (void)viewer_save_document(app, true);
    }
    if ((result & VIEWER_EDITOR_OPEN) != 0U) {
        (void)platform_window_open_panel(app->window);
    }
    if ((result & VIEWER_EDITOR_RELOAD) != 0U) viewer_request_reload_or_open(app);
    if ((result & VIEWER_EDITOR_QUIT) != 0U) viewer_request_close(app);
    if ((result & VIEWER_EDITOR_FIT_CANVAS) != 0U)
        apply_navigation_fit(app, PLATFORM_ACTION_FIT_WINDOW);
    if ((result & VIEWER_EDITOR_ACTUAL_SIZE) != 0U)
        apply_navigation_fit(app, PLATFORM_ACTION_ACTUAL_SIZE);
    if ((result & VIEWER_EDITOR_COPY_FULL_ID) != 0U) {
        uint8_t identifier[64U * 1024U];
        size_t length = 0U;
        if (!viewer_editor_copy_full_id(app->editor, identifier,
                sizeof(identifier), &length) ||
            !platform_clipboard_write_text(identifier, length))
            viewer_editor_set_status(app->editor, "Could not copy the full SVG ID");
        else viewer_editor_set_status(app->editor, "Full SVG ID copied");
    }
    if ((result & VIEWER_EDITOR_HANDLED) != 0U) {
        app->frame_dirty = true;
        (void)viewer_sync_canvas_geometry(app, !app->navigation_user_zoomed);
        (void)viewer_set_window_title(app);
    }
    if (run_selected_command && !app->fatal_error)
        viewer_apply_command(app, selected_command);
    if (app->editor != NULL)
        platform_window_request_animation_ticks(app->window,
            viewer_editor_motion_active(app->editor));
}
