#include "viewer_commands.h"

#include "viewer_editor_internal.h"

#include <ctype.h>
#include <stdio.h>
#include <string.h>

#define CMD(id, group, title, shortcut, key, modifiers, toolbar, palette) \
    {id, group, title, shortcut, key, modifiers, toolbar, palette}

static const viewer_command_descriptor commands[] = {
    CMD(VIEWER_COMMAND_TOOL_SELECT, VIEWER_COMMAND_GROUP_TOOLS, "Select tool", "V", NULL, 0U, EDITOR_BUTTON_TOOL_SELECT, true),
    CMD(VIEWER_COMMAND_TOOL_PAN, VIEWER_COMMAND_GROUP_TOOLS, "Pan tool", "H / Space", NULL, 0U, EDITOR_BUTTON_TOOL_PAN, true),
    CMD(VIEWER_COMMAND_TOOL_ZOOM, VIEWER_COMMAND_GROUP_TOOLS, "Zoom tool", "Z", NULL, 0U, EDITOR_BUTTON_TOOL_ZOOM, true),
    CMD(VIEWER_COMMAND_TOOL_RECTANGLE, VIEWER_COMMAND_GROUP_TOOLS, "Rectangle", "R", NULL, 0U, EDITOR_BUTTON_TOOL_RECTANGLE, true),
    CMD(VIEWER_COMMAND_TOOL_ELLIPSE, VIEWER_COMMAND_GROUP_TOOLS, "Ellipse", "O", NULL, 0U, EDITOR_BUTTON_TOOL_ELLIPSE, true),
    CMD(VIEWER_COMMAND_TOOL_LINE, VIEWER_COMMAND_GROUP_TOOLS, "Line", "L", NULL, 0U, EDITOR_BUTTON_TOOL_LINE, true),
    CMD(VIEWER_COMMAND_TOOL_POLYGON, VIEWER_COMMAND_GROUP_TOOLS, "Polygon", "Y", NULL, 0U, EDITOR_BUTTON_TOOL_POLYGON, true),
    CMD(VIEWER_COMMAND_TOOL_POLYLINE, VIEWER_COMMAND_GROUP_TOOLS, "Polyline", "U", NULL, 0U, EDITOR_BUTTON_TOOL_POLYLINE, true),
    CMD(VIEWER_COMMAND_TOOL_PEN, VIEWER_COMMAND_GROUP_TOOLS, "Pen", "P", NULL, 0U, EDITOR_BUTTON_TOOL_PEN, true),
    CMD(VIEWER_COMMAND_TOOL_NODE, VIEWER_COMMAND_GROUP_TOOLS, "Node edit", "N", NULL, 0U, EDITOR_BUTTON_TOOL_NODE, true),
    CMD(VIEWER_COMMAND_TOOL_TEXT, VIEWER_COMMAND_GROUP_TOOLS, "Text", "T", NULL, 0U, EDITOR_BUTTON_TOOL_TEXT, true),

    CMD(VIEWER_COMMAND_UNDO, VIEWER_COMMAND_GROUP_SELECTION, "Undo", "⌘Z", NULL, 0U, EDITOR_BUTTON_UNDO, true),
    CMD(VIEWER_COMMAND_REDO, VIEWER_COMMAND_GROUP_SELECTION, "Redo", "⇧⌘Z", NULL, 0U, EDITOR_BUTTON_REDO, true),
    CMD(VIEWER_COMMAND_UNDO_HISTORY, VIEWER_COMMAND_GROUP_SELECTION, "Undo History", "", NULL, 0U, EDITOR_BUTTON_HISTORY, true),
    CMD(VIEWER_COMMAND_SELECT_ALL, VIEWER_COMMAND_GROUP_SELECTION, "Select All", "⌘A", NULL, 0U, 0U, true),
    CMD(VIEWER_COMMAND_COPY, VIEWER_COMMAND_GROUP_SELECTION, "Copy", "⌘C", NULL, 0U, 0U, true),
    CMD(VIEWER_COMMAND_CUT, VIEWER_COMMAND_GROUP_SELECTION, "Cut", "⌘X", NULL, 0U, 0U, true),
    CMD(VIEWER_COMMAND_PASTE, VIEWER_COMMAND_GROUP_SELECTION, "Paste", "⌘V", NULL, 0U, 0U, true),
    CMD(VIEWER_COMMAND_PASTE_IN_PLACE, VIEWER_COMMAND_GROUP_SELECTION, "Paste in Place", "⇧⌘V", NULL, 0U, 0U, true),
    CMD(VIEWER_COMMAND_DUPLICATE, VIEWER_COMMAND_GROUP_SELECTION, "Duplicate", "⌘D", NULL, 0U, EDITOR_BUTTON_DUPLICATE, true),
    CMD(VIEWER_COMMAND_DELETE, VIEWER_COMMAND_GROUP_SELECTION, "Delete", "Delete", NULL, 0U, EDITOR_BUTTON_DELETE, true),
    CMD(VIEWER_COMMAND_GROUP, VIEWER_COMMAND_GROUP_SELECTION, "Group", "⌘G", NULL, 0U, EDITOR_BUTTON_GROUP, true),
    CMD(VIEWER_COMMAND_UNGROUP, VIEWER_COMMAND_GROUP_SELECTION, "Ungroup", "⇧⌘G", NULL, 0U, EDITOR_BUTTON_UNGROUP, true),
    CMD(VIEWER_COMMAND_BRING_FORWARD, VIEWER_COMMAND_GROUP_SELECTION, "Bring Forward", "⌘]", NULL, 0U, 0U, true),
    CMD(VIEWER_COMMAND_SEND_BACKWARD, VIEWER_COMMAND_GROUP_SELECTION, "Send Backward", "⌘[", NULL, 0U, 0U, true),
    CMD(VIEWER_COMMAND_BRING_TO_FRONT, VIEWER_COMMAND_GROUP_SELECTION, "Bring to Front", "⇧⌘]", NULL, 0U, 0U, true),
    CMD(VIEWER_COMMAND_SEND_TO_BACK, VIEWER_COMMAND_GROUP_SELECTION, "Send to Back", "⇧⌘[", NULL, 0U, 0U, true),
    CMD(VIEWER_COMMAND_ALIGN_LEFT, VIEWER_COMMAND_GROUP_SELECTION, "Align Left", "⌥⌘←", NULL, 0U, 0U, true),
    CMD(VIEWER_COMMAND_ALIGN_CENTER, VIEWER_COMMAND_GROUP_SELECTION, "Align Center", "⌥⌘H", NULL, 0U, 0U, true),
    CMD(VIEWER_COMMAND_ALIGN_RIGHT, VIEWER_COMMAND_GROUP_SELECTION, "Align Right", "⌥⌘→", NULL, 0U, 0U, true),
    CMD(VIEWER_COMMAND_ALIGN_TOP, VIEWER_COMMAND_GROUP_SELECTION, "Align Top", "⌥⌘↑", NULL, 0U, 0U, true),
    CMD(VIEWER_COMMAND_ALIGN_MIDDLE, VIEWER_COMMAND_GROUP_SELECTION, "Align Middle", "⌥⌘V", NULL, 0U, 0U, true),
    CMD(VIEWER_COMMAND_ALIGN_BOTTOM, VIEWER_COMMAND_GROUP_SELECTION, "Align Bottom", "⌥⌘↓", NULL, 0U, 0U, true),
    CMD(VIEWER_COMMAND_DISTRIBUTE_HORIZONTAL, VIEWER_COMMAND_GROUP_SELECTION, "Distribute Horizontally", "", NULL, 0U, 0U, true),
    CMD(VIEWER_COMMAND_DISTRIBUTE_VERTICAL, VIEWER_COMMAND_GROUP_SELECTION, "Distribute Vertically", "", NULL, 0U, 0U, true),

    CMD(VIEWER_COMMAND_FIT_WIDTH, VIEWER_COMMAND_GROUP_CANVAS, "Fit Width", "W", NULL, 0U, 0U, true),
    CMD(VIEWER_COMMAND_FIT_WINDOW, VIEWER_COMMAND_GROUP_CANVAS, "Fit in Window", "⌘0", "0", VIEWER_COMMAND_MENU_COMMAND, 0U, true),
    CMD(VIEWER_COMMAND_ACTUAL_SIZE, VIEWER_COMMAND_GROUP_CANVAS, "Actual Size", "⌘1", "1", VIEWER_COMMAND_MENU_COMMAND, 0U, true),
    CMD(VIEWER_COMMAND_ZOOM_IN, VIEWER_COMMAND_GROUP_CANVAS, "Zoom In", "⌘+", NULL, 0U, 0U, true),
    CMD(VIEWER_COMMAND_ZOOM_OUT, VIEWER_COMMAND_GROUP_CANVAS, "Zoom Out", "⌘−", NULL, 0U, 0U, true),
    CMD(VIEWER_COMMAND_TOGGLE_SNAPPING, VIEWER_COMMAND_GROUP_CANVAS, "Toggle Snapping", "⌘'", NULL, 0U, 0U, true),
    CMD(VIEWER_COMMAND_TOGGLE_CHECKERBOARD, VIEWER_COMMAND_GROUP_CANVAS, "Checkerboard", "⌘T", "t", VIEWER_COMMAND_MENU_COMMAND, 0U, true),
    CMD(VIEWER_COMMAND_CYCLE_BACKGROUND, VIEWER_COMMAND_GROUP_CANVAS, "Cycle Background", "B / ⌘B", "b", VIEWER_COMMAND_MENU_COMMAND, 0U, true),
    CMD(VIEWER_COMMAND_TOGGLE_PIXEL_GRID, VIEWER_COMMAND_GROUP_CANVAS, "Pixel Grid", "G", "g", 0U, 0U, true),

    CMD(VIEWER_COMMAND_OPEN, VIEWER_COMMAND_GROUP_DOCUMENT, "Open SVG…", "⌘O", "o", VIEWER_COMMAND_MENU_COMMAND, 0U, true),
    CMD(VIEWER_COMMAND_SAVE, VIEWER_COMMAND_GROUP_DOCUMENT, "Save", "⌘S", NULL, 0U, EDITOR_BUTTON_SAVE, true),
    CMD(VIEWER_COMMAND_SAVE_AS, VIEWER_COMMAND_GROUP_DOCUMENT, "Save As…", "⇧⌘S", NULL, 0U, EDITOR_BUTTON_SAVE_AS, true),
    CMD(VIEWER_COMMAND_RELOAD, VIEWER_COMMAND_GROUP_DOCUMENT, "Reload", "⌘R", "r", VIEWER_COMMAND_MENU_COMMAND, 0U, true),
    CMD(VIEWER_COMMAND_EXPORT_PNG, VIEWER_COMMAND_GROUP_DOCUMENT, "Export PNG…", "⇧⌘E", NULL, 0U, 0U, true),
    CMD(VIEWER_COMMAND_COPY_PNG, VIEWER_COMMAND_GROUP_DOCUMENT, "Copy PNG", "⌥⌘C", NULL, 0U, 0U, true),
    CMD(VIEWER_COMMAND_QUIT, VIEWER_COMMAND_GROUP_DOCUMENT, "Quit Vexel", "⌘Q", "q", VIEWER_COMMAND_MENU_COMMAND, 0U, true),
    CMD(VIEWER_COMMAND_SEARCH, VIEWER_COMMAND_GROUP_DOCUMENT, "Search Commands…", "⌘K", "k", VIEWER_COMMAND_MENU_COMMAND, 0U, false)
};

#undef CMD

size_t viewer_command_count(void)
{
    return sizeof(commands) / sizeof(commands[0]);
}

const viewer_command_descriptor *viewer_command_at(size_t index)
{
    return index < viewer_command_count() ? &commands[index] : NULL;
}

const viewer_command_descriptor *viewer_command_find(viewer_command_id id)
{
    for (size_t index = 0U; index < viewer_command_count(); ++index)
        if (commands[index].id == id) return &commands[index];
    return NULL;
}

const viewer_command_descriptor *viewer_command_for_toolbar(uint32_t button_id)
{
    if (button_id == 0U) return NULL;
    for (size_t index = 0U; index < viewer_command_count(); ++index)
        if (commands[index].toolbar_button_id == button_id) return &commands[index];
    return NULL;
}

const char *viewer_command_group_name(viewer_command_group group)
{
    switch (group) {
        case VIEWER_COMMAND_GROUP_TOOLS: return "Tools";
        case VIEWER_COMMAND_GROUP_SELECTION: return "Selection";
        case VIEWER_COMMAND_GROUP_CANVAS: return "Canvas";
        case VIEWER_COMMAND_GROUP_DOCUMENT: return "Document";
    }
    return "Commands";
}

static bool ascii_contains(const char *text, const char *needle,
                           size_t start, size_t length)
{
    size_t text_length = strlen(text);
    if (length == 0U) return true;
    if (start > text_length || length > text_length - start) return false;
    for (size_t offset = 0U; offset <= text_length - length; ++offset) {
        size_t index = 0U;
        while (index < length &&
            tolower((unsigned char)text[offset + index]) ==
            tolower((unsigned char)needle[start + index])) ++index;
        if (index == length) return true;
    }
    return false;
}

static bool command_matches(const viewer_command_descriptor *command,
                            const char *query, size_t length)
{
    size_t cursor = 0U;
    const char *group = viewer_command_group_name(command->group);
    while (cursor < length) {
        while (cursor < length && isspace((unsigned char)query[cursor])) ++cursor;
        if (cursor == length) break;
        size_t start = cursor;
        while (cursor < length && !isspace((unsigned char)query[cursor])) ++cursor;
        size_t token_length = cursor - start;
        if (!ascii_contains(command->title, query, start, token_length) &&
            !ascii_contains(group, query, start, token_length) &&
            !ascii_contains(command->shortcut, query, start, token_length)) return false;
    }
    return true;
}

size_t viewer_command_search(const char *query, size_t query_length,
                             viewer_command_id *output, size_t capacity)
{
    if ((query_length > 0U && query == NULL) || query_length >= 64U ||
        (capacity > 0U && output == NULL)) return 0U;
    size_t found = 0U;
    for (size_t index = 0U; index < viewer_command_count(); ++index) {
        const viewer_command_descriptor *command = &commands[index];
        if (!command->in_palette || !command_matches(command, query, query_length)) continue;
        if (found < capacity) output[found] = command->id;
        ++found;
    }
    return found;
}

bool viewer_command_format_palette_row(const viewer_command_descriptor *command,
    const char *disabled_reason, char *output, size_t capacity)
{
    if (command == NULL || output == NULL || capacity == 0U) return false;
    const char *shortcut = command->shortcut;
    char formatted[64] = {0};
    int amount;
    if (disabled_reason != NULL && disabled_reason[0] != '\0') {
        if (shortcut[0] == '\0') {
            amount = snprintf(formatted, sizeof(formatted), "%s · %s — %s",
                viewer_command_group_name(command->group), command->title,
                disabled_reason);
        } else {
            amount = snprintf(formatted, sizeof(formatted), "%s · %s — %s · %s",
                viewer_command_group_name(command->group), command->title,
                disabled_reason, shortcut);
        }
    } else if (shortcut[0] == '\0') {
        amount = snprintf(formatted, sizeof(formatted), "%s · %s",
            viewer_command_group_name(command->group), command->title);
    } else {
        amount = snprintf(formatted, sizeof(formatted), "%s · %s  %s",
            viewer_command_group_name(command->group), command->title, shortcut);
    }
    if (amount < 0 || (size_t)amount >= sizeof(formatted) ||
        (size_t)amount >= capacity) return false;
    memcpy(output, formatted, (size_t)amount + 1U);
    return true;
}
