#define _POSIX_C_SOURCE 200809L

#include "viewer.h"

#include "frame_compose.h"
#include "platform/platform_metal.h"
#include "platform/platform_task.h"
#include "platform/platform_window.h"
#include "test_driver/native_injection.h"
#include "viewer_load.h"

#include <math.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#define VIEWER_TASK_ID UINT64_C(1)
#define VIEWER_GENERATION UINT64_C(1)
#define VIEWER_FRAME_MAX_AXIS 8192U
#define VIEWER_TITLE_CAP 1024U

typedef struct viewer_app {
    platform_window *window;
    platform_task_queue *tasks;
    viewer_loaded_document document;
    atomic_bool load_cleanup_complete;
    viewer_load_status load_status;
    uint64_t generation;
    uint32_t width;
    uint32_t height;
    double backing_scale;
    ui_viewer_navigation navigation;
    ui_viewer_progress progress;
    uint8_t *frame;
    size_t frame_bytes;
    char display_name[256];
    char title[VIEWER_TITLE_CAP];
    char diagnostic[192];
    bool navigation_ready;
    bool frame_dirty;
    bool load_submitted;
    bool load_result_handled;
    bool close_requested;
    bool running;
    bool fatal_error;
    bool smoke_mode;
    bool smoke_scripted;
    bool smoke_passed;
    unsigned presented_frames;
} viewer_app;

static size_t bounded_length(const char *text, size_t limit)
{
    size_t length = 0U;
    if (text == NULL) return 0U;
    while (length <= limit && text[length] != '\0') length++;
    return length;
}

static void copy_display_name(const char *path, char out[256])
{
    const char *name = path;
    for (const char *cursor = path; *cursor != '\0'; cursor++) {
        if (*cursor == '/') name = cursor + 1;
    }
    if (*name == '\0') name = "SVG";
    size_t length = bounded_length(name, 240U);
    if (length > 240U) length = 240U;
    for (size_t index = 0U; index < length; index++) {
        unsigned char value = (unsigned char)name[index];
        out[index] = value >= 32U && value < 127U ? (char)value : '?';
    }
    out[length] = '\0';
}

static const char *viewer_error_reason(const viewer_app *app)
{
    if (app->document.shape_error != SVG_SHAPE_ERROR_NONE) {
        const char *name = svg_shape_error_name(app->document.shape_error);
        return name == NULL ? "unknown SVG error" : name;
    }
    return app->document.load_error[0] == '\0' ? "document load failed" :
        app->document.load_error;
}

static bool set_window_title(viewer_app *app)
{
    char next[VIEWER_TITLE_CAP];
    int amount = 0;
    if (app->document.shape_error != SVG_SHAPE_ERROR_NONE ||
        app->document.load_error[0] != '\0') {
        if (app->document.shape_error != SVG_SHAPE_ERROR_NONE) {
            amount = snprintf(next, sizeof(next), "vexel — %s — unsupported: %s at byte %zu",
                app->display_name, viewer_error_reason(app),
                app->document.error_offset);
        } else {
            amount = snprintf(next, sizeof(next), "vexel — %s — error: %s",
                app->display_name, viewer_error_reason(app));
        }
    } else if (app->navigation_ready) {
        amount = snprintf(next, sizeof(next), "vexel — %s — %.0f%%",
                          app->display_name, app->navigation.scale * 100.0);
    } else {
        amount = snprintf(next, sizeof(next), "vexel — %s", app->display_name);
    }
    if (amount < 0 || (size_t)amount >= sizeof(next)) return false;
    if (strcmp(next, app->title) == 0) return true;
    size_t length = (size_t)amount;
    if (!platform_window_set_title(app->window, next, length)) return false;
    memcpy(app->title, next, length + 1U);
    return true;
}

static bool allocate_frame(viewer_app *app, uint32_t width, uint32_t height)
{
    if (app == NULL || width == 0U || height == 0U ||
        width > VIEWER_FRAME_MAX_AXIS || height > VIEWER_FRAME_MAX_AXIS) {
        return false;
    }
    size_t w = (size_t)width;
    size_t h = (size_t)height;
    if (w > SIZE_MAX / h) return false;
    size_t pixels = w * h;
    if (pixels > SIZE_MAX / 4U) return false;
    size_t bytes = pixels * 4U;
    uint8_t *frame = malloc(bytes);
    if (frame == NULL) return false;
    free(app->frame);
    app->frame = frame;
    app->frame_bytes = bytes;
    app->width = width;
    app->height = height;
    app->frame_dirty = true;
    return true;
}

typedef struct viewer_load_context {
    viewer_app *app;
    const char *path;
} viewer_load_context;

static bool viewer_load_callback(void *context,
                                 const platform_task_cancel_token *token,
                                 int64_t *out_value)
{
    viewer_load_context *load = context;
    if (load == NULL || load->app == NULL || out_value == NULL) return false;
    viewer_app *app = load->app;
    app->load_status = viewer_load_document(load->path, token, &app->document);
    if (app->load_status == VIEWER_LOAD_OK) {
        *out_value = (int64_t)app->document.tile.byte_count;
        return true;
    }
    return false;
}

static void viewer_load_cleanup(void *context)
{
    viewer_load_context *load = context;
    if (load != NULL && load->app != NULL) {
        atomic_store_explicit(&load->app->load_cleanup_complete, true, memory_order_release);
    }
}

static bool navigation_differs(const ui_viewer_navigation *left,
                               const ui_viewer_navigation *right)
{
    return left->scale != right->scale || left->pan_x != right->pan_x ||
        left->pan_y != right->pan_y || left->viewport_width != right->viewport_width ||
        left->viewport_height != right->viewport_height;
}

static void mark_navigation_changed(viewer_app *app,
                                   const ui_viewer_navigation *previous)
{
    if (navigation_differs(previous, &app->navigation)) {
        app->frame_dirty = true;
        (void)set_window_title(app);
    }
}

static void viewer_report_load_failure(const viewer_app *app)
{
    if (app->document.shape_error != SVG_SHAPE_ERROR_NONE) {
        (void)fprintf(stderr, "viewer: %s at byte %zu\n",
            svg_shape_error_name(app->document.shape_error), app->document.error_offset);
    } else {
        (void)fprintf(stderr, "viewer: %s\n", viewer_error_reason(app));
    }
}

static void finish_load(viewer_app *app, bool succeeded)
{
    app->load_result_handled = true;
    if (app->close_requested) return;
    if (succeeded && app->load_status == VIEWER_LOAD_OK) {
        if (!ui_viewer_navigation_init(&app->navigation, (double)app->width,
                (double)app->height, (double)app->document.tile.width,
                (double)app->document.tile.height) ||
            !ui_viewer_navigation_fit(&app->navigation)) {
            app->document.shape_error = SVG_SHAPE_ERROR_VIEWPORT;
            app->document.error_offset = 0U;
            viewer_report_load_failure(app);
        } else {
            app->navigation_ready = true;
            viewer_navigation_clamp(&app->navigation);
            (void)ui_viewer_progress_update(&app->progress, app->generation, 100U,
                UI_PROGRESS_COMPLETE, "Ready", 5U);
        }
    } else if (app->load_status != VIEWER_LOAD_CANCELED) {
        (void)ui_viewer_progress_update(&app->progress, app->generation, 0U,
            UI_PROGRESS_FAILED, "Failed", 6U);
        viewer_report_load_failure(app);
    }
    app->frame_dirty = true;
    (void)set_window_title(app);
}

static void handle_task_result(viewer_app *app)
{
    if (!app->load_submitted || app->load_result_handled || app->close_requested) return;
    platform_task_result result = {0};
    if (platform_task_poll(app->tasks, &result) && result.task_id == VIEWER_TASK_ID &&
        result.generation == app->generation) {
        finish_load(app, result.succeeded);
    }
}

static void apply_pan_delta(viewer_app *app, double delta_x, double delta_y,
                            double backing_scale)
{
    if (!app->navigation_ready || !isfinite(delta_x) || !isfinite(delta_y) ||
        !isfinite(backing_scale) || !(backing_scale > 0.0)) return;
    ui_viewer_navigation previous = app->navigation;
    double pan_x = app->navigation.pan_x + delta_x * backing_scale;
    double pan_y = app->navigation.pan_y + delta_y * backing_scale;
    if (!isfinite(pan_x) || !isfinite(pan_y)) return;
    app->navigation.pan_x = pan_x;
    app->navigation.pan_y = pan_y;
    viewer_navigation_clamp(&app->navigation);
    mark_navigation_changed(app, &previous);
}

static void apply_key(viewer_app *app, const platform_event *event)
{
    bool command = (event->modifiers & PLATFORM_MODIFIER_COMMAND) != 0U;
    if (command && (strcmp(event->text, "q") == 0 || strcmp(event->text, "Q") == 0)) {
        app->close_requested = true;
        return;
    }
    if (!app->navigation_ready) return;
    ui_viewer_key key;
    bool recognized = true;
    if (event->key_code == 123U) key = UI_VIEWER_KEY_LEFT;
    else if (event->key_code == 124U) key = UI_VIEWER_KEY_RIGHT;
    else if (event->key_code == 125U) key = UI_VIEWER_KEY_DOWN;
    else if (event->key_code == 126U) key = UI_VIEWER_KEY_UP;
    else if (strcmp(event->text, "+") == 0 || strcmp(event->text, "=") == 0) {
        key = UI_VIEWER_KEY_ZOOM_IN;
    } else if (strcmp(event->text, "-") == 0) {
        key = UI_VIEWER_KEY_ZOOM_OUT;
    } else if (strcmp(event->text, "0") == 0) {
        key = UI_VIEWER_KEY_FIT;
    } else {
        recognized = false;
        key = UI_VIEWER_KEY_FIT;
    }
    if (!recognized) return;
    ui_viewer_navigation previous = app->navigation;
    if (ui_viewer_navigation_key(&app->navigation, key)) {
        viewer_navigation_clamp(&app->navigation);
        mark_navigation_changed(app, &previous);
    }
}

static void process_event(viewer_app *app, const platform_event *event)
{
    switch (event->type) {
        case PLATFORM_EVENT_RESIZE:
        case PLATFORM_EVENT_SCALE: {
            if (event->pixel_width == 0U || event->pixel_height == 0U ||
                event->pixel_width > VIEWER_FRAME_MAX_AXIS ||
                event->pixel_height > VIEWER_FRAME_MAX_AXIS ||
                !isfinite(event->backing_scale) || !(event->backing_scale > 0.0)) {
                app->fatal_error = true;
                app->running = false;
                return;
            }
            bool resized = event->pixel_width != app->width || event->pixel_height != app->height;
            app->backing_scale = event->backing_scale;
            if (resized) {
                if (!allocate_frame(app, event->pixel_width, event->pixel_height)) {
                    app->fatal_error = true;
                    app->running = false;
                    return;
                }
                if (app->navigation_ready) {
                    ui_viewer_navigation previous = app->navigation;
                    if (ui_viewer_navigation_resize(&app->navigation,
                            (double)app->width, (double)app->height)) {
                        viewer_navigation_clamp(&app->navigation);
                        mark_navigation_changed(app, &previous);
                    }
                }
            }
            return;
        }
        case PLATFORM_EVENT_POINTER_DOWN:
            if (app->navigation_ready && event->button == 0U) {
                (void)ui_viewer_navigation_drag_begin_scaled(&app->navigation,
                    (double)event->logical_x, (double)event->logical_y, app->backing_scale);
            }
            return;
        case PLATFORM_EVENT_POINTER_MOVE:
            if (app->navigation_ready && app->navigation.dragging) {
                ui_viewer_navigation previous = app->navigation;
                if (ui_viewer_navigation_drag_scaled(&app->navigation,
                        (double)event->logical_x, (double)event->logical_y,
                        app->backing_scale)) {
                    viewer_navigation_clamp(&app->navigation);
                    mark_navigation_changed(app, &previous);
                }
            }
            return;
        case PLATFORM_EVENT_POINTER_UP:
            if (app->navigation_ready && app->navigation.dragging) {
                (void)ui_viewer_navigation_drag_end(&app->navigation);
            }
            return;
        case PLATFORM_EVENT_SCROLL:
            if (!app->navigation_ready) return;
            if ((event->modifiers & (PLATFORM_MODIFIER_COMMAND |
                                     PLATFORM_MODIFIER_CONTROL)) != 0U) {
                ui_viewer_navigation previous = app->navigation;
                if (ui_viewer_navigation_wheel_scaled(&app->navigation, event->delta_y,
                        event->precise_x, event->precise_y, event->backing_scale)) {
                    viewer_navigation_clamp(&app->navigation);
                    mark_navigation_changed(app, &previous);
                }
            } else {
                apply_pan_delta(app, event->delta_x, event->delta_y, event->backing_scale);
            }
            return;
        case PLATFORM_EVENT_MAGNIFY:
            if (app->navigation_ready) {
                ui_viewer_navigation previous = app->navigation;
                if (ui_viewer_navigation_pinch_scaled(&app->navigation,
                        event->magnification, event->precise_x, event->precise_y,
                        event->backing_scale)) {
                    viewer_navigation_clamp(&app->navigation);
                    mark_navigation_changed(app, &previous);
                }
            }
            return;
        case PLATFORM_EVENT_KEY_DOWN:
            apply_key(app, event);
            return;
        case PLATFORM_EVENT_CLOSE_REQUEST:
            app->close_requested = true;
            return;
        case PLATFORM_EVENT_OPEN:
        case PLATFORM_EVENT_NONE:
        case PLATFORM_EVENT_KEY_UP:
        default:
            return;
    }
}

static bool compose_and_present(viewer_app *app)
{
    if (!app->frame_dirty || app->frame == NULL || app->width == 0U || app->height == 0U) {
        return true;
    }
    viewer_frame_kind kind = VIEWER_FRAME_LOADING;
    const raster_tile *tile = NULL;
    const ui_viewer_navigation *navigation = NULL;
    const char *message = NULL;
    if (app->load_result_handled &&
        (app->load_status != VIEWER_LOAD_OK ||
         app->document.shape_error != SVG_SHAPE_ERROR_NONE ||
         app->document.load_error[0] != '\0')) {
        kind = VIEWER_FRAME_ERROR;
        message = viewer_error_reason(app);
    } else if (app->navigation_ready) {
        kind = VIEWER_FRAME_DOCUMENT;
        tile = &app->document.tile;
        navigation = &app->navigation;
    }
    if (!viewer_compose_frame(tile, navigation, &app->progress, kind, message,
            app->frame, app->frame_bytes, app->width, app->height)) {
        (void)fputs("viewer: frame composition failed\n", stderr);
        return false;
    }
    app->frame_dirty = false;
    if (!platform_metal_present(app->window, app->frame, app->frame_bytes,
            app->width, app->height, app->diagnostic, sizeof(app->diagnostic))) {
        (void)fprintf(stderr, "viewer: Metal presentation failed: %s\n",
            app->diagnostic[0] == '\0' ? "unknown error" : app->diagnostic);
        return false;
    }
    app->presented_frames++;
    return true;
}

static bool script_native_smoke(viewer_app *app)
{
    platform_window *window = app->window;
    bool okay = platform_test_inject_pointer(window, PLATFORM_EVENT_POINTER_DOWN, 40, 40) &&
        platform_test_inject_pointer(window, PLATFORM_EVENT_POINTER_MOVE, 80, 72) &&
        platform_test_inject_pointer(window, PLATFORM_EVENT_POINTER_UP, 80, 72) &&
        platform_test_inject_magnify(window, 0.25, 120.0, 100.0) &&
        platform_test_inject_key_text(window, "0", 1U, 29U, 0U) &&
        platform_test_inject_key_text(window, "q", 1U, 12U, PLATFORM_MODIFIER_COMMAND);
    app->smoke_scripted = okay;
    return okay;
}

static void request_cancel(viewer_app *app)
{
    if (!app->load_submitted || atomic_load_explicit(&app->load_cleanup_complete,
                                                       memory_order_acquire)) {
        return;
    }
    (void)platform_task_cancel(app->tasks, VIEWER_TASK_ID);
    if (app->generation < UINT64_MAX) {
        uint64_t next_generation = app->generation + 1U;
        if (platform_task_advance_generation(app->tasks, next_generation) == PLATFORM_TASK_OK) {
            app->generation = next_generation;
        }
    }
}

static void idle_sleep(void)
{
    const struct timespec delay = {0, 6000000L};
    (void)nanosleep(&delay, NULL);
}

static void destroy_viewer(viewer_app *app)
{
    if (app->tasks != NULL) {
        if (app->load_submitted && !atomic_load_explicit(&app->load_cleanup_complete,
                                                        memory_order_acquire)) {
            request_cancel(app);
        }
        platform_task_shutdown(app->tasks);
        platform_task_destroy(app->tasks);
        app->tasks = NULL;
    }
    if (app->window != NULL) {
        platform_metal_reset(app->window);
        platform_window_destroy(app->window);
        app->window = NULL;
    }
    free(app->frame);
    app->frame = NULL;
    viewer_loaded_document_destroy(&app->document);
    platform_shutdown();
}

static int run_viewer(const char *path, bool smoke_mode)
{
    if (path == NULL || path[0] == '\0' || bounded_length(path, 4096U) > 4096U) {
        (void)fputs("viewer: invalid or oversized SVG path\n", stderr);
        return EXIT_FAILURE;
    }
    viewer_app app = {0};
    app.generation = VIEWER_GENERATION;
    app.running = true;
    app.smoke_mode = smoke_mode;
    app.load_status = VIEWER_LOAD_CANCELED;
    copy_display_name(path, app.display_name);
    atomic_init(&app.load_cleanup_complete, false);
    if (!ui_viewer_progress_update(&app.progress, app.generation, 0U,
                                   UI_PROGRESS_ACTIVE, "Loading SVG", 11U)) {
        (void)fputs("viewer: progress state initialization failed\n", stderr);
        return EXIT_FAILURE;
    }
    if (!platform_init()) {
        (void)fputs("viewer: AppKit initialization failed\n", stderr);
        return EXIT_FAILURE;
    }
    platform_task_config task_config = {1U, 1U, NULL, NULL, NULL};
    if (platform_task_queue_create(&task_config, &app.tasks) != PLATFORM_TASK_OK) {
        (void)fputs("viewer: worker queue creation failed\n", stderr);
        platform_shutdown();
        return EXIT_FAILURE;
    }
    char initial_title[VIEWER_TITLE_CAP];
    int title_length = snprintf(initial_title, sizeof(initial_title), "vexel — %s",
                                app.display_name);
    platform_window_config window_config = {
        initial_title,
        title_length > 0 ? (size_t)title_length : 0U,
        1280,
        800,
        true
    };
    if (title_length < 0 || (size_t)title_length >= sizeof(initial_title) ||
        !platform_window_create(&window_config, &app.window)) {
        (void)fputs("viewer: window creation failed\n", stderr);
        destroy_viewer(&app);
        return EXIT_FAILURE;
    }
    (void)snprintf(app.title, sizeof(app.title), "%s", initial_title);
    viewer_load_context load_context = {&app, path};
    platform_task_status submit_status = platform_task_submit_owned(app.tasks,
        VIEWER_TASK_ID, app.generation, viewer_load_callback, &load_context,
        viewer_load_cleanup);
    if (submit_status != PLATFORM_TASK_OK) {
        (void)fprintf(stderr, "viewer: worker task submission failed (%d)\n",
                      (int)submit_status);
        destroy_viewer(&app);
        return EXIT_FAILURE;
    }
    app.load_submitted = true;
    unsigned smoke_poll_count = 0U;
    while (app.running) {
        platform_event event;
        while (platform_poll_event(app.window, &event)) {
            process_event(&app, &event);
            if (!app.running) break;
        }
        if (!app.running) break;
        if (app.close_requested) request_cancel(&app);
        if (!compose_and_present(&app)) {
            app.fatal_error = true;
            app.running = false;
            break;
        }
        handle_task_result(&app);
        if (app.smoke_mode && !app.smoke_scripted && app.navigation_ready &&
            app.presented_frames > 0U) {
            if (!script_native_smoke(&app)) {
                (void)fputs("native viewer smoke: event injection failed\n", stderr);
                app.fatal_error = true;
                app.running = false;
                break;
            }
        }
        if (app.close_requested && (!app.load_submitted ||
                atomic_load_explicit(&app.load_cleanup_complete, memory_order_acquire))) {
            app.running = false;
            break;
        }
        if (app.smoke_mode && !app.close_requested && ++smoke_poll_count > 1000U) {
            (void)fputs("native viewer smoke: timed out waiting for viewer events\n", stderr);
            app.fatal_error = true;
            app.running = false;
            break;
        }
        idle_sleep();
    }
    if (app.smoke_mode && !app.fatal_error) {
        app.smoke_passed = app.smoke_scripted && app.close_requested &&
            app.navigation_ready && app.presented_frames > 0U;
    }
    unsigned presented_frames = app.presented_frames;
    bool smoke_passed = app.smoke_passed;
    bool failed = app.fatal_error;
    destroy_viewer(&app);
    if (smoke_mode) {
        if (!smoke_passed) {
            (void)fprintf(stderr, "native viewer smoke: failed after %u presented frames\n",
                          presented_frames);
            return EXIT_FAILURE;
        }
        (void)printf("native viewer smoke: drag/zoom/fit/quit passed\npresented frames: %u\n",
                     presented_frames);
    }
    return failed ? EXIT_FAILURE : EXIT_SUCCESS;
}

int app_run_viewer(const char *path)
{
    return run_viewer(path, false);
}

int app_run_native_viewer_smoke(const char *path)
{
    return run_viewer(path, true);
}
