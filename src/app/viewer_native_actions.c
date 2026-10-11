#include "viewer_internal.h"

#include "platform/platform_settings.h"

static void apply_zoom_to_selection(viewer_app *app)
{
    if (app == NULL || !app->navigation_ready || !app->document_visible ||
        app->editor == NULL) return;
    svg_shape_rect bounds;
    ui_viewer_navigation previous = app->navigation;
    if (!viewer_editor_get_selection_bounds(app->editor, &app->document.shape,
            &bounds)) {
        if (ui_viewer_navigation_fit(&app->navigation)) {
            app->navigation_user_zoomed = false;
            navigation_changed(app, &previous);
        }
        return;
    }
    if (!ui_viewer_navigation_zoom_to_bounds(&app->navigation, bounds.x, bounds.y,
            bounds.width, bounds.height, 32.0 * app->backing_scale)) return;
    app->navigation_user_zoomed = true;
    navigation_changed(app, &previous);
}

void viewer_request_reload_or_open(viewer_app *app)
{
    if (app->path[0] != '\0') (void)viewer_request_open(app, app->path, false);
    else (void)platform_window_open_panel(app->window);
}

static void toggle_sidebar(viewer_app *app, bool structure)
{
    if (app == NULL || app->editor == NULL) return;
    bool show_structure = true;
    bool show_inspector = true;
    if (!viewer_editor_sidebar_visibility(app->editor, &show_structure,
            &show_inspector)) return;
    if (structure) show_structure = !show_structure;
    else show_inspector = !show_inspector;
    if (!viewer_editor_set_sidebar_visibility(app->editor, show_structure,
            show_inspector)) return;
    (void)viewer_sync_canvas_geometry(app, !app->navigation_user_zoomed);
    app->frame_dirty = true;
}

static void apply_workspace_preference(viewer_app *app)
{
    platform_settings_values preferences = {0};
    platform_settings_get(&preferences);
    if (app->editor != NULL) {
        (void)viewer_editor_set_sidebar_visibility(app->editor,
            preferences.structure_visible, preferences.inspector_visible);
        (void)viewer_sync_canvas_geometry(app, !app->navigation_user_zoomed);
    }
    app->frame_dirty = true;
}

static void refresh_preferences(viewer_app *app)
{
    platform_settings_values preferences = {0};
    platform_settings_get(&preferences);
    app->checkerboard = preferences.default_checkerboard;
    app->pixel_grid = preferences.default_pixel_grid;
    app->modifier_scroll_zooms = preferences.modifier_scroll_zooms;
    apply_workspace_preference(app);
    (void)platform_window_set_option_state(app->window,
        PLATFORM_ACTION_TOGGLE_CHECKERBOARD, app->checkerboard);
    (void)platform_window_set_option_state(app->window,
        PLATFORM_ACTION_TOGGLE_PIXEL_GRID, app->pixel_grid);
    app->frame_dirty = true;
}

void viewer_apply_action(viewer_app *app, platform_viewer_action action)
{
    if (app == NULL) return;
    switch (action) {
        case PLATFORM_ACTION_RELOAD: viewer_request_reload_or_open(app); break;
        case PLATFORM_ACTION_TOGGLE_CHECKERBOARD: viewer_toggle_checkerboard(app); break;
        case PLATFORM_ACTION_CYCLE_BACKGROUND: viewer_cycle_background(app); break;
        case PLATFORM_ACTION_FIT_WIDTH:
        case PLATFORM_ACTION_FIT_WINDOW:
        case PLATFORM_ACTION_ACTUAL_SIZE: apply_navigation_fit(app, action); break;
        case PLATFORM_ACTION_TOGGLE_PIXEL_GRID: viewer_toggle_pixel_grid(app); break;
        case PLATFORM_ACTION_TOGGLE_RULERS:
            if (app->editor != NULL) {
                bool visible = !viewer_editor_rulers_visible(app->editor);
                if (viewer_editor_set_rulers_visible(app->editor, visible)) {
                    (void)platform_window_set_option_state(app->window, action, visible);
                    (void)viewer_sync_canvas_geometry(app,
                        !app->navigation_user_zoomed);
                    app->frame_dirty = true;
                }
            }
            break;
        case PLATFORM_ACTION_TOGGLE_GUIDES:
            if (app->editor != NULL) {
                bool visible = !viewer_editor_guides_visible(app->editor);
                if (viewer_editor_set_guides_visible(app->editor, visible)) {
                    (void)platform_window_set_option_state(app->window, action, visible);
                    app->frame_dirty = true;
                }
            }
            break;
        case PLATFORM_ACTION_TOGGLE_GUIDE_LOCK:
            if (app->editor != NULL) {
                bool locked = !viewer_editor_guides_locked(app->editor);
                if (viewer_editor_set_guides_locked(app->editor, locked)) {
                    (void)platform_window_set_option_state(app->window, action, locked);
                    app->frame_dirty = true;
                }
            }
            break;
        case PLATFORM_ACTION_CLEAR_HORIZONTAL_GUIDES:
        case PLATFORM_ACTION_CLEAR_VERTICAL_GUIDES:
            if (app->editor != NULL) {
                viewer_editor_guide_axis_kind axis =
                    action == PLATFORM_ACTION_CLEAR_HORIZONTAL_GUIDES ?
                    VIEWER_EDITOR_GUIDE_HORIZONTAL : VIEWER_EDITOR_GUIDE_VERTICAL;
                if (viewer_editor_clear_guides(app->editor, axis)) app->frame_dirty = true;
            }
            break;
        case PLATFORM_ACTION_ZOOM_TO_SELECTION: apply_zoom_to_selection(app); break;
        case PLATFORM_ACTION_UI_SCALE_100:
            if (app->editor != NULL && viewer_editor_set_ui_scale(app->editor, 100U))
                app->frame_dirty = true;
            break;
        case PLATFORM_ACTION_UI_SCALE_110:
            if (app->editor != NULL && viewer_editor_set_ui_scale(app->editor, 110U))
                app->frame_dirty = true;
            break;
        case PLATFORM_ACTION_UI_SCALE_120:
            if (app->editor != NULL && viewer_editor_set_ui_scale(app->editor, 120U))
                app->frame_dirty = true;
            break;
        case PLATFORM_ACTION_NEW_DOCUMENT:
            (void)viewer_request_new_document(app);
            break;
        case PLATFORM_ACTION_SAVE:
            (void)viewer_save_document(app, false);
            break;
        case PLATFORM_ACTION_SAVE_AS:
            (void)viewer_save_document(app, true);
            break;
        case PLATFORM_ACTION_EXPORT_PNG:
            (void)viewer_export_png(app, false);
            break;
        case PLATFORM_ACTION_UNDO:
            viewer_apply_synthetic_key(app, 6U, PLATFORM_MODIFIER_COMMAND, "z", 1U);
            break;
        case PLATFORM_ACTION_REDO:
            viewer_apply_synthetic_key(app, 6U,
                PLATFORM_MODIFIER_COMMAND | PLATFORM_MODIFIER_SHIFT, "z", 1U);
            break;
        case PLATFORM_ACTION_CUT:
            viewer_apply_synthetic_key(app, 7U, PLATFORM_MODIFIER_COMMAND, "x", 1U);
            break;
        case PLATFORM_ACTION_COPY:
            viewer_apply_synthetic_key(app, 8U, PLATFORM_MODIFIER_COMMAND, "c", 1U);
            break;
        case PLATFORM_ACTION_PASTE:
            viewer_apply_synthetic_key(app, 9U, PLATFORM_MODIFIER_COMMAND, "v", 1U);
            break;
        case PLATFORM_ACTION_DELETE:
            viewer_apply_synthetic_key(app, 51U, 0U, NULL, 0U);
            break;
        case PLATFORM_ACTION_SELECT_ALL:
            viewer_apply_synthetic_key(app, 0U, PLATFORM_MODIFIER_COMMAND, "a", 1U);
            break;
        case PLATFORM_ACTION_CLOSE_DOCUMENT:
            viewer_request_close(app);
            break;
        case PLATFORM_ACTION_ZOOM_IN:
            viewer_apply_navigation_key(app, UI_VIEWER_KEY_ZOOM_IN);
            break;
        case PLATFORM_ACTION_ZOOM_OUT:
            viewer_apply_navigation_key(app, UI_VIEWER_KEY_ZOOM_OUT);
            break;
        case PLATFORM_ACTION_TOGGLE_STRUCTURE:
            toggle_sidebar(app, true);
            break;
        case PLATFORM_ACTION_TOGGLE_INSPECTOR:
            toggle_sidebar(app, false);
            break;
        case PLATFORM_ACTION_APPLY_WORKSPACE_PREFERENCE:
            apply_workspace_preference(app);
            break;
        case PLATFORM_ACTION_APPLY_CHECKERBOARD_PREFERENCE: {
            platform_settings_values preferences = {0};
            platform_settings_get(&preferences);
            app->checkerboard = preferences.default_checkerboard;
            (void)platform_window_set_option_state(app->window,
                PLATFORM_ACTION_TOGGLE_CHECKERBOARD, app->checkerboard);
            app->frame_dirty = true;
            break;
        }
        case PLATFORM_ACTION_APPLY_PIXEL_GRID_PREFERENCE: {
            platform_settings_values preferences = {0};
            platform_settings_get(&preferences);
            app->pixel_grid = preferences.default_pixel_grid;
            (void)platform_window_set_option_state(app->window,
                PLATFORM_ACTION_TOGGLE_PIXEL_GRID, app->pixel_grid);
            app->frame_dirty = true;
            break;
        }
        case PLATFORM_ACTION_APPLY_NAVIGATION_PREFERENCE: {
            platform_settings_values preferences = {0};
            platform_settings_get(&preferences);
            app->modifier_scroll_zooms = preferences.modifier_scroll_zooms;
            break;
        }
        case PLATFORM_ACTION_REFRESH_PREFERENCES:
            refresh_preferences(app);
            break;
        case PLATFORM_ACTION_SHOW_HOME:
            platform_window_hide_welcome(app->window);
            (void)platform_window_set_visible(app->window, true);
            break;
        case PLATFORM_ACTION_QUIT: viewer_request_close(app); break;
        case PLATFORM_ACTION_NONE: default: break;
    }
}
