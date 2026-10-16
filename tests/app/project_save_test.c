#define _POSIX_C_SOURCE 200809L
#define _DARWIN_C_SOURCE

#include "viewer_internal.h"
#include "viewer_editor_internal.h"

#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#define CHECK(condition) do { if (!(condition)) { \
    (void)fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, #condition); \
    return 1; } } while (0)

bool viewer_update_active_path(viewer_app *app, const char *path, size_t path_length)
{ (void)app; (void)path; (void)path_length; return true; }
void viewer_update_file_stamp(viewer_app *app) { (void)app; }
bool viewer_file_changed_on_disk(viewer_app *app) { (void)app; return false; }
bool viewer_set_window_title(viewer_app *app) { (void)app; return true; }
bool viewer_recovery_cleanup_after_save(viewer_app *app,
    const char *old_path, const char *new_path)
{ (void)app; (void)old_path; (void)new_path; return true; }

static bool set_x(viewer_editor *editor, const char *value)
{
    document_handle rect;
    return document_edit_find_id(editor->edit,
        (base_bytes){(const uint8_t *)"r", 1U}, &rect) &&
        document_edit_set_attribute(editor->edit, rect,
            (base_bytes){(const uint8_t *)"x", 1U},
            (base_bytes){(const uint8_t *)value, strlen(value)}) == DOCUMENT_EDIT_OK;
}

static bool set_id(viewer_editor *editor, const char *value)
{
    document_handle rect;
    if (!document_edit_find_id(editor->edit,
        (base_bytes){(const uint8_t *)"r", 1U}, &rect)) {
        if (!document_edit_find_id(editor->edit,
            (base_bytes){(const uint8_t *)"gone", 4U}, &rect)) return false;
    }
    return document_edit_set_attribute(editor->edit, rect,
        (base_bytes){(const uint8_t *)"id", 2U},
        (base_bytes){(const uint8_t *)value, strlen(value)}) == DOCUMENT_EDIT_OK;
}

int main(void)
{
    static const uint8_t source[] =
        "<svg width='16' height='16'><rect id='r' x='0' y='0' width='4' height='4'/></svg>";
    viewer_app app = {0};
    atomic_init(&app.project_save_cancel_requested, false);
    app.editor = viewer_editor_create_styled((base_bytes){source, sizeof(source) - 1U},
        UI_THEME_LIGHT, 800U, 600U, true);
    CHECK(app.editor != NULL && !viewer_editor_is_dirty(app.editor));
    CHECK(set_x(app.editor, "8") && viewer_editor_is_dirty(app.editor));
    const uint8_t *selected_rect = (const uint8_t *)strstr((const char *)source,
        "<rect");
    CHECK(selected_rect != NULL && viewer_editor_select_at_source_offset(app.editor,
        (size_t)(selected_rect - source) + 1U));
    document_source_span selection_before = {0};
    CHECK(viewer_editor_selection_span(app.editor, &selection_before));

    char path[] = "/tmp/vexel-app-save-XXXXXX";
    int fd = mkstemp(path);
    CHECK(fd >= 0 && close(fd) == 0 && unlink(path) == 0);
    char failed_directory[] = "/tmp/vexel-app-save-failure-XXXXXX";
    CHECK(mkdtemp(failed_directory) != NULL);
    CHECK(!viewer_save_project_file(&app, failed_directory,
        strlen(failed_directory)) && !app.document.native_project &&
        app.document.timeline_session == NULL &&
        app.document.project_owner == NULL && !app.timeline.native_project &&
        viewer_editor_is_dirty(app.editor));
    document_source_span selection_after_failure = {0};
    CHECK(viewer_editor_selection_span(app.editor, &selection_after_failure) &&
        selection_after_failure.offset == selection_before.offset);
    CHECK(rmdir(failed_directory) == 0);
    CHECK(viewer_save_project_file(&app, path, strlen(path)));
    CHECK(!viewer_editor_is_dirty(app.editor) && app.document.native_project);
    CHECK(app.document.timeline_session != NULL &&
        app.document.project_owner != NULL && app.timeline.native_project &&
        viewer_timeline_track_count(&app.timeline) == 0U &&
        app.timeline.dock_open);
    document_source_span selection_after_save = {0};
    CHECK(viewer_editor_selection_span(app.editor, &selection_after_save) &&
        selection_after_save.offset == selection_before.offset);
    CHECK(document_edit_undo(app.editor->edit) == DOCUMENT_EDIT_OK &&
        viewer_editor_is_dirty(app.editor));
    uint8_t undo_scene[sizeof(source) + 8U]; size_t undo_scene_length = 0U;
    CHECK(viewer_editor_materialize(app.editor, undo_scene, sizeof(undo_scene),
        &undo_scene_length) && undo_scene_length == sizeof(source) - 1U &&
        memcmp(undo_scene, source, sizeof(source) - 1U) == 0);
    CHECK(document_edit_redo(app.editor->edit) == DOCUMENT_EDIT_OK &&
        !viewer_editor_is_dirty(app.editor));
    fd = open(path, O_RDONLY);
    CHECK(fd >= 0);
    struct stat status;
    CHECK(fstat(fd, &status) == 0 && status.st_size > (off_t)PROJECT_BUNDLE_HEADER_BYTES);
    size_t encoded_length = (size_t)status.st_size;
    uint8_t *encoded = malloc(encoded_length);
    CHECK(encoded != NULL && read(fd, encoded, encoded_length) == (ssize_t)encoded_length);
    CHECK(close(fd) == 0);
    project_bundle_view view = {0};
    CHECK(project_bundle_decode((base_bytes){encoded, encoded_length}, &view).error ==
        PROJECT_BUNDLE_OK);
    static const uint8_t expected_scene[] =
        "<svg width='16' height='16'><rect id='r' x='8' y='0' width='4' height='4'/></svg>";
    CHECK(view.scene_svg.len == sizeof(expected_scene) - 1U &&
        memcmp(view.scene_svg.ptr, expected_scene, sizeof(expected_scene) - 1U) == 0 &&
        memcmp(view.manifest_json.ptr, "{\"format\":1,\"application\":\"Vexel\"}",
            view.manifest_json.len) == 0);
    project_timeline_context_owner default_context = {0};
    project_timeline_integration_result default_timeline =
        project_timeline_validate_bundle_owned(&view, view.scene_svg,
            NULL, NULL, NULL, &default_context);
    CHECK(default_timeline.validation.error == PROJECT_TIMELINE_OK &&
        default_timeline.compatibility == PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1);
    project_timeline_context_owner_destroy(&default_context);
    free(encoded);
    CHECK(unlink(path) == 0);

    viewer_timeline_destroy(&app.timeline);
    viewer_timeline_init(&app.timeline);
    project_timeline_session_destroy(app.document.timeline_session);
    app.document.timeline_session = NULL;
    project_bundle_asset_overlay_destroy(&app.document.asset_overlay);
    free(app.document.project_owner);
    app.document.project_owner = NULL;
    app.document.project = (project_bundle_view){0};
    app.document.native_project = false;

    static const uint8_t native_timeline[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"p\",\"kind\":\"property\","
        "\"target\":\"r\",\"visible\":true,\"property\":\"opacity\","
        "\"value_type\":\"scalar\",\"interpolation\":\"linear\","
        "\"keys\":[{\"tick\":0,\"value\":1}]}]}";
    static const uint8_t manifest[] = "{}";
    project_bundle_input native_input = {
        .manifest_json = {manifest, sizeof(manifest) - 1U},
        .scene_svg = {source, sizeof(source) - 1U},
        .timeline_json = {native_timeline, sizeof(native_timeline) - 1U}
    };
    project_bundle_result native_size = project_bundle_encoded_size(&native_input);
    CHECK(native_size.error == PROJECT_BUNDLE_OK);
    uint8_t *native_bytes = malloc(native_size.encoded_size);
    CHECK(native_bytes != NULL && project_bundle_encode(&native_input,
        (base_mut_bytes){native_bytes, native_size.encoded_size}).error == PROJECT_BUNDLE_OK);
    CHECK(project_bundle_decode((base_bytes){native_bytes, native_size.encoded_size},
        &app.document.project).error == PROJECT_BUNDLE_OK);
    CHECK(set_id(app.editor, "gone"));
    CHECK(!viewer_save_project_file(&app, path, strlen(path)));
    CHECK(viewer_editor_is_dirty(app.editor));
    CHECK(set_id(app.editor, "r"));
    CHECK(viewer_save_project_file(&app, path, strlen(path)));
    fd = open(path, O_RDONLY);
    CHECK(fd >= 0 && fstat(fd, &status) == 0 && status.st_size > 0);
    encoded_length = (size_t)status.st_size;
    encoded = malloc(encoded_length);
    CHECK(encoded != NULL && read(fd, encoded, encoded_length) ==
        (ssize_t)encoded_length && close(fd) == 0);
    CHECK(project_bundle_decode((base_bytes){encoded, encoded_length}, &view).error ==
        PROJECT_BUNDLE_OK);
    CHECK(view.timeline_json.len == sizeof(native_timeline) - 1U &&
        memcmp(view.timeline_json.ptr, native_timeline,
            sizeof(native_timeline) - 1U) == 0);
    free(encoded);
    CHECK(unlink(path) == 0);

    viewer_timeline_destroy(&app.timeline);
    viewer_timeline_init(&app.timeline);
    project_timeline_session_destroy(app.document.timeline_session);
    app.document.timeline_session = NULL;
    project_bundle_asset_overlay_destroy(&app.document.asset_overlay);
    free(app.document.project_owner);
    app.document.project_owner = NULL;
    app.document.project = (project_bundle_view){0};
    app.document.native_project = false;

    static const uint8_t drawing_timeline[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":24,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\",\"onion\":{\"before\":0,\"after\":0,\"opacity\":1},"
        "\"tracks\":[{\"id\":\"draw\",\"kind\":\"drawing\","
        "\"target\":\"r\",\"visible\":true,\"cels\":["
        "{\"tick\":0,\"asset\":\"cel0\"}]}]}";
    static const uint8_t invalid_svg[] = "<svg><script>active()</script></svg>";
    static const uint8_t cel_name[] = "cel0", svg_media[] = "image/svg+xml";
    project_bundle_asset invalid_drawing = {{cel_name, sizeof(cel_name) - 1U},
        {svg_media, sizeof(svg_media) - 1U},
        {invalid_svg, sizeof(invalid_svg) - 1U}};
    project_bundle_input invalid_drawing_input = {
        .manifest_json = {manifest, sizeof(manifest) - 1U},
        .scene_svg = {source, sizeof(source) - 1U},
        .timeline_json = {drawing_timeline, sizeof(drawing_timeline) - 1U},
        .assets = &invalid_drawing,
        .asset_count = 1U
    };
    project_bundle_result invalid_drawing_size =
        project_bundle_encoded_size(&invalid_drawing_input);
    CHECK(invalid_drawing_size.error == PROJECT_BUNDLE_OK);
    uint8_t *invalid_drawing_bytes = malloc(invalid_drawing_size.encoded_size);
    CHECK(invalid_drawing_bytes != NULL && project_bundle_encode(
        &invalid_drawing_input, (base_mut_bytes){invalid_drawing_bytes,
            invalid_drawing_size.encoded_size}).error == PROJECT_BUNDLE_OK);
    CHECK(project_bundle_decode((base_bytes){invalid_drawing_bytes,
        invalid_drawing_size.encoded_size}, &app.document.project).error ==
        PROJECT_BUNDLE_OK);
    CHECK(set_x(app.editor, "9"));
    bool invalid_saved = viewer_save_project_file(&app, path, strlen(path));
    CHECK(!invalid_saved &&
        viewer_editor_is_dirty(app.editor) &&
        strstr(viewer_editor_status(app.editor), "drawing asset cel0 is invalid") != NULL);
    CHECK(access(path, F_OK) != 0);
    free(invalid_drawing_bytes);

    static const uint8_t drawing_bytes[] =
        "<svg width='16' height='16'><defs><path id='p' d='M0 0L1 1'/></defs>"
        "<use href='#p'/></svg>";
    project_bundle_asset drawing_asset = {{cel_name, sizeof(cel_name) - 1U},
        {svg_media, sizeof(svg_media) - 1U},
        {drawing_bytes, sizeof(drawing_bytes) - 1U}};
    project_bundle_input drawing_input = {
        .manifest_json = {manifest, sizeof(manifest) - 1U},
        .scene_svg = {source, sizeof(source) - 1U},
        .timeline_json = {drawing_timeline, sizeof(drawing_timeline) - 1U},
        .assets = &drawing_asset,
        .asset_count = 1U
    };
    project_bundle_result drawing_size = project_bundle_encoded_size(&drawing_input);
    CHECK(drawing_size.error == PROJECT_BUNDLE_OK);
    uint8_t *drawing_owner = malloc(drawing_size.encoded_size);
    CHECK(drawing_owner != NULL && project_bundle_encode(&drawing_input,
        (base_mut_bytes){drawing_owner, drawing_size.encoded_size}).error ==
        PROJECT_BUNDLE_OK);
    CHECK(project_bundle_decode((base_bytes){drawing_owner, drawing_size.encoded_size},
        &app.document.project).error == PROJECT_BUNDLE_OK);
    project_timeline_context_owner timeline_context = {0};
    project_timeline_integration_result timeline_validation =
        project_timeline_validate_bundle_owned(&app.document.project,
            app.document.project.scene_svg, NULL, NULL, NULL, &timeline_context);
    CHECK(timeline_validation.compatibility ==
        PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1 &&
        timeline_validation.validation.error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_session_create(app.document.project.timeline_json,
        &timeline_context, project_timeline_edit_default_limits(),
        &app.document.timeline_session).error == PROJECT_TIMELINE_OK);
    CHECK(project_timeline_session_set_cel(app.document.timeline_session,
        (base_bytes){(const uint8_t *)"draw", 4U}, 12U,
        (base_bytes){cel_name, sizeof(cel_name) - 1U},
        (base_bytes){(const uint8_t *)"pose", 4U}).error == PROJECT_TIMELINE_OK);
    viewer_cancel_project_save(&app);
    CHECK(!viewer_save_project_file(&app, path, strlen(path)) &&
        access(path, F_OK) != 0);
    atomic_store_explicit(&app.project_save_cancel_requested, false,
                          memory_order_release);
    CHECK(viewer_save_project_file(&app, path, strlen(path)));
    fd = open(path, O_RDONLY);
    CHECK(fd >= 0 && fstat(fd, &status) == 0 && status.st_size > 0);
    encoded_length = (size_t)status.st_size;
    encoded = malloc(encoded_length);
    CHECK(encoded != NULL && read(fd, encoded, encoded_length) ==
        (ssize_t)encoded_length && close(fd) == 0);
    CHECK(project_bundle_decode((base_bytes){encoded, encoded_length}, &view).error ==
        PROJECT_BUNDLE_OK);
    base_bytes edited_timeline = project_timeline_session_source(
        app.document.timeline_session);
    CHECK(view.timeline_json.len == edited_timeline.len &&
        memcmp(view.timeline_json.ptr, edited_timeline.ptr, edited_timeline.len) == 0 &&
        view.timeline_json.len > sizeof(drawing_timeline) - 1U);
    project_bundle_asset saved_drawing;
    CHECK(project_bundle_asset_at(&view, 0U, &saved_drawing) &&
        saved_drawing.name.len == sizeof(cel_name) - 1U &&
        memcmp(saved_drawing.name.ptr, cel_name, sizeof(cel_name) - 1U) == 0 &&
        saved_drawing.data.len == sizeof(drawing_bytes) - 1U &&
        memcmp(saved_drawing.data.ptr, drawing_bytes,
            sizeof(drawing_bytes) - 1U) == 0);
    free(encoded);
    CHECK(unlink(path) == 0);
    free(drawing_owner);
    project_timeline_session_destroy(app.document.timeline_session);
    app.document.timeline_session = NULL;

    static const uint8_t opaque_manifest[] = "{\"title\":\"opaque project\"}";
    static const uint8_t opaque_timeline[] =
        "{\"schema\":\"future.timeline\",\"version\":7,\"value\":1e400}";
    static const uint8_t provenance_name[] = "source", provenance_type[] = "image/png";
    static const uint8_t provenance_data[] = {0x89U, 'P', 'N', 'G', 1U};
    project_bundle_asset provenance = {
        {provenance_name, sizeof(provenance_name) - 1U},
        {provenance_type, sizeof(provenance_type) - 1U},
        {provenance_data, sizeof(provenance_data)}
    };
    project_bundle_input opaque_input = {
        .manifest_json = {opaque_manifest, sizeof(opaque_manifest) - 1U},
        .scene_svg = {source, sizeof(source) - 1U},
        .timeline_json = {opaque_timeline, sizeof(opaque_timeline) - 1U},
        .assets = &provenance,
        .asset_count = 1U
    };
    project_bundle_result opaque_size = project_bundle_encoded_size(&opaque_input);
    CHECK(opaque_size.error == PROJECT_BUNDLE_OK);
    uint8_t *opaque_bytes = malloc(opaque_size.encoded_size);
    CHECK(opaque_bytes != NULL && project_bundle_encode(&opaque_input,
        (base_mut_bytes){opaque_bytes, opaque_size.encoded_size}).error == PROJECT_BUNDLE_OK);
    CHECK(project_bundle_decode((base_bytes){opaque_bytes, opaque_size.encoded_size},
        &app.document.project).error == PROJECT_BUNDLE_OK);
    CHECK(project_bundle_asset_overlay_init(&app.document.asset_overlay,
        &app.document.project));
    static const uint8_t new_drawing[] =
        "<svg width='2' height='2'><path d='M0 0L2 2'/></svg>";
    CHECK(project_bundle_asset_overlay_add_svg(&app.document.asset_overlay,
        (base_bytes){(const uint8_t *)"new-cel", 7U},
        (base_bytes){new_drawing, sizeof(new_drawing) - 1U}).error ==
        PROJECT_BUNDLE_OK);
    CHECK(project_bundle_asset_overlay_is_dirty(&app.document.asset_overlay));
    CHECK(viewer_save_project_file(&app, path, strlen(path)));
    CHECK(!project_bundle_asset_overlay_is_dirty(&app.document.asset_overlay));
    fd = open(path, O_RDONLY);
    CHECK(fd >= 0 && fstat(fd, &status) == 0 && status.st_size > 0);
    encoded_length = (size_t)status.st_size;
    encoded = malloc(encoded_length);
    CHECK(encoded != NULL && read(fd, encoded, encoded_length) ==
        (ssize_t)encoded_length && close(fd) == 0);
    CHECK(project_bundle_decode((base_bytes){encoded, encoded_length}, &view).error ==
        PROJECT_BUNDLE_OK);
    CHECK(view.manifest_json.len == sizeof(opaque_manifest) - 1U &&
        memcmp(view.manifest_json.ptr, opaque_manifest,
            sizeof(opaque_manifest) - 1U) == 0);
    CHECK(view.timeline_json.len == sizeof(opaque_timeline) - 1U &&
        memcmp(view.timeline_json.ptr, opaque_timeline,
            sizeof(opaque_timeline) - 1U) == 0);
    project_bundle_asset saved_provenance;
    CHECK(project_bundle_asset_at(&view, 0U, &saved_provenance));
    CHECK(saved_provenance.name.len == sizeof(provenance_name) - 1U &&
        memcmp(saved_provenance.name.ptr, provenance_name,
            sizeof(provenance_name) - 1U) == 0 &&
        saved_provenance.media_type.len == sizeof(provenance_type) - 1U &&
        memcmp(saved_provenance.media_type.ptr, provenance_type,
            sizeof(provenance_type) - 1U) == 0 &&
        saved_provenance.data.len == sizeof(provenance_data) &&
        memcmp(saved_provenance.data.ptr, provenance_data,
            sizeof(provenance_data)) == 0);
    CHECK(view.asset_count == 2U);
    project_bundle_asset saved_new_drawing;
    CHECK(project_bundle_asset_at(&view, 1U, &saved_new_drawing) &&
        saved_new_drawing.name.len == 7U &&
        memcmp(saved_new_drawing.name.ptr, "new-cel", 7U) == 0 &&
        saved_new_drawing.data.len == sizeof(new_drawing) - 1U &&
        memcmp(saved_new_drawing.data.ptr, new_drawing,
            sizeof(new_drawing) - 1U) == 0);
    free(encoded);
    CHECK(unlink(path) == 0);

    char directory[] = "/tmp/vexel-app-save-dir-XXXXXX";
    CHECK(mkdtemp(directory) != NULL);
    CHECK(set_x(app.editor, "12") && viewer_editor_is_dirty(app.editor));
    CHECK(!viewer_save_project_file(&app, directory, strlen(directory)));
    CHECK(viewer_editor_is_dirty(app.editor));
    CHECK(rmdir(directory) == 0);
    free(native_bytes);
    free(opaque_bytes);
    project_bundle_asset_overlay_destroy(&app.document.asset_overlay);
    viewer_timeline_destroy(&app.timeline);
    viewer_editor_destroy(app.editor);
    return 0;
}
