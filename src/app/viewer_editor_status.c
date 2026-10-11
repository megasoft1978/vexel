#include "viewer_editor_internal.h"

#include "platform/platform_localization.h"

#include <string.h>

static size_t bounded_status_length(const char *text, size_t capacity)
{
    size_t length = 0U;
    if (text == NULL) return 0U;
    while (length < capacity && text[length] != '\0') ++length;
    return length;
}

void editor_status_error(viewer_editor *editor, document_edit_error error)
{
    if (editor == NULL) return;
    editor->status_error = true;
    editor->status_saved = false;
    editor->status_ready = false;
    const char *name = document_edit_error_name(error);
    const char *fallback = "Unknown edit error";
    if (name != NULL) {
        if (strcmp(name, "invalid_argument") == 0) fallback = "Invalid argument";
        else if (strcmp(name, "stale_handle") == 0) fallback = "Stale layer";
        else if (strcmp(name, "not_found") == 0) fallback = "Layer not found";
        else if (strcmp(name, "duplicate_attribute") == 0) fallback = "Duplicate attribute";
        else if (strcmp(name, "invalid_name") == 0) fallback = "Invalid name";
        else if (strcmp(name, "malformed_xml") == 0) fallback = "Malformed XML";
        else if (strcmp(name, "root_delete") == 0) fallback = "Cannot delete the document root";
        else if (strcmp(name, "invalid_operation") == 0) fallback = "Invalid edit operation";
        else if (strcmp(name, "limit") == 0) fallback = "Edit limit reached";
        else if (strcmp(name, "history_limit") == 0) fallback = "Undo history limit reached";
        else if (strcmp(name, "out_of_memory") == 0) fallback = "Not enough memory to edit";
        else if (strcmp(name, "save_failed") == 0) fallback = "Could not save";
    }
    char localized_error[256] = {0};
    size_t error_length = 0U;
    if (!editor_localize_slice(fallback, strlen(fallback), localized_error,
            sizeof(localized_error), &error_length))
        editor_set_text(localized_error, sizeof(localized_error), fallback);
    if (!platform_localization_format_one("editor.status.edit_rejected",
            "Edit rejected: %@", "Document edit rejection status.",
            localized_error, editor->status, sizeof(editor->status), &error_length))
        editor_set_text(editor->status, sizeof(editor->status), "Edit rejected");
    editor->tree_ready = false;
    editor->redraw_requested = true;
}

static bool status_contains(const char *message, size_t message_length,
                            const char *marker)
{
    size_t marker_length = strlen(marker);
    if (marker_length == 0U || marker_length > message_length) return false;
    for (size_t index = 0U; index <= message_length - marker_length; ++index)
        if (memcmp(message + index, marker, marker_length) == 0) return true;
    return false;
}

static bool status_is_error(const char *message, size_t message_length)
{
    static const char *const markers[] = {
        "failed", "Failed", "Could not", "could not", "cannot", "Cannot",
        "exceeds", "exceeded", "limit", "Invalid", "invalid", "Unsupported",
        "unsupported", "unavailable", "not available", "stale", "outside",
        "changed on disk", "error", "Error"
    };
    for (size_t index = 0U; index < sizeof(markers) / sizeof(markers[0]); ++index)
        if (status_contains(message, message_length, markers[index])) return true;
    return false;
}

void editor_status_ok(viewer_editor *editor, const char *message)
{
    if (editor == NULL) return;
    if (message == NULL || message[0] == '\0') message = "Ready";
    char localized[PLATFORM_LOCALIZATION_TEXT_CAP];
    size_t localized_length = 0U;
    size_t message_length = bounded_status_length(message, sizeof(localized));
    editor->status_error = status_is_error(message, message_length);
    editor->status_saved = message_length == sizeof("Saved") - 1U &&
        memcmp(message, "Saved", sizeof("Saved") - 1U) == 0;
    editor->status_ready = message_length == sizeof("Ready") - 1U &&
        memcmp(message, "Ready", sizeof("Ready") - 1U) == 0;
    bool translated = message_length < sizeof(localized) &&
        platform_localization_copy_default(message, message_length, localized,
            sizeof(localized), &localized_length);
    static const struct { const char *prefix; const char *key; const char *fallback;
        const char *comment; } dynamic_statuses[] = {
        {"Save failed:", "editor.status.save_failed", "Save failed: %@",
            "Save failure status with technical detail."},
        {"Preview update failed:", "editor.status.preview_failed",
            "Preview update failed: %@", "Preview failure status with technical detail."},
        {"Preview unavailable:", "editor.status.preview_unavailable",
            "Preview unavailable: %@", "Document preview error with technical detail."},
        {"Agent edited — ", "editor.status.agent_edited", "Agent edited — %@",
            "Assistant edit status with document label."}
    };
    if (!translated && message_length < sizeof(localized)) {
        for (size_t index = 0U; index < sizeof(dynamic_statuses) /
                sizeof(dynamic_statuses[0]); ++index) {
            size_t prefix_length = strlen(dynamic_statuses[index].prefix);
            if (message_length < prefix_length ||
                memcmp(message, dynamic_statuses[index].prefix, prefix_length) != 0)
                continue;
            const char *detail = message + prefix_length;
            if (*detail == ' ') ++detail;
            translated = platform_localization_format_one(dynamic_statuses[index].key,
                dynamic_statuses[index].fallback, dynamic_statuses[index].comment,
                detail, localized, sizeof(localized), &localized_length);
            break;
        }
    }
    if (translated) {
        (void)localized_length;
        editor_set_text(editor->status, sizeof(editor->status), localized);
    } else {
        editor_set_text(editor->status, sizeof(editor->status), message);
    }
    editor->tree_ready = false;
    editor->redraw_requested = true;
}
