#ifndef VEXEL_APP_VIEWER_SESSIONS_H
#define VEXEL_APP_VIEWER_SESSIONS_H

#include <stdbool.h>
#include <stddef.h>

#define VIEWER_SESSION_MAX 8U

typedef struct viewer_app viewer_app;
typedef struct viewer_session_collection {
    viewer_app *items[VIEWER_SESSION_MAX];
    size_t count;
    viewer_app *active;
} viewer_session_collection;

void viewer_sessions_init(viewer_session_collection *sessions);
bool viewer_sessions_add(viewer_session_collection *sessions, viewer_app *app);
bool viewer_sessions_activate(viewer_session_collection *sessions, viewer_app *app);
bool viewer_sessions_remove(viewer_session_collection *sessions, viewer_app *app);

#endif
