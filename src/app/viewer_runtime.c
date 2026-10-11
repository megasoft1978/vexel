#include "viewer.h"
#include "viewer_internal.h"

#include "mcp_broker_hook.h"
#include "platform/platform_metal.h"
#include "test_driver/native_injection.h"
#include "viewer_sessions.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#define VIEWER_INITIAL_WIDTH 1280
#define VIEWER_INITIAL_HEIGHT 800

typedef struct viewer_runtime {
    viewer_session_collection documents;
    bool smoke_mode;
    bool failed;
    unsigned smoke_poll_count;
} viewer_runtime;

static size_t runtime_bounded_length(const char *text, size_t limit)
{
    size_t length = 0U;
    if (text == NULL) return 0U;
    while (length <= limit && text[length] != '\0') length++;
    return length;
}

static viewer_app *runtime_create_session(viewer_runtime *runtime,
    const char *path, bool initial)
{
    if (runtime == NULL || runtime->documents.count >= VIEWER_SESSION_MAX) return NULL;
    size_t path_length = path == NULL ? 0U :
        runtime_bounded_length(path, PLATFORM_PATH_CAP - 1U);
    if (path != NULL && (path_length == 0U || path_length >= PLATFORM_PATH_CAP)) return NULL;
    viewer_app *app = calloc(1U, sizeof(*app));
    if (app == NULL) return NULL;
    viewer_timeline_init(&app->timeline);
    app->generation = VIEWER_GENERATION;
    app->running = true;
    app->smoke_mode = runtime->smoke_mode;
    app->backing_scale = 1.0;
    app->checkerboard = true;
    app->pixel_grid = true;
    app->load_status = VIEWER_LOAD_CANCELED;
    app->theme = UI_THEME_DARK;
    viewer_copy_display_name(path == NULL ? "Open SVG" : path,
        app->display_name);
    if (!viewer_tile_cache_init(&app->sharp_cache, app->generation)) goto failed;

    platform_task_config task_config = {1U, 1U, NULL, NULL, NULL};
    if (platform_task_queue_create(&task_config, &app->tasks) != PLATFORM_TASK_OK) goto failed;
    const char *title = path == NULL ? "vexel — Open SVG" : "vexel — Loading SVG";
    platform_window_config window_config = {title, strlen(title),
        VIEWER_INITIAL_WIDTH, VIEWER_INITIAL_HEIGHT, true};
    if (!platform_window_create(&window_config, &app->window) ||
        !viewer_allocate_frame(app, VIEWER_INITIAL_WIDTH, VIEWER_INITIAL_HEIGHT)) goto failed;
    app->theme = platform_window_is_dark_appearance(app->window) ?
        UI_THEME_DARK : UI_THEME_LIGHT;
    app->recent_file_count = platform_window_recent_files(app->window,
        app->recent_files, PLATFORM_RECENT_FILES_MAX);
    (void)platform_window_set_option_state(app->window,
        PLATFORM_ACTION_TOGGLE_CHECKERBOARD, app->checkerboard);
    (void)platform_window_set_option_state(app->window,
        PLATFORM_ACTION_TOGGLE_PIXEL_GRID, app->pixel_grid);
    (void)snprintf(app->title, sizeof(app->title), "%s", title);
    if (path != NULL && !viewer_request_open(app, path, initial)) goto failed;
    if (!viewer_sessions_add(&runtime->documents, app)) goto failed;
    return app;

failed:
    viewer_app_destroy(app);
    free(app);
    return NULL;
}

static void runtime_report_open_failure(viewer_runtime *runtime)
{
    viewer_app *app = runtime == NULL ? NULL : runtime->documents.active;
    if (app == NULL) return;
    if (runtime->documents.count >= VIEWER_SESSION_MAX) {
        (void)snprintf(app->load_error, sizeof(app->load_error),
            "maximum of %u open SVG windows reached", VIEWER_SESSION_MAX);
    } else {
        (void)snprintf(app->load_error, sizeof(app->load_error),
            "could not create another SVG window");
    }
    app->load_status = VIEWER_LOAD_FAILED;
    app->load_result_handled = true;
    app->failed_path[0] = '\0';
    app->frame_dirty = true;
}

static bool runtime_open_new(viewer_runtime *runtime, const char *path)
{
    viewer_app *app = runtime_create_session(runtime, path, true);
    if (app != NULL) return true;
    runtime_report_open_failure(runtime);
    return false;
}

static bool runtime_request_quit(viewer_runtime *runtime)
{
    if (runtime == NULL) return false;
    for (size_t index = 0U; index < runtime->documents.count; ++index) {
        viewer_app *app = runtime->documents.items[index];
        if (app->close_requested) continue;
        if (!viewer_confirm_unsaved_changes(app, "Quit vexel",
                "Save changes before quitting?")) return false;
    }
    for (size_t index = 0U; index < runtime->documents.count; ++index)
        if (!runtime->documents.items[index]->close_requested)
            viewer_close_confirmed(runtime->documents.items[index]);
    return true;
}

static bool runtime_script_native_smoke(viewer_app *app)
{
    bool okay = platform_test_inject_pointer(app->window, PLATFORM_EVENT_POINTER_DOWN,
            40, 40) &&
        platform_test_inject_pointer(app->window, PLATFORM_EVENT_POINTER_MOVE, 80, 72) &&
        platform_test_inject_pointer(app->window, PLATFORM_EVENT_POINTER_UP, 80, 72) &&
        platform_test_inject_magnify(app->window, 0.25, 120.0, 100.0) &&
        platform_test_inject_key_text(app->window, "0", 1U, 29U, 0U) &&
        platform_test_inject_key_text(app->window, "q", 1U, 12U,
            PLATFORM_MODIFIER_COMMAND);
    app->smoke_scripted = okay;
    return okay;
}

static void runtime_handle_event(viewer_runtime *runtime, viewer_app *app,
                                 const platform_event *event)
{
    if (event->type == PLATFORM_EVENT_FOCUS)
        (void)viewer_sessions_activate(&runtime->documents, app);
    if (event->type == PLATFORM_EVENT_FILE_NEW) {
        size_t length = runtime_bounded_length(event->path, PLATFORM_PATH_CAP - 1U);
        if (length > 0U && length < PLATFORM_PATH_CAP) (void)runtime_open_new(runtime, event->path);
        return;
    }
    if (event->type == PLATFORM_EVENT_FILE_DROP) {
        size_t length = runtime_bounded_length(event->path, PLATFORM_PATH_CAP - 1U);
        if (length > 0U && length < PLATFORM_PATH_CAP) {
            if (!app->document_visible) {
                (void)viewer_request_open(app, event->path, !app->pending_open);
            } else if (viewer_confirm_unsaved_changes(app, "Unsaved Changes",
                    "Save changes before opening the dropped SVG?"))
                (void)runtime_open_new(runtime, event->path);
        }
        app->drop_hover = false;
        app->frame_dirty = true;
        return;
    }
    if (event->type == PLATFORM_EVENT_ACTION &&
        event->action == PLATFORM_ACTION_NEW_WINDOW) {
        (void)runtime_open_new(runtime, NULL);
        return;
    }
    if (event->type == PLATFORM_EVENT_ACTION &&
        event->action == PLATFORM_ACTION_QUIT) {
        (void)runtime_request_quit(runtime);
        return;
    }
    viewer_process_event(app, event);
    if (app->quit_all_requested) {
        app->quit_all_requested = false;
        (void)runtime_request_quit(runtime);
    }
}

static void runtime_remove_session(viewer_runtime *runtime, size_t index)
{
    if (runtime == NULL || index >= runtime->documents.count) return;
    viewer_app *app = runtime->documents.items[index];
    if (app->smoke_mode && !app->fatal_error) {
        app->smoke_passed = app->smoke_scripted && app->close_requested &&
            app->document_visible && app->presented_frames > 0U;
    }
    if (app->fatal_error) runtime->failed = true;
    if (app->smoke_mode && !app->smoke_passed) runtime->failed = true;
    (void)viewer_sessions_remove(&runtime->documents, app);
    viewer_app_destroy(app);
    free(app);
}

static void runtime_sleep(void)
{
    const struct timespec delay = {0, 6000000L};
    (void)nanosleep(&delay, NULL);
}

static int run_viewer(const char *path, bool smoke_mode)
{
    if (path != NULL && (path[0] == '\0' ||
        runtime_bounded_length(path, PLATFORM_PATH_CAP - 1U) >= PLATFORM_PATH_CAP)) {
        (void)fputs("viewer: invalid or oversized SVG path\n", stderr);
        return EXIT_FAILURE;
    }
    if (!platform_init()) {
        (void)fputs("viewer: AppKit initialization failed\n", stderr);
        return EXIT_FAILURE;
    }
    viewer_runtime runtime = {.smoke_mode = smoke_mode};
    viewer_sessions_init(&runtime.documents);
    if (runtime_create_session(&runtime, path, true) == NULL) {
        (void)fputs("viewer: initial window or session creation failed\n", stderr);
        platform_shutdown();
        return EXIT_FAILURE;
    }
    while (runtime.documents.count > 0U) {
        for (size_t index = 0U; index < runtime.documents.count;) {
            viewer_app *app = runtime.documents.items[index];
            platform_event event;
            while (platform_poll_event(app->window, &event)) {
                runtime_handle_event(&runtime, app, &event);
                if (!app->running) break;
            }
            if (app->running) {
                viewer_app_tick(app);
                if (app->smoke_mode && !app->smoke_scripted && app->document_visible &&
                    app->presented_frames > 0U && !runtime_script_native_smoke(app)) {
                    (void)fputs("native viewer smoke: event injection failed\n", stderr);
                    app->fatal_error = true;
                    app->running = false;
                }
                if (app->smoke_mode && !app->close_requested &&
                    ++runtime.smoke_poll_count > 1000U) {
                    (void)fputs("native viewer smoke: timed out waiting for viewer events\n",
                        stderr);
                    app->fatal_error = true;
                    app->running = false;
                }
            }
            if (!app->running) runtime_remove_session(&runtime, index);
            else index++;
        }
        if (runtime.documents.active != NULL && runtime.documents.count > 0U)
            mcp_broker_hook_poll(runtime.documents.active);
        if (runtime.documents.count > 0U) runtime_sleep();
    }
    platform_shutdown();
    if (smoke_mode) {
        if (runtime.failed) {
            (void)fputs("native viewer smoke: failed\n", stderr);
            return EXIT_FAILURE;
        }
        (void)puts("native viewer smoke: drag/zoom/fit/quit passed\npresented frames: 1+");
    }
    return runtime.failed ? EXIT_FAILURE : EXIT_SUCCESS;
}

int app_run_viewer(const char *path)
{
    return run_viewer(path, false);
}

int app_run_native_viewer_smoke(const char *path)
{
    return run_viewer(path, true);
}
