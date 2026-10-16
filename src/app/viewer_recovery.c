#define _POSIX_C_SOURCE 200809L

#include "viewer_internal.h"
#include "platform/macos/platform_dialog.h"

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <time.h>
#include <unistd.h>

#define VIEWER_RECOVERY_INTERVAL_SECONDS 30

static bool canceled(const platform_task_cancel_token *token)
{
    return token != NULL && atomic_load_explicit(&token->canceled,
        memory_order_acquire);
}

bool viewer_recovery_job_run(viewer_job *job,
    const platform_task_cancel_token *token)
{
    if (job == NULL || job->kind != VIEWER_JOB_RECOVERY ||
        job->recovery_scene == NULL || job->recovery_timeline == NULL ||
        canceled(token)) return false;
    size_t scene_length = document_edit_snapshot_byte_length(job->recovery_scene);
    if (scene_length == 0U || scene_length > PROJECT_BUNDLE_MAX_SCENE_BYTES)
        return false;
    uint8_t *scene = malloc(scene_length);
    if (scene == NULL) return false;
    size_t materialized = 0U;
    bool okay = document_edit_snapshot_materialize(job->recovery_scene, scene,
        scene_length, &materialized) && materialized == scene_length &&
        !canceled(token);
    base_bytes timeline = project_timeline_source_lease_bytes(
        job->recovery_timeline);
    project_bundle_input project = {
        .manifest_json = job->recovery_manifest,
        .scene_svg = {scene, materialized},
        .timeline_json = timeline,
        .assets = job->recovery_assets,
        .asset_count = job->recovery_asset_count
    };
    project_recovery_input input = {
        .source_path = {(const uint8_t *)job->recovery_source_path,
            strlen(job->recovery_source_path)},
        .source_size = (uint64_t)job->recovery_source_size,
        .source_modified_seconds = (int64_t)job->recovery_source_modified.tv_sec,
        .source_modified_nanoseconds = (uint32_t)job->recovery_source_modified.tv_nsec,
        .captured_time_nanoseconds = job->recovery_captured_time_ns,
        .project = &project
    };
    if (okay && !canceled(token))
        job->recovery_result = project_recovery_save(
            (base_bytes){(const uint8_t *)job->recovery_path,
                strlen(job->recovery_path)}, &input);
    else job->recovery_result.error = PROJECT_RECOVERY_INVALID_ARGUMENT;
    free(scene);
    if (!job->recovery_result.published) return false;
    viewer_file_stamp after = {0};
    bool stable = viewer_file_stamp_read(job->recovery_source_path, &after) &&
        viewer_file_stamp_equal(&after, &job->recovery_stamp);
    if (!stable && job->recovery_path[0] != '\0')
        (void)unlink(job->recovery_path);
    return stable;
}

bool viewer_recovery_load_job_run(viewer_job *job,
    const platform_task_cancel_token *token)
{
    if (job == NULL || job->kind != VIEWER_JOB_RECOVERY_LOAD) return false;
    uint8_t *source = job->source_snapshot;
    job->source_snapshot = NULL;
    job->load_status = viewer_decode_owned_project(source,
        job->source_snapshot_length, token, &job->loaded);
    return job->load_status == VIEWER_LOAD_OK;
}

void viewer_recovery_job_destroy(viewer_job *job)
{
    if (job == NULL) return;
    document_edit_snapshot_destroy(job->recovery_scene);
    job->recovery_scene = NULL;
    project_timeline_source_lease_destroy(job->recovery_timeline);
    job->recovery_timeline = NULL;
    free(job->recovery_assets);
    job->recovery_assets = NULL;
    free(job->recovery_bytes);
    job->recovery_bytes = NULL;
    job->recovery_length = 0U;
}

bool viewer_recovery_probe(viewer_job *job, const char *capsule_path,
    const platform_task_cancel_token *token)
{
    if (job == NULL || capsule_path == NULL || capsule_path[0] == '\0')
        return false;
    size_t capsule_path_length = strnlen(capsule_path, sizeof(job->recovery_path));
    if (capsule_path_length == 0U || capsule_path_length >= sizeof(job->recovery_path))
        return false;
    memcpy(job->recovery_path, capsule_path, capsule_path_length + 1U);
    FILE *file = fopen(capsule_path, "rb");
    if (file == NULL) {
        if (errno != ENOENT)
            (void)snprintf(job->recovery_notice, sizeof(job->recovery_notice),
                "Recovery snapshot could not be read");
        return errno == ENOENT;
    }
    struct stat status;
    bool sized = fstat(fileno(file), &status) == 0 && S_ISREG(status.st_mode) &&
        status.st_size >= (off_t)PROJECT_RECOVERY_HEADER_BYTES &&
        (uintmax_t)status.st_size <= (uintmax_t)PROJECT_RECOVERY_MAX_BYTES;
    if (!sized) {
        (void)snprintf(job->recovery_notice, sizeof(job->recovery_notice),
            "Recovery snapshot is invalid or exceeds the recovery limit");
        (void)fclose(file);
        return false;
    }
    size_t length = (size_t)status.st_size;
    uint8_t *bytes = malloc(length);
    if (bytes == NULL) {
        (void)snprintf(job->recovery_notice, sizeof(job->recovery_notice),
            "Recovery snapshot could not be allocated");
        (void)fclose(file);
        return false;
    }
    size_t offset = 0U;
    while (offset < length) {
        if (canceled(token)) break;
        size_t amount = length - offset;
        if (amount > 1024U * 1024U) amount = 1024U * 1024U;
        size_t received = fread(bytes + offset, 1U, amount, file);
        if (received != amount) break;
        offset += received;
    }
    bool complete = offset == length && !canceled(token);
    bool closed = fclose(file) == 0;
    bool read_ok = complete && closed;
    if (!read_ok) {
        free(bytes);
        if (!canceled(token))
            (void)snprintf(job->recovery_notice, sizeof(job->recovery_notice),
                "Recovery snapshot could not be read completely");
        return false;
    }
    size_t source_length = strnlen(job->path, sizeof(job->path));
    project_recovery_view view = {0};
    project_recovery_result decoded = project_recovery_decode(
        (base_bytes){bytes, length},
        (base_bytes){(const uint8_t *)job->path, source_length}, &view);
    if (source_length == 0U || source_length >= sizeof(job->path) ||
        decoded.error != PROJECT_RECOVERY_OK) {
        free(bytes);
        (void)snprintf(job->recovery_notice, sizeof(job->recovery_notice),
            "Recovery snapshot is corrupt or belongs to another file");
        return false;
    }
    job->recovery_bytes = bytes;
    job->recovery_length = length;
    job->recovery_source_size_on_disk = view.source_size;
    job->recovery_source_mtime_seconds = view.source_modified_seconds;
    job->recovery_source_mtime_nanoseconds = view.source_modified_nanoseconds;
    job->recovery_available = true;
    viewer_file_stamp current = {0};
    job->recovery_conflict = !viewer_file_stamp_read(job->path, &current) ||
        (uint64_t)current.size != view.source_size ||
        current.modified.tv_sec != (time_t)view.source_modified_seconds ||
        (uint32_t)current.modified.tv_nsec != view.source_modified_nanoseconds;
    return true;
}

bool viewer_recovery_discover(viewer_job *job,
    const platform_task_cancel_token *token)
{
    if (job == NULL || !job->loaded.native_project) return false;
    base_mut_bytes output = {(uint8_t *)job->recovery_path,
        sizeof(job->recovery_path)};
    platform_recovery_store_result resolved = platform_recovery_store_path(
        (base_bytes){NULL, 0U},
        (base_bytes){(const uint8_t *)job->path, strlen(job->path)}, output);
    if (resolved.error != PLATFORM_RECOVERY_STORE_OK) return false;
    return viewer_recovery_probe(job, job->recovery_path, token);
}

void viewer_recovery_after_open(viewer_job *job,
    const platform_task_cancel_token *token)
{
    if (job != NULL && job->app != NULL &&
        job->load_status == VIEWER_LOAD_OK && !job->app->smoke_mode)
        (void)viewer_recovery_discover(job, token);
}

bool viewer_recovery_remove_for_path(const char *source_path)
{
    if (source_path == NULL || source_path[0] == '\0') return false;
    char path[PLATFORM_RECOVERY_STORE_PATH_MAX];
    platform_recovery_store_result resolved = platform_recovery_store_path(
        (base_bytes){NULL, 0U},
        (base_bytes){(const uint8_t *)source_path, strlen(source_path)},
        (base_mut_bytes){(uint8_t *)path, sizeof(path)});
    if (resolved.error != PLATFORM_RECOVERY_STORE_OK) return false;
    return unlink(path) == 0 || errno == ENOENT;
}

bool viewer_recovery_cleanup_after_save(viewer_app *app,
    const char *previous_path, const char *target_path)
{
    if (app == NULL || target_path == NULL || target_path[0] == '\0') return false;
    if (!app->document.native_project && !app->recovered_document) return true;
    char previous_capsule[PLATFORM_RECOVERY_STORE_PATH_MAX] = {0};
    char target_capsule[PLATFORM_RECOVERY_STORE_PATH_MAX] = {0};
    bool resolved = true;
    if (previous_path != NULL && previous_path[0] != '\0' &&
        strcmp(previous_path, target_path) != 0) {
        platform_recovery_store_result old = platform_recovery_store_path(
            (base_bytes){NULL, 0U},
            (base_bytes){(const uint8_t *)previous_path, strlen(previous_path)},
            (base_mut_bytes){(uint8_t *)previous_capsule, sizeof(previous_capsule)});
        if (old.error != PLATFORM_RECOVERY_STORE_OK) resolved = false;
    }
    platform_recovery_store_result target = platform_recovery_store_path(
        (base_bytes){NULL, 0U},
        (base_bytes){(const uint8_t *)target_path, strlen(target_path)},
        (base_mut_bytes){(uint8_t *)target_capsule, sizeof(target_capsule)});
    if (target.error != PLATFORM_RECOVERY_STORE_OK) resolved = false;
    if (target_capsule[0] == '\0') {
        if (app->editor != NULL)
            viewer_editor_set_status(app->editor,
                "Saved; a recovery snapshot could not be removed");
        return false;
    }
    bool okay = viewer_recovery_cleanup_capsule_paths(app,
        previous_capsule[0] == '\0' ? NULL : previous_capsule, target_capsule);
    if (!resolved && app->editor != NULL)
        viewer_editor_set_status(app->editor,
            "Saved; a recovery snapshot could not be removed");
    return okay && resolved;
}

bool viewer_recovery_cleanup_capsule_paths(viewer_app *app,
    const char *previous_capsule_path, const char *target_capsule_path)
{
    if (app == NULL || target_capsule_path == NULL || target_capsule_path[0] == '\0')
        return false;
    if (!app->document.native_project && !app->recovered_document) return true;
    bool okay = true;
    if (previous_capsule_path != NULL && previous_capsule_path[0] != '\0' &&
        strcmp(previous_capsule_path, target_capsule_path) != 0)
        okay = (unlink(previous_capsule_path) == 0 || errno == ENOENT) && okay;
    okay = (unlink(target_capsule_path) == 0 || errno == ENOENT) && okay;
    app->recovered_document = false;
    if (!okay && app->editor != NULL)
        viewer_editor_set_status(app->editor,
            "Saved; a recovery snapshot could not be removed");
    return okay;
}

void viewer_recovery_consider(viewer_app *app, viewer_job *job)
{
    if (app == NULL || job == NULL) return;
    if (!job->recovery_available) {
        if (job->recovery_notice[0] != '\0' && app->editor != NULL)
            viewer_editor_set_status(app->editor, job->recovery_notice);
        return;
    }
    viewer_file_stamp current = {0};
    job->recovery_conflict = !viewer_file_stamp_read(app->path, &current) ||
        (uint64_t)current.size != job->recovery_source_size_on_disk ||
        current.modified.tv_sec != (time_t)job->recovery_source_mtime_seconds ||
        (uint32_t)current.modified.tv_nsec !=
            job->recovery_source_mtime_nanoseconds;
    const char *title = job->recovery_conflict ?
        "Recovery Conflict" : "Recover Unsaved Work?";
    const char *message = job->recovery_conflict ?
        "The source file changed after this snapshot. Restore the snapshot as unsaved work, leaving the source file untouched?" :
        "A validated recovery snapshot is available for this project.";
    platform_recovery_choice choice = platform_dialog_recovery_choice(title,
        strlen(title), message, strlen(message), job->recovery_conflict);
    if (choice == PLATFORM_RECOVERY_DISCARD) {
        bool removed = job->recovery_path[0] != '\0' &&
            (unlink(job->recovery_path) == 0 || errno == ENOENT);
        if (removed)
            viewer_editor_set_status(app->editor, "Recovery snapshot discarded");
        else viewer_editor_set_status(app->editor,
            "Recovery snapshot could not be discarded");
        app->frame_dirty = true;
        return;
    }
    if (choice != PLATFORM_RECOVERY_RESTORE) {
        if (choice == PLATFORM_RECOVERY_CANCEL)
            viewer_editor_set_status(app->editor, "Recovery snapshot kept");
        else viewer_editor_set_status(app->editor,
            "Recovery choice could not be shown");
        app->frame_dirty = true;
        return;
    }
    project_recovery_view view = {0};
    project_recovery_result decoded = project_recovery_decode(
        (base_bytes){job->recovery_bytes, job->recovery_length},
        (base_bytes){(const uint8_t *)app->path, strlen(app->path)}, &view);
    if (decoded.error != PROJECT_RECOVERY_OK || view.project.encoded.ptr == NULL) {
        viewer_editor_set_status(app->editor,
            "Recovery snapshot became invalid; the source remains open");
        app->frame_dirty = true;
        return;
    }
    size_t bundle_length = view.project.encoded.len;
    memmove(job->recovery_bytes, view.project.encoded.ptr, bundle_length);
    viewer_job *restore = calloc(1U, sizeof(*restore));
    if (restore == NULL) {
        viewer_editor_set_status(app->editor,
            "Recovery could not start; the source remains open");
        app->frame_dirty = true;
        return;
    }
    restore->app = app;
    restore->kind = VIEWER_JOB_RECOVERY_LOAD;
    restore->generation = app->generation;
    memcpy(restore->path, app->path, strlen(app->path) + 1U);
    restore->source_snapshot = job->recovery_bytes;
    restore->source_snapshot_length = bundle_length;
    restore->recovery_document = true;
    job->recovery_bytes = NULL;
    job->recovery_length = 0U;
    if (!viewer_submit_job(app, restore)) {
        free(restore->source_snapshot);
        free(restore);
        viewer_editor_set_status(app->editor,
            "Recovery could not start; the source remains open");
    } else viewer_editor_set_status(app->editor, "Restoring recovery snapshot");
    app->frame_dirty = true;
}

int viewer_recovery_test_run(viewer_job *job,
    const platform_task_cancel_token *token)
{
    return viewer_recovery_job_run(job, token) ? 1 : 0;
}

static bool dirty_native_document(viewer_app *app)
{
    return app->document.native_project && app->editor != NULL &&
        app->document.timeline_session != NULL && app->path[0] != '\0' &&
        app->file_stamp.valid &&
        (viewer_editor_is_dirty(app->editor) ||
         project_timeline_session_is_dirty(app->document.timeline_session) ||
         project_bundle_asset_overlay_is_dirty(&app->document.asset_overlay));
}

void viewer_recovery_tick(viewer_app *app)
{
    if (app == NULL || app->smoke_mode || app->job != NULL ||
        app->pending_open || app->pending_new_document || app->close_requested ||
        !dirty_native_document(app)) return;
    struct timespec now = {0};
    if (clock_gettime(CLOCK_MONOTONIC, &now) != 0) return;
    if (app->last_recovery.tv_sec != 0 &&
        now.tv_sec - app->last_recovery.tv_sec < VIEWER_RECOVERY_INTERVAL_SECONDS)
        return;
    viewer_job *job = calloc(1U, sizeof(*job));
    if (job == NULL) return;
    job->app = app;
    job->kind = VIEWER_JOB_RECOVERY;
    job->generation = app->generation;
    if (viewer_editor_snapshot_create(app->editor, &job->recovery_scene) !=
            DOCUMENT_EDIT_OK ||
        project_timeline_session_source_lease_create(
            app->document.timeline_session, &job->recovery_timeline).error !=
            PROJECT_TIMELINE_OK) goto failed;
    job->recovery_asset_count = app->document.project.asset_count;
    if (job->recovery_asset_count > PROJECT_BUNDLE_MAX_ASSET_COUNT ||
        job->recovery_asset_count > SIZE_MAX / sizeof(*job->recovery_assets))
        goto failed;
    if (job->recovery_asset_count > 0U) {
        job->recovery_assets = calloc(job->recovery_asset_count,
            sizeof(*job->recovery_assets));
        if (job->recovery_assets == NULL) goto failed;
        for (size_t index = 0U; index < job->recovery_asset_count; ++index)
            if (!project_bundle_asset_at(&app->document.project, index,
                    &job->recovery_assets[index])) goto failed;
    }
    static const uint8_t default_manifest[] =
        "{\"format\":1,\"application\":\"Vexel\"}";
    job->recovery_manifest = app->document.project.version == PROJECT_BUNDLE_VERSION ?
        app->document.project.manifest_json :
        (base_bytes){default_manifest, sizeof(default_manifest) - 1U};
    size_t path_length = strnlen(app->path, sizeof(app->path));
    if (path_length == 0U || path_length >= sizeof(app->path)) goto failed;
    memcpy(job->recovery_source_path, app->path, path_length + 1U);
    base_mut_bytes recovery_path = {
        (uint8_t *)job->recovery_path, sizeof(job->recovery_path)};
    platform_recovery_store_result resolved = platform_recovery_store_path(
        (base_bytes){NULL, 0U},
        (base_bytes){(const uint8_t *)job->recovery_source_path, path_length},
        recovery_path);
    if (resolved.error != PLATFORM_RECOVERY_STORE_OK) goto failed;
    viewer_file_stamp current_stamp = {0};
    if (!viewer_file_stamp_read(job->recovery_source_path, &current_stamp) ||
        !viewer_file_stamp_equal(&current_stamp, &app->file_stamp)) goto failed;
    job->recovery_stamp = current_stamp;
    job->recovery_source_size = current_stamp.size;
    job->recovery_source_modified = current_stamp.modified;
    struct timespec wall = {0};
    if (clock_gettime(CLOCK_REALTIME, &wall) != 0 || wall.tv_sec < 0 ||
        (uint64_t)wall.tv_sec > UINT64_MAX / UINT64_C(1000000000)) goto failed;
    job->recovery_captured_time_ns = (uint64_t)wall.tv_sec *
        UINT64_C(1000000000) + (uint64_t)wall.tv_nsec;
    if (!viewer_submit_job(app, job)) goto failed;
    app->last_recovery = now;
    app->external_change_pending = true;
    return;
failed:
    if (app->editor != NULL)
        viewer_editor_set_status(app->editor, "Recovery snapshot could not start");
    viewer_recovery_job_destroy(job);
    free(job);
}
