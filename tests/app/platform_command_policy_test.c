#include "app/platform_command_policy.h"

#include <stdio.h>

static int expect(bool condition, const char *message)
{
    if (condition) return 0;
    (void)fprintf(stderr, "platform command policy: %s\n", message);
    return 1;
}

int main(void)
{
    platform_command_state state = {0};
    int failures = 0;
    failures += expect(platform_command_is_enabled(PLATFORM_ACTION_NEW_DOCUMENT, &state),
        "New remains available without a document");
    failures += expect(!platform_command_is_enabled(PLATFORM_ACTION_SAVE, &state),
        "Save is disabled without a document");
    failures += expect(!platform_command_is_enabled(PLATFORM_ACTION_RELOAD, &state),
        "Reload is disabled without a document path");
    state.has_document = true;
    failures += expect(platform_command_is_enabled(PLATFORM_ACTION_SAVE, &state),
        "Save opens Save As for a new untitled document");
    state.has_path = true;
    failures += expect(!platform_command_is_enabled(PLATFORM_ACTION_SAVE, &state),
        "Save is disabled for a clean document");
    failures += expect(!platform_command_is_enabled(PLATFORM_ACTION_UNDO, &state),
        "Undo is disabled without history");
    failures += expect(!platform_command_is_enabled(PLATFORM_ACTION_COPY, &state),
        "Copy is disabled without a selection");
    failures += expect(platform_command_is_enabled(PLATFORM_ACTION_TOGGLE_INSPECTOR, &state),
        "Inspector visibility is enabled with a document");
    state.dirty = true;
    state.can_undo = true;
    state.can_redo = true;
    state.has_selection = true;
    failures += expect(platform_command_is_enabled(PLATFORM_ACTION_SAVE, &state),
        "Save is enabled for a dirty document");
    failures += expect(platform_command_is_enabled(PLATFORM_ACTION_UNDO, &state) &&
        platform_command_is_enabled(PLATFORM_ACTION_REDO, &state),
        "Undo and Redo follow editor history");
    failures += expect(platform_command_is_enabled(PLATFORM_ACTION_COPY, &state) &&
        platform_command_is_enabled(PLATFORM_ACTION_DELETE, &state),
        "selection commands follow the editor selection");
    failures += expect(!platform_command_is_enabled(PLATFORM_ACTION_REFRESH_PREFERENCES, &state),
        "internal preference refresh is not a menu command");
    failures += expect(!platform_command_is_enabled(PLATFORM_ACTION_RELOAD, NULL),
        "null state disables document commands");
    return failures == 0 ? 0 : 1;
}
