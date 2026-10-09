#ifndef VEXEL_UI_DEMO_H
#define VEXEL_UI_DEMO_H

#include "ui_core.h"

bool ui_demo_tree_build(ui_tree *tree, ui_theme theme, uint32_t width, uint32_t height);
bool ui_demo_commands_init(ui_command_registry *registry);
bool ui_demo_render(ui_theme theme, uint32_t width, uint32_t height, raster_tile *tile);

enum {
    UI_DEMO_COMMAND_NEW = 1U,
    UI_DEMO_COMMAND_OPEN,
    UI_DEMO_COMMAND_SAVE,
    UI_DEMO_COMMAND_UNDO,
    UI_DEMO_COMMAND_REDO,
    UI_DEMO_COMMAND_SELECT_TOOL,
    UI_DEMO_COMMAND_NODE_TOOL,
    UI_DEMO_COMMAND_RECT_TOOL,
    UI_DEMO_COMMAND_TEXT_TOOL,
    UI_DEMO_COMMAND_ZOOM_OUT,
    UI_DEMO_COMMAND_FIT,
    UI_DEMO_COMMAND_ZOOM_IN,
    UI_DEMO_COMMAND_SET_OPACITY,
    UI_DEMO_COMMAND_SELECT_LAYER_1,
    UI_DEMO_COMMAND_SELECT_LAYER_2,
    UI_DEMO_COMMAND_SELECT_LAYER_3
};

#endif
