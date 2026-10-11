#include "platform_command_policy.h"

bool platform_command_is_enabled(platform_viewer_action action,
                                 const platform_command_state *state)
{
    if (state == NULL) return false;
    switch (action) {
        case PLATFORM_ACTION_RELOAD: return state->has_document && state->has_path;
        case PLATFORM_ACTION_SAVE:
            return state->has_document && (state->dirty || !state->has_path);
        case PLATFORM_ACTION_SAVE_AS:
        case PLATFORM_ACTION_EXPORT_PNG:
        case PLATFORM_ACTION_SELECT_ALL:
        case PLATFORM_ACTION_PASTE:
        case PLATFORM_ACTION_FIT_WIDTH:
        case PLATFORM_ACTION_FIT_WINDOW:
        case PLATFORM_ACTION_ACTUAL_SIZE:
        case PLATFORM_ACTION_TOGGLE_CHECKERBOARD:
        case PLATFORM_ACTION_CYCLE_BACKGROUND:
        case PLATFORM_ACTION_TOGGLE_PIXEL_GRID:
        case PLATFORM_ACTION_ZOOM_IN:
        case PLATFORM_ACTION_ZOOM_OUT:
        case PLATFORM_ACTION_TOGGLE_STRUCTURE:
        case PLATFORM_ACTION_TOGGLE_INSPECTOR:
            return state->has_document;
        case PLATFORM_ACTION_UNDO: return state->has_document && state->can_undo;
        case PLATFORM_ACTION_REDO: return state->has_document && state->can_redo;
        case PLATFORM_ACTION_CUT:
        case PLATFORM_ACTION_COPY:
        case PLATFORM_ACTION_DELETE:
            return state->has_document && state->has_selection;
        case PLATFORM_ACTION_NEW_DOCUMENT:
        case PLATFORM_ACTION_CLOSE_DOCUMENT:
        case PLATFORM_ACTION_QUIT:
        case PLATFORM_ACTION_NONE:
            return true;
        default: return false;
    }
}
