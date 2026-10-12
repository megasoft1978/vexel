#include "viewer_editor_internal.h"

#include <string.h>

/* Bounded operator/warning/error log, following Blender's Info editor. The log
 * is UI state: fixed capacity, fixed text size, oldest entry dropped first. */

static size_t log_slot(const viewer_editor *editor, size_t index)
{
    return (editor->log_start + index) % EDITOR_LOG_CAP;
}

void viewer_editor_log(viewer_editor *editor, uint8_t severity,
                       const char *text, size_t length)
{
    if (editor == NULL || text == NULL || length == 0U) return;
    if (length > EDITOR_LOG_TEXT_CAP - 1U) length = EDITOR_LOG_TEXT_CAP - 1U;
    size_t slot;
    if (editor->log_count < EDITOR_LOG_CAP) {
        slot = log_slot(editor, editor->log_count);
        ++editor->log_count;
    } else {
        slot = editor->log_start;
        editor->log_start = (editor->log_start + 1U) % EDITOR_LOG_CAP;
    }
    editor_log_entry *entry = &editor->log[slot];
    memcpy(entry->text, text, length);
    entry->text[length] = '\0';
    entry->length = length;
    entry->severity = severity;
    entry->sequence = editor->log_next_sequence++;
    editor->tree_ready = false;
    editor->redraw_requested = true;
}

size_t viewer_editor_log_count(const viewer_editor *editor)
{
    return editor == NULL ? 0U : editor->log_count;
}

bool viewer_editor_log_at(const viewer_editor *editor, size_t index,
                          uint8_t *out_severity, char *out_text, size_t capacity,
                          size_t *out_length)
{
    if (editor == NULL || index >= editor->log_count || out_text == NULL ||
        capacity == 0U) return false;
    const editor_log_entry *entry = &editor->log[log_slot(editor, index)];
    size_t length = entry->length < capacity - 1U ? entry->length : capacity - 1U;
    memcpy(out_text, entry->text, length);
    out_text[length] = '\0';
    if (out_length != NULL) *out_length = length;
    if (out_severity != NULL) *out_severity = entry->severity;
    return true;
}

bool viewer_editor_log_open(const viewer_editor *editor)
{
    return editor != NULL && editor->log_open;
}

bool viewer_editor_set_log_open(viewer_editor *editor, bool open)
{
    if (editor == NULL) return false;
    if (editor->log_open == open) return false;
    editor->log_open = open;
    editor->tree_ready = false;
    editor->redraw_requested = true;
    return true;
}

bool viewer_editor_toggle_log(viewer_editor *editor)
{
    return viewer_editor_set_log_open(editor, !viewer_editor_log_open(editor));
}
