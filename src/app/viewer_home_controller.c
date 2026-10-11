#include "viewer_internal.h"

#include <string.h>

void viewer_home_state_from_app(const viewer_app *app, viewer_home_state *out_state)
{
    if (out_state == NULL) return;
    *out_state = (viewer_home_state){0};
    if (app == NULL) return;
    out_state->has_document = app->document_visible && app->navigation_ready;
    out_state->loading = app->pending_open || (app->job != NULL &&
        app->job->kind == VIEWER_JOB_LOAD);
    out_state->load_failed = app->load_result_handled &&
        app->load_status != VIEWER_LOAD_OK;
    out_state->drop_hover = app->drop_hover;
    out_state->load_error = app->load_error;
    out_state->recent_count = app->recent_file_count;
    if (out_state->recent_count > VIEWER_HOME_RECENT_MAX)
        out_state->recent_count = VIEWER_HOME_RECENT_MAX;
    for (size_t index = 0U; index < out_state->recent_count; ++index)
        out_state->recent_labels[index] = app->recent_files[index].display_name;
    out_state->canvas = app->navigation_canvas;
}

