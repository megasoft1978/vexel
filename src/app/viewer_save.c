#include "viewer_internal.h"

#include "platform/macos/platform_dialog.h"

#include <stdio.h>
#include <string.h>

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

bool viewer_save_document(viewer_app *app, bool force_save_as)
{
    if (app == NULL || app->editor == NULL) return false;
    char chosen_path[PLATFORM_DIALOG_PATH_CAP] = {0};
    size_t chosen_length = 0U;
    const char *path = app->path;
    size_t path_length = viewer_event_path_length(path, PLATFORM_PATH_CAP - 1U);
    bool save_as = force_save_as || path_length == 0U || path_length >= PLATFORM_PATH_CAP;
    if (save_as) {
        size_t name_length = viewer_event_path_length(app->display_name, sizeof(app->display_name) - 1U);
        platform_dialog_status choice = platform_dialog_save_svg(app->display_name,
            name_length, chosen_path, sizeof(chosen_path), &chosen_length);
        if (choice != PLATFORM_DIALOG_ACCEPT) {
            if (choice == PLATFORM_DIALOG_ERROR)
                show_save_error(app, "Save Failed", "The save panel could not provide a valid SVG path.");
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
    document_edit_save_result saved = viewer_editor_save(app->editor,
        (base_bytes){(const uint8_t *)path, path_length});
    if (saved.error != DOCUMENT_EDIT_OK) {
        const char *status = viewer_editor_status(app->editor);
        show_save_error(app, "Save Failed",
            status == NULL ? "The SVG could not be saved." : status);
        return false;
    }
    if (save_as && !viewer_update_active_path(app, path, path_length)) {
        show_save_error(app, "Save Failed",
            "The file was saved, but Vexel could not update its document path.");
        return false;
    }
    viewer_update_file_stamp(app);
    app->frame_dirty = true;
    (void)viewer_set_window_title(app);
    return true;
}

bool viewer_confirm_unsaved_changes(viewer_app *app, const char *title,
                                    const char *message)
{
    if (app == NULL || !viewer_editor_is_dirty(app->editor)) return true;
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
