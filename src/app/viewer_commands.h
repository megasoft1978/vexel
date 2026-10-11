#ifndef VEXEL_APP_VIEWER_COMMANDS_H
#define VEXEL_APP_VIEWER_COMMANDS_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef enum viewer_command_group {
    VIEWER_COMMAND_GROUP_TOOLS,
    VIEWER_COMMAND_GROUP_SELECTION,
    VIEWER_COMMAND_GROUP_CANVAS,
    VIEWER_COMMAND_GROUP_DOCUMENT
} viewer_command_group;

typedef enum viewer_command_id {
    VIEWER_COMMAND_NONE = 0,
    VIEWER_COMMAND_TOOL_SELECT,
    VIEWER_COMMAND_TOOL_PAN,
    VIEWER_COMMAND_TOOL_ZOOM,
    VIEWER_COMMAND_TOOL_RECTANGLE,
    VIEWER_COMMAND_TOOL_ELLIPSE,
    VIEWER_COMMAND_TOOL_LINE,
    VIEWER_COMMAND_TOOL_POLYGON,
    VIEWER_COMMAND_TOOL_POLYLINE,
    VIEWER_COMMAND_TOOL_PEN,
    VIEWER_COMMAND_TOOL_NODE,
    VIEWER_COMMAND_TOOL_TEXT,
    VIEWER_COMMAND_UNDO,
    VIEWER_COMMAND_REDO,
    VIEWER_COMMAND_UNDO_HISTORY,
    VIEWER_COMMAND_SELECT_ALL,
    VIEWER_COMMAND_COPY,
    VIEWER_COMMAND_CUT,
    VIEWER_COMMAND_PASTE,
    VIEWER_COMMAND_PASTE_IN_PLACE,
    VIEWER_COMMAND_DUPLICATE,
    VIEWER_COMMAND_DELETE,
    VIEWER_COMMAND_GROUP,
    VIEWER_COMMAND_UNGROUP,
    VIEWER_COMMAND_BRING_FORWARD,
    VIEWER_COMMAND_SEND_BACKWARD,
    VIEWER_COMMAND_BRING_TO_FRONT,
    VIEWER_COMMAND_SEND_TO_BACK,
    VIEWER_COMMAND_ALIGN_LEFT,
    VIEWER_COMMAND_ALIGN_CENTER,
    VIEWER_COMMAND_ALIGN_RIGHT,
    VIEWER_COMMAND_ALIGN_TOP,
    VIEWER_COMMAND_ALIGN_MIDDLE,
    VIEWER_COMMAND_ALIGN_BOTTOM,
    VIEWER_COMMAND_DISTRIBUTE_HORIZONTAL,
    VIEWER_COMMAND_DISTRIBUTE_VERTICAL,
    VIEWER_COMMAND_FIT_WIDTH,
    VIEWER_COMMAND_FIT_WINDOW,
    VIEWER_COMMAND_ACTUAL_SIZE,
    VIEWER_COMMAND_ZOOM_IN,
    VIEWER_COMMAND_ZOOM_OUT,
    VIEWER_COMMAND_TOGGLE_SNAPPING,
    VIEWER_COMMAND_TOGGLE_CHECKERBOARD,
    VIEWER_COMMAND_CYCLE_BACKGROUND,
    VIEWER_COMMAND_TOGGLE_PIXEL_GRID,
    VIEWER_COMMAND_OPEN,
    VIEWER_COMMAND_SAVE,
    VIEWER_COMMAND_SAVE_AS,
    VIEWER_COMMAND_RELOAD,
    VIEWER_COMMAND_EXPORT_PNG,
    VIEWER_COMMAND_COPY_PNG,
    VIEWER_COMMAND_QUIT,
    VIEWER_COMMAND_SEARCH,
    VIEWER_COMMAND_COUNT
} viewer_command_id;

enum {
    VIEWER_COMMAND_MENU_COMMAND = 1U << 0U,
    VIEWER_COMMAND_MENU_SHIFT = 1U << 1U,
    VIEWER_COMMAND_MENU_OPTION = 1U << 2U
};

typedef struct viewer_command_descriptor {
    viewer_command_id id;
    viewer_command_group group;
    const char *title;
    /* Displayed by shortcut discovery and tooltips. */
    const char *shortcut;
    /* Native key equivalent; NULL leaves the existing app key path in control. */
    const char *menu_key;
    uint8_t menu_modifiers;
    uint32_t toolbar_button_id;
    bool in_palette;
} viewer_command_descriptor;

size_t viewer_command_count(void);
const viewer_command_descriptor *viewer_command_at(size_t index);
const viewer_command_descriptor *viewer_command_find(viewer_command_id id);
const viewer_command_descriptor *viewer_command_for_toolbar(uint32_t button_id);
const char *viewer_command_group_name(viewer_command_group group);
/* Results preserve the descriptor table's stable group and command order. */
size_t viewer_command_search(const char *query, size_t query_length,
                             viewer_command_id *output, size_t capacity);
bool viewer_command_format_palette_row(const viewer_command_descriptor *command,
    const char *disabled_reason, char *output, size_t capacity);

#endif
