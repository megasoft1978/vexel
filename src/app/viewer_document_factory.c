#include "viewer_internal.h"

#include <string.h>

bool viewer_request_new_document(viewer_app *app)
{
    if (app == NULL || app->smoke_mode || !viewer_confirm_unsaved_changes(app,
            "Unsaved Changes", "Save changes before creating a new document?")) return false;
    if (!viewer_bump_generation(app)) return false;
    app->pending_open = false;
    app->pending_path[0] = '\0';
    app->pending_new_document = true;
    app->load_result_handled = false;
    app->load_status = VIEWER_LOAD_CANCELED;
    app->path[0] = '\0';
    app->file_stamp = (viewer_file_stamp){0};
    app->external_change_pending = false;
    app->editor_render_requested = false;
    app->document_visible = false;
    app->navigation_ready = false;
    app->navigation_user_zoomed = false;
    app->pointer_valid = false;
    app->sharp_plan_ready = false;
    viewer_copy_display_name("Untitled.svg", app->display_name);
    app->frame_dirty = true;
    platform_window_hide_welcome(app->window);
    (void)platform_window_set_visible(app->window, true);
    return viewer_set_window_title(app);
}
