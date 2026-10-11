#ifndef VEXEL_PLATFORM_SETTINGS_H
#define VEXEL_PLATFORM_SETTINGS_H

#include "platform_window.h"

#include <stdbool.h>

typedef struct platform_settings_values {
    bool show_welcome_on_launch;
    bool remember_workspace;
    bool structure_visible;
    bool inspector_visible;
    bool default_checkerboard;
    bool default_pixel_grid;
    bool modifier_scroll_zooms;
} platform_settings_values;

void platform_settings_get(platform_settings_values *out_values);
void platform_settings_show(platform_window *owner_window);
void platform_settings_store_sidebar_visibility(bool structure_visible,
                                               bool inspector_visible);

#endif
