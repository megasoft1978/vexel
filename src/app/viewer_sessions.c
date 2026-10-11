#include "viewer_sessions.h"

void viewer_sessions_init(viewer_session_collection *sessions)
{
    if (sessions != NULL) *sessions = (viewer_session_collection){0};
}

bool viewer_sessions_add(viewer_session_collection *sessions, viewer_app *app)
{
    if (sessions == NULL || app == NULL || sessions->count >= VIEWER_SESSION_MAX)
        return false;
    for (size_t index = 0U; index < sessions->count; ++index)
        if (sessions->items[index] == app) return false;
    sessions->items[sessions->count++] = app;
    sessions->active = app;
    return true;
}

bool viewer_sessions_activate(viewer_session_collection *sessions, viewer_app *app)
{
    if (sessions == NULL || app == NULL) return false;
    for (size_t index = 0U; index < sessions->count; ++index) {
        if (sessions->items[index] == app) {
            sessions->active = app;
            return true;
        }
    }
    return false;
}

bool viewer_sessions_remove(viewer_session_collection *sessions, viewer_app *app)
{
    if (sessions == NULL || app == NULL) return false;
    size_t index = 0U;
    while (index < sessions->count && sessions->items[index] != app) index++;
    if (index == sessions->count) return false;
    for (size_t next = index + 1U; next < sessions->count; ++next)
        sessions->items[next - 1U] = sessions->items[next];
    sessions->items[--sessions->count] = NULL;
    if (sessions->active == app)
        sessions->active = sessions->count == 0U ? NULL : sessions->items[sessions->count - 1U];
    return true;
}
