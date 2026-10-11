#include "viewer_internal.h"

#include <stdio.h>
#include <string.h>

bool viewer_set_window_title(viewer_app *app)
{
    if (app == NULL) return false;
    char next[VIEWER_TITLE_CAP];
    int amount;
    const char *dirty = viewer_editor_is_dirty(app->editor) ? "*" : "";
    if (app->load_result_handled && app->load_status != VIEWER_LOAD_OK) {
        amount = snprintf(next, sizeof(next), "vexel — %s%s — error: %s",
            app->display_name, dirty, viewer_error_reason(app));
    } else if (app->pending_open || app->pending_new_document ||
        (app->job != NULL && (app->job->kind == VIEWER_JOB_LOAD ||
            app->job->kind == VIEWER_JOB_NEW_DOCUMENT))) {
        amount = snprintf(next, sizeof(next), "vexel — %s%s — Loading…",
            app->display_name, dirty);
    } else if (app->navigation_ready && app->document_visible) {
        amount = snprintf(next, sizeof(next), "vexel — %s%s — %.0f%%",
            app->display_name, dirty, app->navigation.scale * 100.0);
    } else {
        amount = snprintf(next, sizeof(next), "vexel — %s%s",
            app->display_name, dirty);
    }
    if (amount < 0 || (size_t)amount >= sizeof(next)) return false;
    if (strcmp(next, app->title) == 0) return true;
    size_t length = (size_t)amount;
    if (!platform_window_set_title(app->window, next, length)) return false;
    memcpy(app->title, next, length + 1U);
    return true;
}
