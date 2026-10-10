#define _POSIX_C_SOURCE 200809L
#define _DARWIN_C_SOURCE

#include "viewer.h"
#include "viewer_internal.h"

#include "frame_compose.h"
#include "mcp_broker_hook.h"
#include "platform/platform_metal.h"
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
#include <sys/stat.h>
#include <time.h>

static size_t bounded_length(const char *text, size_t limit)
{
    size_t length = 0U;
    if (text == NULL) return 0U;
    while (length <= limit && text[length] != '\0') length++;
    return length;
}

static double monotonic_ms(void)
{
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now) != 0) return 0.0;
    return (double)now.tv_sec * 1000.0 + (double)now.tv_nsec / 1000000.0;
}

static void copy_display_name(const char *path, char out[256])
{
    const char *name = path;
    for (const char *cursor = path; *cursor != '\0'; cursor++)
        if (*cursor == '/') name = cursor + 1;
    if (*name == '\0') name = "Open SVG";
    size_t length = bounded_length(name, 240U);
    if (length > 240U) length = 240U;
    for (size_t index = 0U; index < length; index++) {
        unsigned char value = (unsigned char)name[index];
        out[index] = value >= 32U && value < 127U ? (char)value : '?';
    }
    out[length] = '\0';
}

const char *viewer_error_reason(const viewer_app *app)
{
    if (app->document.shape_error != SVG_SHAPE_ERROR_NONE) {
        const char *name = svg_shape_error_name(app->document.shape_error);
        return name == NULL ? "unknown SVG error" : name;
    }
    return app->document.load_error[0] == '\0' ? "document load failed" :
        app->document.load_error;
}

bool viewer_set_window_title(viewer_app *app)
{
    char next[VIEWER_TITLE_CAP];
    int amount;
    const char *dirty = viewer_editor_is_dirty(app->editor) ? "*" : "";
    if (app->load_result_handled && app->load_status != VIEWER_LOAD_OK) {
        amount = snprintf(next, sizeof(next), "vexel — %s%s — error: %s",
                          app->display_name, dirty, viewer_error_reason(app));
    } else if (app->pending_open || (app->job != NULL && app->job->kind == VIEWER_JOB_LOAD)) {
        amount = snprintf(next, sizeof(next), "vexel — %s%s — Loading…", app->display_name, dirty);
    } else if (app->navigation_ready && app->document_visible) {
        amount = snprintf(next, sizeof(next), "vexel — %s%s — %.0f%%",
                          app->display_name, dirty, app->navigation.scale * 100.0);
    } else {
        amount = snprintf(next, sizeof(next), "vexel — %s%s", app->display_name, dirty);
    }
    if (amount < 0 || (size_t)amount >= sizeof(next)) return false;
    if (strcmp(next, app->title) == 0) return true;
    size_t length = (size_t)amount;
    if (!platform_window_set_title(app->window, next, length)) return false;
    memcpy(app->title, next, length + 1U);
    return true;
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

static bool file_stamp_read(const char *path, viewer_file_stamp *out)
{
    if (path == NULL || out == NULL) return false;
    struct stat status;
    if (stat(path, &status) != 0 || !S_ISREG(status.st_mode)) return false;
    out->size = status.st_size;
    out->modified = status.st_mtimespec;
    out->valid = true;
    return true;
}

static bool file_stamp_equal(const viewer_file_stamp *left, const viewer_file_stamp *right)
{
    return left->valid && right->valid && left->size == right->size &&
        left->modified.tv_sec == right->modified.tv_sec &&
        left->modified.tv_nsec == right->modified.tv_nsec;
}

void viewer_update_file_stamp(viewer_app *app)
{
    if (app == NULL) return;
    app->file_stamp = (viewer_file_stamp){0};
    if (app->path[0] != '\0') (void)file_stamp_read(app->path, &app->file_stamp);
    app->external_change_pending = false;
}

bool viewer_file_changed_on_disk(viewer_app *app)
{
    if (app == NULL) return false;
    if (app->external_change_pending) return true;
    viewer_file_stamp current = {0};
    (void)file_stamp_read(app->path, &current);
    bool changed = current.valid != app->file_stamp.valid ||
        (current.valid && !file_stamp_equal(&current, &app->file_stamp));
    if (changed) app->external_change_pending = true;
    return changed;
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
    viewer_chrome_layout layout;
    if (app->editor != NULL && viewer_editor_get_chrome_layout(app->editor, &layout))
        next = layout.canvas;
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
    size_t length = bounded_length(path, PLATFORM_PATH_CAP - 1U);
    if (length == 0U || length >= PLATFORM_PATH_CAP) return false;
    if (!initial && viewer_editor_is_dirty(app->editor)) {
        viewer_editor_set_status(app->editor, "Unsaved changes: save or discard before opening another file");
        app->frame_dirty = true;
        return false;
    }
    viewer_editor_destroy(app->editor);
    app->editor = NULL;
    app->editor_render_requested = false;
    app->external_change_pending = false;
    if (!initial && !viewer_bump_generation(app)) return false;
    memcpy(app->path, path, length + 1U);
    memcpy(app->pending_path, path, length + 1U);
    app->pending_open = true;
    app->load_result_handled = false;
    app->load_status = VIEWER_LOAD_CANCELED;
    app->document_visible = false;
    app->navigation_ready = false;
    app->navigation_user_zoomed = false;
    app->pointer_valid = false;
    app->load_ms = 0.0;
    app->render_ms = 0.0;
    app->file_stamp = (viewer_file_stamp){0};
    (void)file_stamp_read(app->path, &app->file_stamp);
    copy_display_name(path, app->display_name);
    viewer_tile_cache_clear(&app->sharp_cache, app->generation);
    app->sharp_plan_ready = false;
    (void)ui_viewer_progress_update(&app->progress, app->generation, 0U,
        UI_PROGRESS_ACTIVE, "Loading SVG", 11U);
    app->frame_dirty = true;
    (void)viewer_set_window_title(app);
    return true;
}

static void start_pending_load(viewer_app *app)
{
    if (!app->pending_open || app->job != NULL || app->close_requested) return;
    viewer_job *job = calloc(1U, sizeof(*job));
    if (job == NULL) goto failed;
    job->app = app;
    job->kind = VIEWER_JOB_LOAD;
    job->generation = app->generation;
    size_t length = bounded_length(app->pending_path, PLATFORM_PATH_CAP - 1U);
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
    (void)viewer_set_window_title(app);
    return;
failed:
    app->pending_open = false;
    app->load_status = VIEWER_LOAD_FAILED;
    app->load_result_handled = true;
    (void)snprintf(app->document.load_error, sizeof(app->document.load_error),
                   "could not start document loading");
    (void)ui_viewer_progress_update(&app->progress, app->generation, 0U,
        UI_PROGRESS_FAILED, "Failed", 6U);
    app->frame_dirty = true;
    (void)viewer_set_window_title(app);
}

static void finish_load(viewer_app *app, viewer_job *job, bool worker_succeeded)
{
    app->load_result_handled = true;
    app->load_status = worker_succeeded ? job->load_status : VIEWER_LOAD_FAILED;
    viewer_editor_destroy(app->editor);
    app->editor = NULL;
    viewer_loaded_document_destroy(&app->document);
    free(app->render_source);
    app->render_source = NULL;
    app->document = job->loaded;
    job->loaded = (viewer_loaded_document){0};
    app->load_ms = job->elapsed_ms;
    if (app->document.source != NULL && app->document.source_length > 0U)
        app->editor = viewer_editor_create((base_bytes){app->document.source,
            app->document.source_length}, UI_THEME_DARK, app->width, app->height);
    viewer_editor_resize(app->editor, app->width, app->height, app->backing_scale);
    (void)viewer_sync_canvas_geometry(app, true);
    if (app->load_status == VIEWER_LOAD_OK &&
        ui_viewer_navigation_init(&app->navigation,
            (double)app->navigation_canvas.width,
            (double)app->navigation_canvas.height,
            (double)app->document.tile.width,
            (double)app->document.tile.height) &&
        ui_viewer_navigation_fit(&app->navigation)) {
        app->navigation_ready = true;
        app->navigation_user_zoomed = false;
        app->document_visible = true;
        viewer_navigation_clamp(&app->navigation);
        (void)ui_viewer_progress_update(&app->progress, app->generation, 100U,
            UI_PROGRESS_COMPLETE, "Ready", 5U);
        (void)platform_window_recent_add(app->window, app->path,
            bounded_length(app->path, PLATFORM_PATH_CAP - 1U));
        viewer_refresh_sharp_plan(app);
    } else {
        app->navigation_ready = false;
        app->document_visible = false;
        if (app->load_status == VIEWER_LOAD_OK) {
            app->document.shape_error = SVG_SHAPE_ERROR_VIEWPORT;
            app->document.error_offset = 0U;
            app->load_status = VIEWER_LOAD_FAILED;
            (void)snprintf(app->document.load_error, sizeof(app->document.load_error),
                           "document dimensions cannot fit this window");
        }
        (void)ui_viewer_progress_update(&app->progress, app->generation, 0U,
            UI_PROGRESS_FAILED, "Failed", 6U);
        (void)fprintf(stderr, "viewer: %s\n", viewer_error_reason(app));
    }
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
    if (current && succeeded && job->kind == VIEWER_JOB_LOAD) {
        finish_load(app, job, true);
    } else if (current && job->kind == VIEWER_JOB_LOAD) {
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
        svg_shape_document_destroy(&app->document.shape);
        raster_tile_destroy(&app->document.tile);
        free(app->render_source);
        app->render_source = job->loaded.source;
        job->loaded.source = NULL;
        app->document.shape = job->loaded.shape;
        job->loaded.shape = (svg_shape_document){0};
        app->document.tile = job->loaded.tile;
        job->loaded.tile = (raster_tile){0};
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
        app->close_requested) return;
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
    if (viewer_editor_is_dirty(app->editor)) {
        viewer_editor_set_status(app->editor, "Unsaved changes: save or discard before closing");
        app->frame_dirty = true;
        return;
    }
    app->close_requested = true;
    app->pending_open = false;
    app->pending_path[0] = '\0';
    if (app->job != NULL && app->job->generation == app->generation)
        (void)viewer_bump_generation(app);
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

static void poll_file_changes(viewer_app *app)
{
    if (!app->load_result_handled || app->path[0] == '\0' || app->close_requested) return;
    double now_ms = monotonic_ms();
    double last_ms = (double)app->last_file_poll.tv_sec * 1000.0 +
        (double)app->last_file_poll.tv_nsec / 1000000.0;
    if (now_ms - last_ms < VIEWER_AUTORELOAD_INTERVAL_MS) return;
    if (clock_gettime(CLOCK_MONOTONIC, &app->last_file_poll) != 0) return;
    viewer_file_stamp current = {0};
    (void)file_stamp_read(app->path, &current);
    bool changed = current.valid != app->file_stamp.valid ||
        (current.valid && !file_stamp_equal(&current, &app->file_stamp));
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
    size_t length = viewer_editor_byte_length(app->editor);
    if (length == 0U || length > (size_t)1024U * 1024U * 1024U) {
        viewer_editor_set_status(app->editor, "Edited document exceeds the render source limit");
        app->editor_render_requested = false;
        app->frame_dirty = true;
        return;
    }
    uint8_t *source = malloc(length);
    if (source == NULL) {
        viewer_editor_set_status(app->editor, "Preview update failed: source allocation failed");
        app->editor_render_requested = false;
        app->frame_dirty = true;
        return;
    }
    size_t materialized = 0U;
    if (!viewer_editor_materialize(app->editor, source, length, &materialized) ||
        materialized != length) {
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
    job->source_snapshot_length = length;
    if (!submit_job(app, job)) {
        viewer_job_destroy(job);
        viewer_editor_set_status(app->editor, "Preview update failed: worker queue unavailable");
        app->frame_dirty = true;
        app->editor_render_requested = false;
        return;
    }
    app->editor_render_requested = false;
}

static void idle_sleep(void)
{
    const struct timespec delay = {0, 6000000L};
    (void)nanosleep(&delay, NULL);
}

static void destroy_viewer(viewer_app *app)
{
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
    viewer_loaded_document_destroy(&app->document);
    free(app->render_source);
    app->render_source = NULL;
    platform_shutdown();
}

static int run_viewer(const char *path, bool smoke_mode)
{
    if (path != NULL && (path[0] == '\0' || bounded_length(path, PLATFORM_PATH_CAP - 1U) >= PLATFORM_PATH_CAP)) {
        (void)fputs("viewer: invalid or oversized SVG path\n", stderr);
        return EXIT_FAILURE;
    }
    viewer_app app = {0};
    app.generation = VIEWER_GENERATION;
    app.running = true;
    app.smoke_mode = smoke_mode;
    app.backing_scale = 1.0;
    app.checkerboard = true;
    app.pixel_grid = true;
    app.load_status = VIEWER_LOAD_CANCELED;
    copy_display_name(path == NULL ? "Open SVG" : path, app.display_name);
    if (!viewer_tile_cache_init(&app.sharp_cache, app.generation)) return EXIT_FAILURE;
    if (!platform_init()) {
        (void)fputs("viewer: AppKit initialization failed\n", stderr);
        viewer_tile_cache_destroy(&app.sharp_cache);
        return EXIT_FAILURE;
    }
    platform_task_config task_config = {1U, 1U, NULL, NULL, NULL};
    if (platform_task_queue_create(&task_config, &app.tasks) != PLATFORM_TASK_OK) {
        (void)fputs("viewer: worker queue creation failed\n", stderr);
        platform_shutdown();
        viewer_tile_cache_destroy(&app.sharp_cache);
        return EXIT_FAILURE;
    }
    const char *initial_title = path == NULL ? "vexel — Open SVG" : "vexel — Loading SVG";
    platform_window_config window_config = {initial_title, strlen(initial_title), 1280, 800, true};
    if (!platform_window_create(&window_config, &app.window) ||
        !viewer_allocate_frame(&app, 1280U, 800U)) {
        (void)fputs("viewer: window creation or frame allocation failed\n", stderr);
        destroy_viewer(&app);
        return EXIT_FAILURE;
    }
    (void)platform_window_set_option_state(app.window,
        PLATFORM_ACTION_TOGGLE_CHECKERBOARD, app.checkerboard);
    (void)platform_window_set_option_state(app.window,
        PLATFORM_ACTION_TOGGLE_PIXEL_GRID, app.pixel_grid);
    (void)snprintf(app.title, sizeof(app.title), "%s", initial_title);
    if (path != NULL && !viewer_request_open(&app, path, true)) {
        (void)fputs("viewer: invalid initial SVG path\n", stderr);
        destroy_viewer(&app);
        return EXIT_FAILURE;
    }
    unsigned smoke_poll_count = 0U;
    while (app.running) {
        platform_event event;
        while (platform_poll_event(app.window, &event)) {
            viewer_process_event(&app, &event);
            if (!app.running) break;
        }
        if (!app.running) break;
        mcp_broker_hook_poll(&app);
        service_job(&app);
        if (app.pending_open && app.job == NULL) start_pending_load(&app);
        if (app.editor_render_requested && app.job == NULL) start_editor_render(&app);
        if (!app.close_requested) start_next_sharp_tile(&app);
        if (viewer_editor_take_redraw_request(app.editor)) app.frame_dirty = true;
        if (!viewer_compose_and_present(&app)) {
            app.fatal_error = true;
            app.running = false;
            break;
        }
        if (app.smoke_mode && !app.smoke_scripted && app.document_visible &&
            app.presented_frames > 0U) {
            if (!script_native_smoke(&app)) {
                (void)fputs("native viewer smoke: event injection failed\n", stderr);
                app.fatal_error = true;
                app.running = false;
                break;
            }
        }
        poll_file_changes(&app);
        if (app.close_requested && app.job == NULL) {
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
            app.document_visible && app.presented_frames > 0U;
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
