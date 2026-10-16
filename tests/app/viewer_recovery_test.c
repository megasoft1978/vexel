#define _DARWIN_C_SOURCE 1
#define _POSIX_C_SOURCE 200809L
#include "viewer_internal.h"
#include "viewer_editor_internal.h"
#include "platform/macos/platform_dialog.h"

#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

static platform_recovery_choice requested_recovery_choice = PLATFORM_RECOVERY_CANCEL;
bool viewer_submit_job(viewer_app *app, viewer_job *job)
{ app->job = job; return true; }
platform_recovery_choice platform_dialog_recovery_choice(const char *title,
    size_t title_length, const char *message, size_t message_length,
    bool conflict)
{ (void)title; (void)title_length; (void)message; (void)message_length;
  (void)conflict; return requested_recovery_choice; }

#define CHECK(c) do { if (!(c)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #c); \
    return EXIT_FAILURE; } } while (0)

int main(void)
{
    static const uint8_t scene[] =
        "<svg width='16' height='16'><rect id='r' x='0' y='0' width='4' height='4'/></svg>";
    static const uint8_t timeline[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,"
        "\"after\":0,\"opacity\":1},\"tracks\":[]}";
    static const uint8_t manifest[] = "{\"title\":\"Recovery\"}";
    static const uint8_t asset_name[] = "source";
    static const uint8_t media_type[] = "image/svg+xml";
    static const uint8_t asset_data[] = "<svg><path d='M0 0L1 1'/></svg>";
    project_bundle_asset asset = {
        {asset_name, sizeof(asset_name) - 1U},
        {media_type, sizeof(media_type) - 1U},
        {asset_data, sizeof(asset_data) - 1U}
    };
    project_bundle_input project = {
        {manifest, sizeof(manifest) - 1U},
        {scene, sizeof(scene) - 1U},
        {timeline, sizeof(timeline) - 1U}, &asset, 1U
    };
    project_bundle_result size = project_bundle_encoded_size(&project);
    CHECK(size.error == PROJECT_BUNDLE_OK);
    uint8_t *bundle_bytes = malloc(size.encoded_size);
    CHECK(bundle_bytes != NULL && project_bundle_encode(&project,
        (base_mut_bytes){bundle_bytes, size.encoded_size}).error == PROJECT_BUNDLE_OK);
    project_bundle_view bundle = {0};
    CHECK(project_bundle_decode((base_bytes){bundle_bytes, size.encoded_size},
        &bundle).error == PROJECT_BUNDLE_OK);
    project_timeline_context_owner context = {0};
    CHECK(project_timeline_validate_bundle_owned(&bundle, bundle.scene_svg,
        NULL, NULL, NULL, &context).validation.error == PROJECT_TIMELINE_OK);
    project_timeline_session *session = NULL;
    CHECK(project_timeline_session_create(bundle.timeline_json, &context,
        project_timeline_edit_default_limits(), &session).error == PROJECT_TIMELINE_OK);
    viewer_editor *editor = viewer_editor_create_styled(bundle.scene_svg,
        UI_THEME_LIGHT, 800U, 600U, true);
    CHECK(editor != NULL);
    document_handle rect;
    CHECK(document_edit_find_id(editor->edit,
        (base_bytes){(const uint8_t *)"r", 1U}, &rect));
    CHECK(document_edit_set_attribute(editor->edit, rect,
        (base_bytes){(const uint8_t *)"x", 1U},
        (base_bytes){(const uint8_t *)"8", 1U}) == DOCUMENT_EDIT_OK);

    char directory[] = "/tmp/vexel-viewer-recovery-XXXXXX";
    CHECK(mkdtemp(directory) != NULL);
    char source_path[1024], capsule_path[1024];
    int count = snprintf(source_path, sizeof(source_path), "%s/source.vexel", directory);
    CHECK(count > 0 && (size_t)count < sizeof(source_path));
    count = snprintf(capsule_path, sizeof(capsule_path), "%s/capsule.vxr", directory);
    CHECK(count > 0 && (size_t)count < sizeof(capsule_path));
    int fd = open(source_path, O_CREAT | O_WRONLY | O_EXCL, 0600);
    CHECK(fd >= 0 && write(fd, bundle_bytes, size.encoded_size) ==
        (ssize_t)size.encoded_size && close(fd) == 0);
    struct stat stat_value;
    CHECK(stat(source_path, &stat_value) == 0);

    viewer_job job = {0};
    job.kind = VIEWER_JOB_RECOVERY;
    CHECK(viewer_editor_snapshot_create(editor, &job.recovery_scene) == DOCUMENT_EDIT_OK);
    CHECK(project_timeline_session_source_lease_create(session,
        &job.recovery_timeline).error == PROJECT_TIMELINE_OK);
    job.recovery_assets = calloc(1U, sizeof(*job.recovery_assets));
    CHECK(job.recovery_assets != NULL && project_bundle_asset_at(&bundle, 0U,
        job.recovery_assets));
    job.recovery_asset_count = 1U;
    job.recovery_manifest = bundle.manifest_json;
    memcpy(job.recovery_source_path, source_path, strlen(source_path) + 1U);
    memcpy(job.recovery_path, capsule_path, strlen(capsule_path) + 1U);
    job.recovery_source_size = stat_value.st_size;
    job.recovery_source_modified = stat_value.st_mtimespec;
    job.recovery_stamp.valid = true;
    job.recovery_stamp.size = stat_value.st_size;
    job.recovery_stamp.modified = job.recovery_source_modified;
    job.recovery_captured_time_ns = UINT64_C(99887766);
    CHECK(viewer_editor_is_dirty(editor));
    CHECK(viewer_recovery_test_run(&job, NULL) == 1);
    CHECK(viewer_editor_is_dirty(editor) && !project_timeline_session_is_dirty(session));
    fd = open(capsule_path, O_RDONLY);
    CHECK(fd >= 0 && fstat(fd, &stat_value) == 0 && stat_value.st_size > 0);
    size_t capsule_length = (size_t)stat_value.st_size;
    uint8_t *capsule = malloc(capsule_length);
    CHECK(capsule != NULL && read(fd, capsule, capsule_length) ==
        (ssize_t)capsule_length && close(fd) == 0);
    project_recovery_view recovery = {0};
    CHECK(project_recovery_decode((base_bytes){capsule, capsule_length},
        (base_bytes){(const uint8_t *)source_path, strlen(source_path)},
        &recovery).error == PROJECT_RECOVERY_OK);
    CHECK(project_bundle_has_magic(recovery.project.encoded));
    CHECK(recovery.project.scene_svg.len > 0U &&
        memchr(recovery.project.scene_svg.ptr, '8', recovery.project.scene_svg.len) != NULL &&
        recovery.project.asset_count == 1U);
    project_bundle_asset decoded_asset;
    CHECK(project_bundle_asset_at(&recovery.project, 0U, &decoded_asset) &&
        decoded_asset.name.len == sizeof(asset_name) - 1U &&
        memcmp(decoded_asset.name.ptr, asset_name, sizeof(asset_name) - 1U) == 0 &&
        decoded_asset.data.len == sizeof(asset_data) - 1U &&
        memcmp(decoded_asset.data.ptr, asset_data, sizeof(asset_data) - 1U) == 0);

    viewer_job probe = {0};
    memcpy(probe.path, source_path, strlen(source_path) + 1U);
    fd = open(source_path, O_WRONLY | O_APPEND);
    CHECK(fd >= 0 && write(fd, "x", 1U) == 1 && close(fd) == 0);
    CHECK(viewer_recovery_probe(&probe, capsule_path, NULL) &&
        probe.recovery_available && probe.recovery_conflict &&
        probe.recovery_length == capsule_length);
    viewer_job wrong_path = {0};
    memcpy(wrong_path.path, source_path, strlen(source_path) + 1U);
    wrong_path.path[strlen(wrong_path.path) - 1U] = 'x';
    CHECK(!viewer_recovery_probe(&wrong_path, capsule_path, NULL) &&
        !wrong_path.recovery_available);
    viewer_recovery_job_destroy(&wrong_path);
    viewer_editor *recovered_editor = viewer_editor_create_styled(
        recovery.project.scene_svg, UI_THEME_LIGHT, 800U, 600U, true);
    CHECK(recovered_editor != NULL && !viewer_editor_is_dirty(recovered_editor));
    viewer_editor_mark_unsaved(recovered_editor);
    CHECK(viewer_editor_is_dirty(recovered_editor));

    viewer_app app = {0};
    app.editor = recovered_editor;
    app.generation = 3U;
    app.document.native_project = true;
    app.recovered_document = true;
    memcpy(app.path, source_path, strlen(source_path) + 1U);
    requested_recovery_choice = PLATFORM_RECOVERY_RESTORE;
    viewer_recovery_consider(&app, &probe);
    CHECK(app.job != NULL && app.job->kind == VIEWER_JOB_RECOVERY_LOAD &&
        app.job->recovery_document && app.job->source_snapshot != NULL);
    CHECK(viewer_recovery_load_job_run(app.job, NULL));
    CHECK(app.job->loaded.native_project && app.job->loaded.project.asset_count == 1U &&
        app.job->loaded.timeline_session != NULL);
    viewer_loaded_document_destroy(&app.job->loaded);
    viewer_recovery_job_destroy(app.job);
    free(app.job);
    app.job = NULL;
    viewer_recovery_job_destroy(&probe);
    CHECK(access(capsule_path, F_OK) == 0);

    CHECK(stat(source_path, &stat_value) == 0);
    job.recovery_source_size = stat_value.st_size;
    job.recovery_source_modified = stat_value.st_mtimespec;
    job.recovery_stamp.valid = true;
    job.recovery_stamp.size = stat_value.st_size;
    job.recovery_stamp.modified = stat_value.st_mtimespec;
    CHECK(viewer_recovery_test_run(&job, NULL) == 1);
    viewer_job discard_probe = {0};
    memcpy(discard_probe.path, source_path, strlen(source_path) + 1U);
    CHECK(viewer_recovery_probe(&discard_probe, capsule_path, NULL) &&
        discard_probe.recovery_available);
    requested_recovery_choice = PLATFORM_RECOVERY_CANCEL;
    viewer_recovery_consider(&app, &discard_probe);
    CHECK(access(capsule_path, F_OK) == 0);
    requested_recovery_choice = PLATFORM_RECOVERY_DISCARD;
    viewer_recovery_consider(&app, &discard_probe);
    CHECK(access(capsule_path, F_OK) != 0);
    viewer_recovery_job_destroy(&discard_probe);

    count = snprintf(capsule_path, sizeof(capsule_path), "%s/save-capsule.vxr",
        directory);
    CHECK(count > 0 && (size_t)count < sizeof(capsule_path));
    fd = open(capsule_path, O_CREAT | O_WRONLY | O_EXCL, 0600);
    CHECK(fd >= 0 && close(fd) == 0);
    app.document.native_project = true;
    app.recovered_document = true;
    CHECK(viewer_recovery_cleanup_capsule_paths(&app, NULL, capsule_path));
    CHECK(access(capsule_path, F_OK) != 0);

    CHECK(mkdir(capsule_path, 0700) == 0);
    app.recovered_document = true;
    CHECK(!viewer_recovery_cleanup_capsule_paths(&app, NULL, capsule_path));
    CHECK(access(capsule_path, F_OK) == 0);
    CHECK(rmdir(capsule_path) == 0);
    viewer_editor_destroy(recovered_editor);

    free(capsule);
    CHECK(unlink(source_path) == 0 && rmdir(directory) == 0);
    viewer_recovery_job_destroy(&job);
    viewer_editor_destroy(editor);
    project_timeline_session_destroy(session);
    free(bundle_bytes);
    return EXIT_SUCCESS;
}
