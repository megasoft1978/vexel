#include "document_edit_internal.h"

#include <string.h>

base_bytes document_edit_history_action_label(document_edit_history_action action)
{
#define LABEL(value) ((base_bytes){(const uint8_t *)(value), sizeof(value) - 1U})
    switch (action) {
        case DOCUMENT_EDIT_HISTORY_ACTION_EDIT_ATTRIBUTE: return LABEL("Edit attribute");
        case DOCUMENT_EDIT_HISTORY_ACTION_ADD_ATTRIBUTE: return LABEL("Add attribute");
        case DOCUMENT_EDIT_HISTORY_ACTION_REMOVE_ATTRIBUTE: return LABEL("Remove attribute");
        case DOCUMENT_EDIT_HISTORY_ACTION_EDIT_TEXT: return LABEL("Edit text");
        case DOCUMENT_EDIT_HISTORY_ACTION_DELETE: return LABEL("Delete");
        case DOCUMENT_EDIT_HISTORY_ACTION_DUPLICATE: return LABEL("Duplicate");
        case DOCUMENT_EDIT_HISTORY_ACTION_MOVE: return LABEL("Move");
        case DOCUMENT_EDIT_HISTORY_ACTION_GROUP: return LABEL("Group");
        case DOCUMENT_EDIT_HISTORY_ACTION_UNGROUP: return LABEL("Ungroup");
        case DOCUMENT_EDIT_HISTORY_ACTION_ADD_RECTANGLE: return LABEL("Add rectangle");
        case DOCUMENT_EDIT_HISTORY_ACTION_ADD_CIRCLE: return LABEL("Add circle");
        case DOCUMENT_EDIT_HISTORY_ACTION_ADD_ELLIPSE: return LABEL("Add ellipse");
        case DOCUMENT_EDIT_HISTORY_ACTION_ADD_LINE: return LABEL("Add line");
        case DOCUMENT_EDIT_HISTORY_ACTION_ADD_PATH: return LABEL("Add path");
        case DOCUMENT_EDIT_HISTORY_ACTION_ADD_GROUP: return LABEL("Add group");
        case DOCUMENT_EDIT_HISTORY_ACTION_ADD_TEXT: return LABEL("Add text");
        case DOCUMENT_EDIT_HISTORY_ACTION_ADD_SHAPE: return LABEL("Add shape");
        case DOCUMENT_EDIT_HISTORY_ACTION_EDIT_OBJECTS: return LABEL("Edit objects");
        case DOCUMENT_EDIT_HISTORY_ACTION_EDIT_PATH: return LABEL("Edit path");
        case DOCUMENT_EDIT_HISTORY_ACTION_RESIZE: return LABEL("Resize");
        case DOCUMENT_EDIT_HISTORY_ACTION_ROTATE: return LABEL("Rotate");
    }
#undef LABEL
    return (base_bytes){0};
}

document_edit_history_action document_edit_history_template_action(
    document_edit_template_kind kind)
{
    switch (kind) {
        case DOCUMENT_EDIT_TEMPLATE_RECT: return DOCUMENT_EDIT_HISTORY_ACTION_ADD_RECTANGLE;
        case DOCUMENT_EDIT_TEMPLATE_CIRCLE: return DOCUMENT_EDIT_HISTORY_ACTION_ADD_CIRCLE;
        case DOCUMENT_EDIT_TEMPLATE_ELLIPSE: return DOCUMENT_EDIT_HISTORY_ACTION_ADD_ELLIPSE;
        case DOCUMENT_EDIT_TEMPLATE_LINE: return DOCUMENT_EDIT_HISTORY_ACTION_ADD_LINE;
        case DOCUMENT_EDIT_TEMPLATE_PATH: return DOCUMENT_EDIT_HISTORY_ACTION_ADD_PATH;
        case DOCUMENT_EDIT_TEMPLATE_GROUP: return DOCUMENT_EDIT_HISTORY_ACTION_ADD_GROUP;
        case DOCUMENT_EDIT_TEMPLATE_TEXT: return DOCUMENT_EDIT_HISTORY_ACTION_ADD_TEXT;
    }
    return DOCUMENT_EDIT_HISTORY_ACTION_ADD_SHAPE;
}

void document_edit_history_relabel_latest(document_edit *edit,
    document_edit_history_action action)
{
    if (edit == NULL || edit->command_count == 0U ||
        edit->command_cursor != edit->command_count) return;
    base_bytes label = document_edit_history_action_label(action);
    if (!base_bytes_valid(label) || label.len == 0U ||
        label.len >= DOCUMENT_EDIT_HISTORY_LABEL_CAP) return;
    document_edit_command *command = &edit->commands[edit->command_count - 1U];
    memcpy(command->label, label.ptr, label.len);
    command->label[label.len] = '\0';
}

bool document_edit_history_entry_at(const document_edit *edit, size_t index,
                                    document_edit_history_entry *out_entry)
{
    if (edit == NULL || out_entry == NULL || index >= edit->command_count) return false;
    const document_edit_command *command = &edit->commands[index];
    size_t length = 0U;
    while (length < sizeof(command->label) && command->label[length] != '\0') ++length;
    if (length == 0U || length >= sizeof(command->label)) return false;
    memset(out_entry, 0, sizeof(*out_entry));
    memcpy(out_entry->label, command->label, length);
    out_entry->state_cursor = index + 1U;
    out_entry->is_current = edit->command_cursor == index + 1U;
    out_entry->is_saved = edit->saved_state_id == command->after_state;
    return true;
}

bool document_edit_history_saved_cursor(const document_edit *edit,
                                        size_t *out_cursor)
{
    if (edit == NULL || out_cursor == NULL) return false;
    if (edit->command_count == 0U) {
        if (edit->saved_state_id != edit->state_id) return false;
        *out_cursor = 0U;
        return true;
    }
    if (edit->saved_state_id == edit->commands[0].before_state) {
        *out_cursor = 0U;
        return true;
    }
    for (size_t index = 0U; index < edit->command_count; ++index) {
        if (edit->saved_state_id == edit->commands[index].after_state) {
            *out_cursor = index + 1U;
            return true;
        }
    }
    return false;
}

document_edit_error document_edit_history_jump(document_edit *edit,
                                                size_t state_cursor)
{
    if (edit == NULL || state_cursor > edit->command_count)
        return DOCUMENT_EDIT_INVALID_ARGUMENT;
    while (edit->command_cursor > state_cursor) {
        document_edit_error error = document_edit_undo(edit);
        if (error != DOCUMENT_EDIT_OK) return error;
    }
    while (edit->command_cursor < state_cursor) {
        document_edit_error error = document_edit_redo(edit);
        if (error != DOCUMENT_EDIT_OK) return error;
    }
    return DOCUMENT_EDIT_OK;
}
