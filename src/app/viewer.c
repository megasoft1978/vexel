#define _POSIX_C_SOURCE 200809L

#include "viewer.h"
#include "viewer_internal.h"


#include "frame_compose.h"
#include "mcp_broker_hook.h"
#include "platform/platform_metal.h"
#include "platform/platform_settings.h"
#include "platform/platform_task.h"
#include "platform/platform_window.h"
#include "raster/shape_render.h"
#include "test_driver/native_injection.h"
#include "viewer_load.h"
#include "viewer_quality.h"
#include "viewer_chrome.h"
#include "viewer_canvas_geometry.h"

#include <math.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

/* Persists a workspace change through the platform settings store. */
static void viewer_persist_workspace(void *context, const char *name)
{
    (void)context;
    platform_settings_store_workspace(name);
}

static double monotonic_ms(void)
{
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now) != 0) return 0.0;
    return (double)now.tv_sec * 1000.0 + (double)now.tv_nsec / 1000000.0;
}

const char *viewer_error_reason(const viewer_app *app)
{
    if (app->load_error[0] != '\0') return app->load_error;
    if (app->document.shape_error != SVG_SHAPE_ERROR_NONE) {
        const char *name = svg_shape_error_name(app->document.shape_error);
        return name == NULL ? "unknown SVG error" : name;
    }
    return app->document.load_error[0] == '\0' ? "document load failed" :
        app->document.load_error;
}

bool viewer_allocate_frame(viewer_app *app, uint32_t width, uint32_t height)
{
    if (app == NULL || width == 0U || height == 0U ||
        width > VIEWER_FRAME_MAX_AXIS || height > VIEWER_FRAME_MAX_AXIS) return false;
    size_t w = (size_t)width;
    size_t h = (size_t)height;
    if (w > SIZE_MAX / h || w * h > SIZE_MAX / 4U) return false;
    size_t bytes = w * h * 4U;
    if (bytes > VIEWER_FRAME_MAX_BYTES) return false;
    uint8_t *frame = realloc(app->frame, bytes);
    if (frame == NULL) return false;
    app->frame = frame;
    app->frame_bytes = bytes;
    app->width = width;
    app->height = height;
    app->frame_dirty = true;
    return true;
}

bool viewer_bump_generation(viewer_app *app)
{
    uint64_t next = 0U;
    if (!viewer_generation_advance(app->generation, &next)) return false;
    if (platform_task_advance_generation(app->tasks, next) != PLATFORM_TASK_OK) return false;
    app->generation = next;
    viewer_tile_cache_clear(&app->sharp_cache, app->generation);
    app->sharp_plan_ready = false;
    return true;
}

bool viewer_sync_canvas_geometry(viewer_app *app, bool refit)
{
    if (app == NULL) return false;
    ui_rect next = {0U, 0U, app->width, app->height};
    if (app->editor != NULL && !viewer_editor_get_navigation_canvas_rect(
            app->editor, NULL, &next)) return false;
    if (next.width == 0U || next.height == 0U) return false;
    ui_rect previous = app->navigation_canvas;
    bool changed = memcmp(&previous, &next, sizeof(next)) != 0;
    if (!changed) return false;
    if (app->navigation_ready) {
        if (!viewer_canvas_navigation_resize(&app->navigation,
                (double)next.width, (double)next.height,
                refit || !app->navigation_user_zoomed)) return false;
        viewer_navigation_clamp(&app->navigation);
    }
    app->navigation_canvas = next;
    if (app->navigation_ready) {
        viewer_navigation_clamp(&app->navigation);
        if (!viewer_bump_generation(app)) {
            app->fatal_error = true;
            app->running = false;
            return false;
        }
        viewer_refresh_sharp_plan(app);
    }
    return true;
}

ui_viewer_navigation viewer_navigation_for_frame(const viewer_app *app)
{
    ui_viewer_navigation navigation = {0};
    if (app == NULL) return navigation;
    navigation = app->navigation;
    navigation.pan_x += (double)app->navigation_canvas.x;
    navigation.pan_y += (double)app->navigation_canvas.y;
    navigation.viewport_width = (double)app->width;
    navigation.viewport_height = (double)app->height;
    return navigation;
}

static uint64_t sharp_work_per_pixel(const viewer_app *app)
{
    uint64_t shapes = (uint64_t)app->document.shape.shape_count;
    uint64_t edges = (uint64_t)app->document.shape.edge_count;
    uint64_t estimate = shapes + edges / 256U + 1U;
    return estimate == 0U ? 1U : estimate;
}

void viewer_refresh_sharp_plan(viewer_app *app)
{
    ui_viewer_navigation frame_navigation = viewer_navigation_for_frame(app);
    app->sharp_plan_ready = app->document_visible && app->navigation_ready &&
        viewer_timeline_track_count(&app->timeline) == 0U &&
        viewer_tile_plan_init(&app->sharp_plan, app->width, app->height,
                              app->generation, &frame_navigation);
    app->frame_dirty = true;
}

static bool render_sharp_region(viewer_job *job, const platform_task_cancel_token *token)
{
    viewer_app *app = job->app;
    const svg_shape_document *source = &app->document.shape;
    viewer_tile_region region = job->sharp_tile.region;
    if (atomic_load_explicit(&token->canceled, memory_order_acquire)) return false;
    double base_scale = fmin((double)app->document.tile.width / source->view_width,
                             (double)app->document.tile.height / source->view_height);
    double offset_x = ((double)app->document.tile.width - source->view_width * base_scale) * 0.5;
    double offset_y = ((double)app->document.tile.height - source->view_height * base_scale) * 0.5;
    double output_scale = job->navigation.scale * base_scale;
    if (!isfinite(base_scale) || !(base_scale > 0.0) || !isfinite(output_scale) ||
        !(output_scale > 0.0)) return false;
    double raster_x = ((double)region.x - job->navigation.pan_x) / job->navigation.scale;
    double raster_y = ((double)region.y - job->navigation.pan_y) / job->navigation.scale;
    svg_shape_document view = *source;
    view.width = region.width;
    view.height = region.height;
    view.view_x = source->view_x + (raster_x - offset_x) / base_scale;
    view.view_y = source->view_y + (raster_y - offset_y) / base_scale;
    view.view_width = (double)region.width / output_scale;
    view.view_height = (double)region.height / output_scale;
    if (!isfinite(view.view_x) || !isfinite(view.view_y) ||
        !isfinite(view.view_width) || !isfinite(view.view_height) ||
        !(view.view_width > 0.0) || !(view.view_height > 0.0)) return false;
    raster_tile output = {0};
    svg_shape_diagnostic diagnostic = {SVG_SHAPE_ERROR_NONE, 0U};
    if (!raster_shape_render(&view, &output, &diagnostic)) {
        const char *reason = svg_shape_error_name(diagnostic.category);
        (void)snprintf(job->diagnostic, sizeof(job->diagnostic), "%s",
            reason == NULL ? "sharp tile render failed" : reason);
        return false;
    }
    if (atomic_load_explicit(&token->canceled, memory_order_acquire)) {
        raster_tile_destroy(&output);
        return false;
    }
    job->sharp_tile.byte_count = output.byte_count;
    job->sharp_tile.rgba = output.rgba;
    output.rgba = NULL;
    raster_tile_destroy(&output);
    return true;
}

static bool viewer_job_callback(void *context, const platform_task_cancel_token *token,
                                int64_t *out_value)
{
    viewer_job *job = context;
    if (job == NULL || token == NULL || out_value == NULL) return false;
    double started = monotonic_ms();
    bool succeeded;
    if (job->kind == VIEWER_JOB_LOAD) {
        job->load_status = viewer_load_document(job->path, token, &job->loaded);
        succeeded = job->load_status != VIEWER_LOAD_CANCELED;
    } else if (job->kind == VIEWER_JOB_SHARP_TILE) {
        succeeded = render_sharp_region(job, token);
    } else {
        uint8_t *source = job->source_snapshot;
        job->source_snapshot = NULL;
        job->load_status = viewer_decode_owned_source(source,
            job->source_snapshot_length, token, &job->loaded);
        succeeded = job->load_status == VIEWER_LOAD_OK;
    }
    double ended = monotonic_ms();
    job->elapsed_ms = ended >= started ? ended - started : 0.0;
    *out_value = succeeded ? 1 : 0;
    return succeeded;
}

static void viewer_job_cleanup(void *context)
{
    viewer_job *job = context;
    if (job != NULL)
        atomic_store_explicit(&job->cleanup_complete, true, memory_order_release);
}

static void viewer_job_destroy(viewer_job *job)
{
    if (job == NULL) return;
    viewer_loaded_document_destroy(&job->loaded);
    free(job->source_snapshot);
    free(job->sharp_tile.rgba);
    free(job);
}

static bool submit_job(viewer_app *app, viewer_job *job)
{
    if (app->job != NULL || job == NULL) return false;
    atomic_init(&job->cleanup_complete, false);
    platform_task_status status = platform_task_submit_owned(app->tasks, VIEWER_TASK_ID,
        app->generation, viewer_job_callback, job, viewer_job_cleanup);
    if (status != PLATFORM_TASK_OK) return false;
    app->job = job;
    return true;
}

bool viewer_request_open(viewer_app *app, const char *path, bool initial)
{
    if (app == NULL) return false;
    size_t length = viewer_bounded_length(path, PLATFORM_PATH_CAP - 1U);
    if (length == 0U || length >= PLATFORM_PATH_CAP) return false;
    if (!initial && !viewer_confirm_unsaved_changes(app, "Unsaved Changes",
            "Save changes before opening another document?")) return false;
    if (!initial && !viewer_bump_generation(app)) return false;
    memcpy(app->pending_path, path, length + 1U);
    app->pending_open = true;
    app->pending_new_document = false;
    app->load_result_handled = false;
    app->load_status = VIEWER_LOAD_CANCELED;
    app->load_error[0] = '\0';
    app->failed_path[0] = '\0';
    app->load_ms = 0.0;
    app->render_ms = 0.0;
    (void)ui_viewer_progress_update(&app->progress, app->generation, 0U,
        UI_PROGRESS_ACTIVE, "Loading SVG", 11U);
    app->frame_dirty = true;
    (void)viewer_set_window_title(app);
    return true;
}

bool viewer_update_active_path(viewer_app *app, const char *path, size_t path_length)
{
    if (app == NULL || path == NULL || path_length == 0U ||
        path_length >= sizeof(app->path) || memchr(path, '\0', path_length) != NULL)
        return false;
    memcpy(app->path, path, path_length);
    app->path[path_length] = '\0';
    viewer_copy_display_name(app->path, app->display_name);
    viewer_update_file_stamp(app);
    app->frame_dirty = true;
    (void)platform_window_recent_add(app->window, path, path_length);
    app->recent_file_count = platform_window_recent_files(app->window,
        app->recent_files, PLATFORM_RECENT_FILES_MAX);
    return viewer_set_window_title(app);
}

static void start_pending_load(viewer_app *app)
{
    if (!app->pending_open || app->job != NULL || app->close_requested) return;
    viewer_job *job = calloc(1U, sizeof(*job));
    if (job == NULL) goto failed;
    job->app = app;
    job->kind = VIEWER_JOB_LOAD;
    job->generation = app->generation;
    size_t length = viewer_bounded_length(app->pending_path, PLATFORM_PATH_CAP - 1U);
    if (length == 0U || length >= PLATFORM_PATH_CAP) {
        free(job);
        goto failed;
    }
    memcpy(job->path, app->pending_path, length + 1U);
    if (!submit_job(app, job)) {
        viewer_job_destroy(job);
        goto failed;
    }
    app->pending_open = false;
    app->pending_path[0] = '\0';
    viewer_tile_cache_clear(&app->sharp_cache, app->generation);
    app->sharp_plan_ready = false;
    (void)viewer_set_window_title(app);
    return;
failed:
    app->pending_open = false;
    app->load_status = VIEWER_LOAD_FAILED;
    app->load_result_handled = true;
    size_t failed_length = viewer_bounded_length(app->pending_path,
        PLATFORM_PATH_CAP - 1U);
    if (failed_length < PLATFORM_PATH_CAP)
        memcpy(app->failed_path, app->pending_path, failed_length + 1U);
    app->pending_path[0] = '\0';
    (void)snprintf(app->load_error, sizeof(app->load_error),
                   "could not start document loading");
    (void)ui_viewer_progress_update(&app->progress, app->generation, 0U,
        UI_PROGRESS_FAILED, "Failed", 6U);
    if (app->document_visible) viewer_refresh_sharp_plan(app);
    app->frame_dirty = true;
    (void)viewer_set_window_title(app);
}

static void finish_load(viewer_app *app, viewer_job *job, bool worker_succeeded)
{
    app->load_ms = job->elapsed_ms;
    if (!worker_succeeded || job->load_status != VIEWER_LOAD_OK) {
        size_t path_length = viewer_bounded_length(job->path,
            PLATFORM_PATH_CAP - 1U);
        if (path_length < PLATFORM_PATH_CAP) {
            memcpy(app->failed_path, job->path, path_length + 1U);
            if (path_length > 0U)
                platform_window_recent_open_failed(app->window, job->path, path_length);
        }
        if (job->loaded.load_error[0] != '\0') {
            (void)snprintf(app->load_error, sizeof(app->load_error), "%.180s",
                job->loaded.load_error);
        } else {
            const char *reason = svg_shape_error_name(job->loaded.shape_error);
            if (reason == NULL || reason[0] == '\0') reason = "could not load SVG";
            (void)snprintf(app->load_error, sizeof(app->load_error), "%.180s", reason);
        }
        app->pending_open = false;
        app->pending_path[0] = '\0';
        app->load_result_handled = true;
        app->load_status = VIEWER_LOAD_FAILED;
        (void)ui_viewer_progress_update(&app->progress, app->generation, 0U,
            UI_PROGRESS_FAILED, "Failed", 6U);
        if (app->document_visible) viewer_refresh_sharp_plan(app);
        app->frame_dirty = true;
        (void)fprintf(stderr, "viewer: %s\n", viewer_error_reason(app));
        (void)viewer_set_window_title(app);
        return;
    }

    viewer_timeline next_timeline;
    viewer_timeline_init(&next_timeline);
    next_timeline.dock_open = app->timeline.dock_open;
    next_timeline.dock_state_explicit = app->timeline.dock_state_explicit;
    next_timeline.dock_height_points = app->timeline.dock_height_points;
    next_timeline.mode = app->timeline.mode;
    next_timeline.playback_speed = app->timeline.playback_speed;
    next_timeline.reduced_motion = app->timeline.reduced_motion;
    viewer_timeline_reset(&next_timeline, &job->loaded.shape);
    viewer_editor *next_editor = viewer_editor_create((base_bytes){job->loaded.source,
        job->loaded.source_length}, app->theme, app->width, app->height);
    if (next_editor != NULL) {
        platform_settings_values preferences = {0};
        platform_settings_get(&preferences);
        viewer_editor_set_workspace_callback(next_editor,
            viewer_persist_workspace, NULL);
        /* Tests pin the starting workspace so a remembered user choice cannot
         * change the fixture layout between runs. */
        const char *test_workspace = getenv("VEXEL_TEST_WORKSPACE");
        if (test_workspace != NULL && test_workspace[0] != '\0')
            (void)viewer_editor_apply_workspace_name(next_editor,
                test_workspace, strlen(test_workspace));
        else if (preferences.remember_workspace && preferences.workspace[0] != '\0')
            (void)viewer_editor_apply_workspace_name(next_editor,
                preferences.workspace, strlen(preferences.workspace));
        (void)viewer_editor_set_sidebar_visibility(next_editor,
            preferences.structure_visible, preferences.inspector_visible);
        (void)viewer_editor_set_timeline_state(next_editor,
            viewer_timeline_track_count(&next_timeline) > 0U,
            next_timeline.dock_open, next_timeline.dock_height_points);
        viewer_editor_resize(next_editor, app->width, app->height,
            app->backing_scale);
    }
    viewer_chrome_layout next_layout = {0};
    ui_rect next_canvas = {0};
    ui_viewer_navigation next_navigation = {0};
    bool document_ready = next_editor != NULL &&
        viewer_editor_get_navigation_canvas_rect(next_editor, &next_layout,
            &next_canvas) &&
        ui_viewer_navigation_init(&next_navigation, (double)next_canvas.width,
            (double)next_canvas.height, (double)job->loaded.tile.width,
            (double)job->loaded.tile.height) && ui_viewer_navigation_fit(&next_navigation);
    if (!document_ready) {
        viewer_editor_destroy(next_editor);
        viewer_timeline_destroy(&next_timeline);
        size_t path_length = viewer_bounded_length(job->path,
            PLATFORM_PATH_CAP - 1U);
        if (path_length < PLATFORM_PATH_CAP)
            memcpy(app->failed_path, job->path, path_length + 1U);
        (void)snprintf(app->load_error, sizeof(app->load_error),
            "document dimensions cannot fit this window");
        app->pending_open = false;
        app->pending_path[0] = '\0';
        app->load_result_handled = true;
        app->load_status = VIEWER_LOAD_FAILED;
        (void)ui_viewer_progress_update(&app->progress, app->generation, 0U,
            UI_PROGRESS_FAILED, "Failed", 6U);
        if (app->document_visible) viewer_refresh_sharp_plan(app);
        app->frame_dirty = true;
        (void)viewer_set_window_title(app);
        return;
    }

    viewer_editor_destroy(app->editor);
    app->editor = next_editor;
    viewer_timeline_destroy(&app->timeline);
    app->timeline = next_timeline;
    next_timeline = (viewer_timeline){0};
    viewer_loaded_document_destroy(&app->document);
    free(app->render_source);
    app->render_source = NULL;
    app->document = job->loaded;
    job->loaded = (viewer_loaded_document){0};
    viewer_timeline_reset(&app->timeline, &app->document.shape);
    app->load_ms = job->elapsed_ms;
    app->navigation_canvas = next_canvas;
    app->navigation = next_navigation;
    app->navigation_ready = true;
    app->navigation_user_zoomed = false;
    app->document_visible = true;
    platform_window_hide_welcome(app->window);
    (void)platform_window_set_visible(app->window, true);
    viewer_navigation_clamp(&app->navigation);
    (void)platform_window_set_option_state(app->window,
        PLATFORM_ACTION_TOGGLE_RULERS, viewer_editor_rulers_visible(app->editor));
    (void)platform_window_set_option_state(app->window,
        PLATFORM_ACTION_TOGGLE_GUIDES, viewer_editor_guides_visible(app->editor));
    (void)platform_window_set_option_state(app->window,
        PLATFORM_ACTION_TOGGLE_GUIDE_LOCK, viewer_editor_guides_locked(app->editor));
    size_t path_length = viewer_bounded_length(job->path, PLATFORM_PATH_CAP - 1U);
    if (path_length < PLATFORM_PATH_CAP)
        (void)viewer_update_active_path(app, job->path, path_length);
    app->failed_path[0] = '\0';
    app->pending_path[0] = '\0';
    app->pending_open = false;
    app->load_error[0] = '\0';
    app->load_result_handled = true;
    app->load_status = VIEWER_LOAD_OK;
    app->external_change_pending = false;
    app->pointer_valid = false;
    (void)ui_viewer_progress_update(&app->progress, app->generation, 100U,
        UI_PROGRESS_COMPLETE, "Ready", 5U);
    viewer_refresh_sharp_plan(app);
    app->frame_dirty = true;
    (void)viewer_set_window_title(app);
}

static void service_job(viewer_app *app)
{
    viewer_job *job = app->job;
    if (job == NULL || !atomic_load_explicit(&job->cleanup_complete, memory_order_acquire)) return;
    platform_task_result result = {0};
    bool have_result = platform_task_poll(app->tasks, &result);
    bool current = viewer_generation_is_current(app->generation, job->generation);
    bool succeeded = have_result && result.task_id == VIEWER_TASK_ID &&
        result.generation == job->generation && result.succeeded;
    app->job = NULL;
    bool document_load = job->kind == VIEWER_JOB_LOAD ||
        job->kind == VIEWER_JOB_NEW_DOCUMENT;
    if (current && succeeded && document_load) {
        finish_load(app, job, true);
    } else if (current && document_load) {
        if (job->load_status == VIEWER_LOAD_CANCELED)
            app->load_status = VIEWER_LOAD_FAILED;
        finish_load(app, job, false);
    } else if (current && succeeded && job->kind == VIEWER_JOB_SHARP_TILE) {
        if (viewer_tile_cache_put(&app->sharp_cache, &job->sharp_tile)) {
            app->render_ms = job->elapsed_ms;
        } else {
            free(job->sharp_tile.rgba);
            job->sharp_tile.rgba = NULL;
            app->sharp_plan_ready = false;
        }
        app->frame_dirty = true;
    } else if (current && job->kind == VIEWER_JOB_SHARP_TILE) {
        app->sharp_plan_ready = false;
        if (job->diagnostic[0] != '\0')
            (void)fprintf(stderr, "viewer: sharp preview stopped: %s\n", job->diagnostic);
        app->frame_dirty = true;
    } else if (current && succeeded && job->kind == VIEWER_JOB_EDIT_RENDER) {
        viewer_timeline_reset(&app->timeline, NULL);
        svg_shape_document_destroy(&app->document.shape);
        raster_tile_destroy(&app->document.tile);
        free(app->render_source);
        app->render_source = job->loaded.source;
        job->loaded.source = NULL;
        app->document.shape = job->loaded.shape;
        job->loaded.shape = (svg_shape_document){0};
        viewer_timeline_reset(&app->timeline, &app->document.shape);
        if (app->editor != NULL)
            (void)viewer_editor_set_timeline_state(app->editor,
                viewer_timeline_track_count(&app->timeline) > 0U,
                app->timeline.dock_open, app->timeline.dock_height_points);
        app->document.tile = job->loaded.tile;
        job->loaded.tile = (raster_tile){0};
        (void)viewer_sync_canvas_geometry(app, !app->navigation_user_zoomed);
        app->document.shape_error = SVG_SHAPE_ERROR_NONE;
        app->document.error_offset = 0U;
        app->document.load_error[0] = '\0';
        app->render_ms = job->elapsed_ms;
        app->editor_render_error[0] = '\0';
        app->load_status = VIEWER_LOAD_OK;
        app->load_result_handled = true;
        if (!app->navigation_ready &&
            ui_viewer_navigation_init(&app->navigation,
                (double)app->navigation_canvas.width,
                (double)app->navigation_canvas.height,
                (double)app->document.tile.width,
                (double)app->document.tile.height) &&
            ui_viewer_navigation_fit(&app->navigation)) {
            app->navigation_ready = true;
            app->document_visible = true;
            (void)ui_viewer_progress_update(&app->progress, app->generation, 100U,
                UI_PROGRESS_COMPLETE, "Ready", 5U);
        }
        viewer_editor_set_status(app->editor, "Preview updated");
        viewer_refresh_sharp_plan(app);
        app->frame_dirty = true;
        (void)viewer_set_window_title(app);
    } else if (current && job->kind == VIEWER_JOB_EDIT_RENDER) {
        const char *reason = job->loaded.load_error[0] != '\0' ?
            job->loaded.load_error : svg_shape_error_name(job->loaded.shape_error);
        if (reason == NULL || reason[0] == '\0') reason = "edited SVG is unsupported";
        int amount = snprintf(app->editor_render_error, sizeof(app->editor_render_error),
            "Preview unavailable: %.120s at byte %zu", reason, job->loaded.error_offset);
        if (amount < 0 || (size_t)amount >= sizeof(app->editor_render_error))
            (void)snprintf(app->editor_render_error, sizeof(app->editor_render_error),
                "Preview unavailable: edited SVG is unsupported");
        viewer_editor_set_status(app->editor, app->editor_render_error);
        app->frame_dirty = true;
    }
    viewer_job_destroy(job);
}

static void start_next_sharp_tile(viewer_app *app)
{
    if (!app->sharp_plan_ready || app->job != NULL || !app->document_visible ||
        app->close_requested || viewer_timeline_track_count(&app->timeline) > 0U) return;
    viewer_tile_region region;
    if (!viewer_tile_plan_next(&app->sharp_plan, &app->sharp_cache,
            sharp_work_per_pixel(app), &region)) {
        app->sharp_plan_ready = false;
        return;
    }
    viewer_job *job = calloc(1U, sizeof(*job));
    if (job == NULL) {
        app->sharp_plan_ready = false;
        return;
    }
    job->app = app;
    job->kind = VIEWER_JOB_SHARP_TILE;
    job->generation = app->generation;
    job->sharp_tile.region = region;
    job->navigation = viewer_navigation_for_frame(app);
    if (!submit_job(app, job)) {
        viewer_job_destroy(job);
        app->sharp_plan_ready = false;
    }
}

void viewer_request_close(viewer_app *app)
{
    if (!viewer_confirm_unsaved_changes(app, "Unsaved Changes",
            "Save changes before closing?")) return;
    viewer_close_confirmed(app);
}

void viewer_close_confirmed(viewer_app *app)
{
    if (app == NULL) return;
    app->close_requested = true;
    app->pending_open = false;
    app->pending_path[0] = '\0';
    if (app->job != NULL && app->job->generation == app->generation)
        (void)viewer_bump_generation(app);
}

static void poll_file_changes(viewer_app *app)
{
    if (!app->load_result_handled || app->path[0] == '\0' || app->close_requested) return;
    double now_ms = monotonic_ms();
    double last_ms = (double)app->last_file_poll.tv_sec * 1000.0 +
        (double)app->last_file_poll.tv_nsec / 1000000.0;
    if (now_ms - last_ms < VIEWER_AUTORELOAD_INTERVAL_MS) return;
    if (clock_gettime(CLOCK_MONOTONIC, &app->last_file_poll) != 0) return;
    viewer_file_stamp current = {0};
    (void)viewer_file_stamp_read(app->path, &current);
    bool changed = current.valid != app->file_stamp.valid ||
        (current.valid && !viewer_file_stamp_equal(&current, &app->file_stamp));
    if (changed) {
        if (viewer_editor_is_dirty(app->editor)) {
            if (!app->external_change_pending) {
                viewer_editor_set_status(app->editor,
                    "File changed on disk; save your edits before reloading");
                app->frame_dirty = true;
            }
            app->external_change_pending = true;
            return;
        }
        (void)viewer_request_open(app, app->path, false);
    }
}

static void start_editor_render(viewer_app *app)
{
    if (!app->editor_render_requested || app->job != NULL || app->editor == NULL ||
        app->close_requested) return;
    const size_t source_limit = (size_t)1024U * 1024U * 1024U;
    size_t length = viewer_editor_byte_length(app->editor);
    if (length == 0U || length > source_limit - VIEWER_EDITOR_PREVIEW_EXTRA_BYTES) {
        viewer_editor_set_status(app->editor, "Edited document exceeds the render source limit");
        app->editor_render_requested = false;
        app->frame_dirty = true;
        return;
    }
    size_t capacity = length + VIEWER_EDITOR_PREVIEW_EXTRA_BYTES;
    uint8_t *source = malloc(capacity);
    if (source == NULL) {
        viewer_editor_set_status(app->editor, "Preview update failed: source allocation failed");
        app->editor_render_requested = false;
        app->frame_dirty = true;
        return;
    }
    size_t materialized = 0U;
    if (!viewer_editor_materialize_preview(app->editor, source, capacity,
            &materialized) || materialized == 0U || materialized > source_limit) {
        free(source);
        viewer_editor_set_status(app->editor, "Preview update failed: source snapshot unavailable");
        app->editor_render_requested = false;
        app->frame_dirty = true;
        return;
    }
    viewer_job *job = calloc(1U, sizeof(*job));
    if (job == NULL) {
        free(source);
        viewer_editor_set_status(app->editor, "Preview update failed: worker allocation failed");
        app->editor_render_requested = false;
        app->frame_dirty = true;
        return;
    }
    job->app = app;
    job->kind = VIEWER_JOB_EDIT_RENDER;
    job->generation = app->generation;
    job->source_snapshot = source;
    job->source_snapshot_length = materialized;
    if (!submit_job(app, job)) {
        viewer_job_destroy(job);
        viewer_editor_set_status(app->editor, "Preview update failed: worker queue unavailable");
        app->frame_dirty = true;
        app->editor_render_requested = false;
        return;
    }
    app->editor_render_requested = false;
}

static void start_pending_new_document(viewer_app *app)
{
    static const char blank_document[] =
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"512\" height=\"512\" "
        "viewBox=\"0 0 512 512\"></svg>";
    if (app == NULL || !app->pending_new_document || app->job != NULL ||
        app->close_requested) return;
    viewer_job *job = calloc(1U, sizeof(*job));
    if (job == NULL) goto failed;
    size_t length = sizeof(blank_document) - 1U;
    job->source_snapshot = malloc(length);
    if (job->source_snapshot == NULL) {
        free(job);
        goto failed;
    }
    memcpy(job->source_snapshot, blank_document, length);
    job->source_snapshot_length = length;
    job->app = app;
    job->kind = VIEWER_JOB_NEW_DOCUMENT;
    job->generation = app->generation;
    if (!submit_job(app, job)) {
        viewer_job_destroy(job);
        goto failed;
    }
    app->pending_new_document = false;
    (void)viewer_set_window_title(app);
    return;
failed:
    app->pending_new_document = false;
    app->load_status = VIEWER_LOAD_FAILED;
    app->load_result_handled = true;
    (void)snprintf(app->load_error, sizeof(app->load_error),
        "could not start blank document creation");
    app->frame_dirty = true;
    (void)viewer_set_window_title(app);
}

void viewer_app_tick(viewer_app *app)
{
    if (app == NULL || !app->running) return;
    service_job(app);
    if (app->pending_new_document && app->job == NULL)
        start_pending_new_document(app);
    if (app->pending_open && app->job == NULL) start_pending_load(app);
    if (app->editor_render_requested && app->job == NULL) start_editor_render(app);
    if (!app->close_requested) start_next_sharp_tile(app);
    if (viewer_editor_take_redraw_request(app->editor)) app->frame_dirty = true;
    viewer_update_native_state(app);
    if (!viewer_compose_and_present(app)) {
        app->fatal_error = true;
        app->running = false;
        return;
    }
    (void)viewer_test_layout_dump(app);
    poll_file_changes(app);
    if (app->close_requested && app->job == NULL) app->running = false;
}

void viewer_app_destroy(viewer_app *app)
{
    if (app == NULL) return;
    if (app->tasks != NULL) {
        if (app->job != NULL && app->job->generation == app->generation)
            (void)viewer_bump_generation(app);
        platform_task_shutdown(app->tasks);
        platform_task_destroy(app->tasks);
        app->tasks = NULL;
    }
    viewer_job_destroy(app->job);
    app->job = NULL;
    if (app->window != NULL) {
        platform_metal_reset(app->window);
        platform_window_destroy(app->window);
        app->window = NULL;
    }
    viewer_tile_cache_destroy(&app->sharp_cache);
    free(app->frame);
    app->frame = NULL;
    mcp_broker_hook_shutdown(app);
    viewer_editor_destroy(app->editor);
    app->editor = NULL;
    viewer_timeline_destroy(&app->timeline);
    viewer_loaded_document_destroy(&app->document);
    free(app->render_source);
    app->render_source = NULL;
}
