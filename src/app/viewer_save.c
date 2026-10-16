#include "viewer_internal.h"
#include "viewer_editor_internal.h"

#include "platform/macos/platform_dialog.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static bool project_save_canceled(void *context)
{
    viewer_app *app = context;
    return atomic_load_explicit(&app->project_save_cancel_requested,
                                memory_order_acquire);
}

void viewer_cancel_project_save(viewer_app *app)
{
    if (app != NULL)
        atomic_store_explicit(&app->project_save_cancel_requested, true,
                              memory_order_release);
}

size_t viewer_event_path_length(const char *path, size_t limit)
{
    size_t length = 0U;
    if (path == NULL) return 0U;
    while (length <= limit && path[length] != '\0') length++;
    return length;
}

static void show_save_error(viewer_app *app, const char *title, const char *message)
{
    if (app->editor != NULL) viewer_editor_set_status(app->editor, message);
    (void)platform_dialog_show_error(title, strlen(title), message, strlen(message));
    app->frame_dirty = true;
}

static bool path_has_extension(const char *path, size_t length, const char *extension)
{
    size_t extension_length = strlen(extension);
    if (length <= extension_length) return false;
    size_t start = length - extension_length;
    if (path[start - 1U] != '.') return false;
    for (size_t index = 0U; index < extension_length; ++index) {
        char value = path[start + index];
        if (value >= 'A' && value <= 'Z') value = (char)(value - 'A' + 'a');
        if (value != extension[index]) return false;
    }
    return true;
}

bool viewer_save_project_file(viewer_app *app, const char *path, size_t path_length)
{
    char previous_path[PLATFORM_PATH_CAP];
    size_t previous_length = strnlen(app->path, sizeof(app->path));
    if (previous_length < sizeof(previous_path))
        memcpy(previous_path, app->path, previous_length + 1U);
    else previous_path[0] = '\0';
    size_t scene_length = viewer_editor_byte_length(app->editor);
    if (scene_length == 0U || scene_length > PROJECT_BUNDLE_MAX_SCENE_BYTES) {
        viewer_editor_set_status(app->editor, "Save failed: scene exceeds the project format limit");
        return false;
    }
    uint8_t *scene = malloc(scene_length);
    if (scene == NULL) {
        viewer_editor_set_status(app->editor, "Save failed: scene allocation failed");
        return false;
    }
    size_t materialized = 0U;
    if (!viewer_editor_materialize(app->editor, scene, scene_length, &materialized) ||
        materialized != scene_length) {
        free(scene);
        viewer_editor_set_status(app->editor, "Save failed: could not serialize the edited scene");
        return false;
    }
    static const uint8_t default_manifest[] = "{\"format\":1,\"application\":\"Vexel\"}";
    static const uint8_t default_timeline[] =
        "{\"schema\":\"vexel.animation\",\"version\":1,"
        "\"ticks_per_second\":24,\"duration_ticks\":240,"
        "\"fps\":{\"numerator\":24,\"denominator\":1},"
        "\"loop\":\"once\","
        "\"onion\":{\"before\":0,\"after\":0,\"opacity\":0.25},"
        "\"tracks\":[]}";
    project_bundle_view *project = &app->document.project;
    project_bundle_asset *assets = NULL;
    size_t asset_count = project->asset_count;
    if (asset_count > 0U) {
        if (asset_count > PROJECT_BUNDLE_MAX_ASSET_COUNT ||
            asset_count > SIZE_MAX / sizeof(*assets)) {
            free(scene);
            viewer_editor_set_status(app->editor, "Save failed: invalid project asset count");
            return false;
        }
        assets = calloc(asset_count, sizeof(*assets));
        if (assets == NULL) {
            free(scene);
            viewer_editor_set_status(app->editor, "Save failed: asset allocation failed");
            return false;
        }
        for (size_t index = 0U; index < asset_count; ++index) {
            if (!project_bundle_asset_at(project, index, &assets[index])) {
                free(assets);
                free(scene);
                viewer_editor_set_status(app->editor, "Save failed: project asset data is invalid");
                return false;
            }
        }
    }
    base_bytes timeline_json = project->version == PROJECT_BUNDLE_VERSION ?
        project->timeline_json : (base_bytes){default_timeline, sizeof(default_timeline) - 1U};
    if (app->document.timeline_session != NULL)
        timeline_json = project_timeline_session_source(app->document.timeline_session);
    project_bundle_input input = {
        .manifest_json = project->version == PROJECT_BUNDLE_VERSION ?
            project->manifest_json : (base_bytes){default_manifest, sizeof(default_manifest) - 1U},
        .scene_svg = {scene, materialized},
        .timeline_json = timeline_json,
        .assets = assets,
        .asset_count = asset_count
    };
    project_bundle_result measured = project_bundle_encoded_size(&input);
    if (measured.error != PROJECT_BUNDLE_OK) {
        free(assets);
        free(scene);
        viewer_editor_set_status(app->editor,
            "Save failed: native project exceeds a format limit");
        return false;
    }
    uint8_t *encoded = malloc(measured.encoded_size);
    if (encoded == NULL) {
        free(assets);
        free(scene);
        viewer_editor_set_status(app->editor,
            "Save failed: native project allocation failed");
        return false;
    }
    project_bundle_result encoded_result = project_bundle_encode(&input,
        (base_mut_bytes){encoded, measured.encoded_size});
    if (encoded_result.error != PROJECT_BUNDLE_OK ||
        encoded_result.encoded_size != measured.encoded_size) {
        free(encoded);
        free(assets);
        free(scene);
        viewer_editor_set_status(app->editor,
            "Save failed: native project encoding failed");
        return false;
    }
    project_bundle_view candidate = {0};
    project_bundle_result decoded = project_bundle_decode(
        (base_bytes){encoded, measured.encoded_size}, &candidate);
    if (decoded.error != PROJECT_BUNDLE_OK) {
        free(encoded);
        free(assets);
        free(scene);
        viewer_editor_set_status(app->editor,
            "Save failed: encoded native project did not validate");
        return false;
    }
    project_timeline_session *candidate_session = NULL;
    project_timeline_context_owner candidate_context = {0};
    project_bundle_asset_overlay candidate_overlay = {0};
    bool install_native_session = !app->document.native_project &&
        app->document.timeline_session == NULL;
    project_timeline_integration_result timeline =
        project_timeline_validate_bundle_owned(&candidate, candidate.scene_svg,
            project_save_canceled, app, NULL, &candidate_context);
    if (timeline.canceled || project_save_canceled(app)) {
        project_timeline_context_owner_destroy(&candidate_context);
        free(encoded);
        free(assets);
        free(scene);
        viewer_editor_set_status(app->editor, "Save canceled");
        return false;
    }
    if (timeline.validation.error != PROJECT_TIMELINE_OK) {
        char message[VIEWER_STATUS_CAP];
        if (timeline.asset_name.len != 0U) {
            int precision = timeline.asset_name.len > 40U ? 40 :
                (int)timeline.asset_name.len;
            (void)snprintf(message, sizeof(message),
                "Save failed: drawing asset %.*s is invalid at byte %zu",
                precision, (const char *)timeline.asset_name.ptr,
                timeline.asset_validation.error_offset);
        } else {
            (void)snprintf(message, sizeof(message),
                "Save failed: native timeline %s at byte %zu",
                project_timeline_error_name(timeline.validation.error),
                timeline.validation.error_offset);
        }
        project_timeline_context_owner_destroy(&candidate_context);
        free(encoded);
        free(assets);
        free(scene);
        viewer_editor_set_status(app->editor, message);
        return false;
    }
    if (install_native_session) {
        if (timeline.compatibility != PROJECT_TIMELINE_COMPATIBILITY_NATIVE_V1 ||
            !project_bundle_asset_overlay_init(&candidate_overlay, &candidate)) {
            project_timeline_context_owner_destroy(&candidate_context);
            free(encoded);
            free(assets);
            free(scene);
            viewer_editor_set_status(app->editor,
                "Save failed: native animation session could not be initialized");
            return false;
        }
        project_timeline_edit_result created = project_timeline_session_create(
            candidate.timeline_json, &candidate_context,
            project_timeline_edit_default_limits(), &candidate_session);
        if (created.error != PROJECT_TIMELINE_OK) {
            project_bundle_asset_overlay_destroy(&candidate_overlay);
            project_timeline_context_owner_destroy(&candidate_context);
            free(encoded);
            free(assets);
            free(scene);
            viewer_editor_set_status(app->editor,
                "Save failed: native animation session could not be initialized");
            return false;
        }
    }
    project_file_result saved = project_file_save_encoded(
        (base_bytes){(const uint8_t *)path, path_length},
        (base_bytes){encoded, measured.encoded_size});
    project_timeline_context_owner_destroy(&candidate_context);
    free(assets);
    free(scene);
    if (!saved.published) {
        char message[VIEWER_STATUS_CAP];
        (void)snprintf(message, sizeof(message), "Save failed: native project %s%s",
            project_file_error_name(saved.error), saved.published ? " (published; durability check failed)" : "");
        viewer_editor_set_status(app->editor, message);
        project_timeline_session_destroy(candidate_session);
        project_bundle_asset_overlay_destroy(&candidate_overlay);
        free(encoded);
        return false;
    }
    if (install_native_session) {
        project_timeline_session_destroy(app->document.timeline_session);
        project_bundle_asset_overlay_destroy(&app->document.asset_overlay);
        free(app->document.project_owner);
        app->document.project = candidate;
        app->document.project_owner = encoded;
        encoded = NULL;
        app->document.timeline_session = candidate_session;
        candidate_session = NULL;
        app->document.asset_overlay = candidate_overlay;
        candidate_overlay = (project_bundle_asset_overlay){0};
        app->document.asset_overlay.bundle = &app->document.project;
        app->document.project.overlay_assets = app->document.asset_overlay.assets;
        app->document.native_project = true;
        (void)viewer_timeline_reset_native(&app->timeline,
            &app->document.shape, app->document.timeline_session,
            &app->document.project);
        (void)viewer_editor_set_timeline_state(app->editor, true,
            app->timeline.dock_open, app->timeline.dock_height_points);
    }
    free(encoded);
    app->document.native_project = true;
    document_edit_mark_saved(app->editor->edit);
    if (app->document.timeline_session != NULL)
        (void)project_timeline_session_mark_saved(app->document.timeline_session);
    project_bundle_asset_overlay_mark_saved(&app->document.asset_overlay);
    char target_path[PLATFORM_PATH_CAP];
    if (path_length < sizeof(target_path)) {
        memcpy(target_path, path, path_length);
        target_path[path_length] = '\0';
        (void)viewer_recovery_cleanup_after_save(app, previous_path, target_path);
    }
    if (saved.error != PROJECT_FILE_OK) {
        viewer_editor_set_status(app->editor,
            "Saved project; folder durability check needs attention");
    }
    else if (install_native_session)
        viewer_editor_set_status(app->editor,
            "Saved native project · timeline ready");
    app->frame_dirty = true;
    return true;
}

bool viewer_save_document(viewer_app *app, bool force_save_as)
{
    if (app == NULL || app->editor == NULL) return false;
    char previous_path[PLATFORM_PATH_CAP];
    size_t previous_length = strnlen(app->path, sizeof(app->path));
    if (previous_length < sizeof(previous_path))
        memcpy(previous_path, app->path, previous_length + 1U);
    else previous_path[0] = '\0';
    char chosen_path[PLATFORM_DIALOG_PATH_CAP] = {0};
    size_t chosen_length = 0U;
    const char *path = app->path;
    size_t path_length = viewer_event_path_length(path, PLATFORM_PATH_CAP - 1U);
    bool save_as = force_save_as || path_length == 0U || path_length >= PLATFORM_PATH_CAP;
    if (save_as) {
        size_t name_length = viewer_event_path_length(app->display_name, sizeof(app->display_name) - 1U);
        platform_dialog_status choice = platform_dialog_save_document(app->display_name,
            name_length, chosen_path, sizeof(chosen_path), &chosen_length);
        if (choice != PLATFORM_DIALOG_ACCEPT) {
            if (choice == PLATFORM_DIALOG_ERROR)
                show_save_error(app, "Save Failed", "The save panel could not provide a valid document path.");
            return false;
        }
        path = chosen_path;
        path_length = chosen_length;
        size_t current_length = viewer_event_path_length(app->path, PLATFORM_PATH_CAP - 1U);
        if (app->path[0] != '\0' && path_length == current_length &&
            memcmp(path, app->path, path_length) == 0 && viewer_file_changed_on_disk(app)) {
            show_save_error(app, "File Changed on Disk",
                "The selected file changed outside Vexel. Save As to a different path to keep both versions.");
            return false;
        }
    } else if (viewer_file_changed_on_disk(app)) {
        show_save_error(app, "File Changed on Disk",
            "The file changed outside Vexel. Use Save As to keep both versions.");
        return false;
    }
    bool project_format = path_has_extension(path, path_length, "vexel");
    if (!project_format && (project_timeline_session_is_dirty(
            app->document.timeline_session) ||
        project_bundle_asset_overlay_is_dirty(&app->document.asset_overlay))) {
        viewer_editor_set_status(app->editor,
            "Save native animation and drawing assets as a .vexel project");
        return false;
    }
    if (project_format)
        atomic_init(&app->project_save_cancel_requested, false);
    bool saved = project_format ? viewer_save_project_file(app, path, path_length) :
        viewer_editor_save(app->editor,
            (base_bytes){(const uint8_t *)path, path_length}).error == DOCUMENT_EDIT_OK;
    if (!saved) {
        const char *status = viewer_editor_status(app->editor);
        show_save_error(app, "Save Failed", status == NULL ?
            "The document could not be saved." : status);
        return false;
    }
    if (save_as && !viewer_update_active_path(app, path, path_length)) {
        show_save_error(app, "Save Failed",
            "The file was saved, but Vexel could not update its document path.");
        return false;
    }
    viewer_update_file_stamp(app);
    if (!project_format)
        (void)viewer_recovery_cleanup_after_save(app, previous_path, path);
    app->frame_dirty = true;
    (void)viewer_set_window_title(app);
    return true;
}

bool viewer_confirm_unsaved_changes(viewer_app *app, const char *title,
                                    const char *message)
{
    if (app == NULL || app->editor == NULL ||
        (!viewer_editor_is_dirty(app->editor) &&
         !project_timeline_session_is_dirty(app->document.timeline_session) &&
         !project_bundle_asset_overlay_is_dirty(&app->document.asset_overlay))) return true;
    platform_unsaved_choice choice = platform_dialog_unsaved_changes(title,
        strlen(title), message, strlen(message));
    if (choice == PLATFORM_UNSAVED_DONT_SAVE) return true;
    if (choice == PLATFORM_UNSAVED_SAVE) return viewer_save_document(app, false);
    if (app->editor != NULL) {
        viewer_editor_set_status(app->editor, choice == PLATFORM_UNSAVED_CANCEL ?
            "Action canceled" : "Could not show the unsaved-changes dialog");
    }
    app->frame_dirty = true;
    return false;
}
