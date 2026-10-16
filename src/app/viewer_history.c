#include "viewer_internal.h"

bool viewer_history_callback(void *context, bool redo, bool execute)
{
    viewer_app *app = context;
    if (app == NULL || app->editor == NULL ||
        viewer_editor_workspace(app->editor) != VIEWER_WORKSPACE_ANIMATE ||
        app->document.timeline_session == NULL) return false;
    project_timeline_session *session = app->document.timeline_session;
    size_t available = redo ? project_timeline_session_redo_count(session) :
        project_timeline_session_undo_count(session);
    if (available == 0U) return false;
    if (!execute) return true;

    project_timeline_edit_result result = redo ?
        project_timeline_session_redo(session) :
        project_timeline_session_undo(session);
    if (result.error != PROJECT_TIMELINE_OK) return false;
    if (!viewer_timeline_refresh_native(&app->timeline)) {
        viewer_editor_set_status(app->editor,
            "Timeline history changed; timeline refresh failed");
    } else {
        viewer_editor_set_status(app->editor,
            redo ? "Timeline edit restored" : "Timeline edit undone");
    }
    viewer_timeline_invalidate_preview(&app->timeline);
    app->frame_dirty = true;
    (void)viewer_set_window_title(app);
    return true;
}
