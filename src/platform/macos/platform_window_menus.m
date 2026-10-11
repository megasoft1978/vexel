#import <Cocoa/Cocoa.h>

#include "platform_window_internal.h"

bool platform_window_set_option_state(platform_window *window, platform_viewer_action action,
                                      bool enabled)
{
    if (![NSThread isMainThread] || window == NULL) return false;
    return platform_macos_menus_set_option_state(window, action, enabled);
}

bool platform_window_show_ruler_context_menu(platform_window *window,
    bool horizontal_axis, int32_t logical_x, int32_t logical_y)
{
    if (![NSThread isMainThread] || window == NULL) return false;
    return platform_macos_menus_show_ruler_context_menu(window,
        horizontal_axis, logical_x, logical_y);
}
