#include "viewer_internal.h"

#include "platform/platform_settings.h"

void viewer_update_native_state(viewer_app *app)
{
    if (app == NULL || app->window == NULL) return;
    platform_command_state state = {0};
    state.has_document = app->document_visible && app->editor != NULL &&
        app->load_status == VIEWER_LOAD_OK;
    state.has_path = state.has_document && app->path[0] != '\0';
    state.dirty = state.has_document && viewer_editor_is_dirty(app->editor);
    state.can_undo = state.has_document && viewer_editor_can_undo(app->editor);
    state.can_redo = state.has_document && viewer_editor_can_redo(app->editor);
    state.has_selection = state.has_document && viewer_editor_has_selection(app->editor);
    state.checkerboard_enabled = app->checkerboard;
    state.pixel_grid_enabled = app->pixel_grid;
    if (state.has_document) {
        (void)viewer_editor_sidebar_visibility(app->editor,
            &state.structure_visible, &state.inspector_visible);
        platform_settings_store_sidebar_visibility(state.structure_visible,
            state.inspector_visible);
    }
    platform_window_update_command_state(app->window, &state);
}
